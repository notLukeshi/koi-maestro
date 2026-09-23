//! ISMCTS tree: information-set-keyed nodes with availability-counted UCB.

use koi_core::{Action, CardSet, KoiGameState, Player, TurnPhase};
use rustc_hash::FxHashMap;
use smallvec::SmallVec;

/// The information set a node belongs to: everything the acting player can
/// observe, in a fixed, hashable layout. The sampled opponent-hand cards and
/// the stock order are *not* part of the key — only their sizes — so worlds
/// indistinguishable to the acting player share statistics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct InfoState {
    /// Whose observation this is (the acting player at the node).
    pub observer: Player,
    /// The observer's own hand — the real cards, which are part of the view.
    pub own_hand: CardSet,
    /// Opponent hand size (identities are hidden).
    pub opponent_hand_count: u8,
    /// Public zones, always fully observable.
    pub field: CardSet,
    pub captured: [CardSet; 2],
    /// Hidden stock size; the drawn card rides in `phase` when public.
    pub stock_count: u8,
    /// Phase tag plus its public payload (drawn card / base score).
    pub phase_tag: u8,
    pub phase_card: u8,
    pub phase_score: u32,
    pub dealer: Player,
    pub turn: u8,
    pub round: u8,
    pub score: [i32; 2],
    pub koi_koi_caller: Option<Player>,
    pub koi_koi_calls: [u8; 2],
    pub last_yaku_score: [u32; 2],
}

impl InfoState {
    /// The acting player's information set for `state`.
    pub(super) fn from_state(state: &KoiGameState) -> Self {
        let observer = state.active;
        let (phase_tag, phase_card, phase_score) = match state.phase {
            TurnPhase::AwaitingHandAction => (0u8, 0xFF, 0),
            TurnPhase::AwaitingStockResolution { drawn } => (1, drawn.index(), 0),
            TurnPhase::AwaitingStopDecision { base_score } => (2, 0xFF, base_score),
            TurnPhase::Ended => (3, 0xFF, 0),
        };
        Self {
            observer,
            own_hand: state.hands[observer.index()],
            opponent_hand_count: state.hands[observer.opponent().index()].count() as u8,
            field: state.field,
            captured: state.captured,
            stock_count: state.stock.len() as u8,
            phase_tag,
            phase_card,
            phase_score,
            dealer: state.dealer,
            turn: state.turn,
            round: state.round,
            score: state.score,
            koi_koi_caller: state.koi_koi_caller,
            koi_koi_calls: state.koi_koi_calls,
            last_yaku_score: state.last_yaku_score,
        }
    }
}

/// One arm of a node: statistics accumulated across determinizations.
#[derive(Debug, Clone)]
pub(super) struct Edge {
    pub action: Action,
    /// Completed propagations through this edge.
    pub visits: u32,
    /// Sum of root-observer-relative rewards.
    pub total_value: f32,
    /// Iterations in which this action was legal — the availability count
    /// that weights exploration, since an arm legal in few worlds should
    /// not accrue exploration bonus from worlds where it cannot be played.
    pub availability: u32,
}

/// A tree node keyed by an information set.
#[derive(Debug, Default)]
pub(super) struct Node {
    /// Arms not yet taken (intersected with the world's legal set per visit).
    pub untried: SmallVec<[Action; 24]>,
    pub edges: Vec<Edge>,
    /// Iterations that reached this node — the UCB denominator's `ln N`.
    pub visits: u32,
}

/// The per-decision tree.
#[derive(Debug, Default)]
pub(super) struct Tree {
    pub nodes: FxHashMap<InfoState, Node>,
}

impl Tree {
    /// Fetches the node for `key`, inserting it (with the world's legal
    /// actions as untried arms) when absent. Returns `None` when the node
    /// budget is exhausted — the caller then rolls out without growing.
    pub(super) fn ensure_node(
        &mut self,
        key: InfoState,
        legal: &SmallVec<[Action; 24]>,
        max_nodes: usize,
    ) -> Option<&mut Node> {
        if !self.nodes.contains_key(&key) && self.nodes.len() >= max_nodes {
            return None;
        }
        Some(self.nodes.entry(key).or_insert_with(|| Node {
            untried: legal.clone(),
            edges: Vec::with_capacity(legal.len()),
            visits: 0,
        }))
    }

    /// Read-only node fetch for the final argmax.
    pub(super) fn node(&self, key: &InfoState) -> Option<&Node> {
        self.nodes.get(key)
    }
}

/// Platform-stable `ln` for positive finite `f32`, built only from
/// IEEE-754-exact ops (mantissa decomposition, add/mul/div). System libm
/// `ln` may differ in the last ulp between MSVC UCRT and glibc, which can
/// flip a UCB argmax on identical inputs — the P2-D4 bit-identical
/// standard (P3-D15 watch item). Range-reduce `x = m * 2^e` with `m` in
/// [sqrt(1/2), sqrt(2)], then `ln(m) = 2z * (1 + z²/3 + z⁴/5 + ...)` for
/// `z = (m-1)/(m+1)` — |z| ≤ 0.172, so the series through z²⁸ leaves a
/// tail ~1e-15, far under one f32 ulp.
fn stable_ln(x: f32) -> f32 {
    debug_assert!(x.is_finite() && x > 0.0);
    let bits = x.to_bits();
    let mut e = ((bits >> 23) & 0xff) as i32 - 127;
    let mut m = f32::from_bits((bits & 0x007f_ffff) | 0x3f80_0000);
    if m > std::f32::consts::SQRT_2 {
        m *= 0.5;
        e += 1;
    }
    // The transcendental tail runs in f64 — also IEEE-754-exact everywhere,
    // and accurate enough that the single f32 cast at the end is correctly
    // rounded for all but adversarial halfway cases.
    let z = ((m - 1.0) / (m + 1.0)) as f64;
    let z2 = z * z;
    let series = 1.0
        + z2 * (1.0 / 3.0
            + z2 * (1.0 / 5.0
                + z2 * (1.0 / 7.0
                    + z2 * (1.0 / 9.0
                        + z2 * (1.0 / 11.0 + z2 * (1.0 / 13.0 + z2 * (1.0 / 15.0 + z2 * (1.0 / 17.0))))))));
    (e as f64 * std::f64::consts::LN_2 + 2.0 * z * series) as f32
}

/// UCB score for an edge at a node, from the acting player's perspective.
/// `exploit` is the edge's mean reward (flipped when the opponent acts);
/// exploration uses the availability count so rarely-legal arms still get
/// explored proportionally to how often they could have been taken.
pub(super) fn ucb_score(edge: &Edge, parent_visits: u32, maximize: bool, exploration: f32) -> f32 {
    let mean = if edge.visits > 0 {
        edge.total_value / edge.visits as f32
    } else {
        0.5
    };
    let exploit = if maximize { mean } else { 1.0 - mean };
    let availability = edge.availability.max(1) as f32;
    let explore = exploration * (stable_ln(parent_visits.max(1) as f32) / availability).sqrt();
    exploit + explore
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The series must track the f64 reference to ~1 ulp of f32 across the
    /// whole visit-count domain — accuracy is what makes the deterministic
    /// value a drop-in for libm `ln`.
    #[test]
    fn stable_ln_matches_f64_reference() {
        let mut max_ulp = 0i64;
        for n in 1u32..=200_000 {
            let x = n as f32;
            let approx = stable_ln(x);
            let reference = (x as f64).ln() as f32;
            max_ulp = max_ulp.max((approx.to_bits() as i64 - reference.to_bits() as i64).abs());
            assert!(max_ulp <= 2, "n={n}: {approx} vs {reference} ({max_ulp} ulp)");
        }
        for pow in 0..24u32 {
            let x = 2f32.powi(pow as i32);
            assert_eq!(stable_ln(x), (x as f64).ln() as f32, "2^{pow}");
        }
        assert_eq!(stable_ln(1.0), 0.0);
    }

    /// Determinism is structural — `stable_ln` uses no libm — but pin the
    /// boundary cases so a future refactor cannot silently reintroduce a
    /// platform-dependent path.
    #[test]
    fn ucb_score_is_stable() {
        let edge = Edge {
            action: Action::KoiKoi,
            visits: 7,
            total_value: 3.5,
            availability: 9,
        };
        let score = ucb_score(&edge, 41, true, 0.7);
        assert!(score.is_finite());
        assert_eq!(score.to_bits(), 0x3f73_1c22, "pinned UCB bits drifted");
    }
}
