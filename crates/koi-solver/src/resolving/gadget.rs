//! The continual-resolving safety gadget (DeepStack / Brown–Sandholm
//! style): at the resolve root the opponent holds a virtual two-action
//! decision per world — opt out and bank the oracle's counterfactual value
//! of that world, or enter the resolved subgame. Because the opt-out value
//! is the oracle's expectation, the resolved strategy cannot hand the
//! opponent less than their oracle value beyond the training slack: they
//! simply opt out of every world where the subgame is worse for them.
//!
//! The gadget needs a value oracle. A *certified* one exists only where a
//! converged tabular blueprint exists — the reduced EFG variants — so
//! [`SafetyGadget`] is constructed for those trees, either from an
//! in-memory [`Blueprint`] or from a persisted `koi-blueprint/v1` artifact
//! via [`super::artifact::load_gadget`]. On the canonical 48-card game no
//! tabular blueprint exists; the oracle there is [`GadgetOracle::Learned`]
//! — a deterministic heuristic rollout. That keeps the gadget's direction
//! only as an *empirical guard*: a two-sided heuristic error does not
//! satisfy the one-sided bound a certificate needs, so canonical gadget
//! use is a guard, never a certified bound. The shrinkage in
//! [`super::model`] likewise remains a regularizer — it prevents belief
//! collapse but bounds no exploitability.

use std::sync::Arc;

use koi_core::{Card, CardSet, KoiGameState, Player, TurnPhase};

use crate::cfr::profile::Blueprint;
use crate::efg::compiler::MarginOracle;
#[cfg(test)]
use crate::efg::tree::NodeType;
use crate::efg::tree::{draw_event_key, EfgTree, InfoSetKey};
use crate::heuristic;

/// A blueprint-backed value oracle for the resolve gadget. The tree is
/// `Arc`-shared so a `Solver` can hold the compiled game across decisions.
pub struct SafetyGadget {
    tree: Arc<EfgTree>,
    /// Average strategy per compiled infoset — the blueprint profile.
    profile: Vec<Vec<f64>>,
}

/// The public anchors a margin evaluation replays from: the dealt field and
/// the observed ledger event stream. The blueprint profile's infoset keys
/// are meaningful only relative to these.
#[derive(Debug, Clone)]
pub struct MarginAnchors {
    /// The dealt field (`InfoSetKey::initial_field`).
    pub initial_field: CardSet,
    /// The observed event stream (`InfoSetKey::public_history` prefix).
    pub history_prefix: Vec<u64>,
}

impl SafetyGadget {
    /// The gadget over `tree` using `blueprint`'s average strategy as the
    /// fallback policy. The blueprint should be converged — its measured
    /// exploitability bounds the gadget's slack.
    pub fn new(tree: Arc<EfgTree>, blueprint: &Blueprint) -> Self {
        Self::from_profile(tree, blueprint.averaged_profile())
    }

    /// The gadget over `tree` using an arbitrary per-infoset strategy. The
    /// profile must carry one normalized row per compiled infoset in
    /// `tree.infosets()` order; `margin_south` fails closed on any row
    /// that cannot distribute over its infoset's actions.
    pub fn from_profile(tree: Arc<EfgTree>, profile: Vec<Vec<f64>>) -> Self {
        Self { tree, profile }
    }

    /// The variant the blueprint was compiled on — the certified resolve's
    /// subgame domain.
    pub fn variant(&self) -> &crate::efg::KoiVariant {
        self.tree.variant()
    }

    /// The blueprint's expected South margin from `state`, evaluated by
    /// playing the profile forward through the kernel and branching
    /// uniformly over remaining stock draws — the same chance model the
    /// tree compiled. `anchors.history_prefix` must be the true public
    /// event stream at `state` (including the pending draw's event when
    /// the state sits at a stock resolution) — the infoset lookups key on
    /// it. Returns `None` when the state falls outside the compiled tree
    /// (an infoset key the variant cannot reach, or a ruleset the profile
    /// was not trained under), which the caller must treat as fail-closed.
    pub fn margin_south(&self, state: &KoiGameState, anchors: &MarginAnchors) -> Option<f64> {
        if state.rules != self.tree.variant().rules {
            // A same-universe/different-ruleset state can share key shape —
            // the rules are not in `InfoSetKey`, so guard explicitly.
            return None;
        }
        if state.is_ended() {
            return Some(state.leg_margin(Player::South) as f64);
        }
        let mut history = anchors.history_prefix.clone();
        let mut budget = Self::EVAL_NODE_BUDGET;
        self.evaluate(state, anchors.initial_field.bits(), &mut history, &mut budget)
    }

    /// Binds the resolve's public anchors into a [`MarginOracle`] for one
    /// subgame compile.
    pub fn oracle<'a>(&'a self, anchors: &'a MarginAnchors) -> BlueprintOracle<'a> {
        BlueprintOracle { gadget: self, anchors }
    }

    /// The maximum nodes one margin call may visit. A well-formed
    /// blueprint on a reduced variant finishes far below this; a
    /// near-uniform profile could otherwise spend seconds inside one
    /// oracle call — exhaustion fails closed like a lookup miss.
    const EVAL_NODE_BUDGET: usize = 2_000_000;

    fn evaluate(
        &self,
        state: &KoiGameState,
        initial_field: u64,
        history: &mut Vec<u64>,
        budget: &mut usize,
    ) -> Option<f64> {
        if state.is_ended() {
            return Some(state.leg_margin(Player::South) as f64);
        }
        *budget = budget.checked_sub(1)?;
        let player = state.active;
        let key = InfoSetKey {
            player,
            own_hand: state.hands[player.index()].bits(),
            initial_field,
            public_history: history.clone(),
        };
        let infoset_id = self.tree.infoset_for(&key)?;
        let infoset = &self.tree.infosets()[infoset_id];
        let strategy = self.profile.get(infoset_id)?;
        if strategy.len() != infoset.actions.len() {
            return None;
        }
        let legal = state.legal_actions();
        let mut value = 0.0;
        for (index, action) in infoset.actions.iter().enumerate() {
            if !legal.contains(action) {
                return None;
            }
            let weight = strategy[index];
            if weight == 0.0 {
                continue;
            }
            let branch = match action {
                koi_core::Action::PlayFromHand { .. } => {
                    history.push(action.action_key());
                    let mid = state.apply_action(*action).ok()?;
                    let TurnPhase::AwaitingStockResolution { .. } = mid.phase else {
                        history.pop();
                        return None;
                    };
                    // Uniform over the remaining stock — the drawn card is
                    // pinned per branch exactly like the compiler's draw
                    // expansion.
                    let stock: Vec<Card> = mid.stock.iter().copied().collect();
                    let count = stock.len();
                    let mut draw_value = 0.0;
                    for (slot, drawn) in stock.iter().enumerate() {
                        let mut tail = Vec::with_capacity(count);
                        tail.push(*drawn);
                        tail.extend(stock.iter().enumerate().filter(|(i, _)| *i != slot).map(|(_, c)| *c));
                        let mut child = mid;
                        child.phase = TurnPhase::AwaitingStockResolution { drawn: *drawn };
                        child.stock = koi_core::Stock::from_slice(&tail).ok()?;
                        history.push(draw_event_key(*drawn));
                        let v = self.evaluate(&child, initial_field, history, budget);
                        history.pop();
                        draw_value += v? / count as f64;
                    }
                    history.pop();
                    draw_value
                }
                _ => {
                    history.push(action.action_key());
                    let next = state.apply_action(*action).ok()?;
                    let v = self.evaluate(&next, initial_field, history, budget);
                    history.pop();
                    v?
                }
            };
            value += weight * branch;
        }
        Some(value)
    }
}

/// The resolve-time [`MarginOracle`] a [`SafetyGadget`] lends out: the
/// blueprint forward evaluation anchored at one resolve's public anchors.
/// The bound blueprint oracle [`GadgetOracle::oracle`] hands to the compile.
/// The `history` argument of each `margin_south` call overrides the anchor
/// prefix — a depth-capped leaf's true stream differs from the root's.
pub struct BlueprintOracle<'a> {
    gadget: &'a SafetyGadget,
    anchors: &'a MarginAnchors,
}

impl MarginOracle for BlueprintOracle<'_> {
    fn margin_south(&self, state: &KoiGameState, history: &[u64]) -> Option<f64> {
        let anchors = MarginAnchors {
            initial_field: self.anchors.initial_field,
            history_prefix: history.to_vec(),
        };
        self.gadget.margin_south(state, &anchors)
    }
}

/// The value oracle pricing the opponent's opt-out bank (and depth-capped
/// leaves). `Blueprint` is certified — per-world margins from a converged
/// profile on a reduced variant. `Learned` is the deterministic heuristic
/// rollout: an *empirically guarded* resolve, never certified (see the
/// module header).
pub enum GadgetOracle {
    /// Certified: per-world margins from the compiled blueprint profile.
    /// Shared so a worker can cache the loaded artifact across decisions.
    Blueprint(Arc<SafetyGadget>),
    /// Empirically guarded: heuristic rollouts price the bank.
    Learned,
}

impl GadgetOracle {
    /// Binds the resolve anchors into a [`MarginOracle`] for the compile.
    /// `Learned` ignores anchors — a rollout needs only the concrete state.
    pub fn oracle<'a>(&'a self, anchors: &'a MarginAnchors) -> OracleRef<'a> {
        match self {
            Self::Blueprint(gadget) => OracleRef::Blueprint(gadget.oracle(anchors)),
            Self::Learned => OracleRef::Learned,
        }
    }

    /// The subgame domain a resolve under this oracle compiles on: the
    /// blueprint's own variant when certified, the canonical game for the
    /// empirical guard.
    pub fn variant(&self) -> crate::efg::KoiVariant {
        match self {
            Self::Blueprint(gadget) => gadget.variant().clone(),
            Self::Learned => crate::efg::KoiVariant::FULL,
        }
    }
}

/// The bound oracle form `compile_gadget_subgame` consumes.
pub enum OracleRef<'a> {
    /// Blueprint-backed oracle.
    Blueprint(BlueprintOracle<'a>),
    /// Heuristic rollout oracle.
    Learned,
}

impl MarginOracle for OracleRef<'_> {
    fn margin_south(&self, state: &KoiGameState, history: &[u64]) -> Option<f64> {
        match self {
            Self::Blueprint(oracle) => oracle.margin_south(state, history),
            Self::Learned => Some(learned_margin_south(state)),
        }
    }
}

/// A leaf-evaluating [`MarginOracle`]: the configured evaluator prices the
/// margin under the resolve root's posterior belief block; a failed call
/// degrades to the fallback oracle and increments `leaf_fallback_count`.
///
/// The evaluator sits behind a `RefCell` — `MarginOracle::margin_south`
/// takes `&self` but model inference needs `&mut`, and a resolve is
/// single-threaded so the borrow can never be contested. A panic inside
/// the evaluator would poison the borrow and surface as `RefCell` errors —
/// equally fail-closed.
pub struct LeafMarginOracle<'a> {
    evaluator: &'a std::cell::RefCell<Box<dyn crate::leaf::LeafEvaluator>>,
    belief: [f32; crate::leaf::CARD_BLOCK],
    fallback: &'a dyn MarginOracle,
}

impl<'a> LeafMarginOracle<'a> {
    /// `belief` is the resolve root's posterior marginal over the unseen
    /// set — recomputed per ladder rung by `resolve_decision`.
    pub fn new(
        evaluator: &'a std::cell::RefCell<Box<dyn crate::leaf::LeafEvaluator>>,
        belief: [f32; crate::leaf::CARD_BLOCK],
        fallback: &'a dyn MarginOracle,
    ) -> Self {
        Self {
            evaluator,
            belief,
            fallback,
        }
    }
}

impl MarginOracle for LeafMarginOracle<'_> {
    fn margin_south(&self, state: &KoiGameState, history: &[u64]) -> Option<f64> {
        let mut evaluator = self.evaluator.borrow_mut();
        match crate::leaf::leaf_eval_for_state(state, &self.belief, evaluator.as_mut()) {
            Ok(out) => Some(match state.active {
                Player::South => f64::from(out.ev),
                Player::North => f64::from(-out.ev),
            }),
            Err(_) => {
                crate::leaf::note_leaf_fallback();
                self.fallback.margin_south(state, history)
            }
        }
    }
}

/// Permutes the stock tail deterministically from the state's own zones.
/// The canonical ascending tail would feed the rollout a fixed, card-
/// index-correlated draw order — a systematic rather than noisy oracle
/// bias (red-team finding: worlds whose tails front-load one seat's
/// cards get systematically mispriced). The permutation key comes from
/// the state's zones so the same state always rolls out the same order —
/// the oracle stays a deterministic function — while different worlds
/// get decorrelated draws. A pending public draw stays pinned at the
/// stock head.
pub(crate) fn permute_stock_for_rollout(state: &mut KoiGameState) {
    use rand::seq::SliceRandom;
    use rand::{rngs::SmallRng, SeedableRng};

    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for bits in [
        state.hands[0].bits(),
        state.hands[1].bits(),
        state.field.bits(),
        state.captured[0].bits(),
        state.captured[1].bits(),
    ] {
        hash = (hash ^ bits).wrapping_mul(0x0000_0100_0000_01b3);
    }
    let mut rng = SmallRng::seed_from_u64(hash);
    let skip = usize::from(matches!(state.phase, TurnPhase::AwaitingStockResolution { .. }));
    state.stock.as_mut_slice()[skip..].shuffle(&mut rng);
}

/// The deterministic heuristic rollout value: play both seats by the P1
/// heuristic to the round's end and take South's leg margin. Deterministic
/// because `heuristic::find_best_action` is — an empirical counterfactual
/// value, not a bound.
pub fn learned_margin_south(state: &KoiGameState) -> f64 {
    let mut state = *state;
    permute_stock_for_rollout(&mut state);
    let mut guard = 0usize;
    while !state.is_ended() && guard < 96 {
        let Some(action) = heuristic::find_best_action(&state) else {
            break;
        };
        state = state.apply_action(action).unwrap_or_else(|_| {
            let mut terminal = state;
            terminal.phase = TurnPhase::Ended;
            terminal
        });
        guard += 1;
    }
    state.leg_margin(Player::South) as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfr::cfr_plus::train_cfr_plus;
    use crate::efg::compiler::{compile_variant, KoiVariant};

    /// Averaging the gadget's per-deal margins over the compiled chance root
    /// must reproduce the profile's exact value — the gadget's forward
    /// evaluation is the blueprint's own expectation conditional on a deal.
    #[test]
    fn blueprint_margin_tracks_the_compiled_value_at_the_root() {
        let tree = compile_variant(&KoiVariant::NANO_6, Player::South).unwrap();
        let blueprint = train_cfr_plus(&tree, 400);
        let profile_value = crate::cfr::profile_value_south(&tree, &blueprint.averaged_profile());
        let gadget = SafetyGadget::new(Arc::new(tree.clone()), &blueprint);

        let hand = usize::from(KoiVariant::NANO_6.hand_size);
        let field_n = usize::from(KoiVariant::NANO_6.field_size);
        let NodeType::Chance { outcomes } = tree.node(tree.root()) else {
            panic!("variant root must be a chance node");
        };
        let mut weighted = 0.0;
        for outcome in outcomes {
            let to_set = |indices: &[u8]| {
                indices
                    .iter()
                    .fold(CardSet::EMPTY, |set, i| set.insert(Card::new(*i).unwrap()))
            };
            let south = to_set(&outcome.dealt[..hand]);
            let north = to_set(&outcome.dealt[hand..hand * 2]);
            let field = to_set(&outcome.dealt[hand * 2..hand * 2 + field_n]);
            let stock: Vec<Card> = KoiVariant::NANO_6
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
                KoiVariant::NANO_6.rules,
            );
            assert!(anomaly.is_none());
            let anchors = MarginAnchors {
                initial_field: field,
                history_prefix: Vec::new(),
            };
            let value = gadget
                .margin_south(&state, &anchors)
                .expect("a dealt variant world must evaluate");
            weighted += outcome.probability * value;
        }
        assert!(
            (weighted - profile_value).abs() < 1e-9,
            "gadget expectation {weighted} must equal the profile value {profile_value}"
        );
    }

    /// States outside the compiled variant fail closed.
    #[test]
    fn blueprint_margin_fails_closed_outside_the_tree() {
        let tree = compile_variant(&KoiVariant::NANO_6, Player::South).unwrap();
        let blueprint = train_cfr_plus(&tree, 50);
        let gadget = SafetyGadget::new(Arc::new(tree), &blueprint);

        // A full-deck state: its hand can never be a variant infoset.
        let (state, anomaly) = KoiGameState::new_deal(koi_core::deal_from_seed(7), koi_core::Ruleset::nintendo());
        assert!(anomaly.is_none());
        let anchors = MarginAnchors {
            initial_field: state.field,
            history_prefix: Vec::new(),
        };
        assert!(gadget.margin_south(&state, &anchors).is_none());
    }

    /// The learned oracle evaluates any live position and terminates.
    #[test]
    fn learned_rollout_prices_live_states() {
        let (state, anomaly) = KoiGameState::new_deal(koi_core::deal_from_seed(11), koi_core::Ruleset::nintendo());
        assert!(anomaly.is_none());
        let value = learned_margin_south(&state);
        assert!(value.is_finite());
        assert!(value.abs() <= 64.0);
    }
}
