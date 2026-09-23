"""Residual-MLP leaf evaluator (Post-P3 architecture contract).

Shared trunk -> value + policy heads:

- input ``f32[B, 384]`` (raw features; scalars are pre-normalized by the
  dataset pipeline) and a legal-action mask ``f32[B, 16]``
- stem ``Linear(384 -> width)`` then ``blocks`` residual blocks of
  ``Linear -> ReLU -> Linear -> +residual -> LayerNorm``
- value head ``Linear(width -> 128) -> ReLU -> Linear(128 -> 1)`` — linear
  output, actor-relative EV margin
- policy head ``Linear(width -> 128) -> ReLU -> Linear(128 -> 16)`` — logits
  over the canonical action order

``forward`` returns raw logits; :class:`ExportableLeafNet` wraps the graph
with masked softmax for ONNX export so the runtime never re-implements the
masking rule.
"""

from __future__ import annotations

import torch
from torch import nn

FEATURE_DIM = 384
MAX_ACTIONS = 16
MASKED_LOGIT = -1e9


class ResidualBlock(nn.Module):
    def __init__(self, width: int) -> None:
        super().__init__()
        self.fc1 = nn.Linear(width, width)
        self.fc2 = nn.Linear(width, width)
        self.norm = nn.LayerNorm(width)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        return self.norm(x + self.fc2(torch.relu(self.fc1(x))))


class LeafNet(nn.Module):
    def __init__(
        self,
        feature_dim: int = FEATURE_DIM,
        width: int = 512,
        blocks: int = 4,
        max_actions: int = MAX_ACTIONS,
        head_width: int = 128,
        dropout: float = 0.0,
    ) -> None:
        super().__init__()
        self.feature_dim = feature_dim
        self.max_actions = max_actions
        self.stem = nn.Linear(feature_dim, width)
        self.blocks = nn.ModuleList(ResidualBlock(width) for _ in range(blocks))
        self.drop = nn.Dropout(dropout) if dropout > 0 else nn.Identity()
        self.value_head = nn.Sequential(
            nn.Linear(width, head_width), nn.ReLU(), nn.Linear(head_width, 1)
        )
        self.policy_head = nn.Sequential(
            nn.Linear(width, head_width), nn.ReLU(), nn.Linear(head_width, max_actions)
        )

    def forward(
        self, features: torch.Tensor, legal_mask: torch.Tensor
    ) -> tuple[torch.Tensor, torch.Tensor]:
        trunk = self.stem(features)
        for block in self.blocks:
            trunk = block(trunk)
        trunk = self.drop(trunk)
        value = self.value_head(trunk).squeeze(-1)
        logits = self.policy_head(trunk)
        return value, logits


def masked_policy_logits(logits: torch.Tensor, legal_mask: torch.Tensor) -> torch.Tensor:
    """Masks illegal actions to ``MASKED_LOGIT`` (the ONNX ``Where`` pattern)."""
    return torch.where(legal_mask > 0.5, logits, torch.full_like(logits, MASKED_LOGIT))


def masked_policy(logits: torch.Tensor, legal_mask: torch.Tensor) -> torch.Tensor:
    """Softmax over legal actions; illegal actions get exactly zero mass."""
    masked = masked_policy_logits(logits, legal_mask)
    return torch.softmax(masked, dim=-1)


class ExportableLeafNet(nn.Module):
    """Export graph: ``(features, legal_mask) -> (value, policy_probs)``.

    Scalar normalization is baked into the graph (Subtract/Div/Clamp over the
    scalar block) so the exported model consumes RAW features — runtimes never
    re-implement the transform and `norm_stats.json` stays a provenance record
    rather than a runtime input. The mask is applied inside the graph for the
    same reason; a state with no legal actions yields a uniform row rather
    than NaNs.
    """

    SCALAR_START = 288
    SCALAR_END = 384

    def __init__(
        self,
        net: LeafNet,
        mean: torch.Tensor | None = None,
        std: torch.Tensor | None = None,
        clip: float = 8.0,
    ) -> None:
        super().__init__()
        self.net = net
        scalar = self.SCALAR_END - self.SCALAR_START
        self.register_buffer("scalar_mean", mean if mean is not None else torch.zeros(scalar))
        self.register_buffer("scalar_std", std if std is not None else torch.ones(scalar))
        self.clip = clip

    def forward(
        self, features: torch.Tensor, legal_mask: torch.Tensor
    ) -> tuple[torch.Tensor, torch.Tensor]:
        normed = features.clone()
        scalars = (
            features[..., self.SCALAR_START : self.SCALAR_END] - self.scalar_mean
        ) / self.scalar_std
        normed[..., self.SCALAR_START : self.SCALAR_END] = scalars.clamp(-self.clip, self.clip)
        value, logits = self.net(normed, legal_mask)
        probs = masked_policy(logits, legal_mask)
        has_legal = legal_mask.sum(dim=-1, keepdim=True) > 0
        uniform = torch.full_like(probs, 1.0 / probs.shape[-1])
        return value, torch.where(has_legal, probs, uniform)
