//! Tournament aggregation: N pairwise `report.json` results → one merged
//! [`TournamentRun`] → the Davidson-ties ranking report.
//!
//! Pairwise manifests keep each run inside the runner's trace-entry bound;
//! this tool reassembles their `report.json` evidence into a
//! `TournamentRun` and drives `assemble_tournament_report`, so the
//! published table comes from the same fitting code an in-process
//! tournament would use — only the scheduling differed.
//!
//! Usage: tournament_rank --tournament-manifest <path> --results-dir
//! <dir with pair-<a>-<b>-out/report.json> --output-dir <dir>

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::Parser;
use serde::Deserialize;

use koi_bench::{
    manifest::{expand_panel, ArtifactIdentity, TournamentManifest},
    report::{assemble_tournament_report, write_tournament_report},
    runner::{PairingRun, SeedResult, TournamentRun},
};

#[derive(Debug, Parser)]
#[command(about = "Assemble a tournament report from per-pairing benchmark results")]
struct Arguments {
    /// The full roster manifest — entrant order, shared protocol block,
    /// panel spec. Loaded via serde (not `TournamentManifest::load`): the
    /// panel is expanded separately so report assembly does not re-deal.
    #[arg(long)]
    tournament_manifest: PathBuf,
    /// Directory holding `pair-<first_label>-<second_label>-out/` results.
    #[arg(long)]
    results_dir: PathBuf,
    /// Report destination (`tournament-results.json` + `-report.md`).
    #[arg(long)]
    output_dir: PathBuf,
}

/// The subset of a pairwise `report.json` this tool consumes. `stopping`
/// must be carried into the merged `PairingRun` — dropping it would
/// silently erase the per-pairing stop reason the report's provenance
/// requires (catalog T-15). Legacy results without the field deserialize
/// as a complete schedule.
#[derive(Deserialize)]
struct PairingResultFile {
    baseline: ArtifactIdentity,
    candidate: ArtifactIdentity,
    seeds: Vec<SeedResult>,
    #[serde(default)]
    stopping: koi_bench::sprt::StoppingOutcome,
    referee: koi_bench::artifact::RefereeIdentity,
    cargo_lock_hash_fx64: String,
}

#[derive(Deserialize)]
struct ReportFile {
    run: PairingResultFile,
}

fn main() -> Result<()> {
    env_logger::init();
    let args = Arguments::parse();
    let manifest: TournamentManifest = serde_json::from_reader(
        std::fs::File::open(&args.tournament_manifest)
            .with_context(|| format!("cannot open {}", args.tournament_manifest.display()))?,
    )
    .with_context(|| format!("cannot parse {}", args.tournament_manifest.display()))?;
    manifest.validate()?;
    let declared = expand_panel(&manifest.panel, manifest.ruleset())?.len();
    let labels: Vec<&str> = manifest.entrants.iter().map(|e| e.label.as_str()).collect();

    // Accepted config-hash forms per entrant. `config_hash_fx64` covers the
    // serialized `koi_solver::Config`; the P6 `leaf_policy` field addition
    // changed that serialization, so evidence produced before and after the
    // schema change carries two valid hashes for the SAME effective config.
    // The merge accepts exactly {new-form, old-form-minus-leaf_policy} of the
    // manifest config — a hash outside that pair is a real config drift.
    let mut accepted_hashes: Vec<Vec<String>> = Vec::with_capacity(manifest.entrants.len());
    for entrant in &manifest.entrants {
        let mut forms = Vec::with_capacity(2);
        let config_bytes = serde_json::to_vec(&entrant.config).context("entrant config serialization")?;
        forms.push(koi_bench::manifest::fnv1a64_hex(&config_bytes));
        // The pre-P6 Config lacked `leaf_policy`; the old binary's hash is
        // the same struct serialization with that key spliced out — byte
        // order must stay the struct's, so splice the raw bytes rather than
        // round-tripping through serde_json::Value (BTreeMap-sorted).
        let legacy_bytes = splice_out_json_field(&config_bytes, "leaf_policy");
        let legacy_hash = koi_bench::manifest::fnv1a64_hex(&legacy_bytes);
        if legacy_hash != forms[0] {
            forms.push(legacy_hash);
        }
        accepted_hashes.push(forms);
    }

    // Every unordered pair i<j must have exactly one results file.
    let mut pairings = Vec::new();
    let mut identities: Vec<Option<ArtifactIdentity>> = vec![None; manifest.entrants.len()];
    let mut additional_builds: Vec<ArtifactIdentity> = Vec::new();
    let mut referee = None;
    let mut lock_hash = None;
    for first in 0..manifest.entrants.len() {
        for second in (first + 1)..manifest.entrants.len() {
            let results_path = args
                .results_dir
                .join(format!("pair-{}-{}-out", labels[first], labels[second]))
                .join("report.json");
            let parsed: ReportFile = serde_json::from_reader(
                std::fs::File::open(&results_path)
                    .with_context(|| format!("cannot open {}", results_path.display()))?,
            )
            .with_context(|| format!("cannot parse {}", results_path.display()))?;
            let parsed = parsed.run;

            // The pairwise run maps Baseline→first and Candidate→second;
            // the labels must agree with the manifest's entrant order or
            // the merged table is silently permuted.
            if parsed.baseline.label != labels[first] || parsed.candidate.label != labels[second] {
                bail!(
                    "{}: labels {} vs {} do not match entrant order {} vs {}",
                    results_path.display(),
                    parsed.baseline.label,
                    parsed.candidate.label,
                    labels[first],
                    labels[second]
                );
            }
            for (index, identity) in [(first, &parsed.baseline), (second, &parsed.candidate)] {
                let serialized = serde_json::to_value(identity).context("artifact identity serialization")?;
                match &identities[index] {
                    None => identities[index] = Some(identity.clone()),
                    Some(existing)
                        if serde_json::to_value(existing).context("artifact identity serialization")? == serialized => {
                    }
                    Some(existing) => {
                        // A later build may carry the same solver config —
                        // the incumbent code paths can be provably
                        // invariant across binaries (supplemental fields).
                        // The solver-relevant binding is label + config
                        // hash; build metadata divergence is disclosed in
                        // `additional_entrant_builds`, never silently pooled.
                        if existing.config_hash_fx64 != identity.config_hash_fx64 {
                            // Divergent recorded hashes are only tolerable
                            // when both are schema-forms of the manifest's
                            // single effective config (the P6 leaf_policy
                            // serialization drift) — anything else is a real
                            // spec change across pairings and must not merge.
                            let accepted = &accepted_hashes[index];
                            if !(accepted.contains(&existing.config_hash_fx64)
                                && accepted.contains(&identity.config_hash_fx64))
                            {
                                bail!(
                                    "entrant {} carries inconsistent solver configs across pairings ({}… vs {}…)",
                                    labels[index],
                                    existing.config_hash_fx64,
                                    identity.config_hash_fx64
                                );
                            }
                        }
                        if !additional_builds
                            .iter()
                            .any(|build| serde_json::to_value(build).ok() == Some(serialized.clone()))
                        {
                            additional_builds.push(identity.clone());
                        }
                    }
                }
            }
            if referee.is_none() {
                referee = Some(parsed.referee);
                lock_hash = Some(parsed.cargo_lock_hash_fx64);
            }
            pairings.push(PairingRun {
                first,
                second,
                stopping: parsed.stopping,
                seeds: parsed.seeds,
            });
        }
    }

    let run = TournamentRun {
        referee: referee.context("no pairings collected")?,
        entrants: identities
            .into_iter()
            .map(|identity| identity.context("entrant never appeared in a pairing"))
            .collect::<Result<Vec<_>>>()?,
        additional_entrant_builds: additional_builds,
        pairings,
        cargo_lock_hash_fx64: lock_hash.unwrap_or_default(),
    };
    let report = assemble_tournament_report(manifest, run, &args.tournament_manifest, &args.output_dir, declared)?;
    let (json_path, markdown_path) = write_tournament_report(&report, &args.output_dir)?;
    println!("wrote {}", json_path.display());
    println!("wrote {}", markdown_path.display());
    Ok(())
}

/// Removes one top-level `"field": <value>` pair from a serialized flat JSON
/// object, keeping every other byte in place. Used to reconstruct the
/// pre-P6 `koi_solver::Config` serialization (no `leaf_policy` field) so the
/// merge can recognize the old binary's config hash for the same effective
/// config. A missing field returns the input unchanged.
fn splice_out_json_field(object: &[u8], field: &str) -> Vec<u8> {
    let needle = format!("\"{field}\":").into_bytes();
    let Some(key_pos) = find_subslice(object, &needle) else {
        return object.to_vec();
    };
    let mut end = key_pos + needle.len();
    // Skip the value: a JSON value ends at the first top-level `,`/`}` —
    // track string state and bracket depth to find it.
    let mut depth: i32 = 0;
    let mut in_string = false;
    let mut escaped = false;
    while end < object.len() {
        let byte = object[end];
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
        } else {
            match byte {
                b'"' => in_string = true,
                b'{' | b'[' => depth += 1,
                b'}' | b']' => {
                    if depth == 0 {
                        break; // the object itself closed: value ended at `end`
                    }
                    depth -= 1;
                }
                b',' if depth == 0 => break,
                _ => {}
            }
        }
        end += 1;
    }
    // Remove the trailing separator when the field isn't last, else the
    // leading one — never both; the object braces stay balanced either way.
    let mut start = key_pos;
    let mut out_end = end;
    if out_end < object.len() && object[out_end] == b',' {
        out_end += 1;
    } else if start > 0 && object[start - 1] == b',' {
        start -= 1;
    }
    let mut out = Vec::with_capacity(object.len() - (out_end - start));
    out.extend_from_slice(&object[..start]);
    out.extend_from_slice(&object[out_end..]);
    out
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|window| window == needle)
}
