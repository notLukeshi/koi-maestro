//! External-sampling Monte Carlo CFR (Lanctot et al., NeurIPS 2009).
//!
//! Each iteration traverses for one updater: the updater's own actions are
//! enumerated, while chance and opponent actions are sampled. A member `h`
//! of an updater infoset is reached with probability exactly `pi_-i(h)` —
//! the chance and opponent reach — so the sampled visit already carries the
//! counterfactual weight and the regret increment is added unweighted.
//! Multiplying the sampled probability in again would square the reach and
//! bias the estimator against vanilla CFR's `r(I,a) = sum_h pi_-i(h) *
//! (v(h.a) - v(h))`.
//!
//! Ported from a sibling research codebase's `solver/cfr/mccfr.rs` — game-agnostic.

use rand::RngExt;

use koi_core::Player;

use super::{
    chance::{CcsDraw, RngDraw, UnitDraw},
    profile::Blueprint,
};
use crate::efg::tree::{EfgTree, NodeId, NodeType};

/// Trains a blueprint with external-sampling MCCFR for `iterations`
/// alternating updates and returns the flushed average profile.
pub fn train_external_sampling(tree: &EfgTree, iterations: usize, rng: &mut impl RngExt) -> Blueprint {
    let action_counts: Vec<usize> = tree.infosets().iter().map(|infoset| infoset.actions.len()).collect();
    let mut blueprint = Blueprint::new(action_counts);
    for iteration in 0..iterations {
        blueprint.advance_iteration();
        let updater = if iteration % 2 == 0 {
            Player::South
        } else {
            Player::North
        };
        let mut draw = RngDraw(rng);
        traverse(tree, tree.root(), updater, &mut blueprint, &mut draw);
    }
    blueprint.flush();
    blueprint
}

/// External-sampling MCCFR with Correlated Chance Sampling (CCS-MCCFR,
/// arXiv 2607.27035): every concrete chance node draws its deviate from a
/// persistent randomized Weyl stream instead of i.i.d. uniforms, which cuts
/// the local chance-frequency error to O(log(N+1)/N). Strategy updates are
/// identical to [`train_external_sampling`].
pub fn train_external_sampling_ccs(tree: &EfgTree, iterations: usize, rng: &mut impl RngExt) -> Blueprint {
    let action_counts: Vec<usize> = tree.infosets().iter().map(|infoset| infoset.actions.len()).collect();
    let mut blueprint = Blueprint::new(action_counts);
    let mut streams = std::collections::HashMap::new();
    for iteration in 0..iterations {
        blueprint.advance_iteration();
        let updater = if iteration % 2 == 0 {
            Player::South
        } else {
            Player::North
        };
        let mut draw = CcsDraw::new(&mut streams, rng);
        traverse(tree, tree.root(), updater, &mut blueprint, &mut draw);
    }
    blueprint.flush();
    blueprint
}

fn traverse(tree: &EfgTree, node: NodeId, updater: Player, blueprint: &mut Blueprint, draw: &mut dyn UnitDraw) -> f64 {
    match tree.node(node) {
        NodeType::Terminal { utility_south } => match updater {
            Player::South => *utility_south,
            Player::North => -utility_south,
        },
        NodeType::Chance { outcomes } => {
            let drawn = draw.next_unit(node);
            let mut cumulative = 0.0;
            for outcome in outcomes {
                cumulative += outcome.probability;
                if drawn < cumulative {
                    return traverse(tree, outcome.child, updater, blueprint, draw);
                }
            }
            let last = outcomes.last().expect("chance nodes have outcomes");
            traverse(tree, last.child, updater, blueprint, draw)
        }
        NodeType::Decision {
            player,
            infoset,
            children,
            ..
        } => {
            let strategy = blueprint.current_strategy(*infoset);
            if *player == updater {
                let mut values = Vec::with_capacity(children.len());
                for child in children {
                    values.push(traverse(tree, *child, updater, blueprint, draw));
                }
                let value: f64 = strategy
                    .iter()
                    .zip(&values)
                    .map(|(probability, value)| probability * value)
                    .sum();
                // Average the strategy that produced these values: accumulate
                // before the regret update changes regret matching.
                blueprint.accumulate_elapsed_with(*infoset, &strategy);
                for (action, action_value) in values.iter().enumerate() {
                    blueprint.add_regret(*infoset, action, action_value - value, false);
                }
                value
            } else {
                let drawn = draw.next();
                let mut cumulative = 0.0;
                for (action, probability) in strategy.iter().enumerate() {
                    cumulative += probability;
                    if drawn < cumulative {
                        return traverse(tree, children[action], updater, blueprint, draw);
                    }
                }
                let last = strategy.len() - 1;
                traverse(tree, children[last], updater, blueprint, draw)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rand::{rngs::SmallRng, SeedableRng};

    use super::super::exploitability::counterfactual_reach;
    use super::*;
    use crate::efg::{compile_variant, KoiVariant};

    /// The exact expected updater utility of a subtree under the uniform
    /// profile a fresh blueprint induces.
    fn uniform_value(tree: &EfgTree, node: NodeId, updater: Player, memo: &mut [Option<f64>]) -> f64 {
        if let Some(value) = memo[node] {
            return value;
        }
        let value = match tree.node(node) {
            NodeType::Terminal { utility_south } => match updater {
                Player::South => *utility_south,
                Player::North => -utility_south,
            },
            NodeType::Chance { outcomes } => outcomes
                .iter()
                .map(|outcome| outcome.probability * uniform_value(tree, outcome.child, updater, memo))
                .sum(),
            NodeType::Decision { children, .. } => {
                children
                    .iter()
                    .map(|&child| uniform_value(tree, child, updater, memo))
                    .sum::<f64>()
                    / children.len() as f64
            }
        };
        memo[node] = Some(value);
        value
    }

    /// The estimator is unbiased: over many fresh traversals the mean regret
    /// increment at an infoset must equal the exact counterfactual regret
    /// `sum_h pi_-i(h) * (v(h.a) - v(h))` computed on the materialized tree.
    /// If the sampled opponent reach were multiplied in again, the mean
    /// would shrink toward `sum_h pi_-i(h)^2 * delta` and fail the bound.
    #[test]
    fn the_sampled_regret_increment_is_unbiased() {
        let tree = compile_variant(&KoiVariant::MICRO_8, Player::South).unwrap();
        let updater = Player::South;
        let uniform: Vec<Vec<f64>> = tree
            .infosets()
            .iter()
            .map(|infoset| vec![1.0 / infoset.actions.len() as f64; infoset.actions.len()])
            .collect();
        let reach = counterfactual_reach(&tree, &uniform, updater);
        let mut memo = vec![None; tree.node_count()];

        // Exact per-infoset expected increment; the check runs on the
        // infoset whose increment magnitude is largest (most statistical
        // power against the zero-mean null).
        let mut target = None;
        let mut target_expected: Vec<f64> = Vec::new();
        let mut target_magnitude = 0.0;
        for (id, infoset) in tree.infosets().iter().enumerate() {
            if infoset.player != updater {
                continue;
            }
            let mut expected = vec![0.0; infoset.actions.len()];
            for &member in &infoset.members {
                let NodeType::Decision { children, .. } = tree.node(member) else {
                    unreachable!("information-set members are decision nodes")
                };
                let member_value = uniform_value(&tree, member, updater, &mut memo);
                for (action, &child) in children.iter().enumerate() {
                    let delta = uniform_value(&tree, child, updater, &mut memo) - member_value;
                    expected[action] += reach[member] * delta;
                }
            }
            let magnitude = expected.iter().fold(0.0_f64, |best, value| best.max(value.abs()));
            if magnitude > target_magnitude {
                target_magnitude = magnitude;
                target = Some(id);
                target_expected = expected;
            }
        }
        let target = target.expect("the updater has at least one infoset");
        assert!(
            target_magnitude > 1e-9,
            "the target infoset must carry a nonzero expected increment"
        );

        let action_counts: Vec<usize> = tree.infosets().iter().map(|i| i.actions.len()).collect();
        let trials = 12_000;
        let mut sum = vec![0.0; target_expected.len()];
        let mut squared = vec![0.0; target_expected.len()];
        let mut rng = SmallRng::seed_from_u64(0x5eed);
        for _ in 0..trials {
            let mut blueprint = Blueprint::new(action_counts.clone());
            let mut draw = RngDraw(&mut rng);
            traverse(&tree, tree.root(), updater, &mut blueprint, &mut draw);
            let increments = blueprint.regrets(target);
            for action in 0..target_expected.len() {
                sum[action] += increments[action];
                squared[action] += increments[action] * increments[action];
            }
        }

        for (action, &exact) in target_expected.iter().enumerate() {
            let mean = sum[action] / trials as f64;
            let variance = (squared[action] / trials as f64 - mean * mean).max(0.0);
            let standard_error = (variance / trials as f64).sqrt();
            assert!(
                (mean - exact).abs() <= 5.0 * standard_error + 1e-12,
                "action {action}: sampled increment mean {mean} vs exact {exact} (se {standard_error})"
            );
        }
    }

    /// The discrimination itself — does the estimator weight by pi_-i or by
    /// pi_-i squared? — is decided on a synthetic tree with finite support:
    /// a single-member infoset reached by a fair coin. The exact increment
    /// is `0.5 * delta`; a reach-weighted bug would converge to `0.25 *
    /// delta`. With only two equiprobable worlds the Monte-Carlo error bars
    /// are narrow, so the separation is decisive.
    #[test]
    fn the_unweighted_increment_is_not_the_reach_squared_one() {
        use crate::efg::tree::{ChanceOutcome, InfoSet, InfoSetKey};

        // Chance(0.5) -> decision(infoset 0, actions -> +4 / 0)
        // Chance(0.5) -> terminal(0)
        let mut tree = EfgTree::new(KoiVariant::MICRO_8);
        let actions = vec![
            koi_core::Action::PlayFromHand {
                card: koi_core::Card::new_unchecked(0),
                capture: koi_core::CaptureChoice::NoMatch,
            },
            koi_core::Action::PlayFromHand {
                card: koi_core::Card::new_unchecked(1),
                capture: koi_core::CaptureChoice::NoMatch,
            },
        ];
        let infoset = tree.push_infoset(
            InfoSetKey {
                player: Player::South,
                own_hand: 0,
                initial_field: 0,
                public_history: Vec::new(),
            },
            InfoSet {
                player: Player::South,
                actions: actions.clone(),
                members: Vec::new(),
            },
        );
        let member = tree.push_decision(Player::South, infoset, actions);
        tree.infoset_mut(infoset).members.push(member);
        let plus = tree.push_terminal(4.0);
        let zero = tree.push_terminal(0.0);
        tree.set_children(member, vec![plus, zero]);
        let dead = tree.push_terminal(0.0);
        let root = tree.push_chance(vec![
            ChanceOutcome {
                probability: 0.5,
                dealt: Vec::new(),
                child: member,
            },
            ChanceOutcome {
                probability: 0.5,
                dealt: Vec::new(),
                child: dead,
            },
        ]);
        tree.set_root(root);

        // Exact expectation: pi(h)=0.5, delta = [+2, -2] -> [+1, -1].
        let exact = [1.0, -1.0];
        // The reach-squared bug's prediction.
        let biased = [0.5, -0.5];

        let trials = 4_000;
        let mut sum = [0.0; 2];
        let mut squared = [0.0; 2];
        let mut rng = SmallRng::seed_from_u64(0xcafe);
        for _ in 0..trials {
            let mut blueprint = Blueprint::new(vec![2]);
            let mut draw = RngDraw(&mut rng);
            traverse(&tree, tree.root(), Player::South, &mut blueprint, &mut draw);
            let increments = blueprint.regrets(0);
            for action in 0..2 {
                sum[action] += increments[action];
                squared[action] += increments[action] * increments[action];
            }
        }
        for action in 0..2 {
            let mean = sum[action] / trials as f64;
            let variance = (squared[action] / trials as f64 - mean * mean).max(0.0);
            let se = (variance / trials as f64).sqrt();
            assert!(
                (mean - exact[action]).abs() <= 5.0 * se + 1e-12,
                "action {action}: mean {mean} vs exact {} (se {se})",
                exact[action]
            );
            assert!(
                (mean - biased[action]).abs() > 5.0 * se,
                "action {action}: mean {mean} did not reject the reach-squared value {} (se {se})",
                biased[action]
            );
        }
    }
}
