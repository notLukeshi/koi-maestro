"""Sharded-npy dataset over Post-P3 leaf corpora.

A leaf dataset directory contains ``dataset_manifest.jsonl`` (one record per
shard, each carrying ``dir``, ``num_samples`` and a ``split`` of
``train``/``val``/``test``) plus per-shard ``.npy`` tensors. Shards are loaded
with ``np.load(..., mmap_mode="r")`` so workers read pages on demand instead of
materializing the corpus.

Sampling follows the two-level scheme: permute shard order, then permute rows
inside each shard. That keeps every epoch sequential-friendly (whole shards
are consumed before moving on) while decorrelating row order — a full uniform
permutation over one giant memmap is deliberately avoided.
"""

from __future__ import annotations

import bisect
import json
import os
from collections.abc import Iterator, Sequence
from dataclasses import dataclass

import numpy as np

FEATURE_DIM = 384
MAX_ACTIONS = 16
BELIEF_DIM = 48
# Scalars live at 288..384; the six card blocks (0..288) are consumed raw,
# per the leaf feature contract in koi-solver's features.rs.
SCALAR_START = 288
SCALAR_END = 384

_SPLITS = ("train", "val", "test")

# Tensors the loader surfaces; ``provenance`` is left on disk (row-level
# bookkeeping, not a model input).
TENSORS = ("features", "legal_mask", "policy_target", "ev_target", "belief_vector")

# The tensor contract the Rust writer emits (learn/npy.rs + features.rs):
# dtype and per-row trailing shape. Checked on first open so a truncated or
# dtype-swapped shard fails at load time, not mid-epoch inside a worker.
_EXPECTED_TENSORS = {
    "features": (np.dtype("<f4"), (FEATURE_DIM,)),
    "legal_mask": (np.dtype("|b1"), (MAX_ACTIONS,)),
    "policy_target": (np.dtype("<f4"), (MAX_ACTIONS,)),
    "ev_target": (np.dtype("<f4"), ()),
    "belief_vector": (np.dtype("<f4"), (BELIEF_DIM,)),
}


@dataclass(frozen=True)
class ShardRecord:
    shard_id: int
    split: str
    num_samples: int
    directory: str


def load_manifest(dataset_dir: str) -> list[ShardRecord]:
    """Reads ``dataset_manifest.jsonl``; fails closed on malformed records."""
    path = os.path.join(dataset_dir, "dataset_manifest.jsonl")
    records: list[ShardRecord] = []
    with open(path, "r", encoding="utf-8") as handle:
        for line_no, line in enumerate(handle, 1):
            line = line.strip()
            if not line:
                continue
            try:
                record = json.loads(line)
                shard = ShardRecord(
                    shard_id=int(record["shard_id"]),
                    split=str(record["split"]),
                    num_samples=int(record["num_samples"]),
                    directory=str(record["dir"]),
                )
            except (KeyError, TypeError, ValueError) as exc:
                raise ValueError(f"malformed manifest line {line_no} in {path}: {exc}") from exc
            if shard.split not in _SPLITS:
                raise ValueError(f"manifest line {line_no}: unknown split {shard.split!r}")
            records.append(shard)
    if not records:
        raise ValueError(f"empty dataset manifest: {path}")
    return records


class ShardView:
    """Memmap-backed tensors for one shard. Opened lazily per worker."""

    __slots__ = ("_dir", "_num_samples", "_tensors")

    def __init__(self, directory: str, num_samples: int) -> None:
        self._dir = directory
        self._num_samples = num_samples
        self._tensors: dict[str, np.ndarray] | None = None

    def _open(self) -> dict[str, np.ndarray]:
        if self._tensors is None:
            tensors = {
                name: np.load(os.path.join(self._dir, f"{name}.npy"), mmap_mode="r")
                for name in TENSORS
            }
            for name, tensor in tensors.items():
                dtype, tail = _EXPECTED_TENSORS[name]
                if (
                    tensor.dtype != dtype
                    or tuple(tensor.shape[1:]) != tail
                    or tensor.shape[0] != self._num_samples
                ):
                    raise ValueError(
                        f"corrupt shard {self._dir}/{name}.npy: "
                        f"dtype {tensor.dtype} (want {dtype}), shape {tuple(tensor.shape)} "
                        f"(want ({self._num_samples}, {', '.join(map(str, tail))}))"
                    )
            self._tensors = tensors
        return self._tensors

    def row(self, index: int) -> dict[str, np.ndarray]:
        tensors = self._open()
        return {name: np.asarray(tensor[index]) for name, tensor in tensors.items()}

    def column(self, name: str) -> np.ndarray:
        return self._open()[name]


class LeafDataset:
    """Flat row view over the shards of one split.

    ``__getitem__`` takes a flat index in shard-major order
    (``flat = cumulative_rows[shard] + row``) and returns a dict of numpy
    arrays; the collate step in :mod:`train` converts to tensors. Memmaps are
    opened lazily so each DataLoader worker opens its own handles.
    """

    def __init__(self, dataset_dir: str, split: str = "train") -> None:
        if split not in _SPLITS:
            raise ValueError(f"unknown split {split!r}")
        records = [r for r in load_manifest(dataset_dir) if r.split == split]
        if not records:
            raise ValueError(f"no {split!r} shards under {dataset_dir}")
        self.shards: list[ShardView] = []
        self._bounds: list[int] = []  # cumulative row counts
        total = 0
        for record in sorted(records, key=lambda r: r.shard_id):
            self.shards.append(
                ShardView(os.path.join(dataset_dir, record.directory), record.num_samples)
            )
            total += record.num_samples
            self._bounds.append(total)
        self._total = total

    def __len__(self) -> int:
        return self._total

    def __getitem__(self, index: int) -> dict[str, np.ndarray]:
        if index < 0 or index >= self._total:
            raise IndexError(index)
        shard = bisect.bisect_right(self._bounds, index)
        row = index - (self._bounds[shard - 1] if shard else 0)
        return self.shards[shard].row(row)

    def scalar_stats(self) -> np.ndarray:
        """Welford mean/std over the scalar block, streamed shard by shard."""
        count = 0
        mean = np.zeros(SCALAR_END - SCALAR_START, dtype=np.float64)
        m2 = np.zeros_like(mean)
        for view in self.shards:
            block = np.asarray(
                view.column("features")[:, SCALAR_START:SCALAR_END], dtype=np.float64
            )
            n = block.shape[0]
            batch_mean = block.mean(axis=0)
            batch_m2 = block.var(axis=0) * n
            delta = batch_mean - mean
            mean += delta * (n / (count + n))
            m2 += batch_m2 + delta * delta * (count * n / (count + n))
            count += n
        std = np.sqrt(m2 / np.maximum(count, 1))
        std[std < 1e-6] = 1.0  # constant dims must not amplify noise
        return np.stack([mean, std])  # (2, SCALAR_DIM)


class ShardRowSampler:
    """Two-level permutation: shuffled shard order, shuffled rows per shard.

    Yields flat shard-major indices. ``set_epoch`` reseeds so every epoch sees
    a different, deterministic interleave.
    """

    def __init__(self, dataset: LeafDataset, seed: int) -> None:
        self._dataset = dataset
        self._seed = seed
        self._epoch = 0

    def set_epoch(self, epoch: int) -> None:
        self._epoch = epoch

    def __len__(self) -> int:
        return len(self._dataset)

    def __iter__(self) -> Iterator[int]:
        rng = np.random.default_rng((self._seed, self._epoch))
        bounds = [0, *self._dataset._bounds]
        shard_order = rng.permutation(len(self._dataset.shards))
        for shard in shard_order:
            base = bounds[shard]
            for row in rng.permutation(bounds[shard + 1] - base):
                yield int(base + row)


def collate_rows(rows: Sequence[dict[str, np.ndarray]]) -> dict[str, object]:
    """Stacks a list of row dicts into torch tensors (torch imported lazily)."""
    import torch

    features = torch.from_numpy(np.stack([r["features"] for r in rows])).float()
    mask = torch.from_numpy(np.stack([r["legal_mask"] for r in rows])).float()
    policy = torch.from_numpy(np.stack([r["policy_target"] for r in rows])).float()
    ev = torch.from_numpy(np.stack([r["ev_target"] for r in rows])).float()
    return {"features": features, "legal_mask": mask, "policy_target": policy, "ev_target": ev}
