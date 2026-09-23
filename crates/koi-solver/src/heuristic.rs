//! Greedy Yaku-aware heuristic: the deterministic baseline solver.
//!
//! The policy is deliberately simple and fully documented so benchmark
//! margins against it are interpretable:
//!
//! - **Captures** score `60 × Δown-yaku + card material + 25 × Δopp-yaku
//!   denied`, where Δown-yaku is the change in `score_yaku` from taking the
//!   capture and Δopp-yaku is the score the opponent could have formed with
//!   exactly those cards (a proxy for denial value).
//! - **No-match plays** pay the played card's material value: the card lands
//!   on the field where the opponent may later capture it.
//! - **Stop decisions** bank `100 × base_score`; continuing adds a gamble
//!   bonus that shrinks as the banked score and the opponent's accumulated
//!   Yaku base grow, and doubles the bank when the opponent has already
//!   called Koi-Koi (the winner's score doubles when the loser called).
//!
//! Ties break on `Action::action_key` so the policy is total, deterministic,
//! and platform-stable.

use koi_core::yaku::{rank, score_yaku, CardRank};
use koi_core::{Action, CaptureChoice, Card, CardSet, KoiGameState, TurnPhase};

/// Weight of a single Yaku point relative to raw card material.
const YAKU_WEIGHT: i64 = 60;
/// Weight of Yaku points the capture denies the opponent.
const DENIAL_WEIGHT: i64 = 25;
/// Scale of a banked round point on the same axis as `YAKU_WEIGHT` deltas.
const BANK_WEIGHT: i64 = 100;
/// Fixed bonus for continuing the round — the expected upside of drawing
/// into more captures before either hand runs out.
const GAMBLE_BONUS: i64 = 250;
/// Per-point discount on the gamble as the banked score grows.
const BANK_DISCOUNT: i64 = 60;
/// Per-point discount for the opponent's accumulated Yaku base — a strong
/// opponent pile makes continuing riskier.
const THREAT_DISCOUNT: i64 = 50;
/// Extra risk taken when we already called Koi-Koi: if we then lose, the
/// opponent's score doubles under rulesets with `opponent_koi_koi_doubles`.
const SELF_CALL_RISK: i64 = 120;

/// The material value of a card: what it contributes beyond Yaku combos.
fn card_value(card: Card) -> i64 {
    match rank(card) {
        CardRank::Hikari => 20,
        CardRank::SakeCup => 14,
        CardRank::Tane => 10,
        CardRank::Tanzaku(_) => 6,
        CardRank::Kasu => 1,
    }
}

fn material(set: CardSet) -> i64 {
    set.into_iter().map(card_value).sum()
}

/// The field cards a capture choice takes for a card of `month`.
fn matched_cards(state: &KoiGameState, month: koi_core::Month, capture: CaptureChoice) -> CardSet {
    match capture {
        CaptureChoice::NoMatch => CardSet::EMPTY,
        CaptureChoice::Single(card) | CaptureChoice::Pair(card) => CardSet::from_card(card),
        CaptureChoice::Triple => state.field.month_cards(month),
    }
}

/// Scores a capture-style action (`PlayFromHand` / `ResolveStock`).
fn evaluate_capture(state: &KoiGameState, played: Card, capture: CaptureChoice) -> i64 {
    let me = state.active.index();
    let opp = state.active.opponent().index();
    let taken = matched_cards(state, played.month(), capture);
    let gained = if capture == CaptureChoice::NoMatch {
        CardSet::EMPTY
    } else {
        taken.insert(played)
    };

    let mine = state.captured[me];
    let theirs = state.captured[opp];
    let own_delta = score_yaku(mine.union(gained), &state.rules) as i64 - score_yaku(mine, &state.rules) as i64;
    // The Yaku the opponent could have formed with these exact cards —
    // taking them denies that value.
    let denied = score_yaku(theirs.union(gained), &state.rules) as i64 - score_yaku(theirs, &state.rules) as i64;

    let mut value = YAKU_WEIGHT * own_delta + material(gained) + DENIAL_WEIGHT * denied;
    if capture == CaptureChoice::NoMatch {
        // The played card is deposited on the field, available to the
        // opponent's future captures.
        value -= card_value(played);
    }
    value
}

/// Scores the stop/continue decision.
fn evaluate_stop(state: &KoiGameState, base_score: u32, action: Action) -> i64 {
    let me = state.active.index();
    let opp = state.active.opponent().index();
    let mut bank = BANK_WEIGHT * base_score as i64;
    if state.koi_koi_caller == Some(state.active.opponent()) {
        // The opponent called: our banked score doubles under rulesets with
        // `opponent_koi_koi_doubles` — banking is strictly more attractive.
        bank *= 2;
    }
    match action {
        Action::Shobu => bank,
        Action::KoiKoi => {
            let threat = score_yaku(state.captured[opp], &state.rules) as i64;
            let mut value = bank + GAMBLE_BONUS - BANK_DISCOUNT * base_score as i64 - THREAT_DISCOUNT * threat;
            if state.koi_koi_caller == Some(state.active) {
                value -= SELF_CALL_RISK;
            }
            let _ = me;
            value
        }
        _ => i64::MIN / 2,
    }
}

/// Scores a single action; unreachable variants score at `i64::MIN/2` so a
/// malformed call never wins the argmax.
fn evaluate_action(state: &KoiGameState, action: Action) -> i64 {
    match (state.phase, action) {
        (TurnPhase::AwaitingHandAction, Action::PlayFromHand { card, capture })
        | (TurnPhase::AwaitingStockResolution { drawn: card }, Action::ResolveStock { capture }) => {
            evaluate_capture(state, card, capture)
        }
        (TurnPhase::AwaitingStopDecision { base_score }, action @ (Action::KoiKoi | Action::Shobu)) => {
            evaluate_stop(state, base_score, action)
        }
        _ => i64::MIN / 2,
    }
}

/// The greedy best legal action, or `None` when the state is terminal.
/// Fully deterministic — identical states always return identical actions.
pub fn find_best_action(state: &KoiGameState) -> Option<Action> {
    let mut best: Option<(i64, u64, Action)> = None;
    for action in state.legal_actions() {
        let value = evaluate_action(state, action);
        let key = action.action_key();
        best = match best {
            Some((best_value, best_key, _)) if best_value > value || (best_value == value && best_key <= key) => best,
            _ => Some((value, key, action)),
        };
    }
    best.map(|(_, _, action)| action)
}

#[cfg(test)]
mod tests {
    use koi_core::{deal_from_seed, Player, Ruleset};

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
    fn heuristic_is_deterministic_and_legal() {
        let state = base_state();
        let legal: Vec<Action> = state.legal_actions().into_iter().collect();
        let first = find_best_action(&state).unwrap();
        let second = find_best_action(&state).unwrap();
        assert_eq!(first, second);
        assert!(legal.contains(&first));
    }

    #[test]
    fn heuristic_prefers_hikari_capture_over_kasu_discard() {
        // August Moon (Hikari, index 28) on the field; hand holds the August
        // Tane (29) and a January Kasu (2) with no January on the field.
        let field = CardSet::from_card(Card::new_unchecked(28));
        let south = CardSet::from_card(Card::new_unchecked(29)).insert(Card::new_unchecked(2));
        let north = CardSet::from_card(Card::new_unchecked(45)).insert(Card::new_unchecked(46));
        let stock: Vec<Card> = (0..48_u8)
            .map(Card::new_unchecked)
            .filter(|card| !field.contains(*card) && !south.contains(*card) && !north.contains(*card))
            .collect();
        let (state, _) = KoiGameState::new_from_parts(
            &stock,
            [south, north],
            field,
            Player::South,
            Player::South,
            Ruleset::nintendo(),
        );
        assert_eq!(
            find_best_action(&state),
            Some(Action::PlayFromHand {
                card: Card::new_unchecked(29),
                capture: CaptureChoice::Single(Card::new_unchecked(28)),
            })
        );
    }

    #[test]
    fn heuristic_banks_a_real_score_and_pushes_small_ones() {
        let base = base_state();
        let state = KoiGameState {
            phase: TurnPhase::AwaitingStopDecision { base_score: 7 },
            ..base
        };
        assert_eq!(find_best_action(&state), Some(Action::Shobu));
        let state = KoiGameState {
            phase: TurnPhase::AwaitingStopDecision { base_score: 1 },
            ..base
        };
        assert_eq!(find_best_action(&state), Some(Action::KoiKoi));
    }
}
