//! Fixed-width feature encoding for the learned leaf evaluator.
//!
//! The vector layout is part of the training-data contract: the data
//! generator, the dataset loader, and the runtime leaf evaluator must agree
//! on it bit for bit. All quantities are from the *acting* player's point of
//! view — "own" is always the player to move, so the network never has to
//! infer seat orientation from a flag it was not given.
//!
//! Ported from a sibling research codebase's `leaf/features.rs`, re-laid-out for the
//! 48-card Hanafuda universe and Koi-Koi's zones (the public drawn card is
//! its own block — the sibling codebase has no equivalent). No hidden membership is ever
//! encoded: the opponent hand appears only as a *count* and as belief
//! marginals, so the encoding is safe to compute on a concrete world state.
//!
//! Layout (`FEATURE_DIM` = 384, rows 32-byte aligned at f32 width):
//!
//! | Offset | Width | Block |
//! |---|---|---|
//! | 0    | 48 | own hand bitset (canonical card index) |
//! | 48   | 48 | field bitset |
//! | 96   | 48 | own captured-pile bitset |
//! | 144  | 48 | opponent captured-pile bitset |
//! | 192  | 48 | belief vector: P(unseen card in opponent hand) |
//! | 240  | 48 | drawn card one-hot (stock-resolution phases only) |
//! | 288  | 96 | scalars and histograms (see `scalar` offsets) |
//!
//! Values are stored RAW (counts, sums, flags). The exported ONNX graph bakes
//! scalar normalization into its own input stage (Subtract/Div/Clamp), so
//! inference feeds raw features and no `norm_stats.json` sidecar is consulted
//! at evaluation time; training still computes the stats to freeze them into
//! the graph at export.

use koi_core::{
    yaku::{hikari_count, kasu_count, score_yaku, tane_count, tanzaku_count},
    Action, CardSet, KoiGameState, Player, TurnPhase,
};

/// Total feature width: six 48-wide card blocks plus a 96-wide scalar
/// block. Pinned: the trained model's input contract — a drift here
/// silently desynchronizes the encoder from the shipped ONNX graphs.
pub const FEATURE_DIM: usize = 384;
/// Canonical card-index width of every bitset block — deck-width by
/// construction, not a free constant.
pub const CARD_BLOCK: usize = 48;
/// Number of scalar slots (offsets 288..384).
const SCALAR_DIM: usize = 96;

const _: () = {
    assert!(
        CARD_BLOCK == 48,
        "the leaf encoder's card blocks are 48-wide by model contract"
    );
    assert!(
        FEATURE_DIM == 384,
        "the leaf encoder's feature width is 384 by model contract"
    );
    assert!(288 + SCALAR_DIM == FEATURE_DIM);
};

/// Scalar offsets inside the feature vector.
pub mod scalar {
    pub const OWN_HAND_COUNT: usize = 288;
    pub const OPP_HAND_COUNT: usize = 289;
    pub const FIELD_COUNT: usize = 290;
    pub const STOCK_COUNT: usize = 291;
    pub const UNSEEN_COUNT: usize = 292;
    pub const TURN: usize = 293;
    pub const ROUND: usize = 294;
    pub const OWN_MATCH_SCORE: usize = 295;
    pub const OPP_MATCH_SCORE: usize = 296;
    pub const OWN_KK_CALLS: usize = 297;
    pub const OPP_KK_CALLS: usize = 298;
    /// Koi-Koi caller one-hot relative to the actor: self / opponent.
    pub const KK_CALLER_SELF: usize = 299;
    pub const KK_CALLER_OPP: usize = 300;
    /// The base score offered at a stop decision, else zero.
    pub const BASE_SCORE: usize = 301;
    pub const LAST_YAKU_OWN: usize = 302;
    pub const LAST_YAKU_OPP: usize = 303;
    /// Phase one-hot: awaiting hand play / stock resolution / stop decision.
    pub const PHASE_HAND: usize = 304;
    pub const PHASE_STOCK: usize = 305;
    pub const PHASE_STOP: usize = 306;
    pub const LEGAL_COUNT: usize = 307;
    /// Any legal action captures field cards.
    pub const HAS_CAPTURE: usize = 308;
    pub const ACTOR_IS_SOUTH: usize = 309;
    pub const ACTOR_IS_DEALER: usize = 310;
    /// Current formed-yaku score per pile (the ruleset's own scorer).
    pub const OWN_YAKU_SCORE: usize = 311;
    pub const OPP_YAKU_SCORE: usize = 312;
    /// Card-type histograms (hikari/tane/tanzaku/kasu), four slots each.
    pub const OWN_CAPT_TYPE_BASE: usize = 313;
    pub const OPP_CAPT_TYPE_BASE: usize = 317;
    pub const OWN_HAND_TYPE_BASE: usize = 321;
    pub const UNSEEN_TYPE_BASE: usize = 325;
    /// Month histograms over the 12 months, twelve slots each.
    pub const FIELD_MONTH_BASE: usize = 329;
    pub const OWN_HAND_MONTH_BASE: usize = 341;
    // 353..384 — slack reserved so future scalars extend the block without
    // shifting the layout.
}

fn scatter_block(out: &mut [f32], offset: usize, set: CardSet) {
    for card in set {
        let index = card.index() as usize;
        debug_assert!(index < CARD_BLOCK);
        out[offset + index] = 1.0;
    }
}

fn month_histogram(out: &mut [f32], base: usize, set: CardSet) {
    for card in set {
        out[base + card.month().index() as usize] += 1.0;
    }
}

fn type_histogram(out: &mut [f32], base: usize, set: CardSet, state: &KoiGameState) {
    out[base] += hikari_count(set) as f32;
    out[base + 1] += tane_count(set) as f32;
    out[base + 2] += tanzaku_count(set) as f32;
    out[base + 3] += kasu_count(set, &state.rules.yaku_points()) as f32;
}

/// The unseen-card pool from the acting player's view over `universe`:
/// every card in the domain not in a public zone. The resolver passes the
/// variant's card set so a reduced-domain belief marginal agrees with
/// `build_belief`'s own partition.
pub fn unseen_mask_in(state: &KoiGameState, universe: CardSet) -> CardSet {
    let observer = state.active;
    universe.difference(
        state.hands[observer.index()]
            .union(state.field)
            .union(state.captured[0])
            .union(state.captured[1]),
    )
}

/// The unseen-card pool from the acting player's view: every card in the
/// domain not in a public zone. On a concrete world state this is still the
/// *observer's* unseen set — the hidden partition inside it never enters
/// the encoding.
pub fn unseen_mask(state: &KoiGameState) -> CardSet {
    unseen_mask_in(state, CardSet::ALL)
}

/// Marginal per-card belief from member weights: `belief[c]` is the
/// probability that unseen card `c` sits in the opponent's hand. Visible
/// cards are always zero. Shared by the offline labeler and the runtime
/// leaf path so both feed the encoder an identical belief block.
pub fn card_marginals(unseen: CardSet, worlds: &[crate::resolving::World], weights: &[f64]) -> [f32; CARD_BLOCK] {
    let mut belief = [0.0_f32; CARD_BLOCK];
    for (world, weight) in worlds.iter().zip(weights.iter()) {
        for card in world.opponent_hand.intersection(unseen) {
            belief[card.index() as usize] += *weight as f32;
        }
    }
    belief
}

/// Encodes `state` for the acting player into a fixed `[f32; FEATURE_DIM]`.
///
/// `belief[c]` is the marginal probability that unseen card `c` sits in the
/// opponent's hand (zero for visible cards); callers choose the provenance
/// — the resolve root's posterior for in-resolve leaves, a fresh
/// `build_belief` sample for standalone labeling. `legal` is the state's
/// canonical-ordered legal list — only its size and capture presence are
/// encoded, never action identity.
pub fn encode(state: &KoiGameState, belief: &[f32; CARD_BLOCK], legal: &[Action]) -> [f32; FEATURE_DIM] {
    let mut out = [0.0_f32; FEATURE_DIM];
    let actor = state.active;
    let opponent = actor.opponent();

    let own_hand = state.hands[actor.index()];
    let opp_hand = state.hands[opponent.index()];
    let field = state.field;
    let own_captured = state.captured[actor.index()];
    let opp_captured = state.captured[opponent.index()];
    let unseen = unseen_mask(state);

    scatter_block(&mut out, 0, own_hand);
    scatter_block(&mut out, 48, field);
    scatter_block(&mut out, 96, own_captured);
    scatter_block(&mut out, 144, opp_captured);
    out[192..240].copy_from_slice(belief);
    if let TurnPhase::AwaitingStockResolution { drawn } = state.phase {
        out[240 + drawn.index() as usize] = 1.0;
    }

    use scalar::*;
    out[OWN_HAND_COUNT] = own_hand.count() as f32;
    // Membership never enters the encoding — the opponent's hand is a
    // count, which is public information.
    out[OPP_HAND_COUNT] = opp_hand.count() as f32;
    out[FIELD_COUNT] = field.count() as f32;
    out[STOCK_COUNT] = state.stock.len() as f32;
    out[UNSEEN_COUNT] = unseen.count() as f32;
    out[TURN] = state.turn as f32;
    out[ROUND] = state.round as f32;
    out[OWN_MATCH_SCORE] = state.score[actor.index()] as f32;
    out[OPP_MATCH_SCORE] = state.score[opponent.index()] as f32;
    out[OWN_KK_CALLS] = state.koi_koi_calls[actor.index()] as f32;
    out[OPP_KK_CALLS] = state.koi_koi_calls[opponent.index()] as f32;
    match state.koi_koi_caller {
        Some(caller) if caller == actor => out[KK_CALLER_SELF] = 1.0,
        Some(_) => out[KK_CALLER_OPP] = 1.0,
        None => {}
    }
    if let TurnPhase::AwaitingStopDecision { base_score } = state.phase {
        out[BASE_SCORE] = base_score as f32;
    }
    out[LAST_YAKU_OWN] = state.last_yaku_score[actor.index()] as f32;
    out[LAST_YAKU_OPP] = state.last_yaku_score[opponent.index()] as f32;
    match state.phase {
        TurnPhase::AwaitingHandAction => out[PHASE_HAND] = 1.0,
        TurnPhase::AwaitingStockResolution { .. } => out[PHASE_STOCK] = 1.0,
        TurnPhase::AwaitingStopDecision { .. } => out[PHASE_STOP] = 1.0,
        TurnPhase::Ended => {}
    }
    out[LEGAL_COUNT] = legal.len() as f32;
    out[HAS_CAPTURE] = legal.iter().any(|action| match action {
        Action::PlayFromHand { capture, .. } | Action::ResolveStock { capture } => {
            !matches!(capture, koi_core::CaptureChoice::NoMatch)
        }
        _ => false,
    }) as u8 as f32;
    out[ACTOR_IS_SOUTH] = (actor == Player::South) as u8 as f32;
    out[ACTOR_IS_DEALER] = (actor == state.dealer) as u8 as f32;
    out[OWN_YAKU_SCORE] = score_yaku(own_captured, &state.rules) as f32;
    out[OPP_YAKU_SCORE] = score_yaku(opp_captured, &state.rules) as f32;

    type_histogram(&mut out, OWN_CAPT_TYPE_BASE, own_captured, state);
    type_histogram(&mut out, OPP_CAPT_TYPE_BASE, opp_captured, state);
    type_histogram(&mut out, OWN_HAND_TYPE_BASE, own_hand, state);
    type_histogram(&mut out, UNSEEN_TYPE_BASE, unseen, state);
    month_histogram(&mut out, FIELD_MONTH_BASE, field);
    month_histogram(&mut out, OWN_HAND_MONTH_BASE, own_hand);

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use koi_core::{deal_from_seed, Ruleset};

    fn deal(seed: u64) -> KoiGameState {
        let (state, _) = KoiGameState::new_deal(deal_from_seed(seed), Ruleset::nintendo());
        state
    }

    #[test]
    fn the_encoding_is_deterministic_and_full_width() {
        let state = deal(4);
        let legal = state.legal_actions();
        let belief = [0.0_f32; CARD_BLOCK];
        let a = encode(&state, &belief, &legal);
        let b = encode(&state, &belief, &legal);
        assert_eq!(a, b);
        assert_eq!(a.len(), FEATURE_DIM);
        assert!(a[353..].iter().all(|v| *v == 0.0), "slack stays zero");
    }

    #[test]
    fn the_bitsets_and_counts_reflect_the_dealt_layout() {
        let state = deal(5);
        let legal = state.legal_actions();
        let out = encode(&state, &[0.0; CARD_BLOCK], &legal);

        let own = state.hands[state.active.index()];
        for (index, value) in out.iter().enumerate().take(48) {
            let expected = own.contains(koi_core::Card::new(index as u8).unwrap());
            assert_eq!(*value, expected as u8 as f32, "hand bit {index}");
        }
        assert_eq!(out[scalar::OWN_HAND_COUNT], 8.0);
        assert_eq!(out[scalar::FIELD_COUNT], 8.0);
        assert_eq!(out[scalar::STOCK_COUNT], 24.0);
        assert_eq!(out[scalar::UNSEEN_COUNT], 8.0 + 24.0);
        assert_eq!(out[scalar::ACTOR_IS_SOUTH], 1.0);
        assert_eq!(out[scalar::PHASE_HAND], 1.0);
        assert_eq!(out[scalar::TURN], 0.0);
    }

    #[test]
    fn the_belief_block_passthrough_carries_the_range() {
        let state = deal(6);
        let legal = state.legal_actions();
        let mut belief = [0.0_f32; CARD_BLOCK];
        belief[7] = 0.5;
        belief[19] = 0.25;
        let out = encode(&state, &belief, &legal);
        assert_eq!(out[192 + 7], 0.5);
        assert_eq!(out[192 + 19], 0.25);
    }

    #[test]
    fn the_drawn_card_block_fires_only_at_stock_resolution() {
        let state = deal(7);
        let legal = state.legal_actions();
        let out = encode(&state, &[0.0; CARD_BLOCK], &legal);
        assert!(
            out[240..288].iter().all(|v| *v == 0.0),
            "no draw pending at a hand decision"
        );

        // Play any hand action; the kernel then draws publicly.
        let next = state.apply_action(legal[0]).unwrap();
        if let TurnPhase::AwaitingStockResolution { drawn } = next.phase {
            let out = encode(&next, &[0.0; CARD_BLOCK], &next.legal_actions());
            assert_eq!(out[240 + drawn.index() as usize], 1.0);
            assert_eq!(out[scalar::PHASE_STOCK], 1.0);
        }
    }
}
