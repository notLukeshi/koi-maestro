//! CFR+ (Tammelin 2014): alternating full-traversal updates with
//! regret-matching+ and linear average-strategy weighting.
//!
//! Ported from a sibling research codebase's `solver/cfr/cfr_plus.rs`; the traversal is
//! game-agnostic over `EfgTree` — only the player/utility names changed
//! (Player1 → South, `utility_p1` → `utility_south`).

use super::regret::ActionBuf;
use koi_core::Player;

use super::profile::Blueprint;
use crate::efg::tree::{EfgTree, NodeId, NodeType};

/// Trains a blueprint with CFR+ for `iterations` alternating updates and
/// returns the flushed average profile.
pub fn train_cfr_plus(tree: &EfgTree, iterations: usize) -> Blueprint {
    let action_counts: Vec<usize> = tree.infosets().iter().map(|infoset| infoset.actions.len()).collect();
    let mut blueprint = Blueprint::new(action_counts.clone());
    // Instant-regret accumulator: RM+ clamps once per infoset on the
    // aggregate `Q[I,a] += sum_h pi_-i(h)*delta(h,a)`, not per member —
    // per-member clamping lets negative members no longer cancel positive
    // ones (correctness review). Zeroed per traversal.
    let mut instants: Vec<Vec<f64>> = action_counts.iter().map(|&count| vec![0.0; count]).collect();
    for iteration in 0..iterations {
        blueprint.advance_iteration();
        let updater = if iteration % 2 == 0 {
            Player::South
        } else {
            Player::North
        };
        for row in instants.iter_mut() {
            row.fill(0.0);
        }
        traverse(
            tree,
            tree.root(),
            updater,
            (iteration + 1) as f64,
            1.0,
            1.0,
            1.0,
            &mut blueprint,
            &mut |profile: &Blueprint, infoset: usize, out: &mut ActionBuf| {
                super::regret::regret_matching_into(profile.regret_row(infoset), out)
            },
            &mut |infoset: usize, instant: &[f64]| {
                for (action, &regret) in instant.iter().enumerate() {
                    instants[infoset][action] += regret;
                }
            },
        );
        for (infoset, row) in instants.iter().enumerate() {
            for (action, &regret) in row.iter().enumerate() {
                blueprint.add_regret(infoset, action, regret, true);
            }
        }
    }
    blueprint.flush();
    blueprint
}

/// One full-traversal pass for `updater` under the CFR-family update shape.
/// Returns the updater's expected utility from this node under the current
/// profile. `pub(super)` so sibling trainers reuse the identical walk:
/// DCFR varies only the average weight, PCFR+ supplies its own strategy
/// read (`R + prediction`) and observes each instant regret vector.
///
/// `strategy_of` maps an infoset to the iterate the traversal acts on;
/// `observe_instant` receives the reach-weighted instant regret row at
/// every updater node — the trainer accumulates those rows per infoset and
/// applies one clamped `add_regret` per infoset after the walk (RM+ clamps
/// on the aggregate, not per member).
///
/// The counterfactual regret weight is `pi_chance * pi_opp` — the full
/// `pi_-i` including nature's contribution (Zinkevich 2008). `pi_chance`
/// matters exactly where chance is non-uniform: `compile_subgame`'s
/// belief-weighted root mixes worlds with different probabilities, so a
/// uniform-regret update would solve the uniform-mixture game, not the
/// stated one. On `compile_variant` trees the factor is a per-infoset
/// constant and cancels inside regret matching — harmless there, required
/// here.
#[allow(clippy::too_many_arguments)]
pub(super) fn traverse(
    tree: &EfgTree,
    node: NodeId,
    updater: Player,
    weight: f64,
    pi_updater: f64,
    pi_opp: f64,
    pi_chance: f64,
    blueprint: &mut Blueprint,
    strategy_of: &mut dyn FnMut(&Blueprint, usize, &mut ActionBuf),
    observe_instant: &mut dyn FnMut(usize, &[f64]),
) -> f64 {
    match tree.node(node) {
        NodeType::Terminal { utility_south } => match updater {
            Player::South => *utility_south,
            Player::North => -utility_south,
        },
        NodeType::Chance { outcomes } => outcomes
            .iter()
            .map(|outcome| {
                outcome.probability
                    * traverse(
                        tree,
                        outcome.child,
                        updater,
                        weight,
                        pi_updater,
                        pi_opp,
                        pi_chance * outcome.probability,
                        blueprint,
                        strategy_of,
                        observe_instant,
                    )
            })
            .sum(),
        NodeType::Decision {
            player,
            infoset,
            children,
            ..
        } => {
            let mut strategy = ActionBuf::new();
            strategy_of(blueprint, *infoset, &mut strategy);
            if *player == updater {
                let mut values = ActionBuf::with_capacity(children.len());
                for (action, child) in children.iter().enumerate() {
                    values.push(traverse(
                        tree,
                        *child,
                        updater,
                        weight,
                        pi_updater * strategy[action],
                        pi_opp,
                        pi_chance,
                        blueprint,
                        strategy_of,
                        observe_instant,
                    ));
                }
                let value: f64 = strategy
                    .iter()
                    .zip(&values)
                    .map(|(probability, value)| probability * value)
                    .sum();
                // The linear average weights the strategy that produced these
                // values: accumulate before the regret update re-clamps.
                blueprint.accumulate_strategy(*infoset, weight * pi_updater, &strategy);
                let instant: ActionBuf = values
                    .iter()
                    .map(|action_value| pi_chance * pi_opp * (action_value - value))
                    .collect();
                observe_instant(*infoset, &instant);
                value
            } else {
                children
                    .iter()
                    .zip(&strategy)
                    .map(|(&child, probability)| {
                        probability
                            * traverse(
                                tree,
                                child,
                                updater,
                                weight,
                                pi_updater,
                                pi_opp * probability,
                                pi_chance,
                                blueprint,
                                strategy_of,
                                observe_instant,
                            )
                    })
                    .sum()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::efg::tree::{ChanceOutcome, InfoSet, InfoSetKey};
    use crate::efg::KoiVariant;
    use koi_core::{Action, CaptureChoice, Card};

    /// Two worlds the updater cannot distinguish (same infoset) with
    /// opposite optimal actions, mixed by a non-uniform chance fork —
    /// `compile_subgame`'s belief-weighted root in miniature.
    ///
    ///   Chance(p) -> world A: a0 -> +2, a1 -> 0
    ///   Chance(1-p) -> world B: a0 -> 0, a1 -> +2
    ///
    /// The correct counterfactual regret is `p * instant_A + (1-p) *
    /// instant_B`; with p = 0.9 action 0 dominates and CFR+ must converge
    /// to it. The pre-fix traversal dropped `pi_chance`, so both worlds
    /// contributed weight 1 and the aggregate cancelled exactly — the
    /// strategy would stay uniform forever on this fixture.
    fn belief_tree(p: f64) -> EfgTree {
        let mut tree = EfgTree::new(KoiVariant::NANO_6);
        let actions = vec![
            Action::PlayFromHand {
                card: Card::new_unchecked(0),
                capture: CaptureChoice::NoMatch,
            },
            Action::PlayFromHand {
                card: Card::new_unchecked(1),
                capture: CaptureChoice::NoMatch,
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
        let world_a = tree.push_decision(Player::South, infoset, actions.clone());
        let world_b = tree.push_decision(Player::South, infoset, actions);
        tree.infoset_mut(infoset).members.push(world_a);
        tree.infoset_mut(infoset).members.push(world_b);
        let win = tree.push_terminal(2.0);
        let lose = tree.push_terminal(0.0);
        tree.set_children(world_a, vec![win, lose]);
        tree.set_children(world_b, vec![lose, win]);
        let root = tree.push_chance(vec![
            ChanceOutcome {
                probability: p,
                dealt: Vec::new(),
                child: world_a,
            },
            ChanceOutcome {
                probability: 1.0 - p,
                dealt: Vec::new(),
                child: world_b,
            },
        ]);
        tree.set_root(root);
        tree
    }

    /// With the 0.9 world dominant the updater must play its action: the
    /// chance-weighted aggregate regret is [+0.8, -0.8] from a uniform
    /// start, so the profile converges to pure action 0 and the
    /// belief-weighted value to 0.9 * 2.
    #[test]
    fn belief_weighted_regrets_converge_to_the_dominant_worlds_action() {
        let tree = belief_tree(0.9);
        let profile = train_cfr_plus(&tree, 200).averaged_profile();
        assert!(
            profile[0][0] > 0.95,
            "chance-weighted regrets must favor the 0.9 world: {:?}",
            profile[0]
        );
        let value = crate::cfr::profile_value(&tree, &profile, Player::South);
        assert!(
            (value - 1.8).abs() < 0.05,
            "the dominant-world value is 1.8, got {value}"
        );
    }

    /// Negative control: the fixture is symmetric at 0.5/0.5 — every
    /// instant cancels exactly, so the strategy must remain uniform
    /// (regrets never move). This isolates the discrimination to the
    /// chance weight itself, not the fixture's geometry.
    #[test]
    fn uniform_belief_leaves_the_symmetric_fixture_indifferent() {
        let tree = belief_tree(0.5);
        let profile = train_cfr_plus(&tree, 200).averaged_profile();
        assert!(
            (profile[0][0] - 0.5).abs() < 1e-9,
            "symmetric beliefs must leave regrets at zero: {:?}",
            profile[0]
        );
    }
}
