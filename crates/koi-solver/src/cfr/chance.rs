//! Chance-outcome sampling for Monte Carlo CFR.
//!
//! [`UnitDraw`] abstracts how a traversal draws the uniform deviate that the
//! chance node's distribution is mapped through: [`RngDraw`] reproduces the
//! standard i.i.d. sampler, [`CcsDraw`] implements Correlated Chance Sampling
//! (CCS-MCCFR, Li, Chen & Huang, arXiv 2607.27035): every concrete chance
//! node owns a persistent randomized Weyl stream, so the N deviates a node
//! consumes across N visits carry the sequence's O(log(N+1)/N) frequency
//! error instead of i.i.d. O(N^-1/2). The strategy updates are untouched —
//! only the chance draw is correlated. The paper's convergence bound applies
//! to the per-traversal-reset variant; for persistent streams we claim the
//! measured convergence improvement, not the guarantee.
//!
//! Ported from a sibling research codebase's `solver/cfr/chance.rs` — game-agnostic.

use std::collections::HashMap;

use rand::RngExt;

use crate::efg::tree::NodeId;

/// The Weyl step: `u_n = frac(alpha * n + shift)`. `alpha` must be
/// irrational for the stream to be low-discrepancy; the golden-ratio
/// conjugate is the standard 1-D choice.
pub const WEYL_STEP: f64 = 0.618_033_988_749_894_9;

/// One concrete chance node's persistent deviate stream. `shift` is the
/// Cranley-Patterson randomization drawn once when the node is first
/// visited; `counter` persists across the whole training run.
#[derive(Debug, Clone)]
pub struct WeylStream {
    shift: f64,
    counter: u64,
}

impl WeylStream {
    pub fn new(shift: f64) -> Self {
        Self { shift, counter: 0 }
    }

    /// The next low-discrepancy deviate in `[0, 1)`. Every index has the
    /// correct marginal law (the shifted sequence is uniform per index)
    /// while consecutive deviates never clump the way i.i.d. draws do.
    pub fn next_unit(&mut self) -> f64 {
        self.counter += 1;
        (WEYL_STEP * self.counter as f64 + self.shift).fract()
    }
}

/// The source of deviates inside one MCCFR traversal: `next_unit` feeds
/// chance nodes (the CCS-correlated path), `next` feeds opponent-action
/// sampling (correlating those is not what CCS does — they stay i.i.d.).
pub trait UnitDraw {
    /// The deviate a chance node maps through its outcome distribution.
    fn next_unit(&mut self, node: NodeId) -> f64;
    /// The deviate for sampling an opponent action.
    fn next(&mut self) -> f64;
}

/// i.i.d. uniform draws — the standard external-sampling behaviour.
pub struct RngDraw<'a, R: RngExt>(pub &'a mut R);

impl<R: RngExt> UnitDraw for RngDraw<'_, R> {
    fn next_unit(&mut self, _node: NodeId) -> f64 {
        self.0.random::<f64>()
    }

    fn next(&mut self) -> f64 {
        self.0.random::<f64>()
    }
}

/// Correlated chance sampling: one [`WeylStream`] per concrete chance node,
/// lazily seeded from `rng` on first visit so stream assignment is
/// deterministic given the trainer's seed.
pub struct CcsDraw<'a, R: RngExt> {
    streams: &'a mut HashMap<NodeId, WeylStream>,
    rng: &'a mut R,
}

impl<'a, R: RngExt> CcsDraw<'a, R> {
    pub fn new(streams: &'a mut HashMap<NodeId, WeylStream>, rng: &'a mut R) -> Self {
        Self { streams, rng }
    }
}

impl<R: RngExt> UnitDraw for CcsDraw<'_, R> {
    fn next_unit(&mut self, node: NodeId) -> f64 {
        self.streams
            .entry(node)
            .or_insert_with(|| WeylStream::new(self.rng.random::<f64>()))
            .next_unit()
    }

    fn next(&mut self) -> f64 {
        self.rng.random::<f64>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Weyl stream's empirical frequency of a probability-`p` event
    /// (threshold cut) converges at the quasi-Monte-Carlo rate.
    #[test]
    fn weyl_stream_hits_target_frequency_at_low_discrepancy() {
        let mut stream = WeylStream::new(0.2718);
        let p = 0.3;
        let draws = 1024;
        let hits = (0..draws).filter(|_| stream.next_unit() < p).count();
        let error = (hits as f64 / draws as f64 - p).abs();
        assert!(
            error < 0.01,
            "weyl frequency error {error} should be far below the i.i.d. scale"
        );
    }

    #[test]
    fn weyl_streams_are_uniform_and_distinct_per_shift() {
        let mut a = WeylStream::new(0.0);
        let mut b = WeylStream::new(0.5);
        let seq_a: Vec<f64> = (0..16).map(|_| a.next_unit()).collect();
        let seq_b: Vec<f64> = (0..16).map(|_| b.next_unit()).collect();
        assert!(seq_a.iter().all(|u| (0.0..1.0).contains(u)));
        assert_ne!(seq_a, seq_b);
        let mut again = WeylStream::new(0.0);
        let seq_again: Vec<f64> = (0..16).map(|_| again.next_unit()).collect();
        assert_eq!(seq_a, seq_again);
    }
}
