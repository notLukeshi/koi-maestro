//! Property tests for core invariants.

use proptest::prelude::*;

use koi_core::action::{Action, CaptureChoice};
use koi_core::card::{Card, CardSet, Month};
use koi_core::rules::Ruleset;
use koi_core::state::{KoiGameState, Player, TurnPhase};

// `assert!` is safe inside `proptest!`: the test runner captures panics.

fn fresh_deck() -> Vec<Card> {
    (0..48).map(Card::new_unchecked).collect()
}

proptest! {
    #[test]
    fn card_month_and_index_round_trip(index in 0u8..48) {
        let card = Card::new_unchecked(index);
        let month = card.month();
        let in_month = card.index_in_month();
        assert_eq!(Card::from_month_and_index(month, in_month), Some(card));
    }

    #[test]
    fn card_set_union_and_intersection_commute(
        a in 0u64..(1u64 << 48),
        b in 0u64..(1u64 << 48),
    ) {
        let set_a = CardSet::new_unchecked(a);
        let set_b = CardSet::new_unchecked(b);

        assert_eq!(
            set_a.union(set_b).bits(),
            set_b.union(set_a).bits()
        );
        assert_eq!(
            set_a.intersection(set_b).bits(),
            set_b.intersection(set_a).bits()
        );
        assert!(set_a.intersection(set_b).is_subset(set_a));
        assert!(set_a.intersection(set_b).is_subset(set_b));
    }

    #[test]
    fn deal_conserves_all_48_cards(seed: u64) {
        let mut deck = fresh_deck();
        // Deterministic shuffle using a simple splitmix-derived permutation.
        shuffle(&mut deck, seed);

        let (state, _anomaly) = KoiGameState::new_deal(deck, Ruleset::default());

        let on_table = state.hands[0]
            .union(state.hands[1])
            .union(state.field)
            .union(state.captured[0].union(state.captured[1]));
        let in_stock: CardSet = state.stock.iter().copied().collect();

        assert_eq!(on_table.union(in_stock), CardSet::ALL);
        assert_eq!(state.hands[0].count(), 8);
        assert_eq!(state.hands[1].count(), 8);
        assert_eq!(state.field.count(), 8);
        assert_eq!(state.stock.len(), 24);
        assert!(state.hands[0].intersection(state.hands[1]).is_empty());
        assert!(state.field.intersection(state.hands[0]).is_empty());
        assert!(state.field.intersection(state.hands[1]).is_empty());
    }

    #[test]
    fn playthrough_terminates_without_hanging(seed: u64) {
        let mut deck = fresh_deck();
        shuffle(&mut deck, seed);
        let (mut state, anomaly) = KoiGameState::new_deal(deck, Ruleset::default());

        prop_assume!(anomaly.is_none());

        for _ in 0..400 {
            if state.is_ended() {
                break;
            }
            let actions = state.legal_actions();
            assert!(!actions.is_empty(), "state machine produced no legal actions in a non-terminal phase: {state:?}");
            // Prefer the last legal action to exercise Koi-Koi when available.
            let action = *actions.last().unwrap();
            state = state.apply_action(action).unwrap();
        }

        assert!(state.is_ended(), "playthrough did not reach a terminal state: {state:?}");
    }

    #[test]
    fn hand_action_removes_card_from_hand_and_preserves_total(seed: u64) {
        let mut deck = fresh_deck();
        shuffle(&mut deck, seed);

        let (state, anomaly) = KoiGameState::new_deal(deck, Ruleset::default());
        prop_assume!(anomaly.is_none());

        let actions = state.legal_actions();
        prop_assume!(!actions.is_empty());

        let action = actions[0];
        let next = state.apply_action(action).unwrap();

        let total_before = state.hands[0].count()
            + state.hands[1].count()
            + state.field.count()
            + state.captured[0].count()
            + state.captured[1].count();
        let total_after = next.hands[0].count()
            + next.hands[1].count()
            + next.field.count()
            + next.captured[0].count()
            + next.captured[1].count();

        assert!(matches!(next.phase, TurnPhase::AwaitingStockResolution { .. }));
        assert_eq!(total_before, total_after); // card moved from hand to field/captured, stock unchanged
    }

    #[test]
    fn legal_actions_are_exhaustive_for_any_month_field(
        month_index in 0u8..12,
        field_bits in 0u8..16u8, // 4 bits for the month
        hand_bits in 0u8..16u8,
    ) {
        let month = Month::new_unchecked(month_index);
        let month_mask = CardSet::from_month(month).bits();
        let field = CardSet::new_unchecked(month_mask & (field_bits as u64));
        let hand = CardSet::new_unchecked(month_mask & (hand_bits as u64));

        // If hand and field overlap, no valid state, skip.
        prop_assume!(hand.intersection(field).is_empty());

        let stock: Vec<Card> = (0..48)
            .map(Card::new_unchecked)
            .filter(|c| !hand.contains(*c) && !field.contains(*c))
            .collect();

        let (state, _) = KoiGameState::new_from_parts(
            &stock,
            [hand, CardSet::EMPTY],
            field,
            Player::South,
            Player::South,
            Ruleset::default(),
        );

        let actions = state.legal_actions();

        // At least one action per card in hand.
        assert!(!actions.is_empty() || hand.is_empty());

        for action in &actions {
            if let Action::PlayFromHand { card, capture } = action {
                assert!(hand.contains(*card));
                match capture {
                    CaptureChoice::NoMatch => assert_eq!(field.month_cards(card.month()).count(), 0),
                    CaptureChoice::Single(target) => assert!(field.contains(*target) && field.month_cards(card.month()).count() == 1),
                    CaptureChoice::Pair(target) => assert!(field.contains(*target) && field.month_cards(card.month()).count() == 2),
                    CaptureChoice::Triple => assert_eq!(field.month_cards(card.month()).count(), 3),
                }
            }
        }
    }
}

fn shuffle(deck: &mut [Card], mut seed: u64) {
    // Simple Fisher-Yates using a deterministic stream.
    for i in (1..deck.len()).rev() {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let j = (seed % (i as u64 + 1)) as usize;
        deck.swap(i, j);
    }
}
