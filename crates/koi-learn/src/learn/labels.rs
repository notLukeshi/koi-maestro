//! Blueprint-derived and resolve-derived labels for the learned leaf
//! evaluator.
//!
//! Two honest label sources, mirroring the sibling codebase pipeline's hierarchy:
//!
//! - [`label_state`]: a converged CFR+ [`Blueprint`] on a compilable
//!   reduced variant supplies both targets — the average strategy at the
//!   sampled infoset (spread over the canonical legal ordering) and the
//!   blueprint's expected margin. The blueprint margin is infoset-valued:
//!   every world in the actor's belief shares the actor's key, so the
//!   DEVN-style expectation collapses to the single lookup.
//! - [`label_state_resolved`]: a per-row root resolve supplies the root
//!   average strategy and the resolved root value — the canonical pathway
//!   where no blueprint exists.
//!
//! Belief handling follows DeepStack's random-range regularization: with
//! probability `perturb_prob` the natural posterior over compatible worlds
//! is replaced by a uniform-Dirichlet draw over the same set, and the
//! emitted belief vector (plus, on the resolve path, the solve itself) is
//! computed under it.

use koi_core::{KoiGameState, Player};
use koi_solver::{
    efg::tree::{EfgTree, InfoSetKey},
    leaf::{actions, features::CARD_BLOCK},
    resolving::{MarginAnchors, SafetyGadget, World},
};
use rand::{Rng, RngExt};

/// One labelled example's targets plus the belief the features encode.
pub struct RowLabels {
    /// Actor-relative expected leg margin under the labeler and the row's
    /// belief.
    pub ev: f32,
    /// Label policy over the canonical action order (zero outside the mask).
    pub policy: [f32; actions::MAX_ACTIONS],
    /// The belief actually used (natural or perturbed): per-card marginal
    /// probability of sitting in the opponent's hand.
    pub belief: [f32; CARD_BLOCK],
    /// Whether the row's belief was a random-range perturbation.
    pub perturbed: bool,
}

/// A uniform-Dirichlet draw over `n` members via normalized exponential
/// weights — the standard random-range perturbation.
pub fn random_member_weights(n: usize, rng: &mut impl Rng) -> Vec<f64> {
    let mut weights: Vec<f64> = (0..n)
        .map(|_| -rng.random::<f64>().clamp(f64::MIN_POSITIVE, 1.0).ln())
        .collect();
    let total: f64 = weights.iter().sum();
    for w in &mut weights {
        *w /= total;
    }
    weights
}

/// Marginal per-card belief from member weights — re-exported so the
/// labeler and the runtime leaf path share one implementation.
pub use koi_solver::leaf::card_marginals;

/// The member weights a row is labelled under: the natural posterior, or a
/// uniform-Dirichlet random range for perturbed rows. `None` on an empty
/// or massless posterior — fail closed.
fn member_weights(worlds: &[World], perturbed: bool, rng: &mut impl Rng) -> Option<Vec<f64>> {
    if perturbed {
        return Some(random_member_weights(worlds.len(), rng));
    }
    let total: f64 = worlds.iter().map(|w| w.weight.max(0.0)).sum();
    if total <= 0.0 {
        return None;
    }
    Some(worlds.iter().map(|w| w.weight.max(0.0) / total).collect())
}

/// The compiled blueprint artifacts a row is labelled under — the tree
/// that indexes infosets, the converged average profile, and the safety
/// gadget that prices opt-outs and leaf values. Bundled because the
/// labeler prices and scores against all three at once.
pub struct BlueprintTarget<'a> {
    pub tree: &'a EfgTree,
    pub profile: &'a [Vec<f64>],
    pub gadget: &'a SafetyGadget,
}

/// Labels `state` under the blueprint's converged profile — the
/// reduced-variant pathway. `anchors` carries the replayed public stream
/// the synthetic trajectory produced (the labeler needs the true prefix,
/// exactly like the resolve gadget's blueprint oracle).
///
/// Returns `None` when the state exceeds the action cap, falls outside the
/// compiled tree, or the blueprint prices no legal action — the caller
/// retries or skips (fail closed).
pub fn label_state(
    state: &KoiGameState,
    anchors: &MarginAnchors,
    bp: &BlueprintTarget<'_>,
    worlds: &[World],
    perturb_prob: f64,
    rng: &mut impl Rng,
) -> Option<RowLabels> {
    let (tree, profile, gadget) = (bp.tree, bp.profile, bp.gadget);
    let legal = actions::canonical_legal(state)?;
    let actor = state.active;
    let key = InfoSetKey {
        player: actor,
        own_hand: state.hands[actor.index()].bits(),
        initial_field: anchors.initial_field.bits(),
        public_history: anchors.history_prefix.to_vec(),
    };
    let infoset = tree.infoset_for(&key)?;
    let node_actions = &tree.infosets()[infoset].actions;
    let strategy = profile.get(infoset)?;

    // Align the blueprint's action-indexed strategy onto the canonical
    // action-key ordering the mask uses.
    let mut aligned = vec![0.0_f64; legal.len()];
    for (index, action) in legal.iter().enumerate() {
        let position = node_actions
            .iter()
            .position(|candidate| candidate.action_key() == action.action_key())?;
        aligned[index] = strategy.get(position).copied().unwrap_or(0.0);
    }
    let policy = actions::spread_over_mask(&aligned, &legal);
    if policy.iter().all(|p| *p == 0.0) {
        // No blueprint mass on any legal action (unreached or edge
        // infoset): an all-zero row carries no signal — fail closed.
        return None;
    }

    let perturbed = rng.random::<f64>() < perturb_prob;
    let weights = member_weights(worlds, perturbed, rng)?;
    let unseen = koi_solver::leaf::unseen_mask_in(state, tree.variant().cards);
    let belief = card_marginals(unseen, worlds, &weights);

    // The blueprint's margin is infoset-valued and every world in the
    // belief shares the actor's infoset — the DEVN expectation over worlds
    // collapses to the single lookup (perturbation only moves the emitted
    // belief, not the infoset value).
    let margin = gadget.margin_south(state, anchors)?;
    let ev = match actor {
        Player::South => margin,
        Player::North => -margin,
    };

    Some(RowLabels {
        ev: ev as f32,
        policy,
        belief,
        perturbed,
    })
}

/// Labels `state` by resolving it under `worlds` — the canonical pathway,
/// where no compiled blueprint exists. `worlds` carries the belief the row
/// is labelled under: for perturbed rows the caller has already overwritten
/// the weights with a random member range, so the solve, the emitted
/// belief, and the EV all live under the same range.
///
/// The targets mirror [`label_state`]: the resolved root average strategy
/// realigned onto the canonical ordering, and `resolved.value` — already
/// observer-relative (the resolver's own convention).
///
/// Returns `None` on an over-wide action list, an empty/invalid belief, or
/// an unresolved root — fail closed.
#[allow(clippy::too_many_arguments)]
pub fn label_state_resolved(
    state: &KoiGameState,
    anchors: &MarginAnchors,
    variant: &koi_solver::efg::KoiVariant,
    worlds: &[World],
    perturbed: bool,
    spec: &koi_solver::resolving::ResolveSpec,
    leaf_evaluator: Option<&std::cell::RefCell<Box<dyn koi_solver::leaf::LeafEvaluator>>>,
    rng: &mut impl Rng,
) -> Option<RowLabels> {
    let legal = actions::canonical_legal(state)?;
    if worlds.is_empty() {
        return None;
    }
    let weights = member_weights(worlds, false, rng)?;
    let unseen = koi_solver::leaf::unseen_mask_in(state, variant.cards);
    let belief = card_marginals(unseen, worlds, &weights);

    let oracle = koi_solver::resolving::GadgetOracle::Learned.oracle(anchors);
    let ctx = koi_solver::resolving::ResolveContext {
        variant,
        anchors,
        oracle: &oracle,
        spec,
        model: None,
        replay: None,
        leaf_evaluator,
    };
    let resolved = koi_solver::resolving::resolve_decision_with_worlds(state, state.active, worlds, &ctx, rng).ok()?;
    if resolved.root_actions.len() != legal.len() || !resolved.value.is_finite() {
        // A row that cannot be placed on the legal set emits a misaligned
        // target — fail closed, never spread-and-hope.
        return None;
    }

    let mut aligned = vec![0.0_f64; legal.len()];
    for (index, action) in legal.iter().enumerate() {
        let position = resolved
            .root_actions
            .iter()
            .position(|candidate| candidate.action_key() == action.action_key())?;
        aligned[index] = resolved.root_strategy[position];
    }
    let policy = actions::spread_over_mask(&aligned, &legal);
    if policy.iter().all(|p| *p == 0.0) {
        return None;
    }

    Some(RowLabels {
        ev: resolved.value as f32,
        policy,
        belief,
        perturbed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use koi_core::{deal_from_seed, Card, CardSet, Ruleset, TurnPhase};
    use koi_solver::{
        cfr::cfr_plus::train_cfr_plus,
        efg::{compiler::compile_variant, KoiVariant},
        resolving::build_belief,
    };
    use rand::{rngs::SmallRng, SeedableRng};
    use std::sync::Arc;

    fn live_state() -> KoiGameState {
        let (state, _) = KoiGameState::new_deal(deal_from_seed(9), Ruleset::nintendo());
        state
    }

    /// A NANO_6 deal from the compiled root — mirrors the resolver test's
    /// `nano_deal` but parameterized on the outcome index.
    fn nano_deal(tree: &EfgTree, outcome: usize) -> KoiGameState {
        let variant = &KoiVariant::NANO_6;
        let koi_solver::efg::tree::NodeType::Chance { outcomes } = tree.node(tree.root()) else {
            panic!("variant root must be a chance node");
        };
        let hand = usize::from(variant.hand_size);
        let field_n = usize::from(variant.field_size);
        let to_set = |indices: &[u8]| {
            indices
                .iter()
                .fold(CardSet::EMPTY, |set, i| set.insert(Card::new(*i).unwrap()))
        };
        let dealt = &outcomes[outcome].dealt;
        let south = to_set(&dealt[..hand]);
        let north = to_set(&dealt[hand..hand * 2]);
        let field = to_set(&dealt[hand * 2..hand * 2 + field_n]);
        let stock: Vec<Card> = variant
            .cards
            .difference(south.union(north).union(field))
            .into_iter()
            .collect();
        let (state, anomaly) = KoiGameState::new_from_parts(
            &stock,
            [south, north],
            field,
            Player::South,
            Player::South,
            variant.rules,
        );
        assert!(anomaly.is_none());
        state
    }

    /// Card marginals put mass exactly on the support of the belief: every
    /// unseen card's marginal equals its weighted world frequency, and the
    /// vector sums to the opponent's hand size.
    #[test]
    fn card_marginals_match_world_frequencies() {
        let mut rng = SmallRng::seed_from_u64(3);
        let state = live_state();
        let observer = state.active;
        let worlds = build_belief(&state, observer, CardSet::ALL, 64, &mut rng).unwrap();
        assert!(!worlds.is_empty());
        let weights: Vec<f64> = worlds.iter().map(|w| w.weight).collect();
        let unseen = koi_solver::leaf::unseen_mask(&state);
        let m = card_marginals(unseen, &worlds, &weights);

        for card in unseen {
            let index = card.index() as usize;
            let expected: f64 = worlds
                .iter()
                .zip(weights.iter())
                .filter(|(w, _)| w.opponent_hand.contains(card))
                .map(|(_, w)| *w)
                .sum();
            assert!((m[index] - expected as f32).abs() < 1e-6, "card {index}");
        }
        let opp_size = state.hands[observer.opponent().index()].count() as f32;
        let sum: f32 = m.iter().sum();
        assert!((sum - opp_size).abs() < 1e-5, "belief sums to the opponent hand size");
    }

    /// The blueprint path on NANO_6: normalized policy over the legal mask
    /// and a bounded margin — the certified labeler end-to-end.
    #[test]
    fn blueprint_labels_cover_the_reduced_domain() {
        let variant = &KoiVariant::NANO_6;
        let tree = Arc::new(compile_variant(variant, Player::South).unwrap());
        let blueprint = train_cfr_plus(&tree, 300);
        let profile = blueprint.averaged_profile();
        let gadget = SafetyGadget::new(tree.clone(), &blueprint);

        let mut rng = SmallRng::seed_from_u64(12);
        let state = nano_deal(&tree, 0);
        let observer = state.active;
        let anchors = MarginAnchors {
            initial_field: state.field,
            history_prefix: Vec::new(),
        };
        let worlds = build_belief(&state, observer, variant.cards, 64, &mut rng).unwrap();
        let bp = BlueprintTarget {
            tree: &tree,
            profile: &profile,
            gadget: &gadget,
        };
        let labels = label_state(&state, &anchors, &bp, &worlds, 0.0, &mut rng).expect("in-variant deal labels");
        let sum: f32 = labels.policy.iter().sum();
        assert!((sum - 1.0).abs() < 1e-5);
        assert!(labels.ev.abs() <= 64.0, "margin bound");
        assert!(!labels.perturbed);
    }

    /// The resolve labeler on the canonical game: a dealt position resolves
    /// to a normalized policy and a bounded actor-relative EV.
    #[test]
    fn resolved_labels_cover_canonical_states() {
        let mut rng = SmallRng::seed_from_u64(31);
        let mut state = live_state();
        let mut history: Vec<u64> = Vec::new();
        let initial_field = state.field;
        // A few plies in, so the row is a real mid-round decision.
        for _ in 0..3 {
            let legal = state.legal_actions();
            if legal.is_empty() || state.is_ended() {
                break;
            }
            let action = legal[0];
            let next = state.apply_action(action).unwrap();
            history.push(action.action_key());
            if let TurnPhase::AwaitingStockResolution { drawn } = next.phase {
                history.push(koi_solver::efg::tree::draw_event_key(drawn));
            }
            if next.is_ended() {
                break;
            }
            state = next;
        }
        let anchors = MarginAnchors {
            initial_field,
            history_prefix: history,
        };
        let observer = state.active;
        let worlds = build_belief(&state, observer, CardSet::ALL, 32, &mut rng).unwrap();
        let spec = koi_solver::resolving::ResolveSpec {
            max_worlds: 8,
            max_nodes: 50_000,
            cfr_iterations: 30,
            max_decision_depth: 2,
        };
        let labels = label_state_resolved(
            &state,
            &anchors,
            &KoiVariant::FULL,
            &worlds,
            false,
            &spec,
            None,
            &mut rng,
        )
        .expect("canonical resolve label");
        let sum: f32 = labels.policy.iter().sum();
        assert!((sum - 1.0).abs() < 1e-5, "policy sums to one");
        let opp_size = state.hands[observer.opponent().index()].count() as f32;
        let belief_sum: f32 = labels.belief.iter().sum();
        assert!(
            (belief_sum - opp_size).abs() < 1e-4,
            "belief sums to the opponent hand size"
        );
    }

    /// A perturbed resolve row: the caller overwrites world weights with a
    /// random range; the emitted belief lives under that range — still a
    /// valid marginal summing to the opponent's hand size.
    #[test]
    fn a_perturbed_resolve_row_stays_consistent() {
        let mut rng = SmallRng::seed_from_u64(37);
        let state = live_state();
        let anchors = MarginAnchors {
            initial_field: state.field,
            history_prefix: Vec::new(),
        };
        let observer = state.active;
        let mut worlds = build_belief(&state, observer, CardSet::ALL, 32, &mut rng).unwrap();
        let range = random_member_weights(worlds.len(), &mut rng);
        for (world, weight) in worlds.iter_mut().zip(range) {
            world.weight = weight;
        }
        let spec = koi_solver::resolving::ResolveSpec {
            max_worlds: 8,
            max_nodes: 50_000,
            cfr_iterations: 30,
            max_decision_depth: 2,
        };
        let labels = label_state_resolved(
            &state,
            &anchors,
            &KoiVariant::FULL,
            &worlds,
            true,
            &spec,
            None,
            &mut rng,
        )
        .expect("perturbed resolve label");
        assert!(labels.perturbed);
        let opp_size = state.hands[observer.opponent().index()].count() as f32;
        let sum: f32 = labels.belief.iter().sum();
        assert!((sum - opp_size).abs() < 1e-4);
    }
}
