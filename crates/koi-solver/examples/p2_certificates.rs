//! P2 solver certificates: the reduced-domain evidence for the CFR/LP stack.
//!
//! Reads a manifest declaring the domains, iteration ladders, seeds, and
//! tolerances; runs the certificates; writes `report.json` + `report.md`
//! into `--out` with the full `koi-build-info` provenance header.
//!
//! Certificates produced:
//! - `lp_certificate`: exact sequence-form equilibrium on `nano_6` — the
//!   joint LP profile must exploit to ~0 (epsilon-Nash certificate).
//! - `convergence`: exploitability-vs-iterations curves for each declared
//!   CFR variant on `micro_8`.
//!
//! Usage:
//!   cargo run -p koi-solver --example p2_certificates --profile release -- \
//!       --manifest docs/benchmarks/manifests/p2-solver-certificates.json \
//!       --out docs/benchmarks/p2-solver-certificates

use std::path::PathBuf;
use std::time::Instant;

use anyhow::{bail, Context, Result};
use koi_core::Player;
use koi_solver::cfr::{
    exploitability, profile_value, train_cfr_plus, train_dcfr, train_external_sampling, train_external_sampling_ccs,
    train_pcfr_plus,
};
use koi_solver::efg::{compile_variant, EfgTree, KoiVariant, SequenceForm};
use koi_solver::endgame::{solve_tree, SolvedStrategy};
use rand::{rngs::SmallRng, SeedableRng};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct Manifest {
    schema_version: u32,
    label: String,
    lp_certificate: LpSpec,
    convergence: ConvergenceSpec,
}

#[derive(Debug, Deserialize)]
struct LpSpec {
    variant: String,
    dealer: String,
    exploitability_tolerance: f64,
}

#[derive(Debug, Deserialize)]
struct ConvergenceSpec {
    variant: String,
    dealer: String,
    algorithms: Vec<AlgoSpec>,
}

#[derive(Debug, Deserialize)]
struct AlgoSpec {
    name: String,
    iterations: Vec<usize>,
    #[serde(default)]
    seed: Option<u64>,
}

#[derive(Debug, Serialize)]
struct Report {
    schema_version: u32,
    label: String,
    /// `canonical` (clean tree + `--profile release`), `development`
    /// (anything else), or `failed` (a certificate errored — the partial
    /// result and the error are still written for diagnosis).
    verdict: String,
    /// Scope qualifiers binding every claim in this report.
    qualifiers: Vec<String>,
    provenance: Provenance,
    manifest_hash: String,
    lp_certificate: Option<LpResult>,
    convergence: Option<ConvergenceResult>,
    /// The first certificate failure, when `verdict == "failed"`.
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct Provenance {
    build_commit: String,
    build_tree: String,
    build_dirty: bool,
    build_profile: String,
    source_digest_fnv1a64: String,
    rustflags: String,
    pgo_profile_fx64: Option<String>,
    cargo_lock_hash: Option<String>,
}

#[derive(Debug, Serialize)]
struct DomainStats {
    variant: String,
    nodes: usize,
    infosets: usize,
    terminals: usize,
    south_sequences: usize,
    north_sequences: usize,
    payoff_cells: usize,
}

#[derive(Debug, Serialize)]
struct LpResult {
    domain: DomainStats,
    value_south: f64,
    value_north: f64,
    exploitability: f64,
    tolerance: f64,
    certified: bool,
    wall_seconds: f64,
}

#[derive(Debug, Serialize)]
struct ConvergenceResult {
    domain: DomainStats,
    curves: Vec<Curve>,
}

#[derive(Debug, Serialize)]
struct Curve {
    algorithm: String,
    seed: Option<u64>,
    points: Vec<Point>,
}

#[derive(Debug, Serialize)]
struct Point {
    iterations: usize,
    exploitability: f64,
    profile_value_south: f64,
    wall_seconds: f64,
}

fn variant_by_name(name: &str) -> Result<KoiVariant> {
    match name {
        "nano_6" => Ok(KoiVariant::NANO_6),
        "micro_8" => Ok(KoiVariant::MICRO_8),
        "fieldvoid_10" => Ok(KoiVariant::FIELDVOID_10),
        "reduced_9" => Ok(KoiVariant::REDUCED_9),
        other => bail!("unknown variant {other:?} in manifest"),
    }
}

fn player_by_name(name: &str) -> Result<Player> {
    match name {
        "south" => Ok(Player::South),
        "north" => Ok(Player::North),
        other => bail!("unknown dealer {other:?} in manifest"),
    }
}

fn domain_stats(variant: &KoiVariant, tree: &EfgTree, form: &SequenceForm) -> DomainStats {
    DomainStats {
        variant: variant.name.to_string(),
        nodes: tree.node_count(),
        infosets: tree.infosets().len(),
        terminals: tree.terminal_count(),
        south_sequences: form.south_sequence_count(),
        north_sequences: form.north_sequence_count(),
        payoff_cells: form.south_sequence_count() * form.north_sequence_count(),
    }
}

/// Lifts both players' per-player solutions onto the tree's global infoset
/// order — the profile `exploitability` consumes.
fn full_profile(tree: &EfgTree, south: &SolvedStrategy, north: &SolvedStrategy) -> Vec<Vec<f64>> {
    let mut profile = vec![Vec::new(); tree.infosets().len()];
    for (row, &global) in south.strategy.iter().zip(&south.global_infosets) {
        profile[global] = row.clone();
    }
    for (row, &global) in north.strategy.iter().zip(&north.global_infosets) {
        profile[global] = row.clone();
    }
    profile
}

fn run_lp_certificate(spec: &LpSpec) -> Result<LpResult> {
    let variant = variant_by_name(&spec.variant)?;
    let dealer = player_by_name(&spec.dealer)?;
    let started = Instant::now();
    let tree = compile_variant(&variant, dealer).context("compiling the LP-certificate domain")?;
    let form = SequenceForm::compile(&tree).context("extracting the sequence form")?;
    let domain = domain_stats(&variant, &tree, &form);
    let south = solve_tree(&tree, Player::South).context("solving South's sequence form")?;
    let north = solve_tree(&tree, Player::North).context("solving North's mirrored form")?;
    let profile = full_profile(&tree, &south, &north);
    let exploit = exploitability(&tree, &profile);
    Ok(LpResult {
        domain,
        value_south: south.value,
        value_north: north.value,
        exploitability: exploit,
        tolerance: spec.exploitability_tolerance,
        certified: exploit <= spec.exploitability_tolerance,
        wall_seconds: started.elapsed().as_secs_f64(),
    })
}

fn train(tree: &EfgTree, name: &str, iterations: usize, seed: Option<u64>) -> Result<Vec<Vec<f64>>> {
    let blueprint = match name {
        "cfr_plus" => train_cfr_plus(tree, iterations),
        "dcfr" => train_dcfr(tree, iterations),
        "pcfr_plus" => train_pcfr_plus(tree, iterations),
        "mccfr" => {
            let mut rng = SmallRng::seed_from_u64(seed.context("mccfr needs a seed")?);
            train_external_sampling(tree, iterations, &mut rng)
        }
        "ccs_mccfr" => {
            let mut rng = SmallRng::seed_from_u64(seed.context("ccs_mccfr needs a seed")?);
            train_external_sampling_ccs(tree, iterations, &mut rng)
        }
        other => bail!("unknown algorithm {other:?} in manifest"),
    };
    Ok(blueprint.averaged_profile())
}

fn run_convergence(spec: &ConvergenceSpec) -> Result<ConvergenceResult> {
    let variant = variant_by_name(&spec.variant)?;
    let dealer = player_by_name(&spec.dealer)?;
    let tree = compile_variant(&variant, dealer).context("compiling the convergence domain")?;
    let form = SequenceForm::compile(&tree).context("extracting the sequence form")?;
    let domain = domain_stats(&variant, &tree, &form);
    let mut curves = Vec::new();
    for algo in &spec.algorithms {
        let mut points = Vec::new();
        for &iterations in &algo.iterations {
            let started = Instant::now();
            let profile = train(&tree, &algo.name, iterations, algo.seed)?;
            let wall = started.elapsed().as_secs_f64();
            points.push(Point {
                iterations,
                exploitability: exploitability(&tree, &profile),
                profile_value_south: profile_value(&tree, &profile, Player::South),
                wall_seconds: wall,
            });
        }
        curves.push(Curve {
            algorithm: algo.name.clone(),
            seed: algo.seed,
            points,
        });
    }
    Ok(ConvergenceResult { domain, curves })
}

fn fnv1a64_hex(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

fn provenance() -> Provenance {
    Provenance {
        build_commit: koi_build_info::BUILD_COMMIT.to_string(),
        build_tree: koi_build_info::BUILD_TREE.to_string(),
        build_dirty: koi_build_info::build_dirty(),
        build_profile: koi_build_info::BUILD_PROFILE.to_string(),
        source_digest_fnv1a64: koi_build_info::BUILD_SOURCE_DIGEST.to_string(),
        rustflags: koi_build_info::BUILD_RUSTFLAGS.to_string(),
        pgo_profile_fx64: koi_build_info::BUILD_PGO_PROFILE_FX64.map(str::to_string),
        cargo_lock_hash: koi_build_info::cargo_lock_hash().ok(),
    }
}

/// Canonical evidence requires a clean tree built under `--profile
/// release`; anything else is development evidence. A certificate error
/// overrides both — the report still records it.
fn verdict_of(provenance: &Provenance, failed: bool) -> (String, Vec<String>) {
    if failed {
        return (
            "failed".to_string(),
            vec!["a certificate errored — partial results only".to_string()],
        );
    }
    let mut qualifiers = Vec::new();
    if provenance.build_dirty {
        qualifiers.push("built from a dirty tree — development evidence only".to_string());
    }
    if provenance.build_profile != "release" {
        qualifiers.push(format!(
            "built under profile '{}' — canonical requires --profile release",
            provenance.build_profile
        ));
    }
    qualifiers.push("exact results cover the declared reduced domains only — not the full 48-card game".to_string());
    qualifiers.push("sampled algorithms (mccfr, ccs_mccfr) carry their declared seeds".to_string());
    let verdict = if provenance.build_dirty || provenance.build_profile != "release" {
        "development"
    } else {
        "canonical"
    };
    (verdict.to_string(), qualifiers)
}

fn render_markdown(report: &Report) -> String {
    let mut out = String::new();
    out.push_str(&format!("# Koi-Maestro P2 solver certificates — {}\n\n", report.label));
    out.push_str(&format!("**verdict: `{}`**\n\n", report.verdict));
    for qualifier in &report.qualifiers {
        out.push_str(&format!("- {qualifier}\n"));
    }
    if let Some(error) = &report.error {
        out.push_str(&format!("\n**error:** `{error}`\n"));
    }
    out.push_str("\n| field | value |\n|---|---|\n");
    out.push_str(&format!("| manifest | `{}` |\n", report.manifest_hash));
    out.push_str(&format!("| referee | `{}` |\n", report.provenance.build_commit));
    out.push_str(&format!("| dirty | `{}` |\n", report.provenance.build_dirty));
    out.push_str(&format!("| profile | `{}` |\n", report.provenance.build_profile));

    if let Some(lp) = &report.lp_certificate {
        out.push_str("\n## Exact LP certificate\n\n");
        out.push_str(&format!(
            "- domain `{}`: {} nodes, {} infosets, {} south / {} north sequences ({} payoff cells)\n",
            lp.domain.variant,
            lp.domain.nodes,
            lp.domain.infosets,
            lp.domain.south_sequences,
            lp.domain.north_sequences,
            lp.domain.payoff_cells
        ));
        out.push_str(&format!(
            "- value (South) `{:.6}`, value (North) `{:.6}`\n",
            lp.value_south, lp.value_north
        ));
        out.push_str(&format!(
            "- joint-profile exploitability `{:.3e}` vs tolerance `{:.1e}` — **{}**\n",
            lp.exploitability,
            lp.tolerance,
            if lp.certified {
                "CERTIFIED epsilon-Nash"
            } else {
                "FAILED"
            }
        ));
        out.push_str(&format!("- wall `{:.3}` s\n", lp.wall_seconds));
        out.push_str(
            "\nScope: exact equilibrium of the declared reduced domain only — \
             not a claim about the full 48-card game.\n",
        );
    }

    if let Some(conv) = &report.convergence {
        out.push_str("\n## CFR-family convergence\n\n");
        out.push_str(&format!(
            "Domain `{}`: {} nodes, {} infosets.\n\n",
            conv.domain.variant, conv.domain.nodes, conv.domain.infosets
        ));
        out.push_str("| algorithm | iterations | exploitability | value (South) | wall s |\n|---|---|---|---|---|\n");
        for curve in &conv.curves {
            for point in &curve.points {
                out.push_str(&format!(
                    "| {} | {} | {:.6} | {:.4} | {:.2} |\n",
                    curve.algorithm,
                    point.iterations,
                    point.exploitability,
                    point.profile_value_south,
                    point.wall_seconds
                ));
            }
        }
        out.push_str(
            "\nScope: exploitability is the exact best-response measure on the \
             declared reduced domain; sampled curves carry their declared seed.\n",
        );
    }
    out
}

fn main() -> Result<()> {
    let mut manifest_path: Option<PathBuf> = None;
    let mut out_dir: Option<PathBuf> = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--manifest" => manifest_path = Some(PathBuf::from(args.next().context("--manifest needs a path")?)),
            "--out" => out_dir = Some(PathBuf::from(args.next().context("--out needs a path")?)),
            other => bail!("unknown argument {other:?}"),
        }
    }
    let manifest_path = manifest_path.context("usage: --manifest <path> --out <dir>")?;
    let out_dir = out_dir.context("usage: --manifest <path> --out <dir>")?;

    let manifest_bytes =
        std::fs::read(&manifest_path).with_context(|| format!("reading manifest {}", manifest_path.display()))?;
    let manifest: Manifest = serde_json::from_slice(&manifest_bytes).context("parsing the certificates manifest")?;
    if manifest.schema_version != 1 {
        bail!("unsupported manifest schema_version {}", manifest.schema_version);
    }

    // Run each certificate independently: a failure must not erase the
    // evidence that preceded it — the report is always written, marked
    // `failed` with the error, before exiting nonzero.
    let provenance = provenance();
    let mut lp_certificate = None;
    let mut convergence = None;
    let mut error = None;
    match run_lp_certificate(&manifest.lp_certificate) {
        Ok(lp) => {
            if !lp.certified {
                error = Some(format!(
                    "lp_certificate: exploitability {:.3e} exceeds tolerance {:.1e} — not epsilon-Nash",
                    lp.exploitability, lp.tolerance
                ));
            }
            lp_certificate = Some(lp);
        }
        Err(e) => error = Some(format!("lp_certificate: {e:#}")),
    }
    // Convergence runs regardless of the LP outcome — a failed run should
    // carry as much diagnostic evidence as it can produce.
    match run_convergence(&manifest.convergence) {
        Ok(conv) => convergence = Some(conv),
        Err(e) => {
            let message = format!("convergence: {e:#}");
            match &mut error {
                Some(existing) => {
                    existing.push_str("; ");
                    existing.push_str(&message);
                }
                None => error = Some(message),
            }
        }
    }

    let (verdict, qualifiers) = verdict_of(&provenance, error.is_some());
    let report = Report {
        schema_version: 1,
        label: manifest.label.clone(),
        verdict,
        qualifiers,
        provenance,
        manifest_hash: fnv1a64_hex(&manifest_bytes),
        lp_certificate,
        convergence,
        error,
    };

    std::fs::create_dir_all(&out_dir).with_context(|| format!("creating {}", out_dir.display()))?;
    let json = serde_json::to_string_pretty(&report)?;
    std::fs::write(out_dir.join("report.json"), &json)?;
    std::fs::write(out_dir.join("report.md"), render_markdown(&report))?;
    println!("wrote {} (+ .md)", out_dir.join("report.json").display());

    // Fail closed — after the diagnostic artifact exists: an uncertified LP
    // result or a certificate error is evidence of a solver bug, never a
    // reportable certificate.
    if let Some(error) = &report.error {
        eprintln!("certificate run FAILED: {error}");
        std::process::exit(2);
    }
    Ok(())
}
