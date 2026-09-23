//! Synthetic training-data generator for the learned leaf evaluator.
//!
//! Deals random mid-leg states of a variant, computes the natural belief
//! over compatible worlds, and emits labels from a converged CFR+
//! blueprint (reduced variants) or a per-row root resolve (the canonical
//! pathway): the policy target is the solved average strategy over the
//! canonical legal ordering and the EV target is the labeler's expected
//! margin — never the greedy rollout heuristic. A fraction of rows replace
//! the natural posterior with a uniform-Dirichlet random member range
//! (DeepStack random-range regularization); on the resolve path the solve
//! itself runs under that range.
//!
//! Determinism is per-row: `seed = splitmix64(global, shard, row, attempt)`
//! (`learn::generate::mix_seed`), so shards are byte-identical regardless
//! of `RAYON_NUM_THREADS`.

use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use clap::Parser;
use rayon::prelude::*;

use koi_build_info::{build_dirty, cargo_lock_hash, BUILD_COMMIT, BUILD_SOURCE_DIGEST, BUILD_TREE};
use koi_core::Player;
use koi_learn::learn::{
    generate::{bucket, generate_row, generate_row_resolved, LabelCtx, ResolveCtx, Row},
    manifest::{
        self, split_for, Counts, DatasetManifestLine, ShardManifest, SolverSpec, TargetSpec, TensorEntry,
        SCHEMA_VERSION,
    },
    npy,
};
use koi_solver::{
    cfr::cfr_plus::train_cfr_plus,
    efg::{compiler::compile_variant, KoiVariant},
    leaf::{
        actions::MAX_ACTIONS,
        features::{CARD_BLOCK, FEATURE_DIM},
    },
    resolving::SafetyGadget,
};

#[derive(Debug, Parser)]
#[command(about = "Sharded synthetic leaf-dataset generator")]
struct Arguments {
    /// Variant: nano_6 | micro_8 | reduced_9 | full
    /// (`--method blueprint` requires a compilable variant; `full` is
    /// resolve-method only).
    #[arg(long)]
    variant: String,
    /// Output dataset root.
    #[arg(long)]
    out: PathBuf,
    /// Total labelled rows to emit.
    #[arg(long, default_value_t = 10_000)]
    rows: usize,
    /// Rows per shard.
    #[arg(long, default_value_t = 2_500)]
    shard_size: usize,
    /// Resume index: skip shards `0..N` (rebuilt into the dataset manifest
    /// from their existing per-shard manifests). Per-row seeding makes a
    /// resumed run byte-identical to a fresh one.
    #[arg(long, default_value_t = 0)]
    start_shard: usize,
    /// Global generation seed.
    #[arg(long, default_value_t = 0x5c0a)]
    seed: u64,
    /// Label solver: blueprint (converged CFR+ profile on a compilable
    /// variant) or resolve (high-budget root resolve per row — the
    /// canonical pathway).
    #[arg(long, default_value = "blueprint")]
    method: String,
    /// CFR+ iterations for the label blueprint (blueprint method) or per
    /// row's gadget solve (resolve method) — recorded in provenance.
    #[arg(long, default_value_t = 2_000)]
    cfr_iterations: usize,
    /// Resolve world cap per row (`--method resolve` only).
    #[arg(long, default_value_t = 12)]
    resolve_worlds: usize,
    /// Resolve node budget per row (`--method resolve` only).
    #[arg(long, default_value_t = 150_000)]
    resolve_nodes: usize,
    /// Resolve decision-depth cap per row (`--method resolve` only).
    #[arg(long, default_value_t = 3)]
    resolve_depth: usize,
    /// Leaf evaluator for resolve-method oracle leaves: an ONNX model path
    /// (requires the `leaf-ort` build feature) or `builtin:handcrafted`.
    /// Absent keeps the learned-rollout margin oracle.
    #[arg(long)]
    oracle_model: Option<String>,
    /// Fraction of rows targeted at each rare-state bucket (equal thirds).
    #[arg(long, default_value_t = 0.0)]
    bucket_frac: f64,
    /// Compatible-world cap for belief enumeration.
    #[arg(long, default_value_t = 256)]
    max_worlds: usize,
    /// Probability of replacing the posterior with a random member range.
    #[arg(long, default_value_t = 0.30)]
    belief_perturb: f64,
    /// Probability of heuristic-guided (vs uniform) action sampling during
    /// state replay.
    #[arg(long, default_value_t = 0.70)]
    guided_sample: f64,
    /// Self-play generation epoch recorded in provenance keys.
    #[arg(long, default_value_t = 0)]
    generation_iter: u32,
}

fn variant_by_name(name: &str) -> Result<&'static KoiVariant> {
    match name {
        "nano_6" => Ok(&KoiVariant::NANO_6),
        "micro_8" => Ok(&KoiVariant::MICRO_8),
        "reduced_9" => Ok(&KoiVariant::REDUCED_9),
        "full" | "full_48" | "canonical" => Ok(&KoiVariant::FULL),
        other => bail!("unknown variant '{other}'"),
    }
}

fn write_shard(dir: &Path, rows: &[Row]) -> Result<Vec<TensorEntry>> {
    let n = rows.len();
    let mut tensors = Vec::new();

    let mut emit = |name: &str,
                    dtype: &str,
                    shape: Vec<usize>,
                    write: &mut dyn FnMut(&Path) -> std::io::Result<()>|
     -> Result<()> {
        let path = dir.join(name);
        write(&path)?;
        tensors.push(TensorEntry {
            file: name.to_string(),
            dtype: dtype.to_string(),
            shape,
            fnv1a64: npy::fnv1a64_file(&path)?,
        });
        Ok(())
    };

    let features: Vec<f32> = rows.iter().flat_map(|r| r.features).collect();
    emit("features.npy", "<f4", vec![n, FEATURE_DIM], &mut |p| {
        npy::write_f32(p, &[n, FEATURE_DIM], &features)
    })?;
    let mask: Vec<bool> = rows.iter().flat_map(|r| r.mask).collect();
    emit("legal_mask.npy", "|b1", vec![n, MAX_ACTIONS], &mut |p| {
        npy::write_bool(p, &[n, MAX_ACTIONS], &mask)
    })?;
    let policy: Vec<f32> = rows.iter().flat_map(|r| r.policy).collect();
    emit("policy_target.npy", "<f4", vec![n, MAX_ACTIONS], &mut |p| {
        npy::write_f32(p, &[n, MAX_ACTIONS], &policy)
    })?;
    let ev: Vec<f32> = rows.iter().map(|r| r.ev).collect();
    emit("ev_target.npy", "<f4", vec![n], &mut |p| npy::write_f32(p, &[n], &ev))?;
    let belief: Vec<f32> = rows.iter().flat_map(|r| r.belief).collect();
    emit("belief_vector.npy", "<f4", vec![n, CARD_BLOCK], &mut |p| {
        npy::write_f32(p, &[n, CARD_BLOCK], &belief)
    })?;
    let prov: Vec<u64> = rows.iter().map(|r| r.provenance).collect();
    emit("provenance.npy", "<u8", vec![n], &mut |p| {
        npy::write_u64(p, &[n], &prov)
    })?;

    Ok(tensors)
}

fn config_hash(args: &Arguments, oracle_digest: &str) -> String {
    let canonical = format!(
        "variant={}|rows={}|shard={}|seed={}|method={}|cfr={}|worlds={}|perturb={}|guided={}|gen={}\
         |resolve_w={}|resolve_n={}|resolve_d={}|oracle={}|bucket_frac={}",
        args.variant,
        args.rows,
        args.shard_size,
        args.seed,
        args.method,
        args.cfr_iterations,
        args.max_worlds,
        args.belief_perturb,
        args.guided_sample,
        args.generation_iter,
        args.resolve_worlds,
        args.resolve_nodes,
        args.resolve_depth,
        oracle_digest,
        args.bucket_frac
    );
    npy::fnv1a64_bytes(canonical.as_bytes())
}

/// The oracle digest recorded in the shard manifest: the model file's own
/// FNV for ONNX paths, `builtin:handcrafted` for the sentinel, or
/// `learned_rollout` when no oracle is configured.
fn oracle_digest(spec: Option<&str>) -> Result<String> {
    match spec {
        None => Ok("learned_rollout".to_string()),
        Some(koi_solver::leaf::BUILTIN_LEAF_MODEL) => Ok(koi_solver::leaf::BUILTIN_LEAF_MODEL.to_string()),
        Some(path) => Ok(format!(
            "ort:{}",
            npy::fnv1a64_file(Path::new(path)).context("oracle digest")?
        )),
    }
}

/// Opens the configured leaf oracle once — validation happens at startup,
/// never at the first row inside a worker thread.
fn open_oracle(spec: Option<&str>) -> Result<Option<Box<dyn koi_solver::leaf::LeafEvaluator>>> {
    match spec {
        None => Ok(None),
        Some(spec) => koi_solver::leaf::open_leaf_evaluator(spec)
            .map(Some)
            .map_err(|error| anyhow::anyhow!("leaf oracle '{spec}': {error}")),
    }
}

fn main() -> Result<()> {
    env_logger::init();
    let args = Arguments::parse();
    let variant = variant_by_name(&args.variant)?;
    if args.rows == 0 || args.shard_size == 0 {
        bail!("--rows and --shard-size must be positive");
    }
    if !(0.0..=1.0).contains(&args.belief_perturb)
        || !(0.0..=1.0).contains(&args.guided_sample)
        || !(0.0..=1.0).contains(&args.bucket_frac)
    {
        bail!("probabilities must lie in [0, 1]");
    }
    if args.method != "resolve" && args.oracle_model.is_some() {
        bail!("--oracle-model only applies to --method resolve");
    }
    if args.method == "blueprint" && *variant == KoiVariant::FULL {
        bail!(
            "canonical labels have no blueprint source: the 48-card tree is far beyond the node \
             budget, and greedy-rollout labels are forbidden. Use --method resolve."
        );
    }
    let digest = oracle_digest(args.oracle_model.as_deref())?;
    // Validate the oracle eagerly — a bad path must fail at startup, not at
    // the first row inside a worker thread.
    let oracle_probe = open_oracle(args.oracle_model.as_deref())?;
    drop(oracle_probe);

    fs::create_dir_all(&args.out)?;
    let total_shards = args.rows.div_ceil(args.shard_size);
    if args.start_shard >= total_shards {
        bail!(
            "--start-shard {} is at or past the {}-shard total",
            args.start_shard,
            total_shards
        );
    }
    let dataset_manifest_path = args.out.join("dataset_manifest.jsonl");
    let mut dataset_manifest = File::create(&dataset_manifest_path)
        .with_context(|| format!("cannot create {}", dataset_manifest_path.display()))?;
    // Resume: rebuild the manifest rows for already-emitted shards from
    // their per-shard manifests. Per-row seeding makes the shard bytes
    // deterministic, so a resumed shard range is identical to a fresh run.
    for shard in 0..args.start_shard {
        let dir = args.out.join(format!("shard_{shard:04}"));
        let text = fs::read_to_string(dir.join("manifest.json"))
            .with_context(|| format!("--start-shard {shard} has no manifest at {}", dir.display()))?;
        let prior: serde_json::Value =
            serde_json::from_str(&text).with_context(|| format!("bad shard manifest at {}", dir.display()))?;
        let split = match prior["split"].as_str() {
            Some("train") => manifest::Split::Train,
            Some("val") => manifest::Split::Validation,
            Some("test") => manifest::Split::Test,
            other => bail!("shard {shard} manifest carries unknown split {other:?}"),
        };
        let line = DatasetManifestLine {
            shard_id: shard,
            split,
            generation_iter: args.generation_iter,
            num_samples: prior["counts"]["rows"]
                .as_u64()
                .with_context(|| format!("shard {shard} manifest lacks counts.rows"))?
                as usize,
            dir: dir.file_name().unwrap().to_string_lossy().to_string(),
            tensors: prior["tensors"]
                .as_array()
                .with_context(|| format!("shard {shard} manifest lacks tensors"))?
                .iter()
                .map(|t| {
                    t["file"]
                        .as_str()
                        .map(str::to_owned)
                        .with_context(|| format!("shard {shard} tensor lacks file"))
                })
                .collect::<Result<Vec<_>>>()?,
            dataset_hash: prior["dataset_hash"]
                .as_str()
                .with_context(|| format!("shard {shard} manifest lacks dataset_hash"))?
                .to_string(),
        };
        writeln!(dataset_manifest, "{}", serde_json::to_string(&line)?)?;
    }

    if args.method == "resolve" {
        run_resolve(&args, variant, &digest, total_shards, &mut dataset_manifest)?;
    } else if args.method == "blueprint" {
        run_blueprint(&args, variant, &digest, total_shards, &mut dataset_manifest)?;
    } else {
        bail!("unknown --method '{}' (blueprint|resolve)", args.method);
    }

    eprintln!("wrote {} shards to {}", total_shards, args.out.display());
    Ok(())
}

/// Shared shard tail: tensors, per-shard manifest, dataset-manifest line.
#[allow(clippy::too_many_arguments)]
fn emit_shard(
    args: &Arguments,
    shard: usize,
    total_shards: usize,
    rows: &[Row],
    counts: Counts,
    solver_config: SolverSpec,
    target_spec: TargetSpec,
    oracle_digest: &str,
    dataset_manifest: &mut File,
) -> Result<()> {
    let dir = args.out.join(format!("shard_{shard:04}"));
    fs::create_dir_all(&dir)?;
    let tensors = write_shard(&dir, rows)?;
    // The dataset hash binds every emitted tensor digest in file order —
    // a tampered shard cannot keep its recorded identity.
    let dataset_hash = npy::fnv1a64_bytes(
        tensors
            .iter()
            .map(|t| t.fnv1a64.as_str())
            .collect::<Vec<_>>()
            .join("|")
            .as_bytes(),
    );
    let manifest = ShardManifest {
        schema_version: SCHEMA_VERSION,
        git_commit: BUILD_COMMIT.to_string(),
        git_tree: BUILD_TREE.to_string(),
        build_dirty: build_dirty(),
        source_digest: BUILD_SOURCE_DIGEST.to_string(),
        cargo_lock_hash: cargo_lock_hash()
            .map(|hash| format!("fnv1a64:{hash}"))
            .unwrap_or_else(|_| "fnv1a64:unavailable".to_string()),
        generator_config_hash: config_hash(args, oracle_digest),
        generator_seed: args.seed,
        generation_iter: args.generation_iter,
        rayon_threads: rayon::current_num_threads(),
        shard_id: shard,
        total_shards,
        split: split_for(shard, total_shards),
        variant: args.variant.clone(),
        solver_config,
        target_spec,
        counts,
        tensors,
        dataset_hash: dataset_hash.clone(),
    };
    let manifest_text = serde_json::to_string_pretty(&manifest)?;
    fs::write(dir.join("manifest.json"), manifest_text)?;
    let line = DatasetManifestLine {
        shard_id: shard,
        split: manifest.split,
        generation_iter: args.generation_iter,
        num_samples: manifest.counts.rows,
        dir: dir.file_name().unwrap().to_string_lossy().to_string(),
        tensors: manifest.tensors.iter().map(|t| t.file.clone()).collect(),
        dataset_hash,
    };
    writeln!(dataset_manifest, "{}", serde_json::to_string(&line)?)?;
    eprintln!(
        "shard {shard}: {} rows ({} skipped, {} perturbed)",
        manifest.counts.rows, manifest.counts.skipped, manifest.counts.perturbed
    );
    Ok(())
}

/// The blueprint labeler: compile the variant's EFG, solve it with CFR+,
/// and label replayed states by blueprint lookup. Reduced variants only —
/// the canonical tree never fits the compiler.
fn run_blueprint(
    args: &Arguments,
    variant: &'static KoiVariant,
    oracle_digest: &str,
    total_shards: usize,
    dataset_manifest: &mut File,
) -> Result<()> {
    eprintln!(
        "compiling {} ({} cards), label method blueprint ({} cfr+ iterations)...",
        args.variant,
        variant.cards.count(),
        args.cfr_iterations
    );
    let tree = std::sync::Arc::new(compile_variant(variant, Player::South)?);
    let blueprint = train_cfr_plus(&tree, args.cfr_iterations);
    let profile = blueprint.averaged_profile();
    let gadget = SafetyGadget::new(tree.clone(), &blueprint);
    let exploitability = koi_solver::cfr::exploitability::exploitability(&tree, &profile);
    eprintln!(
        "tree: {} nodes, label-profile exploitability {:.6}",
        tree.node_count(),
        exploitability
    );

    let ctx = LabelCtx {
        variant,
        tree: &tree,
        profile: &profile,
        gadget: &gadget,
        seed: args.seed,
        max_worlds: args.max_worlds,
        belief_perturb: args.belief_perturb,
        guided_sample: args.guided_sample,
        generation_iter: args.generation_iter,
        solver_budget: args.cfr_iterations as u64,
    };

    for shard in args.start_shard..total_shards {
        let shard_rows = args.shard_size.min(args.rows - shard * args.shard_size);
        let rows: Vec<(Row, u8)> = (0..shard_rows)
            .into_par_iter()
            .filter_map(|row| generate_row(&ctx, shard, row, args.bucket_frac))
            .collect();
        let skipped = shard_rows - rows.len();
        let perturbed = rows.iter().filter(|(r, _)| r.perturbed).count();
        let counts = Counts {
            rows: rows.len(),
            skipped,
            perturbed,
            bucket_stop_call: rows.iter().filter(|(_, f)| f & bucket::STOP_CALL != 0).count(),
            bucket_yaku_pressure: rows.iter().filter(|(_, f)| f & bucket::YAKU_PRESSURE != 0).count(),
            bucket_endgame: rows.iter().filter(|(_, f)| f & bucket::ENDGAME != 0).count(),
        };
        let rows: Vec<Row> = rows.into_iter().map(|(r, _)| r).collect();
        emit_shard(
            args,
            shard,
            total_shards,
            &rows,
            counts,
            SolverSpec {
                method: "cfr_plus_blueprint".into(),
                cfr_iterations: args.cfr_iterations,
                max_worlds: args.max_worlds,
                belief_perturb_prob: args.belief_perturb,
                guided_sample_prob: args.guided_sample,
                resolve_worlds: None,
                resolve_nodes: None,
                resolve_depth: None,
                leaf_oracle: None,
                bucket_frac: (args.bucket_frac > 0.0).then_some(args.bucket_frac),
            },
            TargetSpec {
                ev: "blueprint_margin".into(),
                policy: "blueprint_average_strategy".into(),
                feature_dim: FEATURE_DIM,
                max_actions: MAX_ACTIONS,
                action_order: "action_key".into(),
                belief: "card_marginals_over_members".into(),
            },
            oracle_digest,
            dataset_manifest,
        )?;
    }
    Ok(())
}

/// The resolve labeler — the canonical pathway: each row's targets come
/// from a root resolve (`resolved_root_value` /
/// `resolved_root_average_strategy`), with oracle leaves priced by the
/// configured evaluator — the learned rollout for iteration-0 corpora, a
/// learned model once `--oracle-model` points at an exported net.
fn run_resolve(
    args: &Arguments,
    variant: &'static KoiVariant,
    oracle_digest: &str,
    total_shards: usize,
    dataset_manifest: &mut File,
) -> Result<()> {
    eprintln!(
        "resolve labels for {} ({} cards): {} cfr iterations, {} worlds, depth {}, oracle {}",
        args.variant,
        variant.cards.count(),
        args.cfr_iterations,
        args.resolve_worlds,
        args.resolve_depth,
        oracle_digest,
    );
    let ctx = ResolveCtx {
        variant,
        seed: args.seed,
        max_worlds: args.max_worlds,
        belief_perturb: args.belief_perturb,
        guided_sample: args.guided_sample,
        bucket_frac: args.bucket_frac,
        spec: koi_solver::resolving::ResolveSpec {
            max_worlds: args.resolve_worlds,
            max_nodes: args.resolve_nodes,
            cfr_iterations: args.cfr_iterations,
            max_decision_depth: args.resolve_depth,
        },
        generation_iter: args.generation_iter,
    };
    let oracle_spec = args.oracle_model.clone();

    for shard in args.start_shard..total_shards {
        let shard_rows = args.shard_size.min(args.rows - shard * args.shard_size);
        // One evaluator per worker lane — `LeafEvaluator` is `Send` but not
        // `Sync`, so the shared model is reopened per lane inside a RefCell
        // instead of contended behind a mutex.
        let oracle_lane = oracle_spec.clone();
        let rows: Vec<(Row, u8)> = (0..shard_rows)
            .into_par_iter()
            .map_init(
                move || {
                    open_oracle(oracle_lane.as_deref())
                        .unwrap_or_else(|err| panic!("oracle load: {err}"))
                        .map(std::cell::RefCell::new)
                },
                |eval, row| generate_row_resolved(&ctx, eval.as_ref(), shard, row),
            )
            .filter_map(|outcome| outcome)
            .collect();
        let skipped = shard_rows - rows.len();
        let perturbed = rows.iter().filter(|(r, _)| r.perturbed).count();
        let counts = Counts {
            rows: rows.len(),
            skipped,
            perturbed,
            bucket_stop_call: rows.iter().filter(|(_, f)| f & bucket::STOP_CALL != 0).count(),
            bucket_yaku_pressure: rows.iter().filter(|(_, f)| f & bucket::YAKU_PRESSURE != 0).count(),
            bucket_endgame: rows.iter().filter(|(_, f)| f & bucket::ENDGAME != 0).count(),
        };
        let rows: Vec<Row> = rows.into_iter().map(|(r, _)| r).collect();
        emit_shard(
            args,
            shard,
            total_shards,
            &rows,
            counts,
            SolverSpec {
                method: "resolve".into(),
                cfr_iterations: args.cfr_iterations,
                max_worlds: args.max_worlds,
                belief_perturb_prob: args.belief_perturb,
                guided_sample_prob: args.guided_sample,
                resolve_worlds: Some(args.resolve_worlds),
                resolve_nodes: Some(args.resolve_nodes),
                resolve_depth: Some(args.resolve_depth),
                leaf_oracle: Some(oracle_digest.to_string()),
                bucket_frac: (args.bucket_frac > 0.0).then_some(args.bucket_frac),
            },
            TargetSpec {
                ev: "resolved_root_value".into(),
                policy: "resolved_root_average_strategy".into(),
                feature_dim: FEATURE_DIM,
                max_actions: MAX_ACTIONS,
                action_order: "action_key".into(),
                belief: "card_marginals_over_members".into(),
            },
            oracle_digest,
            dataset_manifest,
        )?;
    }
    Ok(())
}
