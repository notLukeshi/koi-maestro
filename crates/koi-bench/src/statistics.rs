//! Benchmark statistics: cluster bootstrap, latency summaries, and the
//! empirical-Bernstein confidence sequence that stays valid at a
//! data-dependent stopping time.
//!
//! The resampling unit is the deal cluster — both mirrored legs of a seed
//! move together (E1), so within-cluster correlation is carried by the
//! bootstrap and never treated as independent evidence.

use anyhow::{bail, Context, Result};
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

use crate::runner::{ArtifactRole, LegStatus, SeedResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BootstrapSpec {
    pub repetitions: usize,
    pub seed: u64,
    pub confidence_level: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricSummary {
    pub estimate: f64,
    pub ci_low: f64,
    pub ci_high: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSummary {
    pub cluster_count: usize,
    pub leg_count: usize,
    /// Legs that played to a natural or anomaly-resolved end — the count
    /// the vacuous-run gate keys on. `leg_count − valid_legs` legs carry
    /// synthetic forfeit margins, not played evidence.
    pub valid_legs: usize,
    pub candidate_wins: usize,
    pub draws: usize,
    pub candidate_losses: usize,
    /// Protocol-invalid legs — forfeits for replay, framing, or legality
    /// violations, counted intention-to-treat.
    pub invalid_legs: usize,
    /// Legs forfeited on the per-leg latency bank — reported separately
    /// from protocol invalidity.
    pub time_forfeit_legs: usize,
    pub candidate_win_score: MetricSummary,
    pub candidate_margin: MetricSummary,
    pub baseline_latency: LatencySummary,
    pub candidate_latency: LatencySummary,
    pub baseline_leg_time: LegTimeSummary,
    pub candidate_leg_time: LegTimeSummary,
}

/// Referee-measured decision latency for one artifact over every recorded
/// trace decision (valid and invalid legs alike — intention-to-treat).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencySummary {
    pub decisions: usize,
    pub mean_ms: f64,
    pub median_ms: f64,
    pub p95_ms: f64,
    pub max_ms: f64,
}

/// Per-leg total decision time: the summed `decision_latency_ms` an
/// artifact spent inside each leg, plus how many legs crossed the hard cap.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegTimeSummary {
    pub legs: usize,
    pub mean_ms: f64,
    pub median_ms: f64,
    pub p95_ms: f64,
    pub max_ms: f64,
    /// Legs whose summed latency exceeded `time_hard_cap_ms`.
    pub over_cap: usize,
}

impl LatencySummary {
    fn from_latencies(mut latencies: Vec<f64>) -> Self {
        if latencies.is_empty() {
            return Self {
                decisions: 0,
                mean_ms: 0.0,
                median_ms: 0.0,
                p95_ms: 0.0,
                max_ms: 0.0,
            };
        }
        latencies.sort_by(f64::total_cmp);
        let decisions = latencies.len();
        Self {
            decisions,
            mean_ms: mean(&latencies),
            median_ms: percentile(&latencies, 0.5),
            p95_ms: percentile(&latencies, 0.95),
            max_ms: latencies[decisions - 1],
        }
    }
}

impl LegTimeSummary {
    fn from_leg_times(mut leg_times: Vec<f64>, over_cap: usize) -> Self {
        if leg_times.is_empty() {
            return Self {
                legs: 0,
                mean_ms: 0.0,
                median_ms: 0.0,
                p95_ms: 0.0,
                max_ms: 0.0,
                over_cap,
            };
        }
        leg_times.sort_by(f64::total_cmp);
        let legs = leg_times.len();
        Self {
            legs,
            mean_ms: mean(&leg_times),
            median_ms: percentile(&leg_times, 0.5),
            p95_ms: percentile(&leg_times, 0.95),
            max_ms: leg_times[legs - 1],
            over_cap,
        }
    }
}

// ---------------------------------------------------------------------------
// Confidence sequences (anytime-valid inference for sequential stops)
// ---------------------------------------------------------------------------

/// One side of a bounded-mean confidence sequence at a single time —
/// serialized into stopping provenance so stopped runs carry valid
/// inference downstream.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CsInterval {
    pub lower: f64,
    pub upper: f64,
}

/// Empirical-Bernstein confidence sequence for the mean of `[0, 1]`-bounded
/// observations — Howard–Ramdas et al. (2021, §3) / Waudby-Smith & Ramdas
/// (2023, Theorem 2). The interval holds coverage `1 − alpha` *uniformly
/// over all times*, including data-dependent stopping times — the property
/// that makes it the only valid interval to report on a sequentially
/// stopped run.
///
/// Closed-form boundary on the λ-weighted running mean μ̂_n:
/// `half_width_n = [ln(2/α) + Σᵢ ψ_E(λᵢ)·vᵢ] / Σᵢ λᵢ` where
/// `vᵢ = (Xᵢ − μ̂_{i−1})²` and `ψ_E(λ) = −ln(1−λ) − λ`. The multipliers are
/// predictable: `λᵢ = min(½, √(2 ln(2/α) / (σ̂²_{i−1}·i·ln(1+i))))`.
#[derive(Debug, Clone)]
pub struct EmpiricalBernsteinCs {
    alpha: f64,
    n: usize,
    lambda_sum: f64,
    weighted_sum: f64,
    psi_v_sum: f64,
    mean_prev: f64,
    var_prev: f64,
}

impl EmpiricalBernsteinCs {
    /// A fresh sequence; `prior_mean`/`prior_var` seed the predictable
    /// plug-ins (0.5/0.25 are the maximal-variance [0,1] defaults).
    pub fn new(alpha: f64, prior_mean: f64, prior_var: f64) -> Self {
        debug_assert!(alpha > 0.0 && alpha < 1.0);
        Self {
            alpha,
            n: 0,
            lambda_sum: 0.0,
            weighted_sum: 0.0,
            psi_v_sum: 0.0,
            mean_prev: prior_mean,
            var_prev: prior_var.max(1e-6),
        }
    }

    /// Fold one bounded observation into the sequence.
    pub fn observe(&mut self, x: f64) {
        debug_assert!((0.0..=1.0).contains(&x), "EB-CS requires [0, 1]-bounded observations");
        self.n += 1;
        let n = self.n as f64;
        let v = (x - self.mean_prev) * (x - self.mean_prev);
        let lambda = (2.0 * (2.0 / self.alpha).ln() / (self.var_prev * n * (1.0 + n).ln()))
            .sqrt()
            .min(0.5);
        self.lambda_sum += lambda;
        self.weighted_sum += lambda * x;
        self.psi_v_sum += psi_e(lambda) * v;
        self.var_prev += (v - self.var_prev) / n;
        self.mean_prev = self.weighted_sum / self.lambda_sum;
    }

    /// The λ-weighted running mean — the CS's center.
    pub fn mean(&self) -> f64 {
        self.mean_prev
    }

    /// Current interval — trivially the whole support with no observations.
    pub fn interval(&self) -> CsInterval {
        if self.n == 0 {
            return CsInterval { lower: 0.0, upper: 1.0 };
        }
        let half_width = ((2.0 / self.alpha).ln() + self.psi_v_sum) / self.lambda_sum;
        CsInterval {
            lower: self.mean_prev - half_width,
            upper: self.mean_prev + half_width,
        }
    }
}

/// `ψ_E(λ) = −ln(1−λ) − λ`, the exponential-like term of the EB boundary.
fn psi_e(lambda: f64) -> f64 {
    -f64::ln(1.0 - lambda) - lambda
}

/// The win score a margin maps to: sign → {0, 0.5, 1}.
pub fn margin_to_win_score(margin: f64) -> f64 {
    if margin > 0.0 {
        1.0
    } else if margin < 0.0 {
        0.0
    } else {
        0.5
    }
}

pub(crate) fn summarize(seeds: &[SeedResult], bootstrap: &BootstrapSpec) -> Result<BenchmarkSummary> {
    if seeds.is_empty() {
        bail!("benchmark summary requires at least one seed cluster");
    }
    let cluster_win_scores: Vec<f64> = seeds
        .iter()
        .map(|seed| (seed.candidate_deals.candidate_win_score + seed.baseline_deals.candidate_win_score) / 2.0)
        .collect();
    let cluster_margins: Vec<f64> = seeds
        .iter()
        .map(|seed| (seed.candidate_deals.candidate_margin + seed.baseline_deals.candidate_margin) / 2.0)
        .collect();
    let legs = seeds
        .iter()
        .flat_map(|seed| [&seed.candidate_deals, &seed.baseline_deals]);

    let mut candidate_wins = 0;
    let mut draws = 0;
    let mut candidate_losses = 0;
    let mut valid_legs = 0;
    let mut invalid_legs = 0;
    let mut time_forfeit_legs = 0;
    let mut baseline_latencies = Vec::new();
    let mut candidate_latencies = Vec::new();
    let mut baseline_leg_times = Vec::new();
    let mut candidate_leg_times = Vec::new();
    let mut baseline_over_cap = 0;
    let mut candidate_over_cap = 0;
    for leg in legs {
        let mut baseline_leg_ms = 0.0;
        let mut candidate_leg_ms = 0.0;
        for entry in &leg.trace {
            match entry.artifact {
                ArtifactRole::Baseline => {
                    baseline_latencies.push(entry.decision_latency_ms);
                    baseline_leg_ms += entry.decision_latency_ms;
                }
                ArtifactRole::Candidate => {
                    candidate_latencies.push(entry.decision_latency_ms);
                    candidate_leg_ms += entry.decision_latency_ms;
                }
            }
        }
        baseline_leg_times.push(baseline_leg_ms);
        candidate_leg_times.push(candidate_leg_ms);
        match &leg.status {
            LegStatus::Valid => valid_legs += 1,
            LegStatus::Invalid { .. } => invalid_legs += 1,
            LegStatus::TimeForfeit { offender, spent_ms, .. } => {
                time_forfeit_legs += 1;
                // The killing decision's latency is charged into
                // `spent_ms`; on current evidence it is also in the trace,
                // but `spent_ms` stays authoritative so resumed or legacy
                // evidence never undercounts the offender's leg time.
                let slot = match offender {
                    ArtifactRole::Baseline => {
                        baseline_over_cap += 1;
                        baseline_leg_times.last_mut()
                    }
                    ArtifactRole::Candidate => {
                        candidate_over_cap += 1;
                        candidate_leg_times.last_mut()
                    }
                };
                if let Some(leg_ms) = slot {
                    *leg_ms = leg_ms.max(*spent_ms);
                }
            }
        }
        if leg.candidate_win_score == 1.0 {
            candidate_wins += 1;
        } else if leg.candidate_win_score == 0.5 {
            draws += 1;
        } else {
            candidate_losses += 1;
        }
    }

    Ok(BenchmarkSummary {
        cluster_count: seeds.len(),
        leg_count: seeds.len() * 2,
        valid_legs,
        candidate_wins,
        draws,
        candidate_losses,
        invalid_legs,
        time_forfeit_legs,
        candidate_win_score: bootstrap_mean(&cluster_win_scores, bootstrap)?,
        candidate_margin: bootstrap_mean(&cluster_margins, bootstrap)?,
        baseline_latency: LatencySummary::from_latencies(baseline_latencies),
        candidate_latency: LatencySummary::from_latencies(candidate_latencies),
        baseline_leg_time: LegTimeSummary::from_leg_times(baseline_leg_times, baseline_over_cap),
        candidate_leg_time: LegTimeSummary::from_leg_times(candidate_leg_times, candidate_over_cap),
    })
}

fn bootstrap_mean(values: &[f64], spec: &BootstrapSpec) -> Result<MetricSummary> {
    if values.is_empty() {
        bail!("cluster bootstrap requires at least one value");
    }
    if spec.repetitions == 0 {
        bail!("cluster bootstrap requires at least one repetition");
    }
    if !spec.confidence_level.is_finite() || !(0.0..1.0).contains(&spec.confidence_level) {
        bail!("cluster bootstrap confidence level must be finite and in [0, 1)");
    }
    let estimate = mean(values);
    let mut rng = ChaCha8Rng::seed_from_u64(spec.seed);
    let mut replicates = Vec::new();
    replicates
        .try_reserve_exact(spec.repetitions)
        .context("unable to reserve bootstrap replicate storage")?;
    for _ in 0..spec.repetitions {
        let mut total = 0.0;
        for _ in values {
            total += values[rng.random_range(0..values.len())];
        }
        replicates.push(total / values.len() as f64);
    }
    replicates.sort_by(f64::total_cmp);
    let tail = (1.0 - spec.confidence_level) / 2.0;
    Ok(MetricSummary {
        estimate,
        ci_low: percentile(&replicates, tail),
        ci_high: percentile(&replicates, 1.0 - tail),
    })
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

fn percentile(sorted: &[f64], probability: f64) -> f64 {
    let index = ((probability * sorted.len() as f64).ceil() as usize).saturating_sub(1);
    sorted[index]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::{LegResult, Seat};

    fn leg(win_score: f64, margin: f64) -> LegResult {
        LegResult {
            candidate_seat: Seat::South,
            status: LegStatus::Valid,
            south_points: Some(0),
            north_points: Some(0),
            candidate_margin: margin,
            candidate_win_score: win_score,
            actions: 0,
            redeals: 0,
            trace: Vec::new(),
        }
    }

    fn seed(win_a: f64, win_b: f64, margin: f64) -> SeedResult {
        SeedResult {
            seed: 1,
            candidate_deals: leg(win_a, margin),
            baseline_deals: leg(win_b, margin),
        }
    }

    #[test]
    fn zero_fixture_has_degenerate_interval() {
        let spec = BootstrapSpec {
            repetitions: 100,
            seed: 17,
            confidence_level: 0.99,
        };
        let summary = bootstrap_mean(&[0.0, 0.0, 0.0], &spec).unwrap();
        assert_eq!(summary.estimate, 0.0);
        assert_eq!(summary.ci_low, 0.0);
        assert_eq!(summary.ci_high, 0.0);
    }

    #[test]
    fn empty_bootstrap_input_fails_closed() {
        let spec = BootstrapSpec {
            repetitions: 100,
            seed: 17,
            confidence_level: 0.99,
        };
        assert!(bootstrap_mean(&[], &spec).is_err());
        assert!(summarize(&[], &spec).is_err());
    }

    #[test]
    fn invalid_bootstrap_spec_fails_closed() {
        let mut spec = BootstrapSpec {
            repetitions: 0,
            seed: 17,
            confidence_level: 0.99,
        };
        assert!(bootstrap_mean(&[0.0], &spec).is_err());

        spec.repetitions = 1;
        spec.confidence_level = f64::NAN;
        assert!(bootstrap_mean(&[0.0], &spec).is_err());
    }

    #[test]
    fn summarize_counts_categories() {
        let spec = BootstrapSpec {
            repetitions: 200,
            seed: 3,
            confidence_level: 0.99,
        };
        let mut seeds = vec![seed(1.0, 0.5, 3.0), seed(0.0, 1.0, -2.0)];
        seeds[1].baseline_deals.status = LegStatus::TimeForfeit {
            offender: ArtifactRole::Baseline,
            spent_ms: 10.0,
            hard_cap_ms: 5,
        };
        let summary = summarize(&seeds, &spec).unwrap();
        assert_eq!(summary.cluster_count, 2);
        assert_eq!(summary.leg_count, 4);
        assert_eq!(summary.time_forfeit_legs, 1);
        assert_eq!(summary.candidate_wins, 2);
        assert_eq!(summary.draws, 1);
        assert_eq!(summary.candidate_losses, 1);
        assert_eq!(summary.baseline_leg_time.over_cap, 1);
    }

    #[test]
    fn eb_cs_shrinks_and_covers() {
        let mut cs = EmpiricalBernsteinCs::new(0.05, 0.5, 0.25);
        assert_eq!(cs.interval(), CsInterval { lower: 0.0, upper: 1.0 });
        for _ in 0..200 {
            cs.observe(0.8);
        }
        let interval = cs.interval();
        assert!(interval.lower > 0.5 && interval.upper <= 1.0);
        assert!(interval.lower <= cs.mean() && cs.mean() <= interval.upper);
    }
}
