//! Provenance manifests for generated dataset shards.
//!
//! Every shard carries a `manifest.json` with the full generation context —
//! build identity, solver settings, seed schedule, tensor digests — and the
//! run appends one line per shard to `dataset_manifest.jsonl` at the output
//! root. Digests use the repository's established `fnv1a64:` convention
//! (the same one benchmark artifacts attest with); the requirement is a
//! stable, tamper-evident digest, and reusing the benchmark's hashing keeps
//! one convention across the repo.
//!
//! Ported from a sibling research codebase's `learn/manifest.rs` — the bucket fields
//! carry koi's positional taxonomy (stop calls, yaku pressure, endgame).

use serde::Serialize;

/// Dataset schema version — bump when the tuple layout changes.
pub const SCHEMA_VERSION: u32 = 1;

/// Which temporal split a shard belongs to. Splits are assigned by shard
/// order (earliest ~80% train, next ~10% validation, final ~10% test) — a
/// temporal split, never a random one, so future strategy epochs cannot
/// leak into the training set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Split {
    Train,
    /// The Python loader accepts only `"train" | "val" | "test"` — the
    /// serde name must stay `"val"`.
    #[serde(rename = "val")]
    Validation,
    Test,
}

/// Assigns the temporal split for `shard` out of `total` shards — ~80%
/// train, ~10% validation, ~10% test for large runs. Runs of 4+ shards
/// always hold out one validation and one test shard; runs of 3 shards
/// hold out a test shard but cannot validate; runs under 3 shards are
/// degenerate dev runs where everything trains. A usable corpus
/// (train+val+test) therefore needs `total >= 4`.
pub fn split_for(shard: usize, total: usize) -> Split {
    if total < 4 {
        return if total == 3 && shard == 2 {
            Split::Test
        } else {
            Split::Train
        };
    }
    let train_end = ((total * 8) / 10).clamp(1, total - 2);
    let val_end = ((total * 9) / 10).clamp(train_end + 1, total - 1);
    if shard < train_end {
        Split::Train
    } else if shard < val_end {
        Split::Validation
    } else {
        Split::Test
    }
}

/// One emitted tensor file and its digest.
#[derive(Debug, Clone, Serialize)]
pub struct TensorEntry {
    pub file: String,
    pub dtype: String,
    pub shape: Vec<usize>,
    pub fnv1a64: String,
}

/// Per-shard provenance (`manifest.json` inside each shard directory).
#[derive(Debug, Clone, Serialize)]
pub struct ShardManifest {
    pub schema_version: u32,
    pub git_commit: String,
    pub git_tree: String,
    pub build_dirty: bool,
    pub source_digest: String,
    pub cargo_lock_hash: String,
    pub generator_config_hash: String,
    pub generator_seed: u64,
    /// Which regeneration pass produced the shard — iter-0 rows are the
    /// initial corpus; later iterations are re-labels under a newer oracle.
    pub generation_iter: u32,
    /// Thread count at generation time — provenance only; the row seeds are
    /// thread-independent so shard bytes never depend on it.
    pub rayon_threads: usize,
    pub shard_id: usize,
    pub total_shards: usize,
    pub split: Split,
    pub variant: String,
    pub solver_config: SolverSpec,
    pub target_spec: TargetSpec,
    pub counts: Counts,
    pub tensors: Vec<TensorEntry>,
    pub dataset_hash: String,
}

/// The solver settings the labels were computed under.
#[derive(Debug, Clone, Serialize)]
pub struct SolverSpec {
    pub method: String,
    pub cfr_iterations: usize,
    pub max_worlds: usize,
    pub belief_perturb_prob: f64,
    pub guided_sample_prob: f64,
    /// Resolve-method rows only: the resolve's world cap.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolve_worlds: Option<usize>,
    /// Resolve-method rows only: the resolve's node budget.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolve_nodes: Option<usize>,
    /// Resolve-method rows only: the resolve's decision-depth cap.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolve_depth: Option<usize>,
    /// Resolve-method rows only: the depth-0 leaf oracle — `learned_rollout`,
    /// `builtin:handcrafted`, or `ort:<fnv1a64 digest>` of the model file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leaf_oracle: Option<String>,
    /// Resolve-method rows only: fraction of rows targeted at rare-state
    /// buckets (equal thirds).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bucket_frac: Option<f64>,
}

/// The target semantics contract.
#[derive(Debug, Clone, Serialize)]
pub struct TargetSpec {
    /// `blueprint_margin` for the reduced-variant path — the converged
    /// profile's expected margin at the actor's infoset — or
    /// `resolved_root_value` for the resolve path.
    pub ev: String,
    /// `blueprint_average_strategy` or `resolved_root_average_strategy`.
    pub policy: String,
    pub feature_dim: usize,
    pub max_actions: usize,
    pub action_order: String,
    pub belief: String,
}

/// Per-shard row accounting.
#[derive(Debug, Clone, Serialize)]
pub struct Counts {
    pub rows: usize,
    pub skipped: usize,
    pub perturbed: usize,
    /// Rare-state coverage: rows carrying each bucket flag — a row can
    /// count toward several.
    pub bucket_stop_call: usize,
    pub bucket_yaku_pressure: usize,
    pub bucket_endgame: usize,
}

/// One line of `dataset_manifest.jsonl` at the dataset root.
#[derive(Debug, Clone, Serialize)]
pub struct DatasetManifestLine {
    pub shard_id: usize,
    pub split: Split,
    pub generation_iter: u32,
    pub num_samples: usize,
    pub dir: String,
    pub tensors: Vec<String>,
    pub dataset_hash: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_are_temporal_and_cover_every_shard() {
        for total in 1..=25_usize {
            let splits: Vec<Split> = (0..total).map(|s| split_for(s, total)).collect();
            assert!(splits.first() == Some(&Split::Train));
            let rank = |s: Split| match s {
                Split::Train => 0,
                Split::Validation => 1,
                Split::Test => 2,
            };
            for pair in splits.windows(2) {
                assert!(rank(pair[0]) <= rank(pair[1]));
            }
        }
        let ten: Vec<Split> = (0..10).map(|s| split_for(s, 10)).collect();
        assert_eq!(ten.iter().filter(|s| **s == Split::Train).count(), 8);
        assert_eq!(ten.iter().filter(|s| **s == Split::Validation).count(), 1);
        assert_eq!(ten.iter().filter(|s| **s == Split::Test).count(), 1);
    }

    /// Pinned small-total layouts — the contract cannot drift silently:
    /// totals <= 2 are degenerate all-train dev runs, 3 holds out a test
    /// shard only, 4+ always yields train + val + test.
    #[test]
    fn split_layouts_are_pinned_for_small_totals() {
        use Split::{Test, Train, Validation as Val};
        let layout = |total: usize| (0..total).map(|s| split_for(s, total)).collect::<Vec<_>>();
        assert_eq!(layout(2), vec![Train, Train]);
        assert_eq!(layout(3), vec![Train, Train, Test]);
        assert_eq!(layout(4), vec![Train, Train, Val, Test]);
        assert_eq!(layout(5), vec![Train, Train, Train, Val, Test]);
        assert_eq!(
            layout(10),
            vec![Train, Train, Train, Train, Train, Train, Train, Train, Val, Test]
        );
    }
}
