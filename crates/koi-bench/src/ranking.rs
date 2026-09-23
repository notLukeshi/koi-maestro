//! N×N ranking for tournament runs: a Davidson-ties Bradley–Terry strength
//! fit refit inside the cluster bootstrap, plus an intransitivity
//! diagnostic (cyclic win-score triples) and a Nash-averaging report when
//! cycles appear — the published-table contract.
//!
//! The model: for entrants i, j with strengths γ_i, γ_j and tie parameter
//! ν ≥ 0, `P(i beats j) = γ_i / (γ_i + γ_j + ν·√(γ_iγ_j))` and
//! `P(tie) = ν·√(γ_iγ_j) / (same denominator)`. Fitting maximizes the
//! multinomial log-likelihood over the per-pairing win/draw/loss counts in
//! the log parameters `(θ_i = log γ_i, φ = log ν)` — the objective is
//! concave there (log-sum-exp of affine functions), so gradient ascent
//! with backtracking reaches the global maximum.
//!
//! Confidence intervals come from the cluster bootstrap: each resample
//! redraws the deal clusters inside every pairing, rebuilds the W/D/L
//! table, and refits — mirroring the paired harness's statistic bootstrap
//! so strength claims carry the same deal-level uncertainty.
//!
//! Ported from `sibling-bench/src/benchmark/ranking.rs` — the algorithm is
//! unchanged; koi adaptations are the leg pair (`candidate_deals` /
//! `baseline_deals`), the `win_score ∈ {0, 0.5, 1}` estimand on a single
//! margin, and the `TournamentManifest` protocol block.

use anyhow::{bail, Result};
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

use crate::{
    manifest::TournamentManifest,
    runner::{ArtifactRole, PairingRun},
};

/// Log-parameter bounds: an entrant with no observed wins pushes γ_i → 0
/// and a sweep winner pushes it up; these keep the iterates finite so the
/// fit reports an extreme-but-honest strength instead of diverging.
const THETA_BOUND: f64 = 30.0;
const PHI_BOUND: f64 = 30.0;
/// Ascent budget — warm-started resamples converge in a handful of steps;
/// the cold point fit takes a few hundred.
const MAX_FIT_ITERATIONS: usize = 2_000;
/// Convergence on the log-likelihood improvement.
const LL_TOLERANCE: f64 = 1e-10;
/// Fictitious-play budget for the Nash-averaging diagnostic — symmetric
/// zero-sum convergence is fast at N ≤ 16.
const NASH_ITERATIONS: usize = 50_000;

/// One unordered pairing's aggregated leg outcomes.
#[derive(Debug, Clone, Copy)]
pub struct PairTable {
    pub first: usize,
    pub second: usize,
    pub first_wins: usize,
    pub ties: usize,
    pub second_wins: usize,
}

/// The maximum-likelihood Davidson-ties fit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DavidsonFit {
    /// γ, normalized to geometric mean 1 — the scale is unidentified, so
    /// only ratios are meaningful.
    pub strengths: Vec<f64>,
    /// `400·log10(γ)`, re-centered to mean 0 — the published Elo column.
    pub elo: Vec<f64>,
    /// The Davidson tie parameter ν.
    pub tie_nu: f64,
    pub log_likelihood: f64,
    pub iterations: usize,
    /// False when the iteration budget ran out or a parameter sits at its
    /// bound — the fit is still reported, flagged.
    pub converged: bool,
}

fn log_likelihood_and_gradient(theta: &[f64], phi: f64, pairs: &[PairTable]) -> (f64, Vec<f64>, f64) {
    let mut ll = 0.0;
    let mut grad_theta = vec![0.0; theta.len()];
    let mut grad_phi = 0.0;
    for pair in pairs {
        let games = (pair.first_wins + pair.ties + pair.second_wins) as f64;
        if games == 0.0 {
            continue;
        }
        let (i, j) = (pair.first, pair.second);
        let gamma_i = theta[i].exp();
        let gamma_j = theta[j].exp();
        let tie_term = (phi + (theta[i] + theta[j]) / 2.0).exp();
        let denom = gamma_i + gamma_j + tie_term;
        ll += pair.first_wins as f64 * theta[i]
            + pair.second_wins as f64 * theta[j]
            + pair.ties as f64 * (phi + (theta[i] + theta[j]) / 2.0)
            - games * denom.ln();
        // ∂/∂θ_i gets the win mass, half the tie mass (the √ numerator
        // splits its log between the pair), minus the expected terms.
        grad_theta[i] += pair.first_wins as f64 + pair.ties as f64 / 2.0 - games * (gamma_i + tie_term / 2.0) / denom;
        grad_theta[j] += pair.second_wins as f64 + pair.ties as f64 / 2.0 - games * (gamma_j + tie_term / 2.0) / denom;
        grad_phi += pair.ties as f64 - games * tie_term / denom;
    }
    (ll, grad_theta, grad_phi)
}

/// Fits the Davidson-ties model by gradient ascent with backtracking.
/// `warm` optionally starts from a previous fit — the bootstrap refits
/// from the point estimate.
pub fn fit_davidson(entrants: usize, pairs: &[PairTable], warm: Option<(&[f64], f64)>) -> DavidsonFit {
    let mut theta = warm
        .map(|(theta, _)| theta.to_vec())
        .unwrap_or_else(|| vec![0.0; entrants]);
    let mut phi = warm.map(|(_, phi)| phi).unwrap_or(0.0);
    let (mut ll, mut grad_theta, mut grad_phi) = log_likelihood_and_gradient(&theta, phi, pairs);
    let mut converged = false;
    let mut iterations = 0;
    for iteration in 0..MAX_FIT_ITERATIONS {
        iterations = iteration + 1;
        // Backtracking line search: the objective is concave, so any step
        // that increases the likelihood is uphill progress toward the max.
        let mut step = 1.0;
        let mut accepted = false;
        for _ in 0..60 {
            let candidate_theta: Vec<f64> = theta
                .iter()
                .zip(&grad_theta)
                .map(|(t, g)| (t + step * g).clamp(-THETA_BOUND, THETA_BOUND))
                .collect();
            let candidate_phi = (phi + step * grad_phi).clamp(-PHI_BOUND, PHI_BOUND);
            let (candidate_ll, ..) = log_likelihood_and_gradient(&candidate_theta, candidate_phi, pairs);
            if candidate_ll > ll {
                let improvement = candidate_ll - ll;
                theta = candidate_theta;
                phi = candidate_phi;
                accepted = true;
                ll = candidate_ll;
                let (_, next_theta, next_phi) = log_likelihood_and_gradient(&theta, phi, pairs);
                grad_theta = next_theta;
                grad_phi = next_phi;
                // Converged means a near-stationary point: tiny improvement
                // AND a near-zero gradient — a bound-pinned parameter with
                // a live gradient is a diverged MLE, not a stationary one.
                if improvement < LL_TOLERANCE && grad_theta.iter().all(|g| g.abs() < 1e-6) && grad_phi.abs() < 1e-6 {
                    converged = true;
                }
                break;
            }
            step *= 0.5;
            if step < 1e-12 {
                break;
            }
        }
        if converged {
            break;
        }
        if !accepted {
            // No uphill step exists along the gradient — at a maximum or a
            // bound-pinned edge; either way there is nothing left to climb.
            converged = grad_theta.iter().all(|g| g.abs() < 1e-6) && grad_phi.abs() < 1e-6;
            break;
        }
    }
    // Report at the identified scale: geometric mean 1 in γ, mean 0 in Elo.
    // A parameter pinned at its bound means the MLE diverged — the fit is
    // still reported, but `converged` stays false so the flag is honest.
    let pinned = theta.iter().any(|t| t.abs() >= THETA_BOUND - 1e-9) || phi.abs() >= PHI_BOUND - 1e-9;
    let mean_theta = theta.iter().sum::<f64>() / theta.len() as f64;
    let strengths: Vec<f64> = theta.iter().map(|t| (t - mean_theta).exp()).collect();
    let elo: Vec<f64> = strengths
        .iter()
        .map(|gamma| 400.0 * gamma.max(f64::MIN_POSITIVE).log10())
        .collect();
    let mean_elo = elo.iter().sum::<f64>() / elo.len() as f64;
    DavidsonFit {
        strengths,
        elo: elo.iter().map(|value| value - mean_elo).collect(),
        tie_nu: phi.exp(),
        log_likelihood: ll,
        iterations,
        converged: converged && !pinned,
    }
}

/// Per-entrant row of the published table, with cluster-bootstrap CIs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntrantRanking {
    /// Index into `TournamentReport::entrants`.
    pub entrant: usize,
    pub wins: usize,
    pub draws: usize,
    pub losses: usize,
    /// Mean leg win score over every pairing this entrant played.
    pub win_score: f64,
    pub win_score_ci: [f64; 2],
    /// Mean leg margin over the same legs.
    pub margin: f64,
    pub margin_ci: [f64; 2],
    /// Davidson γ at geometric-mean-1 normalization.
    pub strength: f64,
    pub strength_ci: [f64; 2],
    /// Elo at mean-0 centering.
    pub elo: f64,
    pub elo_ci: [f64; 2],
    /// γ_i / γ_min — the x-multiplier: a strength ratio stays informative
    /// when the floor entrant's win score is ≈ 0. The interval resamples
    /// the floor too — each replicate is γ_i/γ_min of *that* fit.
    pub x_multiplier: f64,
    pub x_multiplier_ci: [f64; 2],
    /// Referee-measured per-decision latency, net of the worker solver
    /// cache; both columns carry cluster-bootstrap CIs like every other
    /// published number.
    pub latency_mean_ms: f64,
    pub latency_mean_ci: [f64; 2],
    pub latency_p95_ms: f64,
    pub latency_p95_ci: [f64; 2],
}

/// Cyclic-triple evidence plus, when cycles exist, the Nash-averaging
/// mixture over the empirical win-score game.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntransitivityReport {
    pub total_triples: usize,
    pub cyclic_triples: usize,
    /// Fraction of unordered triples whose pairwise win scores form a
    /// strict cycle — the scalar-rating reliability flag.
    pub fraction: f64,
    /// Up to eight witnessed cycles, strongest first — each as
    /// `[i, j, k]` meaning i beat j, j beat k, k beat i.
    pub cycles: Vec<[usize; 3]>,
    /// Present only when cycles were witnessed: the max-ent NE mixture
    /// (fictitious play on the antisymmetric win-score game) and each
    /// entrant's expected win score under it.
    pub nash_weights: Option<Vec<f64>>,
    pub nash_expected_win_score: Option<Vec<f64>>,
}

/// One pairwise win-score claim: `second`'s mean leg score minus 0.5, so
/// a positive diff says `second` outscored `first` in the mirror.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairwiseTest {
    pub first: usize,
    pub second: usize,
    /// Observed win-score advantage of `second` over `first`.
    pub win_score_diff: f64,
    /// Descriptive cluster-bootstrap interval — *not* the simultaneous
    /// family statement; the Holm-adjusted p carries that.
    pub win_score_diff_ci: [f64; 2],
    /// Two-sided bootstrap p: recentered resample exceedance of the
    /// observed advantage, with the (k+1)/(R+1) Monte-Carlo correction.
    pub raw_p: f64,
    /// Holm step-down adjusted p over every played pairing.
    pub holm_p: f64,
    /// True when `holm_p <= family_alpha` — the multiplicity-controlled
    /// claim that the pair is not level.
    pub significant: bool,
}

/// The multiplicity-controlled layer over the pairing matrix: one test
/// per played pairing on the cluster-bootstrapped win-score advantage,
/// Holm step-down adjusted so the family-wise error rate holds at
/// `family_alpha` regardless of cross-pairing dependence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairwiseComparisons {
    pub method: String,
    pub family_alpha: f64,
    pub family_size: usize,
    pub tests: Vec<PairwiseTest>,
}

/// The full-data fit over a truncated (adaptively stopped) schedule —
/// descriptive context only. Sequential stopping selects the data the
/// fit sees, so these strengths are selection-biased and carry no CIs;
/// the headline ranking is the common-prefix fit (catalog T-04/T-06).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DescriptiveFullFit {
    /// Clusters each pairing actually played — the stopped schedule's
    /// shape. `common_prefix_clusters` is the minimum of these.
    pub clusters_per_pairing: Vec<usize>,
    pub strengths: Vec<f64>,
    pub elo: Vec<f64>,
    pub tie_nu: f64,
    pub log_likelihood: f64,
    pub converged: bool,
    /// Raw pooled (wins, draws, losses) per entrant on the untruncated
    /// data — includes whatever the stopping rule selected.
    pub records: Vec<[u32; 3]>,
}

/// The ranking section of a tournament report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TournamentRanking {
    pub method: String,
    /// How the schedule was stopped — BT strengths fit over
    /// sequentially-stopped pairings carry selection bias, so the
    /// published fit records the provenance.
    pub stopping_provenance: String,
    /// The common-prefix cluster count the headline fit used: every
    /// pairing contributes its first `prefix` clusters, so stopped
    /// pairings enter only through outcome-independent evidence. Equals
    /// the full deal count on a complete schedule.
    pub common_prefix_clusters: usize,
    /// The selection-biased full-data fit — `Some` only when at least
    /// one pairing played beyond the common prefix (an adaptive or
    /// partial schedule), `None` when the bases coincide.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub descriptive_full_fit: Option<DescriptiveFullFit>,
    pub bootstrap_repetitions: usize,
    pub tie_nu: f64,
    pub tie_nu_ci: [f64; 2],
    pub log_likelihood: f64,
    pub converged: bool,
    /// Fraction of bootstrap resamples whose Davidson refit converged —
    /// nonconverged (bound-pinned) replicates still enter the CIs, so a
    /// value materially below 1.0 means the intervals mix in fits the
    /// optimizer never certified.
    pub bootstrap_converged_fraction: f64,
    pub entrants: Vec<EntrantRanking>,
    /// Holm step-down pairwise tests on the cluster-bootstrapped
    /// win-score advantage — the multiplicity-controlled claims; the
    /// strength table above stays descriptive.
    pub pairwise: PairwiseComparisons,
    pub intransitivity: IntransitivityReport,
}

/// Aggregates one pairing's legs into a `PairTable` (second plays the
/// candidate role; its win score is the leg's `candidate_win_score`).
fn pair_table(pairing: &PairingRun, draw: &[usize]) -> PairTable {
    let mut table = PairTable {
        first: pairing.first,
        second: pairing.second,
        first_wins: 0,
        ties: 0,
        second_wins: 0,
    };
    for &index in draw {
        for leg in [
            &pairing.seeds[index].candidate_deals,
            &pairing.seeds[index].baseline_deals,
        ] {
            if leg.candidate_win_score == 1.0 {
                table.second_wins += 1;
            } else if leg.candidate_win_score == 0.5 {
                table.ties += 1;
            } else {
                table.first_wins += 1;
            }
        }
    }
    table
}

/// The full ranking: point fit on the observed table, then a refit per
/// cluster-bootstrap resample for the CIs. `declared_clusters` is the
/// frozen panel's size — the "complete schedule" provenance check compares
/// each pairing's played count against it.
pub fn rank_tournament(
    entrants: usize,
    pairings: &[PairingRun],
    manifest: &TournamentManifest,
    declared_clusters: usize,
) -> Result<TournamentRanking> {
    if entrants < 2 {
        bail!("a ranking requires at least two entrants");
    }
    if pairings.iter().any(|pairing| pairing.seeds.is_empty()) {
        bail!("every pairing needs at least one deal cluster");
    }
    let bootstrap = &manifest.bootstrap;
    // T-06 common-prefix basis: the headline fit truncates every pairing
    // to the shortest played prefix. A pairing stopped at cluster τ > τ_min
    // contributes only its first τ_min clusters — evidence whose inclusion
    // is independent of the outcomes (the stopping decision can only
    // truncate, never select within the prefix). On a complete schedule the
    // prefix is the whole panel and the bases coincide.
    let prefix_len = pairings.iter().map(|pairing| pairing.seeds.len()).min().unwrap_or(0);
    let max_len = pairings.iter().map(|pairing| pairing.seeds.len()).max().unwrap_or(0);
    let truncated = max_len > prefix_len;
    let observed: Vec<PairTable> = pairings
        .iter()
        .map(|pairing| pair_table(pairing, &(0..prefix_len).collect::<Vec<_>>()))
        .collect();
    let fit = fit_davidson(entrants, &observed, None);
    let point_theta: Vec<f64> = fit.strengths.iter().map(|s| s.ln()).collect();
    let point_phi = fit.tie_nu.max(f64::MIN_POSITIVE).ln();

    // The descriptive full-data fit: reported for context when the
    // schedule was truncated, flagged selection-biased, never the headline.
    let descriptive_full_fit = if truncated {
        let observed_full: Vec<PairTable> = pairings
            .iter()
            .map(|pairing| pair_table(pairing, &(0..pairing.seeds.len()).collect::<Vec<_>>()))
            .collect();
        let full = fit_davidson(entrants, &observed_full, None);
        let records = (0..entrants)
            .map(|entrant| {
                let mut record = [0u32; 3];
                for pairing in pairings {
                    let as_candidate = if pairing.second == entrant {
                        Some(true)
                    } else if pairing.first == entrant {
                        Some(false)
                    } else {
                        None
                    };
                    let Some(as_candidate) = as_candidate else { continue };
                    for seed in &pairing.seeds {
                        for leg in [&seed.candidate_deals, &seed.baseline_deals] {
                            let score = if as_candidate {
                                leg.candidate_win_score
                            } else {
                                1.0 - leg.candidate_win_score
                            };
                            if score == 1.0 {
                                record[0] += 1;
                            } else if score == 0.5 {
                                record[1] += 1;
                            } else {
                                record[2] += 1;
                            }
                        }
                    }
                }
                record
            })
            .collect();
        Some(DescriptiveFullFit {
            clusters_per_pairing: pairings.iter().map(|pairing| pairing.seeds.len()).collect(),
            strengths: full.strengths,
            elo: full.elo,
            tie_nu: full.tie_nu,
            log_likelihood: full.log_likelihood,
            converged: full.converged,
            records,
        })
    } else {
        None
    };

    // Per-cluster referee latencies, split by role once so bootstrap
    // resamples can redraw them at cluster granularity — the latency CIs
    // carry the same deal-level uncertainty as the strength columns.
    // `cluster_latencies[p][s]` = (first's ms, second's ms) in cluster s of
    // pairing p; the Candidate role is `second`'s.
    let cluster_latencies: Vec<Vec<(Vec<f64>, Vec<f64>)>> = pairings
        .iter()
        .map(|pairing| {
            pairing
                .seeds
                .iter()
                .map(|seed| {
                    let mut first = Vec::new();
                    let mut second = Vec::new();
                    for leg in [&seed.candidate_deals, &seed.baseline_deals] {
                        for entry in &leg.trace {
                            match entry.artifact {
                                ArtifactRole::Candidate => second.push(entry.decision_latency_ms),
                                ArtifactRole::Baseline => first.push(entry.decision_latency_ms),
                            }
                        }
                    }
                    (first, second)
                })
                .collect()
        })
        .collect();

    // Cluster bootstrap: resample deal indices inside each pairing,
    // rebuild the W/D/L table, refit warm-started at the point estimate.
    // The same draws feed the per-entrant win-score/margin/latency
    // replicates, so every published number carries a cluster-level
    // interval.
    let mut rng = ChaCha8Rng::seed_from_u64(bootstrap.seed);
    let mut strength_replicates: Vec<Vec<f64>> = vec![Vec::new(); entrants];
    let mut elo_replicates: Vec<Vec<f64>> = vec![Vec::new(); entrants];
    let mut win_score_replicates: Vec<Vec<f64>> = vec![Vec::new(); entrants];
    let mut margin_replicates: Vec<Vec<f64>> = vec![Vec::new(); entrants];
    let mut latency_mean_replicates: Vec<Vec<f64>> = vec![Vec::new(); entrants];
    let mut latency_p95_replicates: Vec<Vec<f64>> = vec![Vec::new(); entrants];
    let mut x_replicates: Vec<Vec<f64>> = vec![Vec::new(); entrants];
    let mut nu_replicates = Vec::new();
    // Per-pairing resampled win-score advantages feed the pairwise
    // comparisons — the same cluster draws, so multiplicity inference and
    // the strength CIs share one resample world.
    let mut pair_diff_replicates: Vec<Vec<f64>> = vec![Vec::new(); pairings.len()];
    let mut converged_resamples = 0_usize;
    for _ in 0..bootstrap.repetitions {
        // Joint deal-index bootstrap: one draw over the common prefix
        // applied to every pairing — the same physical deal feeds all
        // C(N,2) legs, so the resample preserves the mirrored-panel
        // covariance across pairings instead of breaking it with
        // independent per-pair draws.
        let draw: Vec<usize> = (0..prefix_len).map(|_| rng.random_range(0..prefix_len)).collect();
        let mut table = Vec::with_capacity(pairings.len());
        let mut score_sums = vec![0.0; entrants];
        let mut margin_sums = vec![0.0; entrants];
        let mut leg_counts = vec![0_usize; entrants];
        let mut latency_draws: Vec<Vec<f64>> = vec![Vec::new(); entrants];
        let mut pair_score_sums = vec![0.0; pairings.len()];
        for (p, pairing) in pairings.iter().enumerate() {
            let mut draw_table = PairTable {
                first: pairing.first,
                second: pairing.second,
                first_wins: 0,
                ties: 0,
                second_wins: 0,
            };
            for &index in &draw {
                let seed = &pairing.seeds[index];
                let (first_lat, second_lat) = &cluster_latencies[p][index];
                latency_draws[pairing.first].extend_from_slice(first_lat);
                latency_draws[pairing.second].extend_from_slice(second_lat);
                for leg in [&seed.candidate_deals, &seed.baseline_deals] {
                    let score = leg.candidate_win_score;
                    let margin = leg.candidate_margin;
                    if score == 1.0 {
                        draw_table.second_wins += 1;
                    } else if score == 0.5 {
                        draw_table.ties += 1;
                    } else {
                        draw_table.first_wins += 1;
                    }
                    pair_score_sums[p] += score;
                    score_sums[pairing.second] += score;
                    margin_sums[pairing.second] += margin;
                    leg_counts[pairing.second] += 1;
                    score_sums[pairing.first] += 1.0 - score;
                    margin_sums[pairing.first] -= margin;
                    leg_counts[pairing.first] += 1;
                }
            }
            pair_diff_replicates[p].push(pair_score_sums[p] / (2 * prefix_len) as f64 - 0.5);
            table.push(draw_table);
        }
        let resample = fit_davidson(entrants, &table, Some((&point_theta, point_phi)));
        converged_resamples += resample.converged as usize;
        let resample_floor = resample
            .strengths
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min)
            .max(1e-9);
        for entrant in 0..entrants {
            strength_replicates[entrant].push(resample.strengths[entrant]);
            elo_replicates[entrant].push(resample.elo[entrant]);
            x_replicates[entrant].push(resample.strengths[entrant] / resample_floor);
            if leg_counts[entrant] > 0 {
                win_score_replicates[entrant].push(score_sums[entrant] / leg_counts[entrant] as f64);
                margin_replicates[entrant].push(margin_sums[entrant] / leg_counts[entrant] as f64);
            }
            let draws = &mut latency_draws[entrant];
            if !draws.is_empty() {
                latency_mean_replicates[entrant].push(mean(draws));
                // Nearest-rank p95 via partial selection — a full sort is
                // wasted work when only one order statistic is needed.
                let rank = ((0.95 * draws.len() as f64).ceil() as usize)
                    .saturating_sub(1)
                    .min(draws.len() - 1);
                let (_, p95, _) = draws.select_nth_unstable_by(rank, |a, b| a.total_cmp(b));
                latency_p95_replicates[entrant].push(*p95);
            }
        }
        nu_replicates.push(resample.tie_nu);
    }
    let tail = (1.0 - bootstrap.confidence_level) / 2.0;
    let ci = |values: &mut Vec<f64>| -> [f64; 2] {
        values.sort_by(f64::total_cmp);
        [percentile_sorted(values, tail), percentile_sorted(values, 1.0 - tail)]
    };
    nu_replicates.sort_by(f64::total_cmp);
    let nu_ci = [
        percentile_sorted(&nu_replicates, tail),
        percentile_sorted(&nu_replicates, 1.0 - tail),
    ];

    // Per-entrant aggregates across every pairing leg it played.
    let mut entrants_ranked = Vec::with_capacity(entrants);
    let min_strength = fit.strengths.iter().copied().fold(f64::INFINITY, f64::min);
    for entrant in 0..entrants {
        let (mut wins, mut draws, mut losses) = (0, 0, 0);
        let mut win_scores = Vec::new();
        let mut margins = Vec::new();
        let mut latencies = Vec::new();
        for pairing in pairings {
            let role = if pairing.second == entrant {
                Some(true)
            } else if pairing.first == entrant {
                Some(false)
            } else {
                None
            };
            let Some(as_candidate) = role else { continue };
            // The headline row shares the fit's common-prefix basis —
            // pooling full stopped data here would re-import the
            // selection bias the prefix exists to exclude.
            for seed in &pairing.seeds[..prefix_len] {
                for leg in [&seed.candidate_deals, &seed.baseline_deals] {
                    let score = if as_candidate {
                        leg.candidate_win_score
                    } else {
                        1.0 - leg.candidate_win_score
                    };
                    let margin = if as_candidate {
                        leg.candidate_margin
                    } else {
                        -leg.candidate_margin
                    };
                    win_scores.push(score);
                    margins.push(margin);
                    if score == 1.0 {
                        wins += 1;
                    } else if score == 0.5 {
                        draws += 1;
                    } else {
                        losses += 1;
                    }
                    for entry in &leg.trace {
                        let entrant_decided = matches!(entry.artifact, ArtifactRole::Candidate if as_candidate)
                            || matches!(entry.artifact, ArtifactRole::Baseline if !as_candidate);
                        if entrant_decided {
                            latencies.push(entry.decision_latency_ms);
                        }
                    }
                }
            }
        }
        latencies.sort_by(f64::total_cmp);
        let strength_ci = ci(&mut strength_replicates[entrant]);
        let elo_ci = ci(&mut elo_replicates[entrant]);
        let win_score_ci = ci(&mut win_score_replicates[entrant]);
        let margin_ci = ci(&mut margin_replicates[entrant]);
        let latency_mean_ci = ci(&mut latency_mean_replicates[entrant]);
        let latency_p95_ci = ci(&mut latency_p95_replicates[entrant]);
        let x_multiplier_ci = ci(&mut x_replicates[entrant]);
        entrants_ranked.push(EntrantRanking {
            entrant,
            wins,
            draws,
            losses,
            win_score: mean(&win_scores),
            win_score_ci,
            margin: mean(&margins),
            margin_ci,
            strength: fit.strengths[entrant],
            strength_ci,
            elo: fit.elo[entrant],
            elo_ci,
            x_multiplier: fit.strengths[entrant] / min_strength.max(1e-9),
            x_multiplier_ci,
            latency_mean_ms: if latencies.is_empty() { 0.0 } else { mean(&latencies) },
            latency_mean_ci,
            latency_p95_ms: if latencies.is_empty() {
                0.0
            } else {
                percentile_sorted(&latencies, 0.95)
            },
            latency_p95_ci,
        });
    }

    // Stopping provenance from the per-pairing outcomes the merge carried
    // through (catalog T-15): a complete fixed schedule is the only
    // provenance free of selection-bias risk; adaptively stopped pairings
    // name their outcome kinds so the report shows which pairs were
    // decided, certified equivalent, capped out, or attrited.
    let complete = pairings.iter().all(|pairing| pairing.seeds.len() == declared_clusters);
    let mut outcome_counts: std::collections::BTreeMap<&'static str, usize> = std::collections::BTreeMap::new();
    for pairing in pairings {
        *outcome_counts.entry(pairing.stopping.kind_name()).or_insert(0) += 1;
    }
    let all_complete_schedule = outcome_counts.len() == 1 && outcome_counts.contains_key("complete_schedule");
    let outcome_summary = outcome_counts
        .iter()
        .map(|(kind, count)| format!("{count}x{kind}"))
        .collect::<Vec<_>>()
        .join(",");
    let stopping_provenance = if all_complete_schedule && complete {
        "fixed_clusters_complete_schedule".to_owned()
    } else if all_complete_schedule {
        // Seeds missing without a stopping record = attrition, the
        // historical partial-schedule hazard. `complete_schedule` is the
        // only outcome kind present, so the suffix adds no information.
        "fixed_clusters_partial_schedule_selection_bias_possible".to_owned()
    } else {
        format!("adaptive_stopping_schedule({outcome_summary})_stopped_pairs_selection_biased")
    };

    // Multiplicity-controlled pairwise layer: each pairing's win-score
    // advantage gets a two-sided bootstrap p from the recentered resample
    // distribution (the standard null-shifted construction), then Holm
    // step-down across the family of played pairings. The family level
    // follows the manifest's confidence level, matching the CIs.
    let family_alpha = 1.0 - bootstrap.confidence_level;
    let mut raw_p = Vec::with_capacity(pairings.len());
    let mut pairwise_tests = Vec::with_capacity(pairings.len());
    for (p, pairing) in pairings.iter().enumerate() {
        // Pairwise tests share the headline's common-prefix basis — the
        // replicates already resample the joint prefix draw.
        let legs = (2 * prefix_len) as f64;
        let observed = pairing.seeds[..prefix_len]
            .iter()
            .flat_map(|seed| {
                [
                    seed.candidate_deals.candidate_win_score,
                    seed.baseline_deals.candidate_win_score,
                ]
            })
            .sum::<f64>()
            / legs
            - 0.5;
        let replicates = &mut pair_diff_replicates[p];
        let exceedances = replicates
            .iter()
            .filter(|replicate| (*replicate - observed).abs() >= observed.abs() - 1e-12)
            .count();
        let p_value = (exceedances + 1) as f64 / (replicates.len() + 1) as f64;
        raw_p.push(p_value);
        replicates.sort_by(f64::total_cmp);
        pairwise_tests.push(PairwiseTest {
            first: pairing.first,
            second: pairing.second,
            win_score_diff: observed,
            win_score_diff_ci: [
                percentile_sorted(replicates, tail),
                percentile_sorted(replicates, 1.0 - tail),
            ],
            raw_p: p_value,
            holm_p: 0.0,
            significant: false,
        });
    }
    let adjusted = holm_adjust(&raw_p);
    for (index, test) in pairwise_tests.iter_mut().enumerate() {
        test.holm_p = adjusted[index];
        test.significant = adjusted[index] <= family_alpha;
    }
    let pairwise = PairwiseComparisons {
        method: "holm_step_down_cluster_bootstrap".to_owned(),
        family_alpha,
        family_size: pairwise_tests.len(),
        tests: pairwise_tests,
    };

    Ok(TournamentRanking {
        method: if truncated {
            "davidson_ties_bradley_terry_common_prefix".to_owned()
        } else {
            "davidson_ties_bradley_terry".to_owned()
        },
        stopping_provenance,
        common_prefix_clusters: prefix_len,
        descriptive_full_fit,
        bootstrap_repetitions: bootstrap.repetitions,
        tie_nu: fit.tie_nu,
        tie_nu_ci: nu_ci,
        log_likelihood: fit.log_likelihood,
        converged: fit.converged,
        bootstrap_converged_fraction: converged_resamples as f64 / bootstrap.repetitions.max(1) as f64,
        entrants: entrants_ranked,
        pairwise,
        intransitivity: detect_cycles(&win_score_matrix(entrants, pairings, prefix_len)),
    })
}

/// Holm step-down: sorted ascending, the rank-k raw p multiplies by
/// (m−k) and the adjusted sequence is the running maximum — the smallest
/// adjusted p consistent with the step-down rule.
fn holm_adjust(raw_p: &[f64]) -> Vec<f64> {
    let m = raw_p.len();
    let mut order: Vec<usize> = (0..m).collect();
    order.sort_by(|a, b| raw_p[*a].total_cmp(&raw_p[*b]));
    let mut adjusted = vec![0.0; m];
    let mut running_max = 0.0_f64;
    for (rank, &index) in order.iter().enumerate() {
        let scaled = ((m - rank) as f64 * raw_p[index]).min(1.0);
        running_max = running_max.max(scaled);
        adjusted[index] = running_max;
    }
    adjusted
}

/// The antisymmetric-off-diagonal win-score matrix: `[i][j]` = i's leg win
/// score against j, `0.5` on the diagonal, `None` unplayed.
pub fn win_score_matrix(entrants: usize, pairings: &[PairingRun], prefix_len: usize) -> Vec<Vec<Option<f64>>> {
    let mut matrix = vec![vec![None; entrants]; entrants];
    for pairing in pairings {
        let mut second_score = 0.0;
        let mut legs = 0;
        for seed in &pairing.seeds[..prefix_len] {
            for leg in [&seed.candidate_deals, &seed.baseline_deals] {
                second_score += leg.candidate_win_score;
                legs += 1;
            }
        }
        if legs > 0 {
            let mean = second_score / legs as f64;
            matrix[pairing.second][pairing.first] = Some(mean);
            matrix[pairing.first][pairing.second] = Some(1.0 - mean);
        }
    }
    matrix
}

/// Counts strict win-score cycles over unordered triples and, when any
/// exist, runs fictitious play on the antisymmetric payoff game for the
/// Nash-averaging report.
fn detect_cycles(matrix: &[Vec<Option<f64>>]) -> IntransitivityReport {
    let n = matrix.len();
    let mut cycles = Vec::new();
    let mut total = 0_usize;
    for i in 0..n {
        for j in (i + 1)..n {
            for k in (j + 1)..n {
                total += 1;
                // A strict cycle reads either i>j>k>i or i>k>j>i — every
                // rotation of a valid orientation is tested here.
                let oriented = [(i, j, k), (i, k, j), (j, i, k), (j, k, i), (k, i, j), (k, j, i)]
                    .into_iter()
                    .find(|(a, b, c)| beats(matrix, *a, *b) && beats(matrix, *b, *c) && beats(matrix, *c, *a));
                if let Some((a, b, c)) = oriented {
                    cycles.push([a, b, c]);
                }
            }
        }
    }
    let cyclic_triples = cycles.len();
    // Strongest cycles first: rank by the weakest link's win-score margin
    // above 0.5 — a cycle held by a razor edge is weaker evidence.
    cycles.sort_by(|a, b| cycle_strength(matrix, b).total_cmp(&cycle_strength(matrix, a)));
    cycles.truncate(8);
    let (nash_weights, nash_expected_win_score) = if cyclic_triples == 0 {
        (None, None)
    } else {
        let (weights, scores) = nash_average(matrix);
        (Some(weights), Some(scores))
    };
    IntransitivityReport {
        total_triples: total,
        cyclic_triples,
        fraction: if total == 0 {
            0.0
        } else {
            cyclic_triples as f64 / total as f64
        },
        cycles,
        nash_weights,
        nash_expected_win_score,
    }
}

/// Strict "beat": i's win score against j exceeds 0.5.
fn beats(matrix: &[Vec<Option<f64>>], i: usize, j: usize) -> bool {
    matrix[i][j].is_some_and(|score| score > 0.5)
}

/// The weakest link's distance above a coin flip — a cycle's evidence.
fn cycle_strength(matrix: &[Vec<Option<f64>>], cycle: &[usize; 3]) -> f64 {
    let [a, b, c] = *cycle;
    [matrix[a][b], matrix[b][c], matrix[c][a]]
        .iter()
        .filter_map(|cell| *cell)
        .map(|score| score - 0.5)
        .fold(f64::INFINITY, f64::min)
}

/// Fictitious play on the symmetric zero-sum win-score game: row and
/// column share one average strategy, which converges to a Nash
/// equilibrium of the empirical game (value 0 by symmetry).
fn nash_average(matrix: &[Vec<Option<f64>>]) -> (Vec<f64>, Vec<f64>) {
    let n = matrix.len();
    let payoff = |i: usize, j: usize| matrix[i][j].unwrap_or(0.5);
    let mut average = vec![1.0 / n as f64; n];
    let mut counts = vec![0.0; n];
    for _ in 0..NASH_ITERATIONS {
        let payoffs: Vec<f64> = (0..n)
            .map(|i| (0..n).map(|j| average[j] * payoff(i, j)).sum::<f64>())
            .collect();
        let best = payoffs
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(index, _)| index)
            .unwrap_or(0);
        counts[best] += 1.0;
        let total = counts.iter().sum::<f64>();
        average = counts.iter().map(|count| count / total).collect();
    }
    let scores: Vec<f64> = (0..n)
        .map(|i| (0..n).map(|j| average[j] * payoff(i, j)).sum::<f64>())
        .collect();
    (average, scores)
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        f64::NAN
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

fn percentile_sorted(sorted: &[f64], probability: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    let index = ((probability * sorted.len() as f64).ceil() as usize).saturating_sub(1);
    sorted[index.min(sorted.len() - 1)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        manifest::{ArtifactSpec, Entrant, PanelSpec, StoppingSpec},
        runner::{LegResult, LegStatus, Seat, SeedResult, TraceEntry},
        sprt::StoppingOutcome,
        statistics::BootstrapSpec,
    };

    /// One leg where `second` (the candidate role) scores `score`/`margin`;
    /// `latencies` are `(role, ms)` referee-measured trace entries — a real
    /// leg carries both artifacts' decisions, tagged by role.
    fn leg(score: f64, margin: f64, latencies: &[(ArtifactRole, f64)]) -> LegResult {
        LegResult {
            candidate_seat: Seat::South,
            status: LegStatus::Valid,
            south_points: None,
            north_points: None,
            candidate_margin: margin,
            candidate_win_score: score,
            actions: latencies.len(),
            redeals: 0,
            trace: latencies
                .iter()
                .enumerate()
                .map(|(turn, &(artifact, latency))| TraceEntry {
                    action_index: turn,
                    seat: Seat::South,
                    artifact,
                    solver_seed: 0,
                    referee_state_digest_fnv1a64: "0".to_owned(),
                    retry: 0,
                    selected_action: crate::protocol::WireAction {
                        kind: "resolve_stock".to_owned(),
                        card: None,
                        capture: Some(crate::protocol::WireCapture {
                            kind: "no_match".to_owned(),
                            card: None,
                        }),
                    },
                    decision_latency_ms: latency,
                    leaf_eval_latency_ms: 0.0,
                })
                .collect(),
        }
    }

    fn seed(index: u64, score_a: f64, score_b: f64) -> SeedResult {
        SeedResult {
            seed: index,
            candidate_deals: leg(
                score_a,
                if score_a == 1.0 { 4.0 } else { -4.0 },
                &[(ArtifactRole::Candidate, 10.0), (ArtifactRole::Baseline, 20.0)],
            ),
            baseline_deals: leg(
                score_b,
                if score_b == 1.0 { 4.0 } else { -4.0 },
                &[(ArtifactRole::Candidate, 12.0), (ArtifactRole::Baseline, 18.0)],
            ),
        }
    }

    fn pairing(first: usize, second: usize, scores: &[(f64, f64)]) -> PairingRun {
        PairingRun {
            first,
            second,
            stopping: StoppingOutcome::CompleteSchedule,
            seeds: scores
                .iter()
                .enumerate()
                .map(|(index, &(a, b))| seed(index as u64, a, b))
                .collect(),
        }
    }

    fn manifest(repetitions: usize) -> TournamentManifest {
        TournamentManifest {
            schema_version: 1,
            run_label: "test".to_owned(),
            ruleset: "nintendo".to_owned(),
            panel: PanelSpec::Explicit {
                seeds: vec![1, 2, 3, 4],
            },
            stopping: StoppingSpec::Fixed,
            worker_timeout_seconds: 60,
            time_hard_cap_ms: Some(30_000),
            invalid_forfeit_margin: 256.0,
            bootstrap: BootstrapSpec {
                repetitions,
                seed: 7,
                confidence_level: 0.99,
            },
            entrants: (0..4)
                .map(|index| Entrant {
                    label: format!("e{index}"),
                    artifact: ArtifactSpec::SelfBinary,
                    config: koi_solver::Config::default(),
                })
                .collect(),
        }
    }

    /// A dominant entrant must land on top: e0 sweeps every pair (the
    /// candidate-role second scores 0 in each mirrored leg).
    #[test]
    fn davidson_ranks_a_dominant_entrant_first() {
        let mut pairings = Vec::new();
        for second in 1..4 {
            pairings.push(pairing(0, second, &[(0.0, 0.0); 8]));
        }
        pairings.push(pairing(1, 2, &[(0.5, 0.5); 8]));
        pairings.push(pairing(1, 3, &[(0.5, 0.5); 8]));
        pairings.push(pairing(2, 3, &[(0.5, 0.5); 8]));
        let ranking = rank_tournament(4, &pairings, &manifest(200), 8).unwrap();
        assert_eq!(ranking.entrants[0].entrant, 0);
        assert!(ranking.entrants[0].elo > ranking.entrants[1].elo);
        assert_eq!(ranking.common_prefix_clusters, 8);
        assert_eq!(ranking.stopping_provenance, "fixed_clusters_complete_schedule");
    }

    /// A stopped pairing must truncate the headline fit to the common
    /// prefix — the full-data fit is reported separately as biased.
    #[test]
    fn stopped_pairs_fit_on_the_common_prefix() {
        let mut stopped = pairing(0, 1, &[(1.0, 1.0); 12]);
        stopped.stopping = StoppingOutcome::GsprtAcceptUpper {
            llr: 6.0,
            clusters_played: 12,
            cap: 64,
            equivalence_cs: None,
        };
        let pairings = vec![
            stopped,
            pairing(0, 2, &vec![(1.0, 1.0); 16]),
            pairing(0, 3, &vec![(1.0, 1.0); 16]),
            pairing(1, 2, &vec![(0.5, 0.5); 16]),
            pairing(1, 3, &vec![(0.5, 0.5); 16]),
            pairing(2, 3, &vec![(0.5, 0.5); 16]),
        ];
        let ranking = rank_tournament(4, &pairings, &manifest(200), 16).unwrap();
        assert_eq!(ranking.common_prefix_clusters, 12);
        assert_eq!(ranking.method, "davidson_ties_bradley_terry_common_prefix");
        assert!(ranking.descriptive_full_fit.is_some());
        assert!(ranking.stopping_provenance.contains("adaptive_stopping_schedule"));
    }

    /// Cyclic triples must be witnessed and reported with Nash weights.
    #[test]
    fn cyclic_triples_trigger_the_nash_report() {
        // e0>e1, e1>e2, e2>e0 — a strict cycle; e3 loses to everyone.
        // `pairing(i, j)` puts j in the candidate role, so all-zero scores
        // mean i sweeps and all-ones mean j sweeps.
        let pairings = vec![
            pairing(0, 1, &[(0.0, 0.0); 8]), // e0>e1
            pairing(0, 2, &[(1.0, 1.0); 8]), // e2>e0
            pairing(0, 3, &[(0.0, 0.0); 8]), // e0>e3
            pairing(1, 2, &[(0.0, 0.0); 8]), // e1>e2
            pairing(1, 3, &[(0.0, 0.0); 8]), // e1>e3
            pairing(2, 3, &[(0.0, 0.0); 8]), // e2>e3
        ];
        let ranking = rank_tournament(4, &pairings, &manifest(200), 8).unwrap();
        assert_eq!(ranking.intransitivity.cyclic_triples, 1);
        assert!(ranking.intransitivity.nash_weights.is_some());
    }
}
