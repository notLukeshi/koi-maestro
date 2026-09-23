//! The public belief state for resolving: the posterior over opponent hands
//! at a decision point.
//!
//! A Koi-Koi public state leaves the opponent's hand and the stock order
//! hidden. Because the compiled subgame re-chances every draw, the stock
//! *order* is semantically dead in a world — the belief support is the
//! opponent-hand assignments only, `C(unseen, opp_hand_size)` worlds. That
//! support is enumerated when it fits `max_worlds` and sampled without
//! replacement otherwise; weights are the chance prior (uniform over
//! compatible partitions) until the opponent model reweights them.
//!
//! Zone validation is fail-closed: the unknown pool must partition exactly
//! into opponent hand + stock tail (with a publicly drawn card pinned at
//! the head during stock resolution), and the hidden zones must be
//! disjoint from every public zone — the kernel's `rebuild_hidden`
//! re-checks the partition per world, so a corrupted view can never
//! produce a silently wrong world model.

use koi_core::{Card, CardSet, KoiGameState, Player, StateError, TurnPhase};
use rand::seq::SliceRandom;
use rand::Rng;

/// One compatible world: an opponent-hand hypothesis plus the canonical
/// stock tail that completes the partition, and the concretized state.
#[derive(Debug, Clone)]
pub struct World {
    /// The hypothesized opponent hand.
    pub opponent_hand: CardSet,
    /// The remaining unknown cards as the stock tail (a publicly drawn
    /// card stays pinned at `stock[0]` and is not part of this tail).
    /// Canonical ascending order — semantically dead because the compiled
    /// subgame re-chances every draw.
    pub stock_tail: Vec<Card>,
    /// Posterior mass; normalized over the belief.
    pub weight: f64,
    /// The world as a concrete kernel state.
    pub state: KoiGameState,
}

/// Why a belief could not be built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeliefError {
    /// The public zones do not partition into a consistent hidden pool:
    /// counts mismatch, a pinned draw is not hidden, or a public card is
    /// also "unseen". Fail-closed — the observation is corrupted.
    IncompatibleObservation,
    /// A world assignment violated kernel invariants.
    InconsistentState,
}

impl std::fmt::Display for BeliefError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IncompatibleObservation => {
                f.write_str("the public zones cannot partition into a consistent hidden pool")
            }
            Self::InconsistentState => f.write_str("a world assignment violated kernel invariants"),
        }
    }
}

impl std::error::Error for BeliefError {}

impl From<StateError> for BeliefError {
    fn from(_: StateError) -> Self {
        Self::InconsistentState
    }
}

/// Builds the belief over opponent hands at `state` from `observer`'s
/// perspective. Compatible hands are enumerated when `C(unseen, h)` fits
/// `max_worlds` and sampled uniformly without replacement otherwise.
///
/// `universe` is the domain's card set — the variant's `cards` field
/// (`CardSet::ALL` on the canonical game) — so the partition check also
/// holds on reduced-variant resolves. The zone check is explicit and
/// fail-closed before any world is built:
/// `unseen = universe \ (own ∪ field ∪ piles)` must have size exactly
/// `opp_hand_count + stock_tail` (a pinned drawn card counts as stock).
/// A view violating that cannot come from a real deal.
pub fn build_belief(
    state: &KoiGameState,
    observer: Player,
    universe: CardSet,
    max_worlds: usize,
    rng: &mut impl Rng,
) -> Result<Vec<World>, BeliefError> {
    let opponent = observer.opponent();

    // Fail-closed zone validation: the four public zones must be pairwise
    // disjoint — a card appearing in two of them can never come from a real
    // deal, and the count check below cannot see it (the union still counts
    // the duplicated card once).
    let public = [
        state.hands[observer.index()],
        state.field,
        state.captured[0],
        state.captured[1],
    ];
    for (i, zone) in public.iter().enumerate() {
        for other in &public[i + 1..] {
            if !zone.intersection(*other).is_empty() {
                return Err(BeliefError::IncompatibleObservation);
            }
        }
    }
    let public_union = public.iter().fold(CardSet::EMPTY, |acc, zone| acc.union(*zone));
    if !public_union.is_subset(universe) {
        // A public card outside the domain's universe — forged zones.
        return Err(BeliefError::IncompatibleObservation);
    }
    let unseen = universe.difference(public_union);

    // The hidden pool must partition `unseen` exactly: opponent placeholders
    // plus stock, no duplicates inside the stock, no overlap between them.
    let opp_count = state.hands[opponent.index()].count() as usize;
    let stock_len = state.stock.len();
    let mut stock_set = CardSet::EMPTY;
    for card in state.stock.iter() {
        if stock_set.contains(*card) {
            return Err(BeliefError::IncompatibleObservation);
        }
        stock_set = stock_set.insert(*card);
    }
    if !state.hands[opponent.index()].intersection(stock_set).is_empty()
        || state.hands[opponent.index()].union(stock_set) != unseen
        || unseen.count() as usize != opp_count + stock_len
        || max_worlds == 0
    {
        return Err(BeliefError::IncompatibleObservation);
    }
    // A publicly drawn card is pinned at stock[0] and leaves the free pool.
    let pinned = match state.phase {
        TurnPhase::AwaitingStockResolution { drawn } => {
            if state.stock.first() != Some(&drawn) || !unseen.contains(drawn) {
                return Err(BeliefError::IncompatibleObservation);
            }
            Some(drawn)
        }
        _ => None,
    };
    let free_count = opp_count + (stock_len - usize::from(pinned.is_some()));
    let mut pool: Vec<Card> = unseen.into_iter().collect();
    if let Some(drawn) = pinned {
        pool.retain(|card| *card != drawn);
    }
    if pool.len() != free_count || opp_count > pool.len() {
        return Err(BeliefError::IncompatibleObservation);
    }

    let hand_support = choose(pool.len(), opp_count);
    let make_world = |hand_cards: &[Card], weight: f64| -> Result<World, BeliefError> {
        let mut hand_mask = CardSet::EMPTY;
        for card in hand_cards {
            hand_mask = hand_mask.insert(*card);
        }
        let mut tail: Vec<Card> = Vec::with_capacity(pool.len() - opp_count + usize::from(pinned.is_some()));
        if let Some(drawn) = pinned {
            tail.push(drawn);
        }
        tail.extend(pool.iter().copied().filter(|card| !hand_mask.contains(*card)));
        let state = state.rebuild_hidden_universe(opponent, hand_mask, &tail, universe)?;
        Ok(World {
            opponent_hand: hand_mask,
            stock_tail: tail,
            weight,
            state,
        })
    };

    let mut worlds = Vec::new();
    if hand_support <= max_worlds {
        let weight = 1.0 / hand_support as f64;
        for hand in combinations(&pool, opp_count) {
            worlds.push(make_world(&hand, weight)?);
        }
    } else {
        // Distinct hands without replacement — duplicate hypotheses would
        // spend two resolver budgets on one continuation.
        let mut seen = rustc_hash::FxHashSet::default();
        let attempts = 8 * max_worlds + 64;
        for _ in 0..attempts {
            if worlds.len() == max_worlds {
                break;
            }
            let mut shuffled = pool.clone();
            shuffled.shuffle(rng);
            let hand_mask = shuffled[..opp_count]
                .iter()
                .fold(CardSet::EMPTY, |mask, card| mask.insert(*card));
            if seen.insert(hand_mask) {
                worlds.push(make_world(&shuffled[..opp_count], 0.0)?);
            }
        }
        if worlds.is_empty() {
            return Err(BeliefError::IncompatibleObservation);
        }
        let weight = 1.0 / worlds.len() as f64;
        for world in &mut worlds {
            world.weight = weight;
        }
    }
    Ok(worlds)
}

fn choose(n: usize, k: usize) -> usize {
    if k > n {
        return 0;
    }
    let k = k.min(n - k);
    let mut count: usize = 1;
    for i in 0..k {
        match count.checked_mul(n - i) {
            Some(product) => count = product / (i + 1),
            None => return usize::MAX,
        }
    }
    count
}

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

#[cfg(test)]
mod tests {
    use koi_core::{deal_from_seed, Ruleset};
    use rand::rngs::SmallRng;
    use rand::SeedableRng;

    use super::*;

    fn base_state() -> KoiGameState {
        for seed in 0..u64::MAX {
            let (state, anomaly) = KoiGameState::new_deal(deal_from_seed(seed), Ruleset::nintendo());
            if anomaly.is_none() {
                return state;
            }
        }
        unreachable!()
    }

    #[test]
    fn late_round_belief_enumerates_exact_hand_support() {
        // Drive the round near its end so the hand support is enumerable.
        let mut state = base_state();
        let observer = state.active;
        while !state.is_ended() && state.hands[observer.opponent().index()].count() > 2 {
            let action = state.legal_actions()[0];
            state = state.apply_action(action).unwrap();
        }
        let unseen = crate::determinization::unseen_mask(&state, observer);
        let h = state.hands[observer.opponent().index()].count() as usize;
        let support = choose(unseen.count() as usize, h);
        let worlds = build_belief(
            &state,
            observer,
            CardSet::ALL,
            support.max(1) + 4,
            &mut SmallRng::seed_from_u64(1),
        )
        .unwrap();
        assert_eq!(worlds.len(), support, "enumeration covers every compatible hand");
        let total: f64 = worlds.iter().map(|w| w.weight).sum();
        assert!((total - 1.0).abs() < 1e-9);
        let mut hands = std::collections::HashSet::new();
        for world in &worlds {
            assert!(world.opponent_hand.is_subset(unseen));
            assert!(hands.insert(world.opponent_hand), "duplicate hand in support");
        }
    }

    #[test]
    fn sampled_belief_normalizes_and_dedupes() {
        let state = base_state();
        let observer = state.active;
        let worlds = build_belief(&state, observer, CardSet::ALL, 24, &mut SmallRng::seed_from_u64(2)).unwrap();
        assert_eq!(worlds.len(), 24);
        let total: f64 = worlds.iter().map(|w| w.weight).sum();
        assert!((total - 1.0).abs() < 1e-9);
        let mut hands = std::collections::HashSet::new();
        for world in &worlds {
            assert!(hands.insert(world.opponent_hand), "sampled worlds must be distinct");
            // The concretized state preserves the public zones.
            assert_eq!(world.state.hands[observer.index()], state.hands[observer.index()]);
            assert_eq!(world.state.field, state.field);
            assert_eq!(world.state.captured, state.captured);
        }
    }

    #[test]
    fn corrupted_partitions_fail_closed() {
        let mut state = base_state();
        let observer = state.active;
        // A hand card also in the field: the partition cannot be consistent.
        let card = state.hands[observer.index()].into_iter().next().unwrap();
        state.field = state.field.insert(card);
        assert!(matches!(
            build_belief(&state, observer, CardSet::ALL, 16, &mut SmallRng::seed_from_u64(3)),
            Err(BeliefError::IncompatibleObservation)
        ));
    }
}
