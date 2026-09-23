//! Tournament manifest expansion: one `TournamentManifest` → the shared
//! anomaly-free panel plus one pairwise `Manifest` per unordered entrant
//! pair — the launch sweep's inputs.
//!
//! The emitted `panel.json` records the expanded seed list and its
//! FNV-1a/64 hash so `freeze.json` and the verifier bind the exact deals
//! every pairing plays (R-02/R-06). Pairwise manifests carry identical
//! protocol fields; only the baseline/candidate entrants differ.
//!
//! Usage: tournament_emit --tournament-manifest <path> --out-dir <dir>

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use serde::Serialize;

use koi_bench::manifest::{expand_panel, fnv1a64_hex, TournamentManifest};

#[derive(Debug, Parser)]
#[command(about = "Expand a tournament manifest into the shared panel + pairwise manifests")]
struct Arguments {
    /// The tournament manifest (roster + shared protocol block).
    #[arg(long)]
    tournament_manifest: PathBuf,
    /// Destination directory — receives `panel.json` and
    /// `manifests/pair-<first>-<second>.json` per unordered pair.
    #[arg(long)]
    out_dir: PathBuf,
}

#[derive(Serialize)]
struct PanelFile<'a> {
    ruleset: &'a str,
    seeds: &'a [u64],
    cluster_count: usize,
    panel_hash_fx64: String,
}

fn main() -> Result<()> {
    env_logger::init();
    let args = Arguments::parse();
    let manifest = TournamentManifest::load(&args.tournament_manifest)?;
    let seeds = expand_panel(&manifest.panel, manifest.ruleset())?;

    let manifest_dir = args.out_dir.join("manifests");
    std::fs::create_dir_all(&manifest_dir).with_context(|| format!("cannot create {}", manifest_dir.display()))?;

    let panel_json = serde_json::to_vec(&seeds)?;
    let panel = PanelFile {
        ruleset: &manifest.ruleset,
        seeds: &seeds,
        cluster_count: seeds.len(),
        panel_hash_fx64: fnv1a64_hex(&panel_json),
    };
    let panel_path = args.out_dir.join("panel.json");
    std::fs::write(&panel_path, serde_json::to_string_pretty(&panel)?)
        .with_context(|| format!("cannot write {}", panel_path.display()))?;

    let entrants = manifest.entrants.len();
    let mut emitted = 0usize;
    for first in 0..entrants {
        for second in (first + 1)..entrants {
            let pair = manifest.pair_manifest(first, second, &seeds)?;
            pair.validate().context("emitted pair manifest failed validation")?;
            let name = format!(
                "pair-{}-{}.json",
                manifest.entrants[first].label, manifest.entrants[second].label
            );
            let path = manifest_dir.join(&name);
            std::fs::write(&path, serde_json::to_string_pretty(&pair)?)
                .with_context(|| format!("cannot write {}", path.display()))?;
            emitted += 1;
        }
    }

    println!(
        "emitted {} pair manifests + panel ({} clusters, hash {}) to {}",
        emitted,
        seeds.len(),
        panel.panel_hash_fx64,
        args.out_dir.display()
    );
    Ok(())
}
