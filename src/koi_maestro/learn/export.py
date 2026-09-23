"""ONNX export + runtime parity verification (Post-P3).

``python -m koi_maestro.learn.export --run runs/leaf --out model.onnx``

- Exports :class:`ExportableLeafNet` via ``torch.onnx.export`` at opset 21
  with a dynamic batch dimension (the runtime batches leaf evaluations).
- Verifies the exported graph against the torch reference with ONNX
  Runtime at the tolerances: value ``atol=1e-5, rtol=1e-3``; policy
  ``atol=1e-4`` — at batch sizes 64 and 128.
- Writes ``golden.json`` — a fixed input batch and the expected outputs —
  consumed by the Rust-side parity test so the Rust backend is checked
  against the same numbers the Python export was.

FP32 export only: FP16 hurts Intel-CPU inference and INT8 can slow small
MLPs; quantization is deferred until profiling says otherwise.
"""

from __future__ import annotations

import argparse
import json
import os
import sys

import numpy as np

from .dataset import FEATURE_DIM, MAX_ACTIONS

OPSET = 20
VALUE_ATOL = 1e-5
VALUE_RTOL = 1e-3
POLICY_ATOL = 1e-4


def _load_safetensors(path: str):
    import torch
    from safetensors.torch import load_file

    from .model import ExportableLeafNet, LeafNet

    run_dir = os.path.dirname(path)
    summary_path = os.path.join(run_dir, "train_summary.json")
    kwargs: dict[str, int] = {}
    if os.path.exists(summary_path):
        with open(summary_path, "r", encoding="utf-8") as handle:
            saved = json.load(handle).get("config", {})
        kwargs = {k: int(saved[k]) for k in ("width", "blocks") if k in saved}
    state = load_file(path)
    net = LeafNet(**kwargs)
    net.load_state_dict(state)
    net.eval()

    # Bake the scalar normalizer into the graph when the sidecar exists.
    stats_path = os.path.join(run_dir, "norm_stats.json")
    if os.path.exists(stats_path):
        with open(stats_path, "r", encoding="utf-8") as handle:
            stats = json.load(handle)
        mean = torch.tensor(stats["mean"], dtype=torch.float32)
        std = torch.tensor(stats["std"], dtype=torch.float32)
        clip = float(stats.get("clip", 8.0))
        return ExportableLeafNet(net, mean=mean, std=std, clip=clip)
    return ExportableLeafNet(net)


def export_onnx(run_dir: str, out_path: str, opset: int = OPSET) -> str:
    """Exports ``run_dir/model.safetensors`` to ONNX; returns the path."""
    import onnx
    import torch

    module = _load_safetensors(os.path.join(run_dir, "model.safetensors")).eval()
    features = torch.zeros(2, FEATURE_DIM)
    mask = torch.ones(2, MAX_ACTIONS)
    batch = torch.export.Dim("batch")
    torch.onnx.export(
        module,
        (features, mask),
        out_path,
        input_names=["features", "legal_mask"],
        output_names=["value", "policy"],
        dynamic_shapes={"features": {0: batch}, "legal_mask": {0: batch}},
        opset_version=opset,
        dynamo=True,
    )
    # The dynamo exporter produces IR 10, but the Rust `ort` crate's
    # bundled ONNX Runtime only supports IR <= 9. Downgrade in-place.
    model = onnx.load(out_path)
    if model.ir_version > 9:
        model.ir_version = 9
        onnx.save(model, out_path)
    return out_path


def verify_parity(onnx_path: str, run_dir: str) -> dict[str, float]:
    """Compares ORT outputs against the torch reference at batches 64/128."""
    import onnxruntime as ort
    import torch

    module = _load_safetensors(os.path.join(run_dir, "model.safetensors")).eval()
    session = ort.InferenceSession(onnx_path, providers=["CPUExecutionProvider"])

    rng = np.random.default_rng(0)
    report: dict[str, float] = {}
    for batch in (64, 128):
        features = rng.standard_normal((batch, FEATURE_DIM)).astype(np.float32)
        mask = (rng.random((batch, MAX_ACTIONS)) > 0.5).astype(np.float32)
        mask[:, 0] = 1.0  # guarantee at least one legal action
        with torch.no_grad():
            ref_v, ref_p = module(torch.from_numpy(features), torch.from_numpy(mask))
        out_v, out_p = session.run(None, {"features": features, "legal_mask": mask})
        v_ok = np.allclose(out_v, ref_v.numpy(), atol=VALUE_ATOL, rtol=VALUE_RTOL)
        p_ok = np.allclose(out_p, ref_p.numpy(), atol=POLICY_ATOL)
        report[f"batch{batch}_value_max_abs"] = float(np.abs(out_v - ref_v.numpy()).max())
        report[f"batch{batch}_policy_max_abs"] = float(np.abs(out_p - ref_p.numpy()).max())
        if not (v_ok and p_ok):
            raise AssertionError(f"ORT parity failed at batch {batch}: {report}")
    return report


def write_golden(onnx_path: str, out_dir: str) -> str:
    """Emits ``golden.json`` — deterministic inputs + ORT outputs for the
    Rust-side parity test to check against."""
    import onnxruntime as ort

    session = ort.InferenceSession(onnx_path, providers=["CPUExecutionProvider"])
    rng = np.random.default_rng(1234)
    features = rng.standard_normal((4, FEATURE_DIM)).astype(np.float32)
    mask = (rng.random((4, MAX_ACTIONS)) > 0.5).astype(np.float32)
    mask[:, 0] = 1.0
    value, policy = session.run(None, {"features": features, "legal_mask": mask})
    payload = {
        "schema_version": 1,
        "feature_dim": FEATURE_DIM,
        "max_actions": MAX_ACTIONS,
        "features": features.tolist(),
        "legal_mask": mask.tolist(),
        "value": np.asarray(value).tolist(),
        "policy": np.asarray(policy).tolist(),
    }
    path = os.path.join(out_dir, "golden.json")
    with open(path, "w", encoding="utf-8") as handle:
        json.dump(payload, handle)
    return path


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description="Export the leaf evaluator to ONNX")
    parser.add_argument("--run", required=True, help="run dir containing model.safetensors")
    parser.add_argument("--out", default=None, help="onnx path (default: <run>/model.onnx)")
    parser.add_argument("--opset", type=int, default=OPSET)
    parser.add_argument("--golden", action="store_true", help="also write golden.json fixture")
    args = parser.parse_args(argv)

    out = args.out or os.path.join(args.run, "model.onnx")
    export_onnx(args.run, out, args.opset)
    report = verify_parity(out, args.run)
    print(f"parity: {report}")
    if args.golden:
        print(f"golden: {write_golden(out, args.run)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
