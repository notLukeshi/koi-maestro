"""Post-training calibration on the validation split (Post-P3).

- Policy temperature: scalar T fitted by minimizing masked cross-entropy of
  ``softmax(logits / T)`` against the policy target over the ``val`` split —
  the ``test`` split stays untouched as the final holdout.
- EV affine: least-squares ``ev ≈ a * pred + b`` over the same split, plus a
  Huber-variant report. Calibration is a *measurement* — the exported ONNX
  graph is unchanged; the scalars land in ``calibration.json`` next to the
  model for consumers that want calibrated values.

``python -m koi_maestro.learn.calibrate --data data/leaves-nano_6 --run runs/n6``
"""

from __future__ import annotations

import argparse
import json
import os
import sys

import numpy as np

from .dataset import SCALAR_END, SCALAR_START, LeafDataset


def _batches(dataset: LeafDataset, batch_size: int):
    for start in range(0, len(dataset), batch_size):
        rows = [dataset[i] for i in range(start, min(start + batch_size, len(dataset)))]
        features = np.stack([r["features"] for r in rows])
        mask = np.stack([r["legal_mask"] for r in rows])
        policy = np.stack([r["policy_target"] for r in rows])
        ev = np.stack([r["ev_target"] for r in rows])
        yield features, mask, policy, ev


def _load_module(run_dir: str):
    from .export import _load_safetensors  # same loader the exporter uses

    return _load_safetensors(os.path.join(run_dir, "model.safetensors")).eval()


def calibrate(data: str, run_dir: str, batch_size: int = 512) -> dict[str, float]:
    import torch
    import torch.nn.functional as F

    module = _load_module(run_dir)
    eval_set = LeafDataset(data, "val")
    device = next(module.parameters()).device

    # Reproduce the export graph's normalization with the run's stats —
    # loaded once, not per batch. The LeafNet takes normalized inputs (the
    # normalization lives inside ExportableLeafNet in the exported graph).
    stats_path = os.path.join(run_dir, "norm_stats.json")
    stats = None
    if os.path.exists(stats_path):
        with open(stats_path, "r", encoding="utf-8") as handle:
            raw = json.load(handle)
        stats = (
            torch.tensor(raw["mean"], dtype=torch.float32, device=device),
            torch.tensor(raw["std"], dtype=torch.float32, device=device),
            float(raw.get("clip", 8.0)),
        )

    logits_all, values_all, masks_all, policies_all, evs_all = [], [], [], [], []
    with torch.no_grad():
        for features, mask, policy, ev in _batches(eval_set, batch_size):
            f = torch.from_numpy(features).float().to(device)
            m = torch.from_numpy(mask).float().to(device)
            if stats is not None:
                mean, std, clip = stats
                scalars = (f[:, SCALAR_START:SCALAR_END] - mean) / std
                f = f.clone()
                f[:, SCALAR_START:SCALAR_END] = scalars.clamp(-clip, clip)
            value, logits = module.net(f, m)
            logits_all.append(logits.float().cpu())
            values_all.append(value.float().cpu())
            masks_all.append(torch.from_numpy(mask))
            policies_all.append(torch.from_numpy(policy))
            evs_all.append(torch.from_numpy(ev))

    logits = torch.cat(logits_all)
    values = torch.cat(values_all)
    masks = torch.cat(masks_all)
    policies = torch.cat(policies_all)
    evs = torch.cat(evs_all)

    # Temperature by masked CE minimization (single scalar — Newton-free
    # grid + refine is robust and dependency-free).
    def masked_ce(temperature: float) -> float:
        scaled = torch.where(masks > 0.5, logits / temperature, torch.full_like(logits, -1e9))
        logp = F.log_softmax(scaled, dim=-1)
        return float(-(policies * logp).sum(dim=-1).mean())

    candidates = np.linspace(0.5, 4.0, 36)
    temperature = float(min(candidates, key=lambda t: masked_ce(float(t))))
    for span in (0.1, 0.02):
        lo, hi = temperature - span * 5, temperature + span * 5
        candidates = np.linspace(max(lo, 0.05), hi, 21)
        temperature = float(min(candidates, key=lambda t: masked_ce(float(t))))

    # EV affine: least squares over the validation split.
    a_num = float(((values - values.mean()) * (evs - evs.mean())).sum())
    a_den = float(((values - values.mean()) ** 2).sum())
    slope = a_num / a_den if a_den > 0 else 1.0
    intercept = float(evs.mean() - slope * values.mean())
    calibrated = slope * values + intercept
    huber = float(F.huber_loss(calibrated, evs, delta=1.0))
    raw_huber = float(F.huber_loss(values, evs, delta=1.0))

    report = {
        "temperature": temperature,
        "ev_slope": slope,
        "ev_intercept": intercept,
        "val_rows": len(evs),
        "raw_value_huber": raw_huber,
        "calibrated_value_huber": huber,
    }
    with open(os.path.join(run_dir, "calibration.json"), "w", encoding="utf-8") as handle:
        json.dump(report, handle, indent=1)
    return report


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(
        description="Calibrate the leaf evaluator on the validation split"
    )
    parser.add_argument("--data", required=True)
    parser.add_argument("--run", required=True)
    args = parser.parse_args(argv)
    report = calibrate(args.data, args.run)
    print(json.dumps(report, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
