"""Dataset-loader contract tests for the Post-P3 learn pipeline.

Builds a synthetic leaf corpus in the exact on-disk shape the Rust
`generate_leaf_data` writer emits (per-shard npy tensors + a
`dataset_manifest.jsonl` index) and checks the loader contract:
shard-major flat indexing, declared dtype/shape validation, Welford
scalar stats, and the two-level epoch sampler.

Runs under plain pytest or standalone (`python test_learn_dataset.py`).
Requires numpy — a declared runtime dependency of the wheel.
"""

from __future__ import annotations

import json
import os
import struct
import sys
import tempfile

import numpy as np

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

# Load learn.dataset standalone: the source tree lacks the compiled
# `_engine` extension the package __init__ imports eagerly.
import importlib.util
import types

_pkg = types.ModuleType("koi_maestro")
_pkg.__path__ = [os.path.join(os.path.dirname(__file__), "..", "koi_maestro")]
sys.modules.setdefault("koi_maestro", _pkg)
_lpkg = types.ModuleType("koi_maestro.learn")
_lpkg.__path__ = [
    os.path.join(os.path.dirname(__file__), "..", "koi_maestro", "learn")
]
sys.modules.setdefault("koi_maestro.learn", _lpkg)


def _load(name: str):
    spec = importlib.util.spec_from_file_location(
        f"koi_maestro.learn.{name}",
        os.path.join(
            os.path.dirname(__file__), "..", "koi_maestro", "learn", f"{name}.py"
        ),
    )
    mod = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = mod
    spec.loader.exec_module(mod)
    return mod


dataset = _load("dataset")


def _write_npy(path: str, array: np.ndarray) -> None:
    """Minimal .npy v1.0 writer — same byte contract as the Rust writer."""
    dtype_map = {np.dtype("<f4"): "<f4", np.dtype("|b1"): "|b1", np.dtype("<u8"): "<u8"}
    descr = dtype_map[array.dtype]
    shape = "(" + ", ".join(str(d) for d in array.shape) + ("," if array.ndim == 1 else "") + ")"
    header = f"{{'descr': '{descr}', 'fortran_order': False, 'shape': {shape}, }}"
    header_bytes = header.encode("latin1")
    pad = 16 - ((10 + len(header_bytes) + 1) % 16)
    header_bytes += b" " * pad + b"\n"
    with open(path, "wb") as handle:
        handle.write(b"\x93NUMPY\x01\x00")
        handle.write(struct.pack("<H", len(header_bytes)))
        handle.write(header_bytes)
        handle.write(array.tobytes(order="C"))


def _make_shard(directory: str, rows: int, seed: int) -> None:
    os.makedirs(directory, exist_ok=True)
    rng = np.random.default_rng(seed)
    features = rng.standard_normal((rows, dataset.FEATURE_DIM)).astype("<f4")
    mask = rng.random((rows, dataset.MAX_ACTIONS)) > 0.4
    mask[:, 0] = True
    policy = rng.random((rows, dataset.MAX_ACTIONS)).astype("<f4") * mask
    policy /= policy.sum(axis=1, keepdims=True)
    ev = rng.standard_normal(rows).astype("<f4") * 20.0
    belief = (rng.random((rows, dataset.BELIEF_DIM)) > 0.5).astype("<f4")
    provenance = np.arange(rows, dtype="<u8")
    for name, tensor in (
        ("features", features),
        ("legal_mask", mask),
        ("policy_target", policy),
        ("ev_target", ev),
        ("belief_vector", belief),
        ("provenance", provenance),
    ):
        _write_npy(os.path.join(directory, f"{name}.npy"), tensor)


def _make_dataset(root: str, shards: list[tuple[int, str, int]]) -> None:
    """Each tuple is (shard_id, split, rows)."""
    lines = []
    for shard_id, split, rows in shards:
        directory = f"shard_{shard_id:04d}"
        _make_shard(os.path.join(root, directory), rows, shard_id * 131 + 7)
        lines.append(
            json.dumps(
                {
                    "shard_id": shard_id,
                    "split": split,
                    "generation_iter": 0,
                    "num_samples": rows,
                    "dir": directory,
                    "tensors": [
                        "features.npy",
                        "legal_mask.npy",
                        "policy_target.npy",
                        "ev_target.npy",
                        "belief_vector.npy",
                        "provenance.npy",
                    ],
                    "dataset_hash": "fnv1a64:test",
                }
            )
        )
    with open(os.path.join(root, "dataset_manifest.jsonl"), "w", encoding="utf-8") as handle:
        handle.write("\n".join(lines) + "\n")


def test_flat_indexing_spans_shards_in_order():
    with tempfile.TemporaryDirectory(ignore_cleanup_errors=True) as root:
        _make_dataset(root, [(0, "train", 3), (1, "train", 2), (2, "val", 4)])
        train = dataset.LeafDataset(root, "train")
        assert len(train) == 5
        first_shard_row = train[2]
        second_shard_row = train[3]
        # Row 3 is shard 1's first row — provenance resets per shard.
        assert second_shard_row["features"].shape == (dataset.FEATURE_DIM,)
        assert not np.array_equal(first_shard_row["features"], second_shard_row["features"])
        val = dataset.LeafDataset(root, "val")
        assert len(val) == 4


def test_split_selection_and_missing_split():
    with tempfile.TemporaryDirectory(ignore_cleanup_errors=True) as root:
        _make_dataset(root, [(0, "train", 3)])
        assert len(dataset.LeafDataset(root, "train")) == 3
        try:
            dataset.LeafDataset(root, "val")
            raise AssertionError("missing split must fail closed")
        except ValueError as exc:
            assert "no 'val' shards" in str(exc)


def test_corrupt_shard_fails_closed():
    with tempfile.TemporaryDirectory(ignore_cleanup_errors=True) as root:
        _make_dataset(root, [(0, "train", 3)])
        # Lie about the row count — the loader must catch it on first read.
        with open(os.path.join(root, "dataset_manifest.jsonl"), "w", encoding="utf-8") as handle:
            handle.write(
                json.dumps(
                    {
                        "shard_id": 0,
                        "split": "train",
                        "generation_iter": 0,
                        "num_samples": 4,
                        "dir": "shard_0000",
                        "tensors": [],
                        "dataset_hash": "x",
                    }
                )
                + "\n"
            )
        corrupted = dataset.LeafDataset(root, "train")
        try:
            corrupted[0]
            raise AssertionError("row-count drift must fail closed")
        except ValueError as exc:
            assert "corrupt shard" in str(exc)


def test_scalar_stats_match_numpy_reference():
    with tempfile.TemporaryDirectory(ignore_cleanup_errors=True) as root:
        _make_dataset(root, [(0, "train", 64), (1, "train", 32)])
        ds = dataset.LeafDataset(root, "train")
        mean, std = ds.scalar_stats()
        block = np.concatenate(
            [
                np.asarray(v.column("features"))[:, dataset.SCALAR_START : dataset.SCALAR_END]
                for v in ds.shards
            ]
        )
        np.testing.assert_allclose(mean, block.mean(axis=0), atol=1e-5)
        np.testing.assert_allclose(std, np.maximum(block.std(axis=0), 1e-6), atol=1e-5)


def test_sampler_covers_every_row_and_reseeds_per_epoch():
    with tempfile.TemporaryDirectory(ignore_cleanup_errors=True) as root:
        _make_dataset(root, [(0, "train", 8), (1, "train", 8), (2, "train", 8)])
        ds = dataset.LeafDataset(root, "train")
        sampler = dataset.ShardRowSampler(ds, seed=5)
        epoch0 = list(sampler)
        assert sorted(epoch0) == list(range(len(ds)))
        sampler.set_epoch(1)
        epoch1 = list(sampler)
        assert sorted(epoch1) == list(range(len(ds)))
        assert epoch0 != epoch1
        # Determinism: a fresh sampler at the same seed replays the order.
        assert list(dataset.ShardRowSampler(ds, seed=5)) == epoch0


def test_manifest_rejects_unknown_split():
    with tempfile.TemporaryDirectory(ignore_cleanup_errors=True) as root:
        _make_dataset(root, [(0, "train", 2)])
        with open(os.path.join(root, "dataset_manifest.jsonl"), "a", encoding="utf-8") as handle:
            handle.write(
                json.dumps(
                    {
                        "shard_id": 9,
                        "split": "holdout",
                        "generation_iter": 0,
                        "num_samples": 1,
                        "dir": "shard_0009",
                        "tensors": [],
                        "dataset_hash": "x",
                    }
                )
                + "\n"
            )
        try:
            dataset.load_manifest(root)
            raise AssertionError("unknown split must fail closed")
        except ValueError as exc:
            assert "unknown split" in str(exc)


if __name__ == "__main__":
    failures = 0
    for name, fn in sorted({k: v for k, v in globals().items() if k.startswith("test_")}.items()):
        try:
            fn()
            print(f"PASS {name}")
        except Exception as exc:  # noqa: BLE001 — standalone runner reports all failures
            failures += 1
            print(f"FAIL {name}: {exc}")
    sys.exit(1 if failures else 0)
