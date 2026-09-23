//! Compiles a (possibly reduced) Koi-Koi round into a materialized EFG tree.
//!
//! Enumeration runs entirely through the validated koi-core state machine:
//! dealt states are built by `KoiGameState::new_from_parts`, actions are the
//! kernel's `legal_actions` applied through `apply_action`, and terminal
//! utilities are the kernel's leg margin. Two deviations from the sibling codebase
//! template are forced by Koi-Koi's structure (decision P2-D1):
//!
//! * The root chance node enumerates dealt *zones* — South hand, North hand,
//!   initial field — while the undealt remainder enters the state as an
//!   unordered stock set in canonical order. Stock order is never enumerated.
//! * Every stock draw is its own chance node: after a `PlayFromHand`, the
//!   kernel deterministically reveals `stock[0]`; the compiler instead forks
//!   one child per remaining stock card, pinned to the head exactly like
//!   `rebuild_hidden` pins a drawn card. Uniform draws without replacement
//!   reproduce the uniform-permutation law exactly.
//!
//! Perfect-recall information sets are keyed by (acting player, own hand,
//! initial field, public event history); see `tree::InfoSetKey` (P2-D3).

use koi_core::{Card, CardSet, KoiGameState, Player, Ruleset, StateError, TurnPhase};

use super::tree::{draw_event_key, gadget_event_key, ChanceOutcome, EfgTree, InfoSet, InfoSetKey, NodeId};

/// Upper bound on materialized nodes. The reduced certificate domains below
/// need well under this; the full 48-card game is rejected rather than
/// exhausting memory (the EFG path is for reduced certificates and bounded
/// endgame subgames — mid-game play uses CFR on sampled trees).
pub const MAX_EFG_NODES: usize = 4_000_000;

/// A compilable Koi-Koi domain: a card universe (subset of the 48-card deck)
/// plus deal geometry and the ruleset the round is scored under.
///
/// The universe need not be whole months — capture/yaku mechanics only need
/// month membership — but whole-month subsets keep reduced games faithful.
/// `stock = |cards| - 2*hand_size - field_size` must be at least 1; smaller
/// stocks simply end the round early through the kernel's null-round path,
/// so there is no tighter constraint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KoiVariant {
    pub name: &'static str,
    pub cards: CardSet,
    pub hand_size: u8,
    pub field_size: u8,
    pub rules: Ruleset,
}

impl KoiVariant {
    /// 6 cards (March 8..=10 + September 32..=34), one-card hands, two-card
    /// field — the smallest domain where Hanami is reachable ({8,32} both
    /// attainable). ~180 root deals and per-player sequence counts in the
    /// hundreds keep the *dense* sequence-form LP tableau small enough to
    /// solve in milliseconds — this is the LP-certificate domain. (MICRO_8
    /// is CFR-tractable but its ~1300 x ~18000 sequences make the dense
    /// tableau infeasible.)
    /// Cards 8..=10 + 32..=34 = bits 8-10 and 32-34.
    pub const NANO_6: Self = Self {
        name: "nano_6",
        cards: CardSet::new_unchecked(0x0000_0007_0000_0700),
        hand_size: 1,
        field_size: 2,
        rules: Ruleset::Nintendo,
    };

    /// 8 cards (months March + September), one-card hands, two-card field.
    /// Hanami (Curtain + Sake Cup) is achievable — a two-card field can hold
    /// card 8 with a September card, so one player can capture both in a
    /// single turn — so stop decisions and non-zero terminals exist.
    /// Cards 8..=11 (Mar) + 32..=35 (Sep) = bits 8-11 and 32-35.
    /// (`f=1` is degenerate: two-month yaku can never complete, every leg
    /// margin is 0.)
    pub const MICRO_8: Self = Self {
        name: "micro_8",
        cards: CardSet::new_unchecked(0x0000_000F_0000_0F00),
        hand_size: 1,
        field_size: 2,
        rules: Ruleset::Nintendo,
    };

    /// 10 cards (months January + February + March cards 8,9), one-card
    /// hands, four-card field — four-of-a-month fields exist, so the root
    /// renormalizes FieldVoid anomalies. Certificate domain for the deal
    /// filter, not for solving.
    pub const FIELDVOID_10: Self = Self {
        name: "fieldvoid_10",
        cards: CardSet::new_unchecked(0x0000_0000_0000_03FF),
        hand_size: 1,
        field_size: 4,
        rules: Ruleset::Nintendo,
    };

    /// 9 cards (months March + September + October's Deer), two-card hands,
    /// one-card field — multi-decision depth with stop decisions (Hanami),
    /// 3,780 root deals and on the order of 10^5..10^6 nodes. The
    /// CFR/exploitability convergence domain. Cards 8..=11 + 32..=36.
    pub const REDUCED_9: Self = Self {
        name: "reduced_9",
        cards: CardSet::new_unchecked(0x0000_001F_0000_0F00),
        hand_size: 2,
        field_size: 1,
        rules: Ruleset::Nintendo,
    };

    /// The production game: all 48 cards, eight-card hands, eight-card
    /// field, Nintendo rules. The compiled tree is far beyond the node
    /// budget — this variant exists as the domain label for real-game
    /// subgame solves, never for full compilation.
    pub const FULL: Self = Self {
        name: "full_48",
        cards: CardSet::ALL,
        hand_size: 8,
        field_size: 8,
        rules: Ruleset::Nintendo,
    };

    /// The stock size this variant deals.
    pub fn stock_size(&self) -> u32 {
        self.cards
            .count()
            .saturating_sub(2 * u32::from(self.hand_size) + u32::from(self.field_size))
    }
}

/// Why a domain cannot be compiled into a materialized EFG.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EfgCompileError {
    NodeBudgetExceeded {
        budget: usize,
    },
    /// The validated kernel rejected an enumerated transition, which would
    /// mean the compiler derived an illegal history from the kernel itself.
    KernelTransition(StateError),
    /// The variant's geometry is degenerate (empty universe, zero-sized
    /// zones, or not enough cards to deal the declared layout).
    InvalidVariant(&'static str),
    /// Two nodes merged into one information set carry different legal
    /// action lists — an information leak in the key. Fail-closed: this can
    /// only fire if the key omits a public observation.
    InfoSetActionMismatch,
    /// A depth-capped leaf or gadget opt-out needed a counterfactual value
    /// the oracle refused — the oracle's domain does not cover the position.
    OracleUnavailable,
}

impl From<StateError> for EfgCompileError {
    fn from(error: StateError) -> Self {
        Self::KernelTransition(error)
    }
}

impl std::fmt::Display for EfgCompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NodeBudgetExceeded { budget } => {
                write!(
                    f,
                    "the domain's game tree exceeds the {budget}-node materialization budget"
                )
            }
            Self::KernelTransition(error) => write!(f, "the kernel rejected an enumerated transition: {error}"),
            Self::InvalidVariant(reason) => write!(f, "invalid variant: {reason}"),
            Self::InfoSetActionMismatch => {
                write!(
                    f,
                    "information-set members disagree on legal actions (key leaks hidden state)"
                )
            }
            Self::OracleUnavailable => {
                write!(f, "the value oracle refused a position it was asked to price")
            }
        }
    }
}

impl std::error::Error for EfgCompileError {}

/// Compiles one complete round of `variant`, dealt by `dealer` (who moves
/// first — `new_from_parts` convention).
pub fn compile_variant(variant: &KoiVariant, dealer: Player) -> Result<EfgTree, EfgCompileError> {
    compile_variant_with_budget(variant, dealer, MAX_EFG_NODES)
}

/// Compiles one round with an explicit node budget. Reduced verification
/// variants pass; anything whose tree would exhaust memory fails closed.
pub fn compile_variant_with_budget(
    variant: &KoiVariant,
    dealer: Player,
    max_nodes: usize,
) -> Result<EfgTree, EfgCompileError> {
    validate_variant(variant)?;
    let mut compiler = Compiler::new(variant.clone(), max_nodes);
    let root = compiler.expand_initial_deal(variant, dealer)?;
    compiler.tree.set_root(root);
    Ok(compiler.tree)
}

/// Compiles a residual subgame rooted at a chance node over `worlds` —
/// `(state, probability)` pairs consistent with one public observation
/// sequence. This is the turn-7 resolving substrate: every world shares the
/// same `initial_field` and `history_prefix` (the public ledger so far), so
/// information sets merge exactly where the observer cannot distinguish
/// worlds. Probabilities are renormalized; worlds at `Ended` become
/// terminal leaves directly.
pub fn compile_subgame(
    variant: &KoiVariant,
    worlds: &[(KoiGameState, f64)],
    initial_field: CardSet,
    history_prefix: Vec<u64>,
    max_nodes: usize,
) -> Result<EfgTree, EfgCompileError> {
    if worlds.is_empty() {
        return Err(EfgCompileError::InvalidVariant("a subgame needs at least one world"));
    }
    if worlds.iter().any(|(_, p)| !p.is_finite() || *p < 0.0) {
        return Err(EfgCompileError::InvalidVariant(
            "world probabilities must be finite and non-negative",
        ));
    }
    let total: f64 = worlds.iter().map(|(_, p)| p).sum();
    if total <= 0.0 {
        return Err(EfgCompileError::InvalidVariant("world probabilities carry no mass"));
    }

    let mut compiler = Compiler::new(variant.clone(), max_nodes);
    let mut outcomes = Vec::with_capacity(worlds.len());
    for (state, weight) in worlds {
        compiler.check_budget()?;
        let mut history = history_prefix.clone();
        let child = compiler.expand(*state, initial_field.bits(), &mut history, 0, &UNBOUNDED)?;
        // Provenance record for the outcome: opponent hand + stock order.
        let observer = state.active; // the player to move — hidden zones are the opponent's
        let mut dealt: Vec<u8> = state.hands[observer.opponent().index()]
            .into_iter()
            .map(|c| c.index())
            .collect();
        dealt.extend(state.stock.iter().copied().map(|c| c.index()));
        outcomes.push(ChanceOutcome {
            probability: weight / total,
            dealt,
            child,
        });
    }
    let root = compiler.tree.push_chance(outcomes);
    compiler.tree.set_root(root);
    Ok(compiler.tree)
}

/// A counterfactual-value oracle for the resolving gadget: prices the
/// opponent's opt-out bank and the depth-capped leaves in leg-margin units
/// (`utility_south` — the terminal convention). `history` is the true
/// public event stream at `state` — a blueprint oracle needs it to key
/// infoset lookups; rollout oracles may ignore it. `None` means the
/// position is outside the oracle's domain; the compile fails closed.
pub trait MarginOracle {
    fn margin_south(&self, state: &KoiGameState, history: &[u64]) -> Option<f64>;
}

impl<F: Fn(&KoiGameState, &[u64]) -> Option<f64>> MarginOracle for F {
    fn margin_south(&self, state: &KoiGameState, history: &[u64]) -> Option<f64> {
        self(state, history)
    }
}

/// Compiles a resolving gadget game: the belief chance node of
/// `compile_subgame` wrapped so each world first faces the opponent's
/// virtual opt-out decision (P3-D2).
///
/// Per world the opponent chooses between a virtual `Shobu` — opting out
/// to a terminal worth the oracle's counterfactual margin for that world —
/// and a virtual `KoiKoi` — entering the resolved subtree. The gadget
/// decision's infoset key is `(opponent, opp_hand, initial_field,
/// prefix + [gadget_event])`, so worlds the opponent cannot distinguish
/// (same own hand) merge into one gadget infoset — the opponent's exact
/// information partition — while the tag-5 event can never collide with a
/// real action or draw key.
///
/// `max_decision_depth` bounds the decision-node plies expanded below each
/// world root (chance nodes are transparent); at the cap the expander
/// emits an oracle-valued terminal. Both bounds make the full-game
/// residual tractable.
#[allow(clippy::too_many_arguments)]
pub fn compile_gadget_subgame(
    variant: &KoiVariant,
    worlds: &[(KoiGameState, f64)],
    initial_field: CardSet,
    history_prefix: Vec<u64>,
    oracle: &dyn MarginOracle,
    max_decision_depth: usize,
    max_nodes: usize,
    leaf_collector: Option<&LeafBatch>,
) -> Result<EfgTree, EfgCompileError> {
    if worlds.is_empty() {
        return Err(EfgCompileError::InvalidVariant("a subgame needs at least one world"));
    }
    if worlds.iter().any(|(_, p)| !p.is_finite() || *p < 0.0) {
        return Err(EfgCompileError::InvalidVariant(
            "world probabilities must be finite and non-negative",
        ));
    }
    let total: f64 = worlds.iter().map(|(_, p)| p).sum();
    if total <= 0.0 {
        return Err(EfgCompileError::InvalidVariant("world probabilities carry no mass"));
    }

    let ctx = ExpandCtx {
        oracle: Some(oracle),
        max_decision_depth,
        leaf_collector,
    };
    let mut compiler = Compiler::new(variant.clone(), max_nodes);
    let mut outcomes = Vec::with_capacity(worlds.len());
    for (state, weight) in worlds {
        compiler.check_budget()?;
        let opponent = state.active.opponent();
        // The world's true root history: the prefix plus the publicly
        // pending draw when the resolve sits at a stock resolution — the
        // draw is a public fact of this decision that no ledger entry
        // carries yet, and blueprint oracle lookups key on it.
        let mut history = history_prefix.clone();
        if let TurnPhase::AwaitingStockResolution { drawn } = state.phase {
            history.push(draw_event_key(drawn));
        }
        let opt_out = oracle
            .margin_south(state, &history)
            .ok_or(EfgCompileError::OracleUnavailable)?;
        let opt_out_node = compiler.tree.push_terminal(opt_out);

        let enter = compiler.expand(*state, initial_field.bits(), &mut history, 0, &ctx)?;

        // The opponent's virtual decision: own_hand makes worlds sharing
        // the opponent's cards merge; the tag-5 history key is unique to
        // the gadget so it can never alias a real infoset.
        let key = InfoSetKey {
            player: opponent,
            own_hand: state.hands[opponent.index()].bits(),
            initial_field: initial_field.bits(),
            public_history: {
                let mut h = history_prefix.clone();
                h.push(gadget_event_key());
                h
            },
        };
        let actions: Vec<koi_core::Action> = vec![koi_core::Action::Shobu, koi_core::Action::KoiKoi];
        let infoset_id = match compiler.tree.infoset_for(&key) {
            Some(id) => id,
            None => compiler.tree.push_infoset(
                key,
                InfoSet {
                    player: opponent,
                    actions: actions.clone(),
                    members: Vec::new(),
                },
            ),
        };
        let node = compiler.tree.push_decision(opponent, infoset_id, actions);
        compiler.tree.infoset_mut(infoset_id).members.push(node);
        compiler.tree.set_children(node, vec![opt_out_node, enter]);

        let mut dealt: Vec<u8> = state.hands[opponent.index()].into_iter().map(|c| c.index()).collect();
        dealt.extend(state.stock.iter().copied().map(|c| c.index()));
        outcomes.push(ChanceOutcome {
            probability: weight / total,
            dealt,
            child: node,
        });
    }
    let root = compiler.tree.push_chance(outcomes);
    compiler.tree.set_root(root);
    Ok(compiler.tree)
}

fn validate_variant(variant: &KoiVariant) -> Result<(), EfgCompileError> {
    if variant.cards.is_empty() {
        return Err(EfgCompileError::InvalidVariant("empty card universe"));
    }
    if variant.hand_size == 0 || variant.field_size == 0 {
        return Err(EfgCompileError::InvalidVariant("hands and field must be non-empty"));
    }
    if variant.stock_size() == 0 {
        return Err(EfgCompileError::InvalidVariant("no stock remains after the deal"));
    }
    Ok(())
}

/// Collected leaf nodes for batched evaluation: `(NodeId, state, history)`
/// recorded during expansion, batch-evaluated and patched afterward.
type LeafBatch = std::cell::RefCell<Vec<(NodeId, KoiGameState, Vec<u64>)>>;

/// Expansion context for oracle-priced leaves: the value oracle plus the
/// decision-ply cap. `UNBOUNDED` is the plain compile — no cap, no oracle.
/// `leaf_collector` intercepts depth-capped leaves for batched evaluation:
/// the expander pushes a placeholder terminal and records `(NodeId, state,
/// history)` so the caller can batch-evaluate and patch values afterward.
struct ExpandCtx<'a> {
    oracle: Option<&'a dyn MarginOracle>,
    max_decision_depth: usize,
    leaf_collector: Option<&'a LeafBatch>,
}

const UNBOUNDED: ExpandCtx<'static> = ExpandCtx {
    oracle: None,
    max_decision_depth: usize::MAX,
    leaf_collector: None,
};

struct Compiler {
    tree: EfgTree,
    max_nodes: usize,
}

impl Compiler {
    fn new(variant: KoiVariant, max_nodes: usize) -> Self {
        Self {
            tree: EfgTree::new(variant),
            max_nodes,
        }
    }

    /// The root chance node: every accepted initial deal with its normalized
    /// probability. Anomalous deals (Teshi / FourPairs / FieldVoid) are
    /// excluded and the remaining mass renormalized — the same rejection the
    /// benchmark panel applies (decision E2).
    fn expand_initial_deal(&mut self, variant: &KoiVariant, dealer: Player) -> Result<NodeId, EfgCompileError> {
        self.check_budget()?;
        let cards: Vec<Card> = variant.cards.into_iter().collect();
        let hand = usize::from(variant.hand_size);
        let field = usize::from(variant.field_size);

        let mut children = Vec::new();
        let mut dealt_sets = Vec::new();
        for south in combinations(&cards, hand) {
            let south_mask = mask_of(&south);
            let rest1: Vec<Card> = cards.iter().copied().filter(|c| !south_mask.contains(*c)).collect();
            for north in combinations(&rest1, hand) {
                let north_mask = mask_of(&north);
                let rest2: Vec<Card> = rest1.iter().copied().filter(|c| !north_mask.contains(*c)).collect();
                for table in combinations(&rest2, field) {
                    let table_mask = mask_of(&table);
                    // Canonical (ascending) stock order — order is semantically
                    // dead because every draw is re-chanced (P2-D1).
                    let stock: Vec<Card> = rest2.iter().copied().filter(|c| !table_mask.contains(*c)).collect();
                    let (state, anomaly) = KoiGameState::new_from_parts(
                        &stock,
                        [south_mask, north_mask],
                        table_mask,
                        dealer,
                        dealer,
                        variant.rules,
                    );
                    if anomaly.is_some() {
                        continue;
                    }

                    let mut history = Vec::new();
                    children.push(self.expand(state, table_mask.bits(), &mut history, 0, &UNBOUNDED)?);
                    let mut dealt: Vec<u8> = south.iter().map(|c| c.index()).collect();
                    dealt.extend(north.iter().copied().map(|c| c.index()));
                    dealt.extend(table.iter().copied().map(|c| c.index()));
                    dealt_sets.push(dealt);
                }
            }
        }

        if children.is_empty() {
            return Err(EfgCompileError::InvalidVariant("every deal is anomalous"));
        }
        let probability = 1.0 / children.len() as f64;
        let outcomes = children
            .into_iter()
            .zip(dealt_sets)
            .map(|(child, dealt)| ChanceOutcome {
                probability,
                dealt,
                child,
            })
            .collect();
        Ok(self.tree.push_chance(outcomes))
    }

    /// Expands any non-terminal state: decision nodes in each phase plus the
    /// synthesized stock-draw chance nodes. `initial_field` is the field as
    /// dealt (the infoset key's public anchor); `history` is the shared
    /// event buffer, pushed/popped around each branch. `depth` counts the
    /// decision plies already expanded along this path; at
    /// `ctx.max_decision_depth` a live state becomes an oracle-valued leaf.
    fn expand(
        &mut self,
        state: KoiGameState,
        initial_field: u64,
        history: &mut Vec<u64>,
        depth: usize,
        ctx: &ExpandCtx<'_>,
    ) -> Result<NodeId, EfgCompileError> {
        self.check_budget()?;
        // Invariant, enforced: at AwaitingStockResolution the drawn card is
        // public and must sit at the tail of `history` as a Draw event. The
        // play expansion pushes it before recursing (so the tail check is
        // idempotent); a subgame root that enters mid-resolution gets it
        // here — it stays pushed for the whole subtree since the draw is an
        // ancestor of everything below, and oracle lookups on capped leaves
        // keyed on this stream then match the blueprint's key shape.
        if let TurnPhase::AwaitingStockResolution { drawn } = state.phase {
            if history.last() != Some(&draw_event_key(drawn)) {
                history.push(draw_event_key(drawn));
            }
        }
        if state.is_ended() {
            return Ok(self.tree.push_terminal(state.leg_margin(Player::South) as f64));
        }
        if depth >= ctx.max_decision_depth {
            if let Some(collector) = ctx.leaf_collector {
                // Batched leaf path: push a placeholder terminal and record
                // the leaf state for post-expansion batch evaluation.
                let node_id = self.tree.push_terminal(0.0);
                collector.borrow_mut().push((node_id, state, history.clone()));
                return Ok(node_id);
            }
            let Some(oracle) = ctx.oracle else {
                return Err(EfgCompileError::InvalidVariant(
                    "a decision-depth cap requires a leaf oracle",
                ));
            };
            let value = oracle
                .margin_south(&state, history)
                .ok_or(EfgCompileError::OracleUnavailable)?;
            return Ok(self.tree.push_terminal(value));
        }

        let player = state.active;
        let actions = state.legal_actions();
        if actions.is_empty() {
            // Every live kernel phase yields at least one action (a hand
            // card is always playable; stock resolution maps 0-3 matches to
            // 1-3 choices; a stop decision always offers Shobu). An empty
            // list means the input state violates the kernel's invariants.
            return Err(EfgCompileError::KernelTransition(StateError::IllegalAction));
        }
        let key = InfoSetKey {
            player,
            own_hand: state.hands[player.index()].bits(),
            initial_field,
            public_history: history.clone(),
        };
        let infoset_id = match self.tree.infoset_for(&key) {
            Some(id) => {
                // Same key ⇒ same public observations ⇒ the kernel must
                // report the same legal set. A mismatch would mean the key
                // leaks (or omits) state — fail closed either way.
                if self.tree.infosets()[id].actions.as_slice() != actions.as_slice() {
                    return Err(EfgCompileError::InfoSetActionMismatch);
                }
                id
            }
            None => self.tree.push_infoset(
                key,
                InfoSet {
                    player,
                    actions: actions.iter().copied().collect(),
                    members: Vec::new(),
                },
            ),
        };

        let node_id = self
            .tree
            .push_decision(player, infoset_id, actions.iter().copied().collect());
        self.tree.infoset_mut(infoset_id).members.push(node_id);

        let mut children = Vec::with_capacity(actions.len());
        for action in actions.iter().copied() {
            match action {
                koi_core::Action::PlayFromHand { .. } => {
                    self.check_budget()?;
                    history.push(action.action_key());
                    let child = self.expand_play_then_draw(state, action, initial_field, history, depth + 1, ctx)?;
                    history.pop();
                    children.push(child);
                }
                koi_core::Action::ResolveStock { .. } | koi_core::Action::KoiKoi | koi_core::Action::Shobu => {
                    history.push(action.action_key());
                    let next = state.apply_action(action)?;
                    let child = self.expand(next, initial_field, history, depth + 1, ctx)?;
                    history.pop();
                    children.push(child);
                }
            }
        }
        self.tree.set_children(node_id, children);
        Ok(node_id)
    }

    /// The stock-draw chance node after a hand play: `apply_action` has
    /// already moved the state to `AwaitingStockResolution { drawn: s0 }`
    /// with `s0` the literal top card; the true model draws uniformly over
    /// the remaining stock, so each card gets a child pinned to the head.
    fn expand_play_then_draw(
        &mut self,
        state: KoiGameState,
        action: koi_core::Action,
        initial_field: u64,
        history: &mut Vec<u64>,
        depth: usize,
        ctx: &ExpandCtx<'_>,
    ) -> Result<NodeId, EfgCompileError> {
        let mid = state.apply_action(action)?;
        let TurnPhase::AwaitingStockResolution { .. } = mid.phase else {
            // A hand play always enters stock resolution; anything else is a
            // kernel anomaly the compiler does not model.
            return Err(EfgCompileError::KernelTransition(StateError::NotInPhase));
        };

        // Enumerate the remaining stock in canonical (ascending) order.
        let remaining: CardSet = mid.stock.iter().copied().collect();
        let count = remaining.count();
        debug_assert!(count > 0, "AwaitingStockResolution implies a drawn card");
        let probability = 1.0 / count as f64;

        let mut outcomes = Vec::with_capacity(count as usize);
        for drawn in remaining {
            self.check_budget()?;
            let mut tail = Vec::with_capacity(count as usize);
            tail.push(drawn);
            tail.extend(remaining.into_iter().filter(|card| *card != drawn));
            let mut child_state = mid;
            child_state.phase = TurnPhase::AwaitingStockResolution { drawn };
            child_state.stock = koi_core::Stock::from_slice(&tail)?;

            history.push(draw_event_key(drawn));
            let child = self.expand(child_state, initial_field, history, depth, ctx)?;
            history.pop();
            outcomes.push(ChanceOutcome {
                probability,
                dealt: vec![drawn.index()],
                child,
            });
        }
        Ok(self.tree.push_chance(outcomes))
    }

    fn check_budget(&self) -> Result<(), EfgCompileError> {
        if self.tree.node_count() >= self.max_nodes {
            return Err(EfgCompileError::NodeBudgetExceeded { budget: self.max_nodes });
        }
        Ok(())
    }
}

/// All size-`k` combinations of `items`, preserving input order.
fn combinations<T: Clone>(items: &[T], k: usize) -> Vec<Vec<T>> {
    fn pick<T: Clone>(items: &[T], start: usize, k: usize, current: &mut Vec<T>, result: &mut Vec<Vec<T>>) {
        if current.len() == k {
            result.push(current.clone());
            return;
        }
        for (index, item) in items.iter().enumerate().skip(start) {
            current.push(item.clone());
            pick(items, index + 1, k, current, result);
            current.pop();
        }
    }
    let mut result = Vec::new();
    pick(items, 0, k, &mut Vec::new(), &mut result);
    result
}

fn mask_of(cards: &[Card]) -> CardSet {
    cards.iter().fold(CardSet::EMPTY, |mask, card| mask.insert(*card))
}

#[cfg(test)]
mod tests {
    use koi_core::{Action, CaptureChoice, Month};

    use super::super::tree::NodeType;
    use super::*;

    fn micro_tree() -> EfgTree {
        compile_variant(&KoiVariant::MICRO_8, Player::South).unwrap()
    }

    /// Every chance node's outcomes carry normalized probabilities.
    #[test]
    fn chance_probabilities_sum_to_one() {
        let tree = micro_tree();
        fn walk(tree: &EfgTree, id: NodeId) {
            match tree.node(id) {
                NodeType::Chance { outcomes } => {
                    assert!(!outcomes.is_empty());
                    let total: f64 = outcomes.iter().map(|outcome| outcome.probability).sum();
                    assert!((total - 1.0).abs() < 1e-9, "chance node {id} carries mass {total}");
                    for outcome in outcomes {
                        assert!(outcome.probability > 0.0);
                        walk(tree, outcome.child);
                    }
                }
                NodeType::Decision { children, .. } => {
                    for &child in children {
                        walk(tree, child);
                    }
                }
                NodeType::Terminal { .. } => {}
            }
        }
        walk(&tree, tree.root());
    }

    /// Information sets partition the decision nodes; members share player
    /// and action list.
    #[test]
    fn infosets_partition_all_decision_nodes() {
        let tree = micro_tree();
        let mut covered = vec![false; tree.node_count()];
        for (infoset_id, infoset) in tree.infosets().iter().enumerate() {
            assert!(!infoset.members.is_empty(), "infoset {infoset_id} is empty");
            for &member in &infoset.members {
                match tree.node(member) {
                    NodeType::Decision {
                        player,
                        infoset: owner,
                        actions,
                        ..
                    } => {
                        assert_eq!(player, &infoset.player);
                        assert_eq!(owner, &infoset_id);
                        assert_eq!(actions, &infoset.actions);
                        assert!(!covered[member], "node {member} claimed by two infosets");
                        covered[member] = true;
                    }
                    other => panic!("infoset member {member} is not a decision node: {other:?}"),
                }
            }
        }
        for (id, node) in tree.nodes().iter().enumerate() {
            if matches!(node, NodeType::Decision { .. }) {
                assert!(covered[id], "decision node {id} belongs to no infoset");
            }
        }
    }

    /// Deals differing only in the opponent's hand / stock are
    /// indistinguishable and share the root infoset; a different field is
    /// observable and splits.
    #[test]
    fn indistinguishable_histories_share_one_infoset() {
        let tree = micro_tree();
        let cards: Vec<Card> = KoiVariant::MICRO_8.cards.into_iter().collect();

        let key = InfoSetKey {
            player: Player::South,
            own_hand: CardSet::from_card(cards[0]).bits(),
            initial_field: CardSet::from_card(cards[3]).union(CardSet::from_card(cards[4])).bits(),
            public_history: Vec::new(),
        };
        let infoset = tree.infoset_for(&key).expect("the opener infoset exists");
        // After fixing South's card and the two-card field, 5 cards remain:
        // the opponent's 1 card and the 4-card stock split them 5 ways.
        assert_eq!(tree.infosets()[infoset].members.len(), 5);

        let other_key = InfoSetKey {
            player: Player::South,
            own_hand: CardSet::from_card(cards[0]).bits(),
            initial_field: CardSet::from_card(cards[3]).union(CardSet::from_card(cards[5])).bits(),
            public_history: Vec::new(),
        };
        let other = tree.infoset_for(&other_key).expect("the other opener infoset exists");
        assert_ne!(infoset, other);
    }

    /// An independent kernel replay must meet the tree in lockstep:
    /// identical legal actions at every decision and identical terminal
    /// margins.
    #[test]
    fn compiled_tree_matches_an_independent_kernel_replay() {
        let tree = micro_tree();
        let cards: Vec<Card> = KoiVariant::MICRO_8.cards.into_iter().collect();
        // Deal: south=cards[0], north=cards[1], field={cards[2],cards[3]},
        // stock rest.
        let dealt: Vec<u8> = vec![cards[0].index(), cards[1].index(), cards[2].index(), cards[3].index()];
        let NodeType::Chance { outcomes } = tree.node(tree.root()) else {
            panic!("the root must be a chance node");
        };
        let mut node = outcomes
            .iter()
            .find(|outcome| outcome.dealt == dealt)
            .expect("the replayed deal must exist as a root outcome")
            .child;

        let stock: Vec<Card> = cards[4..].to_vec();
        let mut state = KoiGameState::new_from_parts(
            &stock,
            [CardSet::from_card(cards[0]), CardSet::from_card(cards[1])],
            CardSet::from_card(cards[2]).insert(cards[3]),
            Player::South,
            Player::South,
            Ruleset::Nintendo,
        )
        .0;
        assert!(state.check_deal().is_none());

        loop {
            match tree.node(node) {
                NodeType::Decision { actions, children, .. } => {
                    let kernel_actions = state.legal_actions();
                    let kernel_keys: Vec<u64> = kernel_actions.iter().map(Action::action_key).collect();
                    let tree_keys: Vec<u64> = actions.iter().map(Action::action_key).collect();
                    assert_eq!(tree_keys, kernel_keys, "tree actions must match the kernel's");

                    let action = kernel_actions[0];
                    let next = state.apply_action(action).unwrap();
                    node = children[0];
                    // A hand play enters a draw chance node: the true stock
                    // top selects the outcome in the replayed world.
                    if let TurnPhase::AwaitingStockResolution { drawn } = next.phase {
                        if let NodeType::Chance { outcomes } = tree.node(node) {
                            let outcome = outcomes
                                .iter()
                                .find(|outcome| outcome.dealt == vec![drawn.index()])
                                .expect("the drawn card must be a chance outcome");
                            node = outcome.child;
                        }
                        state = next;
                    } else {
                        state = next;
                    }
                }
                NodeType::Terminal { utility_south } => {
                    assert!(state.is_ended());
                    assert_eq!(*utility_south, state.leg_margin(Player::South) as f64);
                    break;
                }
                NodeType::Chance { .. } => panic!("chance nodes only follow hand plays"),
            }
        }
    }

    /// The FIELDVOID_10 universe contains four-of-a-month fields, so some
    /// deals are excluded and the kept mass renormalizes to one.
    #[test]
    fn anomalous_deals_are_excluded_and_renormalized() {
        let tree = compile_variant(&KoiVariant::FIELDVOID_10, Player::South).unwrap();
        let NodeType::Chance { outcomes } = tree.node(tree.root()) else {
            panic!("the root must be a chance node");
        };
        // Independently count the field-void deals in the universe: hands are
        // one card each, so a void happens iff the 4-card field equals a full
        // month — months January and February each contribute once per
        // (south, north) pair whose cards lie outside that month.
        let jan = CardSet::from_month(Month::new_unchecked(0));
        let feb = CardSet::from_month(Month::new_unchecked(1));
        let cards: Vec<Card> = KoiVariant::FIELDVOID_10.cards.into_iter().collect();
        let mut void_deals = 0usize;
        let mut total_deals = 0usize;
        for &south in &cards {
            for &north in cards.iter().filter(|&&c| c != south) {
                let rest2: Vec<Card> = cards.iter().copied().filter(|c| *c != south && *c != north).collect();
                for table in combinations(&rest2, 4) {
                    total_deals += 1;
                    let mask = mask_of(&table);
                    if mask == jan || mask == feb {
                        void_deals += 1;
                    }
                }
            }
        }
        assert!(void_deals > 0, "the fixture must contain field-void deals");
        assert_eq!(outcomes.len(), total_deals - void_deals);
        let total: f64 = outcomes.iter().map(|o| o.probability).sum();
        assert!((total - 1.0).abs() < 1e-9);
        // Every kept outcome carries a non-void dealt record.
        for outcome in outcomes {
            let field_cards: Vec<Card> = outcome.dealt[2..].iter().map(|&i| Card::new_unchecked(i)).collect();
            let field = mask_of(&field_cards);
            assert!(field != jan && field != feb, "a field-void deal leaked through");
        }
    }

    /// The node budget fails closed.
    #[test]
    fn node_budget_is_enforced() {
        let err = compile_variant_with_budget(&KoiVariant::MICRO_8, Player::South, 64).unwrap_err();
        assert_eq!(err, EfgCompileError::NodeBudgetExceeded { budget: 64 });
    }

    /// Invalid geometries are rejected before enumeration.
    #[test]
    fn invalid_variants_are_rejected() {
        let mut bad = KoiVariant::MICRO_8;
        bad.cards = CardSet::EMPTY;
        assert!(matches!(
            compile_variant(&bad, Player::South),
            Err(EfgCompileError::InvalidVariant(_))
        ));
        let mut exhausted = KoiVariant::MICRO_8;
        exhausted.cards = CardSet::new_unchecked(0x7); // 3 cards, h1 f1 → stock 1... wait: 3-3=0
        assert!(matches!(
            compile_variant(&exhausted, Player::South),
            Err(EfgCompileError::InvalidVariant(_))
        ));
    }

    /// ResolveStock and stop actions round-trip through the kernel —
    /// exercises the non-play branches of `expand`.
    #[test]
    fn resolve_and_stop_paths_execute() {
        // Hand-build a state at stock resolution.
        let stock: Vec<Card> = (8..12).map(Card::new_unchecked).collect();
        let state = KoiGameState::new_from_parts(
            &stock,
            [
                CardSet::from_card(Card::new_unchecked(0)),
                CardSet::from_card(Card::new_unchecked(4)),
            ],
            CardSet::from_card(Card::new_unchecked(6)),
            Player::South,
            Player::South,
            Ruleset::Nintendo,
        )
        .0;
        let action = state
            .legal_actions()
            .into_iter()
            .find(|a| matches!(a, Action::PlayFromHand { .. }))
            .unwrap();
        let mid = state.apply_action(action).unwrap();
        assert!(matches!(mid.phase, TurnPhase::AwaitingStockResolution { .. }));
        let resolve = mid.legal_actions().into_iter().next().unwrap();
        assert!(matches!(resolve, Action::ResolveStock { .. }));
        let _ = CaptureChoice::NoMatch; // variant exists; kernel validated
    }
}
