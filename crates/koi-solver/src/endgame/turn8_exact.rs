//! Turn-8 exact endgame play: exhaustive world enumeration plus per-world
//! perfect-information minimax, with the root action committed across all
//! worlds of the observer's information set (P2-D6).
//!
//! Semantics: at a position where the observer has at most one decision
//! left — the round's final turn — this is *exact*: the opponent's replies
//! inside each enumerated world are that world's true hidden state, and the
//! observer's choice binds uniformly across the belief. Deeper in the game
//! the same evaluation is the clairvoyant (PIMC-style) relaxation: the
//! observer's later decisions see the world, so it is an optimistic bound —
//! use `turn7_subgame` for a faithful imperfect-information solve there.
//!
//! The enumeration fails closed above `MAX_EXACT_WORLDS`; the minimax
//! traversal fails closed above `MAX_EXACT_NODES`. Both keep adversarial
//! positions from exhausting the worker.

use koi_core::{Action, KoiGameState, Player, StateError};

use crate::determinization;

/// Hard cap on enumerated worlds for an exact call.
pub const MAX_EXACT_WORLDS: usize = 65_536;

/// Hard cap on minimax nodes across all worlds of one call.
pub const MAX_EXACT_NODES: usize = 8_000_000;

/// Why an exact evaluation cannot run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndgameError {
    /// The unknown-card set exceeds the enumeration budget.
    TooManyWorlds { needed_over: usize },
    /// The perfect-info traversal exceeded its node budget.
    NodeBudgetExceeded { budget: usize },
    /// The observer is not the player to move — there is no decision to take.
    NotObserversTurn,
    /// The round is already over.
    Terminal,
    /// A determinized world violated kernel invariants.
    InconsistentState(StateError),
}

impl std::fmt::Display for EndgameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooManyWorlds { needed_over } => {
                write!(
                    f,
                    "the unknown-card set exceeds the {needed_over}-world enumeration budget"
                )
            }
            Self::NodeBudgetExceeded { budget } => {
                write!(f, "the endgame traversal exceeded the {budget}-node budget")
            }
            Self::NotObserversTurn => f.write_str("the observer is not the player to move"),
            Self::Terminal => f.write_str("the round is already over"),
            Self::InconsistentState(e) => write!(f, "a determinized world is inconsistent: {e}"),
        }
    }
}

impl std::error::Error for EndgameError {}

impl From<StateError> for EndgameError {
    fn from(e: StateError) -> Self {
        Self::InconsistentState(e)
    }
}

/// Per-action exact expected margin for the deciding player (`state.active`),
/// uniform over every world consistent with the observer's public view.
///
/// Returns actions in the kernel's legal order with their expected
/// `observer`-margin values. Fails closed — never returns sampled evidence
/// as if it were exhaustive.
pub fn exhaustive_action_values(
    state: &KoiGameState,
    observer: Player,
    max_worlds: usize,
) -> Result<Vec<(Action, f64)>, EndgameError> {
    if state.is_ended() {
        return Err(EndgameError::Terminal);
    }
    if state.active != observer {
        return Err(EndgameError::NotObserversTurn);
    }
    let worlds =
        determinization::generate_exhaustive(state, observer, max_worlds).ok_or(EndgameError::TooManyWorlds {
            needed_over: max_worlds,
        })??;
    let mut budget = Budget {
        remaining: MAX_EXACT_NODES,
    };
    let actions = state.legal_actions();
    let mut out = Vec::with_capacity(actions.len());
    for action in actions.iter().copied() {
        let mut expected = 0.0;
        for world in &worlds {
            let next = world.apply_action(action)?;
            expected += minimax(&next, observer, &mut budget)? / worlds.len() as f64;
        }
        out.push((action, expected));
    }
    Ok(out)
}

/// The best root action by expected margin — the argmax of
/// [`exhaustive_action_values`], ties broken by canonical action order.
pub fn best_action(state: &KoiGameState, observer: Player, max_worlds: usize) -> Result<Action, EndgameError> {
    let values = exhaustive_action_values(state, observer, max_worlds)?;
    values
        .into_iter()
        .max_by(|(a, va), (b, vb)| {
            va.partial_cmp(vb)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(b.action_key().cmp(&a.action_key()))
        })
        .map(|(action, _)| action)
        .ok_or(EndgameError::Terminal)
}

struct Budget {
    remaining: usize,
}

/// Perfect-information minimax within one world: every zone is known, so
/// stock draws are deterministic — the value is the active player's max or
/// the opponent's min of the `observer` margin.
fn minimax(state: &KoiGameState, observer: Player, budget: &mut Budget) -> Result<f64, EndgameError> {
    if state.is_ended() {
        return Ok(state.leg_margin(observer) as f64);
    }
    if budget.remaining == 0 {
        return Err(EndgameError::NodeBudgetExceeded {
            budget: MAX_EXACT_NODES,
        });
    }
    budget.remaining -= 1;

    let actions = state.legal_actions();
    if state.active == observer {
        let mut best = f64::NEG_INFINITY;
        for action in actions.iter().copied() {
            best = best.max(minimax(&state.apply_action(action)?, observer, budget)?);
        }
        Ok(best)
    } else {
        let mut best = f64::INFINITY;
        for action in actions.iter().copied() {
            best = best.min(minimax(&state.apply_action(action)?, observer, budget)?);
        }
        Ok(best)
    }
}

#[cfg(test)]
mod tests {
    use koi_core::{Card, CardSet, PublicView, Ruleset, TurnPhase};

    use super::*;

    /// A consistent late-round position: two hidden cards only (one opponent
    /// hand slot + one stock card), everything else public. Determinization
    /// yields exactly C(2,1)·1! = 2 worlds; the unknowns are chosen so no
    /// remaining play can capture anything.
    ///
    /// `south_hand`/`field`/`piles` vary per test; `turn` is 15 so the round
    /// ends right after South's play (or earlier via the null-round path).
    fn two_world_state(
        south_hand: CardSet,
        field: CardSet,
        captured: [CardSet; 2],
        last_yaku_score: [u32; 2],
    ) -> KoiGameState {
        let unknown = CardSet::from_card(Card::new_unchecked(20)) // Jun tane
            .insert(Card::new_unchecked(24)); // Jul tane
        let public = south_hand.union(field).union(captured[0]).union(captured[1]);
        // The two unknowns must be exactly the cards outside the public zones.
        assert_eq!(public.union(unknown), CardSet::ALL);
        assert!(public.intersection(unknown).is_empty());
        let view = PublicView {
            observer: Player::South,
            rules: Ruleset::nintendo(),
            dealer: Player::South,
            active: Player::South,
            turn: 15,
            round: 0,
            score: [0, 0],
            phase: TurnPhase::AwaitingHandAction,
            own_hand: south_hand,
            opponent_hand_count: 1,
            field,
            captured,
            stock_count: 1,
            koi_koi_caller: None,
            koi_koi_calls: [0, 0],
            last_yaku_score,
        };
        KoiGameState::from_public_view(&view).unwrap()
    }

    /// Any 44-card split for the captured piles (piles are public — the same
    /// in every world, so their content only shifts the margin constant).
    fn fill_captured(used: CardSet) -> [CardSet; 2] {
        let mut piles = [CardSet::EMPTY, CardSet::EMPTY];
        for card in CardSet::ALL.difference(used) {
            if piles[0].count() < 22 {
                piles[0] = piles[0].insert(card);
            } else {
                piles[1] = piles[1].insert(card);
            }
        }
        piles
    }

    /// One forced line: South holds a single card that cannot match the
    /// field; every world plays out identically (draws match nothing), so
    /// the expected margin equals the margin of any replayed world.
    #[test]
    fn forced_line_value_matches_kernel_margin() {
        let south_hand = CardSet::from_card(Card::new_unchecked(0)); // Jan
        let field = CardSet::from_card(Card::new_unchecked(44)); // Dec
        let used = south_hand
            .union(field)
            .insert(Card::new_unchecked(20))
            .insert(Card::new_unchecked(24));
        let state = two_world_state(south_hand, field, fill_captured(used), [0, 0]);

        let values = exhaustive_action_values(&state, Player::South, MAX_EXACT_WORLDS).unwrap();
        assert_eq!(values.len(), 1, "South's lone card has one legal action");

        // Independent check: replay the forced line in each of the two
        // worlds and take the mean of the kernel's leg margin.
        let worlds = crate::determinization::generate_exhaustive(&state, Player::South, 16)
            .unwrap()
            .unwrap();
        assert_eq!(worlds.len(), 2);
        let mean: f64 = worlds
            .iter()
            .map(|world| {
                let mut s = *world;
                while !s.is_ended() {
                    let actions = s.legal_actions();
                    assert_eq!(actions.len(), 1, "the fixture must be fully forced");
                    s = s.apply_action(actions[0]).unwrap();
                }
                s.leg_margin(Player::South) as f64
            })
            .sum::<f64>()
            / 2.0;
        assert_eq!(values[0].1, mean);
    }

    /// A captured yaku must dominate: South can play August's card to take
    /// the Moon (28) into a pile already holding the Sake Cup (32) —
    /// Tsukimi — or discard April's card. The capture must strictly win.
    #[test]
    fn a_yaku_capture_is_valued_positively() {
        let south_hand = CardSet::from_card(Card::new_unchecked(29)) // Aug
            .insert(Card::new_unchecked(12)); // Apr
        let field = CardSet::from_card(Card::new_unchecked(28)); // Aug Moon
                                                                 // South's pile holds the Sake Cup (32); the rest is filler dealt so
                                                                 // the partition is exact. Piles are public, so their content shifts
                                                                 // the margin constant identically in every world.
        let taken = south_hand
            .union(field)
            .insert(Card::new_unchecked(20))
            .insert(Card::new_unchecked(24));
        let mut pile_south = CardSet::from_card(Card::new_unchecked(32));
        for card in CardSet::ALL.difference(taken) {
            if pile_south.count() >= 22 {
                break;
            }
            pile_south = pile_south.insert(card);
        }
        let pile_north = CardSet::ALL.difference(taken).difference(pile_south);
        // `last_yaku_score` set to the pile's current base so a no-capture
        // line does not re-trigger a stop decision.
        let base = koi_core::yaku::score_yaku(pile_south, &Ruleset::nintendo());
        let state = two_world_state(south_hand, field, [pile_south, pile_north], [base, 0]);

        let values = exhaustive_action_values(&state, Player::South, MAX_EXACT_WORLDS).unwrap();
        assert_eq!(values.len(), 2);
        let capture = Action::PlayFromHand {
            card: Card::new_unchecked(29),
            capture: koi_core::CaptureChoice::Single(Card::new_unchecked(28)),
        };
        let discard = Action::PlayFromHand {
            card: Card::new_unchecked(12),
            capture: koi_core::CaptureChoice::NoMatch,
        };
        let v_capture = values.iter().find(|(a, _)| *a == capture).unwrap().1;
        let v_discard = values.iter().find(|(a, _)| *a == discard).unwrap().1;
        // Capturing the Moon banks Tsukimi on top of the pile's base score;
        // the swing is exactly 2 * apply_multipliers(5).
        assert!(
            v_capture > v_discard,
            "capture {v_capture} must beat discard {v_discard}"
        );
        assert_eq!(best_action(&state, Player::South, MAX_EXACT_WORLDS).unwrap(), capture);
    }

    /// Fail-closed: a mid-game 48-card position cannot enumerate.
    #[test]
    fn mid_game_positions_refuse_exhaustion() {
        // Find a non-anomalous full deal.
        let mut found = None;
        for seed in 0..256u64 {
            let (state, anomaly) = KoiGameState::new_deal(koi_core::deal_from_seed(seed), Ruleset::nintendo());
            if anomaly.is_none() && !state.is_ended() {
                found = Some(state);
                break;
            }
        }
        let Some(state) = found else { return };
        assert!(matches!(
            exhaustive_action_values(&state, state.active, 64),
            Err(EndgameError::TooManyWorlds { .. })
        ));
    }

    /// The observer/actor mismatch and terminal guards fail closed.
    #[test]
    fn guards_reject_non_decision_calls() {
        let south_hand = CardSet::from_card(Card::new_unchecked(0));
        let field = CardSet::from_card(Card::new_unchecked(44));
        let used = south_hand
            .union(field)
            .insert(Card::new_unchecked(20))
            .insert(Card::new_unchecked(24));
        let state = two_world_state(south_hand, field, fill_captured(used), [0, 0]);
        assert_eq!(
            exhaustive_action_values(&state, Player::North, 16),
            Err(EndgameError::NotObserversTurn)
        );
        let mut ended = state;
        ended.phase = TurnPhase::Ended;
        assert_eq!(
            exhaustive_action_values(&ended, Player::South, 16),
            Err(EndgameError::Terminal)
        );
    }
}
