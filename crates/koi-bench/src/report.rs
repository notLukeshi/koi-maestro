//! Run reports: a machine-readable JSON payload and a human Markdown
//! summary, both carrying the full provenance header.
//!
//! The JSON is the evidence record — the run plus the computed summary.
//! The Markdown is the journal-ready rendering: outcome first, then the
//! paired metrics with their confidence intervals, latency tables, and a
//! per-cluster ledger.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::manifest::Manifest;
use crate::runner::{BenchmarkRun, LegStatus, Seat};
use crate::statistics::{summarize, BenchmarkSummary};

/// Paths the report writer produced.
pub struct ReportPaths {
    pub json: PathBuf,
    pub markdown: PathBuf,
}

/// The run's headline verdict: `vacuous` when no leg produced played
/// evidence, otherwise `canonical` only when every attested build is
/// clean — a dirty referee or entrant demotes the evidence to
/// `development`.
pub fn verdict(run: &BenchmarkRun, summary: &BenchmarkSummary) -> &'static str {
    if summary.valid_legs == 0 {
        "vacuous"
    } else if run.referee.build_dirty || run.baseline.build_dirty || run.candidate.build_dirty {
        "development"
    } else {
        "canonical"
    }
}

/// Caveat strings qualifying the run's claims — every report carries them
/// so downstream consumers cannot strip them silently.
fn qualifiers(run: &BenchmarkRun, summary: &BenchmarkSummary, manifest: &Manifest) -> Vec<String> {
    let mut out = Vec::new();
    let synthetic = summary.invalid_legs + summary.time_forfeit_legs;
    if synthetic > 0 {
        out.push(format!(
            "{synthetic} of {} legs are forfeits carrying synthetic margins (±{}), not played evidence",
            summary.leg_count, manifest.invalid_forfeit_margin
        ));
    }
    if summary.valid_legs == 0 {
        out.push(
            "zero valid legs: no played evidence was produced; point estimates and stopping outcomes are not interpretable"
                .to_owned(),
        );
    }
    if summary.cluster_count < manifest.seeds.len() && !run.stopping.stopped_early() {
        out.push(format!(
            "only {} of {} declared clusters played",
            summary.cluster_count,
            manifest.seeds.len()
        ));
    }
    if run.stopping.stopped_early() {
        out.push(
            "sequential stop: point estimates are selection-biased; the boundary decision is the evidence".to_owned(),
        );
    }
    if verdict(run, summary) == "development" {
        out.push("at least one attested build is dirty — development evidence, not canonical".to_owned());
    }
    out
}

/// Serializes the run + summary to `report.json` and renders `report.md`.
pub fn write_reports(out_dir: &Path, run: &BenchmarkRun, manifest: &Manifest) -> Result<ReportPaths> {
    let summary = summarize(&run.seeds, &manifest.bootstrap).context("failed to summarize the run")?;
    let verdict = verdict(run, &summary);
    let qualifiers = qualifiers(run, &summary, manifest);

    #[derive(serde::Serialize)]
    struct Report<'a> {
        run: &'a BenchmarkRun,
        summary: &'a BenchmarkSummary,
        verdict: &'a str,
        clusters_declared: usize,
        qualifiers: &'a [String],
    }
    let payload = Report {
        run,
        summary: &summary,
        verdict,
        clusters_declared: manifest.seeds.len(),
        qualifiers: &qualifiers,
    };
    let json_path = out_dir.join("report.json");
    std::fs::write(&json_path, serde_json::to_string_pretty(&payload)?)
        .with_context(|| format!("failed to write {}", json_path.display()))?;

    let markdown_path = out_dir.join("report.md");
    let markdown = render_markdown(run, &summary, manifest, verdict, &qualifiers);
    std::fs::write(&markdown_path, markdown).with_context(|| format!("failed to write {}", markdown_path.display()))?;

    Ok(ReportPaths {
        json: json_path,
        markdown: markdown_path,
    })
}

fn render_markdown(
    run: &BenchmarkRun,
    summary: &BenchmarkSummary,
    manifest: &Manifest,
    verdict: &str,
    qualifiers: &[String],
) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# Koi-Koi paired benchmark — {}", manifest.run_label);
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "**verdict: `{verdict}`** — {} of {} declared clusters played",
        summary.cluster_count,
        manifest.seeds.len()
    );
    for qualifier in qualifiers {
        let _ = writeln!(out, "- {qualifier}");
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "| field | value |");
    let _ = writeln!(out, "|---|---|");
    let _ = writeln!(out, "| ruleset | `{}` |", run.ruleset);
    let _ = writeln!(out, "| manifest | `{}` |", run.manifest_hash_fx64);
    let _ = writeln!(
        out,
        "| referee | `{}`{} |",
        run.referee.build_commit,
        if run.referee.build_dirty { " (dirty)" } else { "" }
    );
    let _ = writeln!(
        out,
        "| baseline | `{}` @ `{}`{} |",
        run.baseline.label,
        run.baseline.build_commit,
        if run.baseline.build_dirty { " (dirty)" } else { "" }
    );
    let _ = writeln!(
        out,
        "| candidate | `{}` @ `{}`{} |",
        run.candidate.label,
        run.candidate.build_commit,
        if run.candidate.build_dirty { " (dirty)" } else { "" }
    );
    let _ = writeln!(
        out,
        "| stopping | `{}`{} |",
        run.stopping.kind_name(),
        if run.stopping.stopped_early() {
            " — sequential stop: point estimates are selection-biased; the boundary decision is the evidence"
        } else {
            ""
        }
    );
    let _ = writeln!(out);

    let _ = writeln!(out, "## Result");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "- **candidate win score**: `{:.4}`  CI99 `[{:.4}, {:.4}]`",
        summary.candidate_win_score.estimate, summary.candidate_win_score.ci_low, summary.candidate_win_score.ci_high
    );
    let _ = writeln!(
        out,
        "- **candidate margin**: `{:.2}`  CI99 `[{:.2}, {:.2}]`",
        summary.candidate_margin.estimate, summary.candidate_margin.ci_low, summary.candidate_margin.ci_high
    );
    let _ = writeln!(
        out,
        "- clusters `{}`, legs `{}` — wins `{}`, draws `{}`, losses `{}`, invalid `{}`, time-forfeits `{}`",
        summary.cluster_count,
        summary.leg_count,
        summary.candidate_wins,
        summary.draws,
        summary.candidate_losses,
        summary.invalid_legs,
        summary.time_forfeit_legs
    );
    let _ = writeln!(out);

    let _ = writeln!(out, "## Latency (referee-measured decision ms)");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |"
    );
    let _ = writeln!(out, "|---|---|---|---|---|---|---|---|");
    for (name, latency, leg_time) in [
        ("baseline", &summary.baseline_latency, &summary.baseline_leg_time),
        ("candidate", &summary.candidate_latency, &summary.candidate_leg_time),
    ] {
        let _ = writeln!(
            out,
            "| {name} | {} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {} |",
            latency.decisions,
            latency.mean_ms,
            latency.median_ms,
            latency.p95_ms,
            latency.max_ms,
            leg_time.mean_ms,
            leg_time.over_cap
        );
    }
    let _ = writeln!(out);

    let _ = writeln!(out, "## Clusters");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "| seed | candidate-deals margin | status | baseline-deals margin | status |"
    );
    let _ = writeln!(out, "|---|---|---|---|---|");
    for seed in &run.seeds {
        let _ = writeln!(
            out,
            "| `{}` | {:.2} | {} | {:.2} | {} |",
            seed.seed,
            seed.candidate_deals.candidate_margin,
            status_name(&seed.candidate_deals),
            seed.baseline_deals.candidate_margin,
            status_name(&seed.baseline_deals),
        );
    }
    let _ = writeln!(out);
    out
}

fn status_name(leg: &crate::runner::LegResult) -> String {
    let seat = match leg.candidate_seat {
        Seat::South => "S",
        Seat::North => "N",
    };
    match &leg.status {
        LegStatus::Valid => format!("valid (cand {seat})"),
        LegStatus::Invalid { offender, .. } => format!("invalid: {offender:?} forfeits"),
        LegStatus::TimeForfeit { offender, .. } => format!("time-forfeit: {offender:?}"),
    }
}

// ---------------------------------------------------------------------------
// Tournament reports — the merged pairwise field's published table.
// ---------------------------------------------------------------------------

pub const TOURNAMENT_RESULTS_FILE: &str = "tournament-results.json";
pub const TOURNAMENT_REPORT_FILE: &str = "tournament-report.md";

/// The host the report was assembled on — part of the evidence record.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EnvironmentIdentity {
    pub operating_system: String,
    pub architecture: String,
    pub logical_cpus: usize,
    pub rustc: String,
    pub cargo: String,
}

fn environment_identity() -> EnvironmentIdentity {
    EnvironmentIdentity {
        operating_system: std::env::consts::OS.to_string(),
        architecture: std::env::consts::ARCH.to_string(),
        logical_cpus: std::thread::available_parallelism().map_or(1, usize::from),
        rustc: option_env!("KOI_RUSTC_VERSION")
            .unwrap_or("rustc (unstamped)")
            .to_string(),
        cargo: "cargo (unstamped)".to_string(),
    }
}

/// One unordered pairing's summarized evidence plus the full leg record.
/// `second` plays the `Candidate` role, so `summary.candidate_*` is the
/// second entrant's score — mirrored into both matrix cells.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PairingEvidence {
    /// Index into `TournamentReport::entrants` mapped to the `Baseline` role.
    pub first: usize,
    /// Index into `TournamentReport::entrants` mapped to the `Candidate` role.
    pub second: usize,
    /// The stopping outcome the pairwise run recorded.
    pub stopping: crate::sprt::StoppingOutcome,
    pub summary: BenchmarkSummary,
    pub seeds: Vec<crate::runner::SeedResult>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TournamentReport {
    /// 1: Davidson-ties BT fit inside the joint deal-index bootstrap,
    /// Holm pairwise layer, cyclic-triple scan, stopping provenance.
    pub report_schema_version: u32,
    pub manifest_path: String,
    pub output_directory: String,
    pub manifest: crate::manifest::TournamentManifest,
    pub referee: crate::artifact::RefereeIdentity,
    pub entrants: Vec<crate::manifest::ArtifactIdentity>,
    /// Non-primary artifact builds attested for entrants across pairings
    /// — a merged report whose supplemental pairs ran under a different
    /// binary discloses every build here rather than implying one.
    #[serde(default)]
    pub additional_entrant_builds: Vec<crate::manifest::ArtifactIdentity>,
    pub cargo_lock_hash_fx64: String,
    pub environment: EnvironmentIdentity,
    pub pairings: Vec<PairingEvidence>,
    /// `win_score_matrix[i][j]` = entrant i's leg win score against j over
    /// that pairing's mirrored legs; the diagonal is empty.
    pub win_score_matrix: Vec<Vec<Option<f64>>>,
    /// `margin_matrix[i][j]` = entrant i's mean margin against j.
    pub margin_matrix: Vec<Vec<Option<f64>>>,
    pub total_invalid_legs: usize,
    pub total_time_forfeit_legs: usize,
    /// The Davidson-ties Bradley–Terry strength fit with cluster-bootstrap
    /// CIs and the cyclic-triple diagnostic — the published ranking.
    pub ranking: crate::ranking::TournamentRanking,
    /// The expanded declared panel size — how many clusters each pairing
    /// *could* have played under the frozen protocol, distinct from how
    /// many sequential stopping actually consumed.
    pub declared_clusters: usize,
}

/// Merges per-pairing run evidence into the tournament report. The caller
/// supplies the expanded panel size for the completeness provenance check.
pub fn assemble_tournament_report(
    manifest: crate::manifest::TournamentManifest,
    run: crate::runner::TournamentRun,
    manifest_path: &Path,
    output_directory: &Path,
    declared_clusters: usize,
) -> Result<TournamentReport> {
    let n = run.entrants.len();
    // Rank before consuming `run.pairings`: the borrow ends before the
    // seeds are moved into evidence, so peak memory stays at one copy of
    // the traces, not two.
    let ranking = crate::ranking::rank_tournament(n, &run.pairings, &manifest, declared_clusters)?;
    let mut win_score_matrix = vec![vec![None; n]; n];
    let mut margin_matrix = vec![vec![None; n]; n];
    let mut pairings = Vec::with_capacity(run.pairings.len());
    let mut total_invalid_legs = 0;
    let mut total_time_forfeit_legs = 0;
    for pairing in run.pairings {
        let summary = summarize(&pairing.seeds, &manifest.bootstrap)?;
        // `candidate_*` is the second entrant's score against the first;
        // the matrix mirrors it into both orientations.
        win_score_matrix[pairing.second][pairing.first] = Some(summary.candidate_win_score.estimate);
        win_score_matrix[pairing.first][pairing.second] = Some(1.0 - summary.candidate_win_score.estimate);
        margin_matrix[pairing.second][pairing.first] = Some(summary.candidate_margin.estimate);
        margin_matrix[pairing.first][pairing.second] = Some(-summary.candidate_margin.estimate);
        total_invalid_legs += summary.invalid_legs;
        total_time_forfeit_legs += summary.time_forfeit_legs;
        pairings.push(PairingEvidence {
            first: pairing.first,
            second: pairing.second,
            stopping: pairing.stopping,
            summary,
            seeds: pairing.seeds,
        });
    }
    Ok(TournamentReport {
        report_schema_version: 1,
        manifest_path: manifest_path.display().to_string(),
        output_directory: output_directory.display().to_string(),
        manifest,
        referee: run.referee,
        entrants: run.entrants,
        additional_entrant_builds: run.additional_entrant_builds,
        cargo_lock_hash_fx64: run.cargo_lock_hash_fx64,
        environment: environment_identity(),
        pairings,
        win_score_matrix,
        margin_matrix,
        total_invalid_legs,
        total_time_forfeit_legs,
        ranking,
        declared_clusters,
    })
}

/// Writes `tournament-results.json` + `tournament-report.md`.
pub fn write_tournament_report(report: &TournamentReport, output_dir: &Path) -> Result<(PathBuf, PathBuf)> {
    std::fs::create_dir_all(output_dir)?;
    let json_path = output_dir.join(TOURNAMENT_RESULTS_FILE);
    std::fs::write(&json_path, serde_json::to_string_pretty(report)?)
        .with_context(|| format!("failed to write {}", json_path.display()))?;
    let markdown_path = output_dir.join(TOURNAMENT_REPORT_FILE);
    std::fs::write(&markdown_path, tournament_markdown(report))
        .with_context(|| format!("failed to write {}", markdown_path.display()))?;
    Ok((json_path, markdown_path))
}

fn tournament_markdown(report: &TournamentReport) -> String {
    let mut output = format!(
        "# Tournament: {}\n\n\
         Entrants: **{}**; pairings: **{}**; clusters per pairing (declared panel): **{}**  \n\
         Referee: `{}`{}  \n\
         Total invalid legs: **{}**; total time-forfeit legs: **{}**  \n\
         Hard worker deadline: `{}` seconds per decision\n\n",
        report.manifest.run_label,
        report.entrants.len(),
        report.pairings.len(),
        report.declared_clusters,
        report.referee.build_commit,
        if report.referee.build_dirty { " (dirty)" } else { "" },
        report.total_invalid_legs,
        report.total_time_forfeit_legs,
        report.manifest.worker_timeout_seconds,
    );

    output.push_str(
        "## Provenance\n\n| # | Entrant | Commit | Tree digest | Binary hash | Dirty |\n|---|---|---|---|---|---|\n",
    );
    for (index, entrant) in report.entrants.iter().enumerate() {
        output.push_str(&format!(
            "| {index} | `{}` | `{}` | `{}` | `{}` | `{}` |\n",
            entrant.label, entrant.build_commit, entrant.build_tree, entrant.binary_hash_fnv1a64, entrant.build_dirty,
        ));
    }
    if !report.additional_entrant_builds.is_empty() {
        output.push_str(
            "\nAdditional builds attested across pairings (same label + \
             config hash; legs pooled across binaries — per-pairing \
             attestation lives in each pair's report):\n\n\
             | Entrant | Commit | Tree digest | Binary hash | Dirty |\n|---|---|---|---|---|\n",
        );
        for build in &report.additional_entrant_builds {
            output.push_str(&format!(
                "| `{}` | `{}` | `{}` | `{}` | `{}` |\n",
                build.label, build.build_commit, build.build_tree, build.binary_hash_fnv1a64, build.build_dirty,
            ));
        }
    }
    output.push('\n');

    output.push_str(&matrix_table(
        "Win score (row vs column)",
        &report.entrants,
        &report.win_score_matrix,
        3,
    ));
    output.push_str(&matrix_table(
        "Mean margin (row vs column)",
        &report.entrants,
        &report.margin_matrix,
        2,
    ));
    output.push_str("## Ranking (Davidson-ties Bradley–Terry)\n\n");
    output.push_str(&format!(
        "Tie parameter nu: **{:.4}** [{:.4}, {:.4}]; log-likelihood {:.3}; converged: `{}` (bootstrap refits converged: {:.1}%); stopping: `{}`; headline fit on the {}-cluster common prefix  \n",
        report.ranking.tie_nu,
        report.ranking.tie_nu_ci[0],
        report.ranking.tie_nu_ci[1],
        report.ranking.log_likelihood,
        report.ranking.converged,
        100.0 * report.ranking.bootstrap_converged_fraction,
        report.ranking.stopping_provenance,
        report.ranking.common_prefix_clusters,
    ));
    output.push_str(
        "\n| Entrant | W/D/L | Win score | Margin | Elo | x-Multiplier | Latency mean/p95 |\n\
         |---|---|---:|---:|---:|---:|---|\n",
    );
    let mut rows: Vec<_> = report.ranking.entrants.iter().collect();
    rows.sort_by(|a, b| b.elo.total_cmp(&a.elo));
    for ranked in rows {
        let entrant = &report.entrants[ranked.entrant];
        output.push_str(&format!(
            "| `{}` | {}/{}/{} | {:.4} [{:.4}, {:.4}] | {:+.3} [{:+.3}, {:+.3}] | {:+.1} [{:+.1}, {:+.1}] | {:.2}x [{:.2}, {:.2}] | {:.1} [{:.1}, {:.1}] / {:.1} [{:.1}, {:.1}] ms |\n",
            entrant.label,
            ranked.wins,
            ranked.draws,
            ranked.losses,
            ranked.win_score,
            ranked.win_score_ci[0],
            ranked.win_score_ci[1],
            ranked.margin,
            ranked.margin_ci[0],
            ranked.margin_ci[1],
            ranked.elo,
            ranked.elo_ci[0],
            ranked.elo_ci[1],
            ranked.x_multiplier,
            ranked.x_multiplier_ci[0],
            ranked.x_multiplier_ci[1],
            ranked.latency_mean_ms,
            ranked.latency_mean_ci[0],
            ranked.latency_mean_ci[1],
            ranked.latency_p95_ms,
            ranked.latency_p95_ci[0],
            ranked.latency_p95_ci[1],
        ));
    }
    output.push_str(&format!(
        "\n*Legend: win score is the {{0, 0.5, 1}} leg mean; margin is the mean yaku-point \
         difference; Elo is the Davidson-ties Bradley-Terry strength (gamma normalized to \
         geometric mean 1, reported as 400*log10 gamma centered to mean 0); the x-multiplier \
         is the strength ratio gamma_i/gamma_min -- a win-score ratio degenerates when the floor is ~0; \
         latency is the referee-measured per-decision milliseconds. Every number carries a \
         {:.0}% cluster-bootstrap interval over {} resamples (one deal-index draw over the \
         common prefix applied jointly to every pairing -- the mirrored-panel covariance \
         across pairings is preserved). The headline fit uses only the common prefix shared \
         by every pairing, so sequentially stopped pairs contribute outcome-independent \
         evidence; stopping provenance: `{}`. Rankings are the best among the evaluated \
         configurations under the frozen protocol and budget -- measured tournament \
         evidence, not a certified optimality claim.*\n\n",
        report.manifest.bootstrap.confidence_level * 100.0,
        report.ranking.bootstrap_repetitions,
        report.ranking.stopping_provenance,
    ));
    if let Some(full) = &report.ranking.descriptive_full_fit {
        output.push_str(
            "### Descriptive full-data fit (selection-biased -- not the headline)\n\n\
             *The fit below pools each pairing's untruncated evidence, including clusters \
             the stopping rule selected. Sequential stopping makes this basis \
             selection-biased; it is reported for context only and carries no intervals.*\n\n\
             | Entrant | W/D/L (full data) | Elo (full-data fit) |\n|---|---:|---:|\n",
        );
        let mut rows: Vec<usize> = (0..full.elo.len()).collect();
        rows.sort_by(|a, b| full.elo[*b].total_cmp(&full.elo[*a]));
        for entrant in rows {
            let [wins, draws, losses] = full.records[entrant];
            output.push_str(&format!(
                "| `{}` | {}/{}/{} | {:+.1} |\n",
                report.entrants[entrant].label, wins, draws, losses, full.elo[entrant],
            ));
        }
        output.push_str(&format!(
            "\n*Full-data fit: log-likelihood {:.3}, nu {:.4}, converged: `{}`; clusters per \
             pairing: {:?}.*\n\n",
            full.log_likelihood, full.tie_nu, full.converged, full.clusters_per_pairing,
        ));
    }
    output.push_str(&intransitivity_markdown(report));
    output.push_str(&pairwise_markdown(report));

    output.push_str("## Pairings\n\n| First | Second | Second win score | Second margin | Stopping | Invalid | Time forfeit |\n|---|---|---:|---:|---|---:|---:|\n");
    for pairing in &report.pairings {
        let first = &report.entrants[pairing.first];
        let second = &report.entrants[pairing.second];
        output.push_str(&format!(
            "| `{}` | `{}` | {:.6} | {:.3} | `{}` | {} | {} |\n",
            first.label,
            second.label,
            pairing.summary.candidate_win_score.estimate,
            pairing.summary.candidate_margin.estimate,
            pairing.stopping.kind_name(),
            pairing.summary.invalid_legs,
            pairing.summary.time_forfeit_legs,
        ));
    }
    output.push('\n');
    output
}

/// The cyclic-triple diagnostic, with the Nash-averaging mixture appended
/// only when cycles were actually witnessed — a transitive table needs no
/// caveat, a cyclic one reports the game-theoretic mixture instead of
/// pretending the scalar rating is safe.
fn intransitivity_markdown(report: &TournamentReport) -> String {
    let cycles = &report.ranking.intransitivity;
    if cycles.cyclic_triples == 0 {
        return format!(
            "Intransitivity: none detected (0 of {} triples cyclic) — the scalar ranking is reliable.\n\n",
            cycles.total_triples,
        );
    }
    let mut output = format!(
        "**Intransitivity detected: {} of {} triples ({:.1}%) are strict win-score cycles.** \
         The scalar BT rating can mislead under cycles — the Nash-averaging mixture below is the \
         reliable summary. Witnessed cycles (i beats j beats k beats i):\n\n",
        cycles.cyclic_triples,
        cycles.total_triples,
        cycles.fraction * 100.0,
    );
    for [a, b, c] in &cycles.cycles {
        output.push_str(&format!(
            "- `{}` → `{}` → `{}` → `{}`\n",
            report.entrants[*a].label, report.entrants[*b].label, report.entrants[*c].label, report.entrants[*a].label,
        ));
    }
    if let (Some(weights), Some(scores)) = (&cycles.nash_weights, &cycles.nash_expected_win_score) {
        output.push_str("\n| Entrant | Nash weight | Expected win score under the mixture |\n|---|---:|---:|\n");
        for entrant in 0..report.entrants.len() {
            output.push_str(&format!(
                "| `{}` | {:.4} | {:.4} |\n",
                report.entrants[entrant].label, weights[entrant], scores[entrant],
            ));
        }
        output.push('\n');
    }
    output
}

/// The multiplicity-controlled pairwise layer: Holm step-down over every
/// played pairing on the cluster-bootstrapped win-score advantage. This
/// section is where "i beats j" claims live — the matrices and the BT
/// table above remain descriptive evidence.
fn pairwise_markdown(report: &TournamentReport) -> String {
    let pairwise = &report.ranking.pairwise;
    let mut output = format!(
        "## Pairwise comparisons (Holm step-down, family alpha = {:.3})\n\n\
         Each row tests one played pairing's win-score advantage (`second` − even) against \
         level play; raw p is the two-sided recentered cluster-bootstrap exceedance, the CI \
         is its descriptive percentile interval. The Holm column controls the family-wise \
         error rate across all {} tests at alpha = {:.3} — rows marked **separated** are the \
         multiplicity-controlled claims; everything else is descriptive only.\n\n\
         | First | Second | Second advantage | CI | p (raw) | p (Holm) | Verdict |\n\
         |---|---|---:|---:|---:|---:|---|\n",
        pairwise.family_alpha, pairwise.family_size, pairwise.family_alpha,
    );
    let mut rows: Vec<_> = pairwise.tests.iter().collect();
    rows.sort_by(|a, b| a.holm_p.total_cmp(&b.holm_p));
    for test in rows {
        let verdict = if test.significant { "**separated**" } else { "level" };
        output.push_str(&format!(
            "| `{}` | `{}` | {:+.4} | [{:+.4}, {:+.4}] | {:.4} | {:.4} | {} |\n",
            report.entrants[test.first].label,
            report.entrants[test.second].label,
            test.win_score_diff,
            test.win_score_diff_ci[0],
            test.win_score_diff_ci[1],
            test.raw_p,
            test.holm_p,
            verdict,
        ));
    }
    output
}

fn matrix_table(
    title: &str,
    entrants: &[crate::manifest::ArtifactIdentity],
    matrix: &[Vec<Option<f64>>],
    decimals: usize,
) -> String {
    let mut output = format!("## {title}\n\n| |");
    for entrant in entrants {
        output.push_str(&format!(" `{}` |", entrant.label));
    }
    output.push_str("\n|---|");
    for _ in entrants {
        output.push_str("---:|");
    }
    output.push('\n');
    for (row_index, row) in matrix.iter().enumerate() {
        output.push_str(&format!("| `{}` |", entrants[row_index].label));
        for cell in row {
            match cell {
                Some(value) => output.push_str(&format!(" {:.decimals$} |", value)),
                None => output.push_str(" — |"),
            }
        }
        output.push('\n');
    }
    output.push('\n');
    output
}
