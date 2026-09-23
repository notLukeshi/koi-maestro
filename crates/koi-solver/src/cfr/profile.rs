//! The blueprint: per-information-set regret and average-strategy tables.
//!
//! Ported from a sibling research codebase's `solver/cfr/profile.rs` — the lazy-weighted
//! average-strategy machinery is game-agnostic.

/// Cumulative regret and average-strategy tables for one player's
/// information sets, keyed by the tree's infoset ids.
///
/// The average strategy uses lazy weighting: each visit adds the current
/// strategy scaled by the elapsed iteration count, so unvisited iterations
/// correctly re-weight the last observed strategy when the infoset is next
/// updated. [`Blueprint::flush`] closes the final gap at reporting time.
#[derive(Debug, Clone)]
pub struct Blueprint {
    regrets: Vec<Vec<f64>>,
    strategy_sum: Vec<Vec<f64>>,
    last_update: Vec<usize>,
    iterations: usize,
}

impl Blueprint {
    /// Builds empty tables from each infoset's action count.
    pub fn new(action_counts: Vec<usize>) -> Self {
        let regrets = action_counts.iter().map(|&count| vec![0.0; count]).collect();
        let strategy_sum = action_counts.iter().map(|&count| vec![0.0; count]).collect();
        Self {
            regrets,
            strategy_sum,
            last_update: vec![0; action_counts.len()],
            iterations: 0,
        }
    }

    pub fn infoset_count(&self) -> usize {
        self.regrets.len()
    }

    pub fn iterations(&self) -> usize {
        self.iterations
    }

    /// The current behavioral strategy at an infoset (regret matching).
    pub fn current_strategy(&self, infoset: usize) -> Vec<f64> {
        super::regret::regret_matching(&self.regrets[infoset])
    }

    /// Adds counterfactual regret to one action. With `clamp_positive`
    /// (CFR+) the whole table is re-clamped at zero after the update.
    pub fn add_regret(&mut self, infoset: usize, action: usize, regret: f64, clamp_positive: bool) {
        let table = &mut self.regrets[infoset];
        table[action] += regret;
        if clamp_positive {
            for value in table.iter_mut() {
                *value = value.max(0.0);
            }
        }
    }

    /// The cumulative regret row at an infoset. Read-only: predictive
    /// variants (PCFR+) compose `regrets + prediction` outside the table.
    pub fn regret_row(&self, infoset: usize) -> &[f64] {
        &self.regrets[infoset]
    }

    /// Multiplies every stored regret at an infoset by its sign-appropriate
    /// factor — DCFR's positive/negative discounting. Applied to the
    /// updater's tables once per its own update iteration.
    pub fn discount_regrets(&mut self, infoset: usize, positive_factor: f64, negative_factor: f64) {
        for value in &mut self.regrets[infoset] {
            *value *= if *value > 0.0 { positive_factor } else { negative_factor };
        }
    }

    /// Accumulates the current strategy into the average with an explicit
    /// weight (CFR+'s linear `t * pi_updater` weighting) and marks the
    /// infoset as current through this iteration.
    pub fn accumulate_weighted(&mut self, infoset: usize, weight: f64) {
        let current = self.current_strategy(infoset);
        self.accumulate_strategy(infoset, weight, &current);
    }

    /// Accumulates an already-computed current strategy with an explicit
    /// weight: the traversals hold the strategy they acted on, so the
    /// regret-matching recompute inside `accumulate_weighted` is skipped.
    pub fn accumulate_strategy(&mut self, infoset: usize, weight: f64, strategy: &[f64]) {
        let sum = &mut self.strategy_sum[infoset];
        for (slot, &value) in sum.iter_mut().zip(strategy) {
            *slot += weight * value;
        }
        self.last_update[infoset] = self.iterations;
    }

    /// Lazy averaging (external-sampling MCCFR): weights the traversed
    /// strategy by the iterations elapsed since the infoset's last update,
    /// so iterations that never sampled this infoset still count its last
    /// observed strategy.
    pub fn accumulate_elapsed_with(&mut self, infoset: usize, strategy: &[f64]) {
        let elapsed = self.iterations - self.last_update[infoset];
        self.accumulate_strategy(infoset, elapsed as f64, strategy);
    }

    /// Advances the iteration counter.
    pub fn advance_iteration(&mut self) {
        self.iterations += 1;
    }

    /// Adds the current strategy for the iterations elapsed since each
    /// infoset's last update, closing the lazy-averaging gap.
    pub fn flush(&mut self) {
        for infoset in 0..self.regrets.len() {
            let elapsed = self.iterations - self.last_update[infoset];
            if elapsed > 0 {
                self.accumulate_weighted(infoset, elapsed as f64);
            }
        }
    }

    /// The average strategy at an infoset; uniform before any update.
    pub fn average_strategy(&self, infoset: usize) -> Vec<f64> {
        let sum = &self.strategy_sum[infoset];
        let total: f64 = sum.iter().sum();
        if total > 0.0 {
            sum.iter().map(|value| value / total).collect()
        } else {
            let uniform = 1.0 / sum.len() as f64;
            vec![uniform; sum.len()]
        }
    }

    /// The full average-strategy profile, flushed lazily on a clone.
    pub fn averaged_profile(&self) -> Vec<Vec<f64>> {
        (0..self.regrets.len())
            .map(|infoset| self.average_strategy(infoset))
            .collect()
    }

    /// The cumulative regrets at an infoset (test-only introspection for
    /// estimator unbiasedness checks).
    #[cfg(test)]
    pub(crate) fn regrets(&self, infoset: usize) -> &[f64] {
        &self.regrets[infoset]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blueprint() -> Blueprint {
        Blueprint::new(vec![2, 3])
    }

    #[test]
    fn fresh_tables_play_uniform_and_average_uniform() {
        let blueprint = blueprint();
        assert_eq!(blueprint.current_strategy(0), vec![0.5, 0.5]);
        assert_eq!(blueprint.average_strategy(0), vec![0.5, 0.5]);
        assert_eq!(blueprint.iterations(), 0);
    }

    #[test]
    fn lazy_averaging_weights_elapsed_iterations() {
        let mut blueprint = blueprint();
        blueprint.advance_iteration();
        blueprint.add_regret(0, 0, 1.0, false);
        blueprint.accumulate_weighted(0, 2.0);
        blueprint.advance_iteration();
        blueprint.flush();
        assert_eq!(blueprint.average_strategy(0), vec![1.0, 0.0]);
    }

    #[test]
    fn flush_weights_the_unvisited_tail_with_the_current_strategy() {
        let mut blueprint = blueprint();
        blueprint.advance_iteration();
        blueprint.add_regret(0, 1, 1.0, false);
        blueprint.accumulate_weighted(0, 1.0);
        blueprint.advance_iteration();
        blueprint.advance_iteration();
        blueprint.flush();
        assert_eq!(blueprint.average_strategy(0), vec![0.0, 1.0]);
    }

    #[test]
    fn cfr_plus_clamping_keeps_regrets_non_negative() {
        let mut blueprint = blueprint();
        blueprint.add_regret(1, 0, -3.0, true);
        assert_eq!(blueprint.regrets[1], vec![0.0, 0.0, 0.0]);
        blueprint.add_regret(1, 2, 4.0, true);
        assert_eq!(blueprint.regrets[1], vec![0.0, 0.0, 4.0]);
    }
}
