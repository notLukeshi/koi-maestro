//! Predictive CFR+ (Farina, Kroer & Sandholm, AAAI 2021): regret-matching+
//! applied to the *predicted* next cumulative regret `R + m`, where `m` is
//! the most recently observed instantaneous regret at the infoset. The
//! prediction makes the method optimistic — it converges faster than CFR+
//! when consecutive instant regrets are correlated, which they are in
//! practice. The average strategy uses quadratic `t^2` weighting.
//!
//! Ported from a sibling research codebase's `solver/cfr/pcfr.rs` — game-agnostic.

use koi_core::Player;

use super::{cfr_plus, profile::Blueprint};
use crate::efg::tree::EfgTree;

/// Trains a blueprint with PCFR+ for `iterations` alternating updates and
/// returns the flushed average profile.
pub fn train_pcfr_plus(tree: &EfgTree, iterations: usize) -> Blueprint {
    let action_counts: Vec<usize> = tree.infosets().iter().map(|infoset| infoset.actions.len()).collect();
    let mut blueprint = Blueprint::new(action_counts.clone());
    // The prediction each infoset's next strategy is built from: the last
    // instantaneous regret vector observed there. Trainer-owned, not part of
    // `Blueprint` — only predictive variants pay for the third table.
    let mut predictions: Vec<Vec<f64>> = action_counts.iter().map(|&count| vec![0.0; count]).collect();
    // Per-traversal instant accumulator: an infoset's prediction is the
    // instant regret SUMMED over all of its members, matching the aggregate
    // `add_regret` receives — not the last member's partial vector.
    let mut instants = predictions.clone();
    let mut update_counts = [0usize; 2];
    for iteration in 0..iterations {
        blueprint.advance_iteration();
        let updater = if iteration % 2 == 0 {
            Player::South
        } else {
            Player::North
        };
        update_counts[updater.index()] += 1;
        let weight = {
            let t = update_counts[updater.index()] as f64;
            t * t
        };
        for row in instants.iter_mut() {
            row.fill(0.0);
        }
        // The shared CFR-family walk: PCFR+ reads the strategy from
        // `R + prediction` and records each instant vector for the next
        // iteration's prediction.
        cfr_plus::traverse(
            tree,
            tree.root(),
            updater,
            weight,
            1.0,
            1.0,
            1.0,
            &mut blueprint,
            &mut |profile: &Blueprint, infoset: usize, out: &mut super::regret::ActionBuf| {
                let mut predicted = super::regret::ActionBuf::new();
                for (regret, prediction) in profile.regret_row(infoset).iter().zip(&predictions[infoset]) {
                    predicted.push(regret + prediction);
                }
                super::regret::regret_matching_into(&predicted, out)
            },
            &mut |infoset: usize, instant: &[f64]| {
                for (action, regret) in instant.iter().enumerate() {
                    instants[infoset][action] += regret;
                }
            },
        );
        // Land the prediction only at the updater's infosets: predictions
        // are per-player — an infoset's prediction is the instant vector it
        // produced the last time ITS player was the updater. Copying the
        // whole instants buffer would zero the opponent's rows and make the
        // updater always read `R + 0`, degenerating PCFR+ to CFR+.
        for (id, infoset) in tree.infosets().iter().enumerate() {
            if infoset.player == updater {
                predictions[id].copy_from_slice(&instants[id]);
            }
        }
        // The aggregate RM+ update: one clamped add_regret per infoset on
        // the summed instant, not per member (shared CFR-family pattern).
        for (infoset, row) in instants.iter().enumerate() {
            for (action, &regret) in row.iter().enumerate() {
                blueprint.add_regret(infoset, action, regret, true);
            }
        }
    }
    blueprint.flush();
    blueprint
}

#[cfg(test)]
mod tests {
    use super::super::regret::regret_matching;
    use super::*;
    use crate::{
        cfr::exploitability::exploitability,
        efg::{compile_variant, KoiVariant},
    };

    #[test]
    fn pcfr_plus_converges_on_the_reduced_domain() {
        let tree = compile_variant(&KoiVariant::MICRO_8, Player::South).unwrap();
        let profile = train_pcfr_plus(&tree, 2_000).averaged_profile();
        let expl = exploitability(&tree, &profile);
        assert!(
            expl < 1e-2,
            "PCFR+ exploitability after 2000 iterations should be small, got {expl}"
        );
    }

    /// The predictive strategy must read `R + m`: with stored regrets
    /// [0, 0] and prediction [3, 1], the next strategy is 3:1.
    #[test]
    fn the_strategy_uses_the_prediction() {
        let mut blueprint = Blueprint::new(vec![2]);
        let mut predictions = [vec![3.0, 1.0]];
        let regrets = blueprint.regret_row(0);
        let predicted: Vec<f64> = regrets
            .iter()
            .zip(&predictions[0])
            .map(|(regret, prediction)| regret + prediction)
            .collect();
        let strategy = regret_matching(&predicted);
        assert_eq!(strategy, vec![0.75, 0.25]);
        blueprint.add_regret(0, 0, 1.0, true);
        predictions[0] = vec![-5.0, 2.0];
        let regrets = blueprint.regret_row(0);
        let predicted: Vec<f64> = regrets
            .iter()
            .zip(&predictions[0])
            .map(|(regret, prediction)| regret + prediction)
            .collect();
        // R = [1, 0], m = [-5, 2] -> R + m = [-4, 2] -> pure action 1.
        assert_eq!(regret_matching(&predicted), vec![0.0, 1.0]);
    }
}
