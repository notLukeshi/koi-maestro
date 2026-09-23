"""Training loop for the leaf evaluator (Post-P3).

``python -m koi_maestro.learn.train --data data/leaves-nano_6 --out runs/n6``

Implements the training contract:

- AdamW + 5% linear warmup -> cosine decay to 1e-6, grad clip 1.0
- BF16 autocast (no GradScaler; CPU autocast is supported for smoke runs and
  the production target is CUDA); ``--fp32`` for debugging
- EMA weights (``--ema-tau``, default 1e-3) — the EMA copy is what gets
  exported and early-stopped on
- Huber(delta=1.0) value loss + masked cross-entropy with label smoothing on
  the policy target, weighted by ``--value-weight``/``--policy-weight``
- Temporal split honored: the dataset's ``val``/``test`` shards come from
  later generation than ``train`` — no reshuffling across the boundary
- Checkpoints: best weights as ``model.safetensors`` (never a pickled
  module), optimizer/scheduler/EMA state as ``training_state.pt``
- ``norm_stats.json``: Welford mean/std over the scalar block, recorded for
  provenance — normalization is baked into the exported ONNX graph, so the
  Rust evaluator applies it in-graph (the sidecar is not read at inference)
"""

from __future__ import annotations

import argparse
import json
import math
import os
import sys
from dataclasses import dataclass, field
from typing import TYPE_CHECKING

import numpy as np

if TYPE_CHECKING:
    import torch

from .dataset import (
    SCALAR_END,
    SCALAR_START,
    LeafDataset,
    ShardRowSampler,
    collate_rows,
)


@dataclass
class TrainConfig:
    data: str
    out: str = "runs/leaf"
    epochs: int = 30
    batch_size: int = 1024
    lr: float = 2e-4
    weight_decay: float = 3e-4
    warmup_frac: float = 0.05
    min_lr: float = 1e-6
    clip: float = 1.0
    ema_tau: float = 1e-3
    value_weight: float = 1.0
    policy_weight: float = 1.0
    label_smoothing: float = 0.05
    huber_delta: float = 1.0
    patience: int = 6
    width: int = 512
    blocks: int = 4
    dropout: float = 0.0
    bf16: bool = True
    seed: int = 17
    workers: int = 4
    device: str = "cpu"
    extra: dict[str, object] = field(default_factory=dict)


def _build_loader(
    dataset: LeafDataset, cfg: TrainConfig, sampler: ShardRowSampler | None, drop_last: bool = True
):
    import torch

    kwargs: dict[str, object] = {}
    if cfg.workers > 0:
        ctx = torch.multiprocessing.get_context("spawn")
        kwargs = {
            "num_workers": cfg.workers,
            "multiprocessing_context": ctx,
            "persistent_workers": True,
            "prefetch_factor": 2,
            "pin_memory": True,
        }
    return torch.utils.data.DataLoader(
        dataset,
        batch_size=cfg.batch_size,
        shuffle=False,  # order comes entirely from the two-level sampler
        sampler=sampler,
        collate_fn=collate_rows,
        drop_last=drop_last,
        **kwargs,
    )


def _normalizer(stats: np.ndarray, device: torch.device):
    """Returns a closure mapping raw features -> normalized features.

    The card blocks pass through; the scalar block is standardized.
    Everything stays in torch so the same transform runs identically on GPU
    batches. Stats move to the device once — a per-batch H2D copy would
    serialize the loader's pinned-memory pipeline.
    """
    import torch

    mean = torch.from_numpy(stats[0]).float().to(device)
    std = torch.from_numpy(stats[1]).float().to(device)

    def apply(features: torch.Tensor) -> torch.Tensor:
        out = features.clone()
        out[..., SCALAR_START:SCALAR_END] = (features[..., SCALAR_START:SCALAR_END] - mean) / std
        return out.clamp_(-8.0, 8.0)

    return apply


def _write_norm_stats(stats: np.ndarray, out_dir: str) -> None:
    payload = {
        "schema_version": 1,
        "scalar_start": SCALAR_START,
        "scalar_end": SCALAR_END,
        "clip": 8.0,
        "mean": stats[0].astype(np.float32).tolist(),
        "std": stats[1].astype(np.float32).tolist(),
    }
    with open(os.path.join(out_dir, "norm_stats.json"), "w", encoding="utf-8") as handle:
        json.dump(payload, handle)


def _policy_loss(logits, mask, target, smoothing):
    """Masked cross-entropy with label smoothing spread over legal actions."""
    import torch
    import torch.nn.functional as F

    from .model import MASKED_LOGIT

    masked = torch.where(mask > 0.5, logits, torch.full_like(logits, MASKED_LOGIT))
    logp = F.log_softmax(masked, dim=-1)
    n_legal = mask.sum(dim=-1, keepdim=True).clamp(min=1.0)
    smooth = mask * (smoothing / n_legal)
    target = target * (1.0 - smoothing) + smooth
    return -(target * logp).sum(dim=-1).mean()


def _evaluate(model, loader, normalize, cfg, device) -> dict[str, float]:
    import torch
    import torch.nn.functional as F

    model.eval()
    totals = {"value": 0.0, "policy": 0.0, "n": 0}
    with torch.no_grad():
        for batch in loader:
            features = normalize(batch["features"].to(device, non_blocking=True))
            mask = batch["legal_mask"].to(device, non_blocking=True)
            value, logits = model(features, mask)
            totals["value"] += F.huber_loss(
                value,
                batch["ev_target"].to(device, non_blocking=True),
                delta=cfg.huber_delta,
                reduction="sum",
            ).item()
            totals["policy"] += (
                _policy_loss(
                    logits,
                    mask,
                    batch["policy_target"].to(device, non_blocking=True),
                    cfg.label_smoothing,
                ).item()
                * features.shape[0]
            )
            totals["n"] += features.shape[0]
    n = max(totals["n"], 1)
    return {
        "value_huber": totals["value"] / n,
        "policy_ce": totals["policy"] / n,
        "combined": cfg.value_weight * totals["value"] / n
        + cfg.policy_weight * totals["policy"] / n,
    }


def train(cfg: TrainConfig) -> dict[str, object]:
    import torch
    from safetensors.torch import save_file

    from .model import LeafNet

    torch.manual_seed(cfg.seed)
    np.random.seed(cfg.seed)
    os.makedirs(cfg.out, exist_ok=True)
    device = torch.device(cfg.device)
    device_name = torch.cuda.get_device_name(device) if device.type == "cuda" else "cpu"
    print(f"device: {device} ({device_name})", file=sys.stderr, flush=True)

    train_set = LeafDataset(cfg.data, "train")
    val_set = LeafDataset(cfg.data, "val") if _has_split(cfg.data, "val") else None
    stats = train_set.scalar_stats()
    _write_norm_stats(stats, cfg.out)
    normalize = _normalizer(stats, device)

    sampler = ShardRowSampler(train_set, cfg.seed)
    train_loader = _build_loader(train_set, cfg, sampler)
    val_loader = _build_loader(val_set, cfg, None, drop_last=False) if val_set is not None else None

    model = LeafNet(width=cfg.width, blocks=cfg.blocks, dropout=cfg.dropout).to(device)
    ema = LeafNet(width=cfg.width, blocks=cfg.blocks, dropout=cfg.dropout).to(device)
    ema.load_state_dict(model.state_dict())
    for p in ema.parameters():
        p.requires_grad_(False)

    optimizer = torch.optim.AdamW(model.parameters(), lr=cfg.lr, weight_decay=cfg.weight_decay)
    steps_per_epoch = max(len(train_loader), 1)
    total_steps = steps_per_epoch * cfg.epochs
    warmup_steps = max(1, int(total_steps * cfg.warmup_frac))

    def lr_at(step: int) -> float:
        if step < warmup_steps:
            return cfg.lr * (step + 1) / warmup_steps
        progress = (step - warmup_steps) / max(total_steps - warmup_steps, 1)
        return cfg.min_lr + 0.5 * (cfg.lr - cfg.min_lr) * (1.0 + math.cos(math.pi * progress))

    use_bf16 = cfg.bf16 and device.type in ("cpu", "cuda")
    autocast = torch.autocast(device_type=device.type, dtype=torch.bfloat16, enabled=use_bf16)

    best_val = math.inf
    epochs_without_gain = 0
    step = 0
    history = []
    import torch.nn.functional as F

    for epoch in range(cfg.epochs):
        model.train()
        sampler.set_epoch(epoch)
        for group in optimizer.param_groups:
            group["lr"] = lr_at(step)
        for batch in train_loader:
            features = normalize(batch["features"].to(device, non_blocking=True))
            mask = batch["legal_mask"].to(device, non_blocking=True)
            with autocast:
                value, logits = model(features, mask)
                v_loss = F.huber_loss(
                    value.float(),
                    batch["ev_target"].to(device, non_blocking=True),
                    delta=cfg.huber_delta,
                )
                p_loss = _policy_loss(
                    logits.float(),
                    mask,
                    batch["policy_target"].to(device, non_blocking=True),
                    cfg.label_smoothing,
                )
                loss = cfg.value_weight * v_loss + cfg.policy_weight * p_loss
            optimizer.zero_grad(set_to_none=True)
            loss.backward()
            torch.nn.utils.clip_grad_norm_(model.parameters(), cfg.clip)
            for group in optimizer.param_groups:
                group["lr"] = lr_at(step)
            optimizer.step()
            step += 1
            with torch.no_grad():
                for ema_p, p in zip(ema.parameters(), model.parameters()):
                    ema_p.lerp_(p, cfg.ema_tau)
                for ema_b, b in zip(ema.buffers(), model.buffers()):
                    ema_b.copy_(b)

        if val_loader is None:
            # No held-out val shards (small dev corpora are train/test only):
            # checkpoint the EMA weights every epoch so artifacts always exist.
            save_file(ema.state_dict(), os.path.join(cfg.out, "model.safetensors"))
            torch.save(
                {"epoch": epoch, "optimizer": _cpu_state(optimizer.state_dict())},
                os.path.join(cfg.out, "training_state.pt"),
            )
            continue
        metrics = _evaluate(ema, val_loader, normalize, cfg, device)
        history.append({"epoch": epoch, **metrics})
        print(
            f"epoch {epoch}: val huber={metrics['value_huber']:.5f} "
            f"ce={metrics['policy_ce']:.5f} combined={metrics['combined']:.5f}",
            file=sys.stderr,
        )
        if metrics["combined"] < best_val - 1e-5:
            best_val = metrics["combined"]
            epochs_without_gain = 0
            save_file(ema.state_dict(), os.path.join(cfg.out, "model.safetensors"))
            torch.save(
                _cpu_state(
                    {
                        "epoch": epoch,
                        "optimizer": optimizer.state_dict(),
                        "ema": dict(ema.state_dict()),
                        "val_combined": best_val,
                        "config": {k: v for k, v in vars(cfg).items() if k != "extra"},
                    }
                ),
                os.path.join(cfg.out, "training_state.pt"),
            )
        else:
            epochs_without_gain += 1
            if epochs_without_gain >= cfg.patience:
                break

    summary = {
        "best_val_combined": best_val if math.isfinite(best_val) else None,
        "epochs_ran": len(history),
        "history": history,
        "device": {
            "device": str(device),
            "name": device_name,
            "torch": torch.__version__,
            "cuda": torch.version.cuda,
        },
        "config": {k: v for k, v in vars(cfg).items() if k != "extra"},
    }
    with open(os.path.join(cfg.out, "train_summary.json"), "w", encoding="utf-8") as handle:
        json.dump(summary, handle, indent=1)
    return summary


def _cpu_state(obj):
    """Move every tensor in a nested state dict to CPU — keeps
    training_state.pt loadable on CPU-only hosts (map_location='cpu'
    contract)."""
    import torch

    if torch.is_tensor(obj):
        return obj.detach().cpu()
    if isinstance(obj, dict):
        return {k: _cpu_state(v) for k, v in obj.items()}
    if isinstance(obj, (list, tuple)):
        return type(obj)(_cpu_state(v) for v in obj)
    return obj


def _has_split(data_dir: str, split: str) -> bool:
    from .dataset import load_manifest

    return any(r.split == split for r in load_manifest(data_dir))


def _cuda_available() -> bool:
    try:
        import torch

        return torch.cuda.is_available()
    except (ImportError, OSError, RuntimeError):
        return False


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description="Train the Koi-Koi leaf evaluator")
    parser.add_argument(
        "--data", required=True, help="dataset dir containing dataset_manifest.jsonl"
    )
    parser.add_argument("--out", default="runs/leaf")
    parser.add_argument("--epochs", type=int, default=30)
    parser.add_argument("--batch-size", type=int, default=1024)
    parser.add_argument("--lr", type=float, default=2e-4)
    parser.add_argument("--weight-decay", type=float, default=3e-4)
    parser.add_argument("--ema-tau", type=float, default=1e-3)
    parser.add_argument("--value-weight", type=float, default=1.0)
    parser.add_argument("--policy-weight", type=float, default=1.0)
    parser.add_argument("--label-smoothing", type=float, default=0.05)
    parser.add_argument("--patience", type=int, default=6)
    parser.add_argument("--width", type=int, default=512)
    parser.add_argument("--blocks", type=int, default=4)
    parser.add_argument("--dropout", type=float, default=0.0)
    parser.add_argument("--fp32", action="store_true", help="disable BF16 autocast (debug)")
    parser.add_argument("--seed", type=int, default=17)
    parser.add_argument("--workers", type=int, default=4)
    parser.add_argument(
        "--device",
        default="cuda" if _cuda_available() else "cpu",
        help="torch device — auto-detects CUDA (production target), pass 'cpu' to force",
    )
    args = parser.parse_args(argv)

    cfg = TrainConfig(
        data=args.data,
        out=args.out,
        epochs=args.epochs,
        batch_size=args.batch_size,
        lr=args.lr,
        weight_decay=args.weight_decay,
        ema_tau=args.ema_tau,
        value_weight=args.value_weight,
        policy_weight=args.policy_weight,
        label_smoothing=args.label_smoothing,
        patience=args.patience,
        width=args.width,
        blocks=args.blocks,
        dropout=args.dropout,
        bf16=not args.fp32,
        seed=args.seed,
        workers=args.workers,
        device=args.device,
    )
    train(cfg)
    return 0


if __name__ == "__main__":
    sys.exit(main())
