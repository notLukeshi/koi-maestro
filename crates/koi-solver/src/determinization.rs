//! Hidden-state determinization: sample (or, when the count fits the cap,
//! enumerate) the worlds consistent with an observer's public view.
//!
//! From observer `P` the unknown zones are exactly the opponent's hand and
//! the stock order — everything else (own hand, field, captured piles, the
//! face-up drawn card during a stock resolution) is public. A
//! determinization assigns `|opponent_hand|` of the unknown cards to the
//! opponent and orders the remainder as the stock tail, keeping a publicly
//! drawn card pinned at `stock[0]`.
//!
//! The world count is `C(n, h) * (n-h)! = n!/h!` for `n` unknown cards and
//! an `h`-card opponent hand. It is enumerable only late in a round; the
//! bound is checked with early-exit arithmetic so a 40-card unknown set
//! never materializes a count.

use koi_core::{Card, CardSet, KoiGameState, Player, StateError, TurnPhase};
use rand::seq::SliceRandom;
use rand::Rng;

/// The cards the observer cannot place: everything outside their own hand,
/// the field, and the captured piles.
pub(crate) fn unseen_mask(state: &KoiGameState, observer: Player) -> CardSet {
    let known = state.hands[observer.index()]
        .union(state.field)
        .union(state.captured[0])
        .union(state.captured[1]);
    CardSet::ALL.difference(known)
}

/// Unknown cards as a vector — the order callers permute.
fn unseen_cards(state: &KoiGameState, observer: Player) -> Vec<Card> {
    let unseen = unseen_mask(state, observer);
    let mut cards: Vec<Card> = unseen.into_iter().collect();
    // A publicly drawn card is not part of the shuffled unknown set: it is
    // pinned at the head of the rebuilt stock.
    if let TurnPhase::AwaitingStockResolution { drawn } = state.phase {
        cards.retain(|card| *card != drawn);
    }
    cards
}

/// The opponent-hand size the determinization must fill.
fn hidden_hand_count(state: &KoiGameState, observer: Player) -> usize {
    state.hands[observer.opponent().index()].count() as usize
}

/// Builds one world: `hand` to the opponent, `tail` behind any pinned draw.
fn assign_world(
    state: &KoiGameState,
    observer: Player,
    hand: &[Card],
    tail: &[Card],
) -> Result<KoiGameState, StateError> {
    let opponent = observer.opponent();
    let mut opponent_hand = CardSet::EMPTY;
    for card in hand {
        opponent_hand = opponent_hand.insert(*card);
    }
    let mut stock = Vec::with_capacity(tail.len() + 1);
    if let TurnPhase::AwaitingStockResolution { drawn } = state.phase {
        stock.push(drawn);
    }
    stock.extend_from_slice(tail);
    state.rebuild_hidden(opponent, opponent_hand, &stock)
}

/// Exact world count `C(n,h) * (n-h)!`, or `None` once it exceeds `cap`.
fn bounded_world_count(n: usize, h: usize, cap: usize) -> Option<usize> {
    if h > n {
        return None;
    }
    let mut count: usize = 1;
    // C(n, h) computed with interleaved division so intermediates stay small.
    let k = h.min(n - h);
    for i in 0..k {
        count = count.checked_mul(n - i)? / (i + 1);
        if count > cap {
            return None;
        }
    }
    // (n - h)! with early exit.
    for i in 2..=(n - h) {
        count = count.checked_mul(i)?;
        if count > cap {
            return None;
        }
    }
    Some(count)
}

/// All `k`-subsets of `0..n` via an index odometer (lexicographic order).
fn combinations(n: usize, k: usize) -> Vec<Vec<usize>> {
    if k > n {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut combo: Vec<usize> = (0..k).collect();
    loop {
        out.push(combo.clone());
        let mut i = k;
        while i > 0 && combo[i - 1] == n - k + (i - 1) {
            i -= 1;
        }
        if i == 0 {
            break;
        }
        combo[i - 1] += 1;
        for j in i..k {
            combo[j] = combo[j - 1] + 1;
        }
    }
    out
}

/// All permutations of `items` via Heap's algorithm.
fn permutations(items: &[Card]) -> Vec<Vec<Card>> {
    let mut out = Vec::new();
    let mut data = items.to_vec();
    let len = data.len();
    heap_permute(&mut data, len, &mut out);
    out
}

fn heap_permute(data: &mut Vec<Card>, k: usize, out: &mut Vec<Vec<Card>>) {
    if k <= 1 {
        out.push(data.clone());
        return;
    }
    heap_permute(data, k - 1, out);
    for i in 0..k - 1 {
        let swap = if k.is_multiple_of(2) { i } else { 0 };
        data.swap(swap, k - 1);
        heap_permute(data, k - 1, out);
    }
}

/// One sampled world consistent with `observer`'s view.
pub(crate) fn sample_determinization(
    state: &KoiGameState,
    observer: Player,
    rng: &mut impl Rng,
) -> Result<KoiGameState, StateError> {
    let mut unknown = unseen_cards(state, observer);
    let hand_count = hidden_hand_count(state, observer);
    unknown.shuffle(rng);
    let (hand, tail) = unknown.split_at(hand_count.min(unknown.len()));
    assign_world(state, observer, hand, tail)
}

/// The number of worlds `generate_determinizations` will produce: the exact
/// count when it fits `max_count`, else `max_count` sampled worlds.
pub(crate) fn planned_determinization_count(state: &KoiGameState, observer: Player, max_count: usize) -> usize {
    let unknown = unseen_cards(state, observer);
    let h = hidden_hand_count(state, observer);
    match bounded_world_count(unknown.len(), h, max_count) {
        Some(exact) => exact.min(max_count),
        None => max_count,
    }
}

/// Every consistent world, or `None` when the count exceeds `max_count`.
/// The exact path of `generate_determinizations` without the sampling
/// fallback — the endgame solvers fail closed rather than return partial
/// evidence (no RNG needed).
pub(crate) fn generate_exhaustive(
    state: &KoiGameState,
    observer: Player,
    max_count: usize,
) -> Option<Result<Vec<KoiGameState>, StateError>> {
    let unknown = unseen_cards(state, observer);
    let hand_count = hidden_hand_count(state, observer);
    let n = unknown.len();
    bounded_world_count(n, hand_count, max_count)?;
    let mut worlds = Vec::new();
    let result = (|| {
        for combo in combinations(n, hand_count) {
            let mut hand = Vec::with_capacity(hand_count);
            let mut rest = Vec::with_capacity(n - hand_count);
            let mut taken = vec![false; n];
            for &index in &combo {
                taken[index] = true;
                hand.push(unknown[index]);
            }
            for (index, card) in unknown.iter().enumerate() {
                if !taken[index] {
                    rest.push(*card);
                }
            }
            for tail in permutations(&rest) {
                worlds.push(assign_world(state, observer, &hand, &tail)?);
            }
        }
        Ok(worlds)
    })();
    Some(result)
}

/// Up to `max_count` determinized worlds. When the exact world count fits
/// the cap the enumeration is exhaustive (every consistent world, deduped
/// by construction); otherwise the worlds are independent shuffled samples.
pub(crate) fn generate_determinizations(
    state: &KoiGameState,
    observer: Player,
    max_count: usize,
    rng: &mut impl Rng,
) -> Result<Vec<KoiGameState>, StateError> {
    let unknown = unseen_cards(state, observer);
    let hand_count = hidden_hand_count(state, observer);
    let n = unknown.len();
    match bounded_world_count(n, hand_count, max_count) {
        Some(count) => {
            // Exhaustive: choose the opponent hand, then order the stock tail.
            let mut worlds = Vec::with_capacity(count);
            for combo in combinations(n, hand_count) {
                let mut hand = Vec::with_capacity(hand_count);
                let mut rest = Vec::with_capacity(n - hand_count);
                let mut taken = vec![false; n];
                for &index in &combo {
                    taken[index] = true;
                    hand.push(unknown[index]);
                }
                for (index, card) in unknown.iter().enumerate() {
                    if !taken[index] {
                        rest.push(*card);
                    }
                }
                for tail in permutations(&rest) {
                    worlds.push(assign_world(state, observer, &hand, &tail)?);
                }
            }
            Ok(worlds)
        }
        None => {
            let mut worlds = Vec::with_capacity(max_count);
            for _ in 0..max_count {
                let mut shuffled = unknown.clone();
                shuffled.shuffle(rng);
                let (hand, tail) = shuffled.split_at(hand_count.min(shuffled.len()));
                worlds.push(assign_world(state, observer, hand, tail)?);
            }
            Ok(worlds)
        }
    }
}

#[cfg(test)]
mod tests {
    use koi_core::{deal_from_seed, Action, CaptureChoice, Ruleset};
    use rand::SeedableRng;

    use super::*;

    fn base_state() -> KoiGameState {
        // Find a seed whose deal carries no anomaly; seeds are cheap.
        for seed in 0..u64::MAX {
            let (state, anomaly) = KoiGameState::new_deal(deal_from_seed(seed), Ruleset::nintendo());
            if anomaly.is_none() {
                return state;
            }
        }
        unreachable!()
    }

    #[test]
    fn unseen_is_exactly_opponent_hand_plus_stock() {
        let state = base_state();
        let observer = state.active;
        let unseen = unseen_mask(&state, observer);
        let hidden = state.stock.len() + state.hands[observer.opponent().index()].count() as usize;
        assert_eq!(unseen.count() as usize, hidden);
        assert!(!unseen.intersects(state.hands[observer.index()]));
        assert!(!unseen.intersects(state.field));
    }

    #[test]
    fn determinization_preserves_public_zones() {
        let state = base_state();
        let observer = state.active;
        let mut rng = rand::rngs::StdRng::seed_from_u64(11);
        for _ in 0..8 {
            let world = sample_determinization(&state, observer, &mut rng).unwrap();
            assert_eq!(world.hands[observer.index()], state.hands[observer.index()]);
            assert_eq!(world.field, state.field);
            assert_eq!(world.captured, state.captured);
            assert_eq!(world.stock.len(), state.stock.len());
            assert_eq!(
                world.hands[observer.opponent().index()].count(),
                state.hands[observer.opponent().index()].count()
            );
            // Conservation: the world's hidden zones are exactly the unseen set.
            let world_hidden =
                world.hands[observer.opponent().index()].union(world.stock.iter().copied().collect::<CardSet>());
            assert_eq!(world_hidden, unseen_mask(&state, observer));
        }
    }

    #[test]
    fn pinned_draw_stays_at_stock_head() {
        let state = base_state();
        let observer = state.active;
        let action = state
            .legal_actions()
            .into_iter()
            .find(|action| {
                matches!(
                    action,
                    Action::PlayFromHand {
                        capture: CaptureChoice::NoMatch,
                        ..
                    }
                )
            })
            .or_else(|| state.legal_actions().into_iter().next())
            .unwrap();
        let state = state.apply_action(action).unwrap();
        assert!(matches!(state.phase, TurnPhase::AwaitingStockResolution { .. }));
        let mut rng = rand::rngs::StdRng::seed_from_u64(17);
        let world = sample_determinization(&state, observer, &mut rng).unwrap();
        assert_eq!(world.stock.first(), state.stock.first());
    }

    #[test]
    fn enumeration_is_exhaustive_for_tiny_unknowns() {
        // 45 public cards → 3 unknowns; opponent hand of 1, stock tail 2:
        // C(3,1) * 2! = 6 worlds, all enumerable under a cap of 16.
        let mut known = CardSet::EMPTY;
        for card in 0..45_u8 {
            known = known.insert(Card::new_unchecked(card));
        }
        let own_hand = CardSet::from_card(Card::new_unchecked(0));
        let field = CardSet::from_card(Card::new_unchecked(4));
        let captured = [
            CardSet::new_unchecked(known.bits() & !own_hand.bits() & !field.bits()),
            CardSet::EMPTY,
        ];
        let view = koi_core::PublicView {
            observer: Player::South,
            rules: Ruleset::nintendo(),
            dealer: Player::South,
            active: Player::South,
            turn: 15,
            round: 0,
            score: [0, 0],
            phase: TurnPhase::AwaitingHandAction,
            own_hand,
            opponent_hand_count: 1,
            field,
            captured,
            stock_count: 2,
            koi_koi_caller: None,
            koi_koi_calls: [0, 0],
            last_yaku_score: [0, 0],
        };
        let state = KoiGameState::from_public_view(&view).unwrap();
        let mut rng = rand::rngs::StdRng::seed_from_u64(19);
        assert_eq!(planned_determinization_count(&state, Player::South, 16), 6);
        let worlds = generate_determinizations(&state, Player::South, 16, &mut rng).unwrap();
        assert_eq!(worlds.len(), 6);
        // Exhaustive enumeration produces six distinct worlds.
        let mut signatures: Vec<_> = worlds
            .iter()
            .map(|world| {
                (
                    world.hands[Player::North.index()].bits(),
                    world.stock.iter().map(|card| card.index()).collect::<Vec<_>>(),
                )
            })
            .collect();
        signatures.sort();
        signatures.dedup();
        assert_eq!(signatures.len(), 6);
    }

    #[test]
    fn sampling_caps_at_max_count() {
        let state = base_state();
        let observer = state.active;
        let mut rng = rand::rngs::StdRng::seed_from_u64(23);
        assert_eq!(planned_determinization_count(&state, observer, 5), 5);
        let worlds = generate_determinizations(&state, observer, 5, &mut rng).unwrap();
        assert_eq!(worlds.len(), 5);
    }

    #[test]
    fn bounded_count_arithmetic() {
        assert_eq!(bounded_world_count(3, 1, 100), Some(6));
        assert_eq!(bounded_world_count(4, 2, 100), Some(12)); // C(4,2)·2! = 12
        assert_eq!(bounded_world_count(4, 2, 5), None);
        assert_eq!(bounded_world_count(48, 8, 4_096), None);
        assert_eq!(bounded_world_count(2, 5, 10), None);
    }
}
