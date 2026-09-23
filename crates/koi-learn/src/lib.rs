//! `koi-learn` — the offline data pipeline for the learned leaf evaluator.
//!
//! Generates sharded `.npy` corpora: features/masks/policy/ev/belief rows
//! labelled by a CFR+ blueprint (reduced variants) or a per-row root
//! resolve (canonical), plus per-shard provenance manifests.

pub mod learn;
