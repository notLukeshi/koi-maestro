//! Sequential stopping for paired confirmation matches: a generalized SPRT
//! over the mirrored deal cluster.
//!
//! A paired leg outcome on a shared deal is *not* an independent Bernoulli —
//! the two mirrored legs share deal difficulty, so treating each leg as its
//! own trial overstates the evidence. The honest unit is the deal cluster
//! itself: its score `s ∈ {0, 0.5, 1, 1.5, 2}` is one pentanomial
//! observation, and all sequential inference below operates on cluster
//! counts, never on legs.
//!
//! The likelihood ratio follows the fishtest GSPRT construction (Li–Liu–Ying
//! generalized SPRT for separate families). Each hypothesis is a point
//! constraint on the expected per-leg score `E[s]/2`; the constrained MLE of
//! the observed multinomial under `E[s]/2 = s_j` is the exponential tilt
//! `p_i ∝ π_i / (1 + x·(a_i − s_j))` with `x` solving the secular equation
//! `Σ π_i·(a_i − s_j)/(1 + x·(a_i − s_j)) = 0`. Because the tilt is anchored
//! on the *empirical* five-bin distribution, within-cluster correlation is
//! carried by the data itself — the hypotheses constrain only the mean.
//!
//! Decisions use the standard SPRT boundaries `a = ln(β/(1−α))`,
//! `b = ln((1−β)/α)` evaluated on the GLLR. Empty bins are regularized with a
//! small pseudo-count so every hypothesis remains interior to the support.

use crate::runner::SeedResult;
use crate::statistics::{CsInterval, EmpiricalBernsteinCs};
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

/// Per-leg score scale of the five pentanomial bins: the cluster scores
/// `{0, 0.5, 1, 1.5, 2}` divided by two, `{0, 0.25, 0.5, 0.75, 1}`.
pub const BIN_SCORES: [f64; 5] = [0.0, 0.25, 0.5, 0.75, 1.0];
/// Pseudo-count mixed into every bin before profiling, mirroring the
/// regularization the reference GSPRT uses so the constrained MLE always
/// has full support and degenerate count vectors stay finite.
const REGULARIZER: f64 = 1e-3;
/// Bisection iterations for the secular root; the interval is open so the
/// iterate is kept strictly inside by construction.
const SECULAR_ITERATIONS: usize = 200;

/// Sequential test specification declared on a manifest's stopping rule.
/// `elo` bounds are logistic Elo on the candidate-minus-baseline scale.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GsprtSpec {
    pub elo0: f64,
    pub elo1: f64,
    #[serde(default = "default_error_rate")]
    pub alpha: f64,
    #[serde(default = "default_error_rate")]
    pub beta: f64,
    /// Minimum deal clusters observed before any stop is evaluated. Keeps
    /// the discrete-time overshoot and the regularized tilt honest on
    /// tiny samples; also guarantees at least this much evidence reaches
    /// the fixed-cap analysis even when the LLR starts extreme.
    #[serde(default = "default_min_clusters")]
    pub min_clusters: usize,
}

fn default_error_rate() -> f64 {
    0.05
}
fn default_min_clusters() -> usize {
    8
}

/// Hard bound on the Elo hypotheses. Beyond ±2000 the score bound sits
/// within ~1e-5 of the [0,1] support edge and the secular-equation tilt
/// can no longer resolve the constrained MLE — the run would silently
/// never stop and record a fabricated interior LLR. No plausible engine
/// gap approaches this anyway.
const MAX_ELO_BOUND: f64 = 2000.0;

impl GsprtSpec {
    pub fn validate(&self) -> Result<()> {
        if !self.elo0.is_finite() || !self.elo1.is_finite() || self.elo0 >= self.elo1 {
            bail!("gsprt requires finite elo bounds with elo0 < elo1");
        }
        if self.elo0.abs() > MAX_ELO_BOUND || self.elo1.abs() > MAX_ELO_BOUND {
            bail!(
                "gsprt elo bounds must stay inside ±{MAX_ELO_BOUND} — beyond that the constrained MLE is unresolvable"
            );
        }
        for (name, rate) in [("alpha", self.alpha), ("beta", self.beta)] {
            if !(rate > 0.0 && rate < 0.5) {
                bail!("gsprt {name} must be in (0, 0.5)");
            }
        }
        if self.min_clusters == 0 {
            bail!("gsprt min_clusters must be at least 1");
        }
        Ok(())
    }

    /// Logistic Elo → expected per-leg score.
    fn score_bound(elo: f64) -> f64 {
        1.0 / (1.0 + 10f64.powf(-elo / 400.0))
    }
    pub fn lower_bound(&self) -> f64 {
        (self.beta / (1.0 - self.alpha)).ln()
    }
    pub fn upper_bound(&self) -> f64 {
        ((1.0 - self.beta) / self.alpha).ln()
    }
}

/// Indifference-band equivalence certification (catalog T-04/D-05): an
/// empirical-Bernstein confidence sequence on the mean per-leg score, armed
/// alongside the GSPRT. When the whole CS fits inside
/// `(0.5 − delta, 0.5 + delta)` the pair is certified *equivalent* at level
/// `alpha` — the decisive answer for near-tied pairs the GSPRT's two-sided
/// boundaries can never reach (a truly tied pair never crosses either
/// boundary, LAW-2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EquivalenceSpec {
    /// Half-width of the indifference band on the per-leg score scale:
    /// equivalence means the mean leg win-score lies inside
    /// `0.5 ± delta` — `delta = 0.05` certifies a ±5% band, matching the
    /// ±35-Elo GSPRT indifference zone (`L(34.9) ≈ 0.55`).
    pub delta: f64,
    /// Confidence-sequence level: coverage `1 − alpha` holds uniformly
    /// over all times, so a false equivalence certification is bounded
    /// by `alpha` — no deflation needed (the CS is exactly valid, not
    /// asymptotic like the GSPRT).
    #[serde(default = "default_error_rate")]
    pub alpha: f64,
    /// Minimum clusters before the equivalence stop is evaluated. The
    /// manifest validator requires `>= gsprt.min_clusters` — the floor is
    /// the protocol's minimum-evidence guarantee for *every* stop kind.
    #[serde(default = "default_equivalence_min_clusters")]
    pub min_clusters: usize,
}

fn default_equivalence_min_clusters() -> usize {
    64
}

impl EquivalenceSpec {
    pub fn validate(&self) -> Result<()> {
        if !self.delta.is_finite() || !(0.0..0.5).contains(&self.delta) {
            bail!("equivalence delta must be finite and inside (0, 0.5)");
        }
        if !(self.alpha > 0.0 && self.alpha < 0.5) {
            bail!("equivalence alpha must be in (0, 0.5)");
        }
        if self.min_clusters == 0 {
            bail!("equivalence min_clusters must be at least 1");
        }
        Ok(())
    }
}

/// What the sequential test concluded about a run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StoppingOutcome {
    /// Every scheduled deal cluster was played; no sequential rule was
    /// armed or it never triggered within the cap.
    CompleteSchedule,
    /// GLLR crossed the lower boundary: accept the candidate is at most
    /// `elo0` stronger than baseline. `equivalence_cs` is the EB-CS at the
    /// stop when an equivalence spec was armed — the anytime-valid
    /// inference on the stopped evidence (T-05).
    GsprtAcceptLower {
        llr: f64,
        clusters_played: usize,
        cap: usize,
        #[serde(default)]
        equivalence_cs: Option<CsInterval>,
    },
    /// GLLR crossed the upper boundary: accept the candidate is at least
    /// `elo1` stronger than baseline.
    GsprtAcceptUpper {
        llr: f64,
        clusters_played: usize,
        cap: usize,
        #[serde(default)]
        equivalence_cs: Option<CsInterval>,
    },
    /// The cluster cap was reached with the GLLR still interior; the
    /// test is inconclusive and the run's evidence is the full panel.
    /// `None` when the cap was hit before `min_clusters` — the test never
    /// evaluated, so no LLR exists to record.
    GsprtCapReached {
        llr: Option<f64>,
        #[serde(default)]
        clusters_played: usize,
        #[serde(default)]
        cap: usize,
        #[serde(default)]
        equivalence_cs: Option<CsInterval>,
    },
    /// The equivalence CS fit wholly inside the indifference band: the
    /// pair is certified equivalent at the declared level and band
    /// (catalog T-04/D-05). `cs_lower`/`cs_upper` are the CS at the stop.
    EquivalenceCertified {
        cs_lower: f64,
        cs_upper: f64,
        delta: f64,
        alpha: f64,
        clusters_played: usize,
        cap: usize,
    },
}

impl Default for StoppingOutcome {
    /// Legacy evidence predates sequential stopping: a run without a
    /// recorded outcome played its complete fixed schedule.
    fn default() -> Self {
        StoppingOutcome::CompleteSchedule
    }
}

impl StoppingOutcome {
    /// True when a sequential boundary decided the run before the cap.
    /// Reports downstream must treat such evidence as selection-biased.
    /// Equivalence certification is a data-dependent stop too: the pair's
    /// point estimates carry the same stopped-τ bias flag even though the
    /// certification itself is exactly valid.
    pub fn stopped_early(&self) -> bool {
        matches!(
            self,
            StoppingOutcome::GsprtAcceptLower { .. }
                | StoppingOutcome::GsprtAcceptUpper { .. }
                | StoppingOutcome::EquivalenceCertified { .. }
        )
    }

    /// The stable outcome name for provenance strings and merge reports.
    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::CompleteSchedule => "complete_schedule",
            Self::GsprtAcceptLower { .. } => "gsprt_accept_lower",
            Self::GsprtAcceptUpper { .. } => "gsprt_accept_upper",
            Self::GsprtCapReached { .. } => "gsprt_cap_reached",
            Self::EquivalenceCertified { .. } => "equivalence_certified",
        }
    }

    /// Attach the EB-CS observed at the stop to a GSPRT outcome (T-05).
    /// No-op for variants that are their own CS record.
    pub fn with_equivalence_cs(self, cs: Option<CsInterval>) -> Self {
        match self {
            Self::GsprtAcceptLower {
                llr,
                clusters_played,
                cap,
                equivalence_cs: _,
            } => Self::GsprtAcceptLower {
                llr,
                clusters_played,
                cap,
                equivalence_cs: cs,
            },
            Self::GsprtAcceptUpper {
                llr,
                clusters_played,
                cap,
                equivalence_cs: _,
            } => Self::GsprtAcceptUpper {
                llr,
                clusters_played,
                cap,
                equivalence_cs: cs,
            },
            Self::GsprtCapReached {
                llr,
                clusters_played,
                cap,
                equivalence_cs: _,
            } => Self::GsprtCapReached {
                llr,
                clusters_played,
                cap,
                equivalence_cs: cs,
            },
            other => other,
        }
    }
}

/// The live state of a GSPRT evaluated once per completed cluster.
pub struct GsprtEvaluator {
    spec: GsprtSpec,
    counts: [f64; 5],
    played: usize,
    cap: usize,
}

impl GsprtEvaluator {
    pub fn new(spec: GsprtSpec, cap: usize) -> Self {
        Self {
            spec,
            counts: [0.0; 5],
            played: 0,
            cap,
        }
    }

    /// Fold one completed deal cluster into the counts.
    pub fn observe(&mut self, seed: &SeedResult) {
        let score = seed.candidate_deals.candidate_win_score + seed.baseline_deals.candidate_win_score;
        let bin = (score * 2.0).round() as usize;
        self.counts[bin.min(4)] += 1.0;
        self.played += 1;
    }

    /// Current generalized LLR of `s ≥ s1` against `s ≤ s0`, or `Ok(None)`
    /// while fewer than `min_clusters` clusters are observed. A constrained-
    /// MLE failure propagates rather than being swallowed — a sequential
    /// rule that cannot evaluate must abort the run, not fake interior.
    pub fn llr(&self) -> Result<Option<f64>> {
        if self.played < self.spec.min_clusters {
            return Ok(None);
        }
        gsprt_llr(&self.counts, &self.spec).map(Some)
    }

    /// Evaluate the stopping rule over the clusters observed so far.
    pub fn decision(&self) -> Result<Option<StoppingOutcome>> {
        let Some(llr) = self.llr()? else {
            return Ok(None);
        };
        if llr <= self.spec.lower_bound() {
            return Ok(Some(StoppingOutcome::GsprtAcceptLower {
                llr,
                clusters_played: self.played,
                cap: self.cap,
                equivalence_cs: None,
            }));
        }
        if llr >= self.spec.upper_bound() {
            return Ok(Some(StoppingOutcome::GsprtAcceptUpper {
                llr,
                clusters_played: self.played,
                cap: self.cap,
                equivalence_cs: None,
            }));
        }
        Ok(None)
    }

    /// Outcome to record when the loop ends without a boundary crossing.
    /// `llr` stays `None` when the run capped out before `min_clusters` —
    /// the test never evaluated, so no number is recorded.
    pub fn exhausted(&self) -> Result<StoppingOutcome> {
        Ok(StoppingOutcome::GsprtCapReached {
            llr: self.llr()?,
            clusters_played: self.played,
            cap: self.cap,
            equivalence_cs: None,
        })
    }
}

/// The live state of the equivalence CS evaluated once per completed
/// cluster — the near-tie certification the GSPRT cannot express.
pub struct EquivalenceEvaluator {
    spec: EquivalenceSpec,
    cs: EmpiricalBernsteinCs,
    played: usize,
    cap: usize,
}

impl EquivalenceEvaluator {
    pub fn new(spec: EquivalenceSpec, cap: usize) -> Self {
        Self {
            cs: EmpiricalBernsteinCs::new(spec.alpha, 0.5, 0.25),
            spec,
            played: 0,
            cap,
        }
    }

    /// Fold one completed deal cluster into the sequence. The observation
    /// is the per-leg mean score `(south + north)/2 ∈ [0, 1]` — the
    /// cluster-mean unit keeps the bounded-support requirement and matches
    /// the GSPRT's score scale.
    pub fn observe(&mut self, seed: &SeedResult) {
        let x = (seed.candidate_deals.candidate_win_score + seed.baseline_deals.candidate_win_score) / 2.0;
        self.cs.observe(x);
        self.played += 1;
    }

    /// The current confidence sequence — valid at every observed time.
    pub fn interval(&self) -> CsInterval {
        self.cs.interval()
    }

    /// Certify equivalence when the whole CS lies inside the band.
    pub fn decision(&self) -> Option<StoppingOutcome> {
        if self.played < self.spec.min_clusters {
            return None;
        }
        let interval = self.cs.interval();
        if interval.lower > 0.5 - self.spec.delta && interval.upper < 0.5 + self.spec.delta {
            return Some(StoppingOutcome::EquivalenceCertified {
                cs_lower: interval.lower,
                cs_upper: interval.upper,
                delta: self.spec.delta,
                alpha: self.spec.alpha,
                clusters_played: self.played,
                cap: self.cap,
            });
        }
        None
    }
}

/// Generalized LLR of the upper hypothesis against the lower over
/// regularized cluster counts. `counts` may hold raw or regularized bin
/// totals; the regularizer is added here, so pass raw counts.
pub fn gsprt_llr(counts: &[f64; 5], spec: &GsprtSpec) -> Result<f64> {
    let mut probs = [0.0f64; 5];
    let mut total = 0.0;
    for (i, count) in counts.iter().enumerate() {
        probs[i] = count + REGULARIZER;
        total += probs[i];
    }
    if !total.is_finite() || total <= 0.0 {
        bail!("gsprt requires a finite, nonempty count vector");
    }
    for p in &mut probs {
        *p /= total;
    }
    let s0 = GsprtSpec::score_bound(spec.elo0);
    let s1 = GsprtSpec::score_bound(spec.elo1);
    let p0 = mle_expected(&probs, s0)?;
    let p1 = mle_expected(&probs, s1)?;
    // Empirical expectation of the per-bin log ratio: N·Σ π_i ln(p1_i/p0_i)
    // where N counts the regularized mass, matching the reference
    // construction's `N = sum(regularized results)`.
    let llr = probs
        .iter()
        .zip(p0.iter().zip(p1.iter()))
        .map(|(pi, (q0, q1))| pi * (q1.ln() - q0.ln()))
        .sum::<f64>()
        * total;
    if !llr.is_finite() {
        bail!("gsprt llr was not finite");
    }
    Ok(llr)
}

/// Constrained multinomial MLE under `E[x] = s`, as an exponential tilt of
/// the empirical `probs`. The tilt solves the secular equation
/// `Σ_i π_i·d_i/(1 + x·d_i) = 0` on the open interval `(−1/d_max, −1/d_min)`
/// where `d_i = a_i − s`; the root exists and is unique whenever `s` lies
/// strictly inside the support range, which the regularizer guarantees.
fn mle_expected(probs: &[f64; 5], s: f64) -> Result<[f64; 5]> {
    let d: Vec<f64> = BIN_SCORES.iter().map(|a| a - s).collect();
    let (d_min, d_max) = (
        d.iter().cloned().fold(f64::INFINITY, f64::min),
        d.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
    );
    if !(d_min < 0.0 && d_max > 0.0) {
        bail!("gsprt hypothesis {s} lies outside the cluster-score support");
    }
    // f(x) = Σ π_i·d_i/(1 + x·d_i) is strictly decreasing on the interval;
    // f → +∞ at −1/d_max and → −∞ at −1/d_min, so bisection converges.
    let f = |x: f64| -> f64 {
        probs
            .iter()
            .zip(d.iter())
            .map(|(pi, di)| pi * di / (1.0 + x * di))
            .sum()
    };
    let (mut lo, mut hi) = (-1.0 / d_max * (1.0 - 1e-12), -1.0 / d_min * (1.0 - 1e-12));
    // f(lo) > 0, f(hi) < 0 up to the endpoint shrink; verify the bracket.
    if !(f(lo) > 0.0 && f(hi) < 0.0) {
        bail!("gsprt secular bracket failed for hypothesis {s}");
    }
    for _ in 0..SECULAR_ITERATIONS {
        let mid = 0.5 * (lo + hi);
        if f(mid) > 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let x = 0.5 * (lo + hi);
    let mut mle = [0.0f64; 5];
    for i in 0..5 {
        mle[i] = probs[i] / (1.0 + x * d[i]);
    }
    Ok(mle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::{LegResult, LegStatus, Seat};
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    fn leg(seat: Seat, score: f64) -> LegResult {
        LegResult {
            candidate_seat: seat,
            status: LegStatus::Valid,
            south_points: Some(0),
            north_points: Some(0),
            candidate_margin: score * 2.0 - 1.0,
            candidate_win_score: score,
            actions: 0,
            redeals: 0,
            trace: Vec::new(),
        }
    }

    fn seed_result(seed: u64, p1: f64, p2: f64) -> SeedResult {
        SeedResult {
            seed,
            candidate_deals: leg(Seat::South, p1),
            baseline_deals: leg(Seat::North, p2),
        }
    }

    fn spec() -> GsprtSpec {
        GsprtSpec {
            elo0: 0.0,
            elo1: 10.0,
            alpha: 0.05,
            beta: 0.05,
            min_clusters: 4,
        }
    }

    /// Wide indifference zone for mechanic-level tests: near-extremal
    /// hypotheses make a handful of decisive clusters cross the boundary.
    fn wide_spec() -> GsprtSpec {
        GsprtSpec {
            elo0: -400.0,
            elo1: 400.0,
            alpha: 0.05,
            beta: 0.05,
            min_clusters: 4,
        }
    }

    /// Synthetic-validation spec: a 40-Elo indifference zone resolves in
    /// a few hundred clusters, comfortably inside the 2000-cap panel.
    fn synthetic_spec() -> GsprtSpec {
        GsprtSpec {
            elo0: 0.0,
            elo1: 40.0,
            alpha: 0.05,
            beta: 0.05,
            min_clusters: 8,
        }
    }

    /// Splitmix64 — deterministic, dependency-free synthetic generator.
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
            z ^ (z >> 31)
        }
        fn uniform(&mut self) -> f64 {
            (self.next() >> 11) as f64 / (1u64 << 53) as f64
        }
    }

    /// Logistic trinomial leg draw at the candidate's true `elo`
    /// advantage: the expected per-leg score is `L(elo)`, so with draw
    /// rate `p_draw` the leg-level win probability is `L(elo) − p_draw/2`.
    /// `correlation` mixes a shared deal factor: with that probability the
    /// second leg copies the first instead of drawing independently, so
    /// the synthetic clusters carry realistic within-cluster dependence.
    fn simulate_cluster(rng: &mut Rng, elo: f64, draw: f64, correlation: f64) -> SeedResult {
        let expected = 1.0 / (1.0 + 10f64.powf(-elo / 400.0));
        let p_draw = draw.min(2.0 * expected.min(1.0 - expected));
        let p_win = expected - p_draw / 2.0;
        let leg_draw = |u: f64| {
            if u < p_draw {
                0.5
            } else if u < p_draw + p_win {
                1.0
            } else {
                0.0
            }
        };
        let first = leg_draw(rng.uniform());
        let correlated = rng.uniform() < correlation;
        let second = if correlated { first } else { leg_draw(rng.uniform()) };
        let mut h = DefaultHasher::new();
        rng.next().hash(&mut h);
        seed_result(h.finish(), first, second)
    }

    fn run_trial(rng: &mut Rng, elo: f64, draw: f64, corr: f64, spec: &GsprtSpec, cap: usize) -> StoppingOutcome {
        let mut eval = GsprtEvaluator::new(spec.clone(), cap);
        for _ in 0..cap {
            eval.observe(&simulate_cluster(rng, elo, draw, corr));
            if let Some(outcome) = eval.decision().unwrap() {
                return outcome;
            }
        }
        eval.exhausted().unwrap()
    }

    #[test]
    fn counts_from_cluster_scores() {
        let mut eval = GsprtEvaluator::new(spec(), 100);
        eval.observe(&seed_result(1, 1.0, 1.0)); // s = 2.0
        eval.observe(&seed_result(2, 1.0, 0.5)); // s = 1.5
        eval.observe(&seed_result(3, 0.5, 0.5)); // s = 1.0
        eval.observe(&seed_result(4, 0.0, 0.5)); // s = 0.5
        eval.observe(&seed_result(5, 0.0, 0.0)); // s = 0.0
        assert_eq!(eval.counts, [1.0, 1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn llr_antisymmetric_under_hypothesis_swap() {
        let counts = [4.0, 3.0, 10.0, 3.0, 4.0];
        let spec = spec();
        let forward = gsprt_llr(&counts, &spec).unwrap();
        let swapped = gsprt_llr(
            &counts,
            &GsprtSpec {
                elo0: spec.elo1,
                elo1: spec.elo0,
                ..spec.clone()
            },
        )
        .unwrap();
        assert!((forward + swapped).abs() < 1e-6, "{forward} vs {swapped}");
    }

    #[test]
    fn strong_evidence_pushes_llr_in_the_right_direction() {
        let spec = spec();
        let dominant = [0.0, 0.0, 1.0, 2.0, 200.0];
        assert!(gsprt_llr(&dominant, &spec).unwrap() > spec.upper_bound());
        let dominated = [200.0, 2.0, 1.0, 0.0, 0.0];
        assert!(gsprt_llr(&dominated, &spec).unwrap() < spec.lower_bound());
    }

    #[test]
    fn decision_respects_min_clusters() {
        let mut eval = GsprtEvaluator::new(wide_spec(), 100);
        eval.observe(&seed_result(1, 1.0, 1.0));
        eval.observe(&seed_result(2, 1.0, 1.0));
        assert!(eval.decision().unwrap().is_none());
        for i in 3..=4 {
            eval.observe(&seed_result(i, 1.0, 1.0));
        }
        assert!(matches!(
            eval.decision().unwrap(),
            Some(StoppingOutcome::GsprtAcceptUpper { .. })
        ));
    }

    #[test]
    fn empty_counts_do_not_decide() {
        let eval = GsprtEvaluator::new(spec(), 100);
        assert!(eval.decision().unwrap().is_none());
        // Below min_clusters no LLR was ever computed — the record stays
        // honest rather than fabricating 0.0.
        assert!(matches!(
            eval.exhausted().unwrap(),
            StoppingOutcome::GsprtCapReached {
                llr: None,
                equivalence_cs: None,
                ..
            }
        ));
    }

    #[test]
    fn invalid_spec_fails_closed() {
        assert!(GsprtSpec {
            elo0: 5.0,
            elo1: 5.0,
            ..spec()
        }
        .validate()
        .is_err());
        assert!(GsprtSpec {
            elo0: 0.0,
            elo1: f64::NAN,
            ..spec()
        }
        .validate()
        .is_err());
        assert!(GsprtSpec { alpha: 0.0, ..spec() }.validate().is_err());
        assert!(GsprtSpec { beta: 0.9, ..spec() }.validate().is_err());
        // Extreme-but-finite bounds must fail validation: beyond the elo
        // bound the score hypothesis leaves the support and the
        // constrained MLE cannot resolve — the run would silently never
        // stop and record a fabricated interior LLR.
        assert!(GsprtSpec {
            elo0: 0.0,
            elo1: 10000.0,
            ..spec()
        }
        .validate()
        .is_err());
        assert!(GsprtSpec {
            elo0: -3000.0,
            elo1: 0.0,
            ..spec()
        }
        .validate()
        .is_err());
    }

    #[test]
    fn secular_mle_hits_the_target_mean() {
        let counts = [2.0, 5.0, 20.0, 8.0, 5.0];
        let mut probs = [0.0f64; 5];
        let n: f64 = counts.iter().sum::<f64>() + 5.0 * REGULARIZER;
        for i in 0..5 {
            probs[i] = (counts[i] + REGULARIZER) / n;
        }
        for s in [0.3, 0.5, 0.65] {
            let mle = mle_expected(&probs, s).unwrap();
            let mean: f64 = mle.iter().zip(BIN_SCORES.iter()).map(|(p, a)| p * a).sum();
            assert!((mean - s).abs() < 1e-9, "mean {mean} != {s}");
            assert!(mle.iter().all(|p| *p > 0.0));
        }
    }

    /// At the upper boundary the test should pass with rate ≈ 1−β and
    /// never misclassify as a failure beyond a small tolerance. The draw
    /// rate and correlation are nontrivial so the pentanomial — not a
    /// trinomial — is the honest model.
    #[test]
    fn synthetic_upper_boundary_passes() {
        let spec = synthetic_spec();
        let trials = 300;
        let mut rng = Rng(0xA11CE);
        let mut passed = 0;
        let mut failed = 0;
        let mut used = 0usize;
        for _ in 0..trials {
            match run_trial(&mut rng, spec.elo1, 0.35, 0.3, &spec, 2000) {
                StoppingOutcome::GsprtAcceptUpper { clusters_played, .. } => {
                    passed += 1;
                    used += clusters_played;
                }
                StoppingOutcome::GsprtAcceptLower { .. } => failed += 1,
                _ => {}
            }
        }
        assert!(passed as f64 / trials as f64 >= 0.90, "pass rate {passed}/{trials}");
        // Nominal β is 5%; the observed seeded rate runs a little above
        // because the plug-in tilt and discrete-time overshoot inflate
        // early crossings (the GSPRT guarantee is asymptotic). The bound
        // here separates correct behavior from a sign-flipped or
        // independence-assuming implementation, which would sit >15%.
        assert!(failed as f64 / trials as f64 <= 0.08, "false fails {failed}/{trials}");
        // The whole point of a sequential test: the boundary case should
        // resolve well inside the hard cap, not exhaust it.
        assert!(used as f64 / passed.max(1) as f64 <= 1200.0);
    }

    #[test]
    fn synthetic_lower_boundary_fails() {
        let spec = synthetic_spec();
        let trials = 300;
        let mut rng = Rng(0xBEEF);
        let mut passed = 0;
        let mut failed = 0;
        for _ in 0..trials {
            match run_trial(&mut rng, spec.elo0, 0.35, 0.3, &spec, 2000) {
                StoppingOutcome::GsprtAcceptLower { .. } => failed += 1,
                StoppingOutcome::GsprtAcceptUpper { .. } => passed += 1,
                _ => {}
            }
        }
        assert!(failed as f64 / trials as f64 >= 0.90, "fail rate {failed}/{trials}");
        assert!(passed as f64 / trials as f64 <= 0.05, "false passes {passed}/{trials}");
    }

    /// Under strong within-cluster correlation the pentanomial still
    /// behaves — this is precisely why the test runs on cluster counts.
    #[test]
    fn synthetic_correlated_clusters_stay_valid() {
        let spec = synthetic_spec();
        let trials = 250;
        let mut rng = Rng(0xC0FFEE);
        let mut passed = 0;
        for _ in 0..trials {
            if matches!(
                run_trial(&mut rng, spec.elo1 + 10.0, 0.30, 0.8, &spec, 2500),
                StoppingOutcome::GsprtAcceptUpper { .. }
            ) {
                passed += 1;
            }
        }
        assert!(passed as f64 / trials as f64 >= 0.85, "pass rate {passed}/{trials}");
    }

    /// Equal-strength candidates must mostly not pass the upper bound.
    #[test]
    fn synthetic_null_rarely_passes() {
        let spec = synthetic_spec();
        let trials = 300;
        let mut rng = Rng(0x5EED);
        let mut passed = 0;
        for _ in 0..trials {
            if matches!(
                run_trial(&mut rng, 0.0, 0.35, 0.3, &spec, 2000),
                StoppingOutcome::GsprtAcceptUpper { .. }
            ) {
                passed += 1;
            }
        }
        assert!(
            passed as f64 / trials as f64 <= 0.08,
            "false-pass rate {passed}/{trials}"
        );
    }
}
