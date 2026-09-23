//! The ISMCTS search loop: determinize, walk, expand, roll out, propagate.
//!
//! One iteration samples a world consistent with the root observer's view,
//! descends the tree picking untried arms or availability-weighted UCB
//! edges, rolls out greedily from the frontier, and backpropagates the
//! root-observer-relative reward. Opponent nodes minimize that reward —
//! `ucb_score` flips the mean when the acting player is not the observer.
//!
//! Parallel mode runs `ISMCTS_PARALLEL_TREE_COUNT` independent trees on
//! derived seed streams and merges root-edge statistics in fixed tree
//! order. The tree count is a compile-time constant, so results are
//! schedule-invariant: the same seed yields the same action on any
//! thread pool.

use koi_core::{derive_named_seed, Action, KoiGameState, Player};
use rand::SeedableRng;
use rand::{Rng, RngExt};
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;
use smallvec::SmallVec;

use super::tree::{ucb_score, Edge, InfoState, Tree};
use crate::config::MAX_ISMCTS_TREE_NODES_PER_DECISION;
use crate::determinization::sample_determinization;
use crate::error::SolverError;
use crate::simulation::rollout;

/// Fixed root-parallel tree count — a compile-time constant so parallel
/// runs cannot drift with thread scheduling.
pub(crate) const ISMCTS_PARALLEL_TREE_COUNT: usize = 8;
/// UCB exploration constant.
const EXPLORATION: f32 = 0.7;

/// Selects an untried arm that is legal in `world`, uniformly at random,
/// and promotes it to an edge.
fn pick_untried(node: &mut super::tree::Node, legal: &SmallVec<[Action; 24]>, rng: &mut impl Rng) -> Option<Action> {
    let available: SmallVec<[usize; 24]> = node
        .untried
        .iter()
        .enumerate()
        .filter(|(_, action)| legal.contains(*action))
        .map(|(index, _)| index)
        .collect();
    if available.is_empty() {
        return None;
    }
    let pick = available[rng.random_range(0..available.len())];
    let action = node.untried.remove(pick);
    node.edges.push(Edge {
        action,
        visits: 0,
        total_value: 0.0,
        availability: 0,
    });
    Some(action)
}

/// The best UCB edge among arms legal in `world`.
fn ucb_pick(node: &super::tree::Node, legal: &SmallVec<[Action; 24]>, maximize: bool) -> Option<Action> {
    node.edges
        .iter()
        .filter(|edge| legal.contains(&edge.action))
        .max_by(|a, b| {
            let sa = ucb_score(a, node.visits, maximize, EXPLORATION);
            let sb = ucb_score(b, node.visits, maximize, EXPLORATION);
            sa.total_cmp(&sb)
                .then(b.action.action_key().cmp(&a.action.action_key()))
        })
        .map(|edge| edge.action)
}

/// Runs `iterations` ISMCTS iterations into `tree` on one RNG stream.
fn run_iterations(
    tree: &mut Tree,
    root: &KoiGameState,
    observer: Player,
    iterations: usize,
    score_norm: f32,
    rng: &mut ChaCha8Rng,
) {
    for _ in 0..iterations {
        let mut world = match sample_determinization(root, observer, rng) {
            Ok(world) => world,
            Err(_) => break, // an inconsistent root fails the whole decision
        };
        let mut path: Vec<(InfoState, Action)> = Vec::with_capacity(64);

        while !world.is_ended() {
            let key = InfoState::from_state(&world);
            let legal = world.legal_actions();
            if legal.is_empty() {
                break;
            }
            let Some(node) = tree.ensure_node(key, &legal, MAX_ISMCTS_TREE_NODES_PER_DECISION) else {
                break; // node budget exhausted — roll out from the frontier
            };
            // Availability: an arm's exploration weight counts only the
            // iterations in which it was actually legal in the sampled world.
            for edge in &mut node.edges {
                if legal.contains(&edge.action) {
                    edge.availability += 1;
                }
            }
            node.visits += 1;
            let maximize = world.active == observer;
            let action = pick_untried(node, &legal, rng).or_else(|| ucb_pick(node, &legal, maximize));
            let Some(action) = action else { break };
            path.push((key, action));
            match world.apply_action(action) {
                Ok(next) => world = next,
                Err(_) => break,
            }
        }

        let reward = rollout(world, observer, score_norm);
        for (key, action) in path {
            if let Some(node) = tree.nodes.get_mut(&key) {
                if let Some(edge) = node.edges.iter_mut().find(|edge| edge.action == action) {
                    edge.visits += 1;
                    edge.total_value += reward;
                }
            }
        }
    }
}

/// The robust child at the root: argmax visits, ties on `action_key`.
fn best_root_action(tree: &Tree, root: &KoiGameState) -> Option<Action> {
    let key = InfoState::from_state(root);
    tree.node(&key).and_then(|node| {
        node.edges
            .iter()
            .max_by(|a, b| {
                a.visits
                    .cmp(&b.visits)
                    .then(b.action.action_key().cmp(&a.action.action_key()))
            })
            .map(|edge| edge.action)
    })
}

/// Sequential ISMCTS on a seeded stream.
pub(crate) fn find_best_action_ismcts(
    state: &KoiGameState,
    observer: Player,
    iterations: usize,
    score_norm: f32,
    use_parallel: bool,
    seed: u64,
) -> Result<Option<Action>, SolverError> {
    if state.is_ended() {
        return Ok(None);
    }
    if iterations == 0 {
        return Err(SolverError::NoCompleteEvaluation);
    }

    if use_parallel {
        let trees: Vec<Tree> = (0..ISMCTS_PARALLEL_TREE_COUNT)
            .into_par_iter()
            .map(|tree_index| {
                let mut rng = ChaCha8Rng::seed_from_u64(derive_named_seed(seed, 0x15_4c_53 | tree_index as u64));
                let mut tree = Tree::default();
                run_iterations(
                    &mut tree,
                    state,
                    observer,
                    iterations.div_ceil(ISMCTS_PARALLEL_TREE_COUNT),
                    score_norm,
                    &mut rng,
                );
                tree
            })
            .collect();
        // Deterministic merge: fixed tree order, per-action accumulation.
        let key = InfoState::from_state(state);
        let mut merged: std::collections::BTreeMap<u64, (Action, u32, f32)> = std::collections::BTreeMap::new();
        for tree in &trees {
            if let Some(node) = tree.node(&key) {
                for edge in &node.edges {
                    let entry = merged.entry(edge.action.action_key()).or_insert((edge.action, 0, 0.0));
                    entry.1 += edge.visits;
                    entry.2 += edge.total_value;
                }
            }
        }
        return Ok(merged
            .values()
            .max_by(|a, b| a.1.cmp(&b.1).then(b.0.action_key().cmp(&a.0.action_key())))
            .map(|(action, _, _)| *action));
    }

    let mut rng = ChaCha8Rng::seed_from_u64(derive_named_seed(seed, 0x15_4c_53));
    let mut tree = Tree::default();
    run_iterations(&mut tree, state, observer, iterations, score_norm, &mut rng);
    Ok(best_root_action(&tree, state))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ismcts::tree::Node;

    /// The audit's F1 reproduction: a node with two tried edges must pick
    /// the higher-scoring one — the old `.reverse()` made it pick the worst.
    #[test]
    fn ucb_pick_selects_the_higher_scoring_edge() {
        let mut node = Node::default();
        let good = Action::KoiKoi;
        let bad = Action::Shobu;
        node.visits = 10;
        // `good` has more visits and higher mean — it must win.
        node.edges.push(crate::ismcts::tree::Edge {
            action: good,
            visits: 8,
            total_value: 6.0,
            availability: 10,
        });
        node.edges.push(crate::ismcts::tree::Edge {
            action: bad,
            visits: 2,
            total_value: 0.0,
            availability: 10,
        });
        let legal: SmallVec<[Action; 24]> = smallvec::smallvec![good, bad];
        let picked = ucb_pick(&node, &legal, true);
        assert_eq!(picked, Some(good), "ucb_pick must select the higher-scoring edge");
    }

    /// Ties break to the *smallest* action_key — the codebase's own
    /// convention (matches `best_root_action`, `pimc.rs`, `turn8_exact`).
    #[test]
    fn ucb_pick_tie_breaks_to_smallest_action_key() {
        let mut node = Node::default();
        let a = Action::Shobu;
        let b = Action::KoiKoi;
        node.visits = 10;
        // Identical scores — the smaller action_key wins.
        node.edges.push(crate::ismcts::tree::Edge {
            action: a,
            visits: 5,
            total_value: 3.0,
            availability: 10,
        });
        node.edges.push(crate::ismcts::tree::Edge {
            action: b,
            visits: 5,
            total_value: 3.0,
            availability: 10,
        });
        let legal: SmallVec<[Action; 24]> = smallvec::smallvec![a, b];
        let picked = ucb_pick(&node, &legal, true);
        assert_eq!(picked, Some(b), "tie must break to the smallest action_key");
    }
}
