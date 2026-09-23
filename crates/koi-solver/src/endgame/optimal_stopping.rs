//! The Shōbu/Koi-Koi decision evaluator (P2-D6, ARCHITECTURE §7).
//!
//! `Shōbu` banks immediately: the round ends with the scorer's margin at
//! `+2 * apply_multipliers(base)` — a known scalar. `Koi-Koi` continues the
//! round; its value is the residual subgame value solved under the honest
//! belief (turn7 machinery). The evaluator returns both branches so callers
//! can inspect the decision surface, not just the argmax.

use koi_core::{Action, KoiGameState, Player, TurnPhase};

use super::turn7_subgame::{solve_subgame, SubgameContext, SubgameError, SubgameSolution};
use super::turn8_exact::EndgameError;

/// Why a stop decision cannot be evaluated.
#[derive(Debug)]
pub enum StopError {
    /// The state is not at a Shōbu/Koi-Koi decision.
    NotAStopDecision,
    /// The deciding player is not the requested observer.
    NotObserversTurn,
    /// The residual solve failed.
    Subgame(SubgameError),
    /// The kernel rejected a stop-branch probe.
    State(koi_core::StateError),
}

impl std::fmt::Display for StopError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotAStopDecision => f.write_str("the state is not at a Shōbu/Koi-Koi decision"),
            Self::NotObserversTurn => f.write_str("the observer is not the deciding player"),
            Self::Subgame(e) => write!(f, "the Koi-Koi residual solve failed: {e}"),
            Self::State(e) => write!(f, "a stop-branch probe was rejected: {e}"),
        }
    }
}

impl std::error::Error for StopError {}

impl From<SubgameError> for StopError {
    fn from(e: SubgameError) -> Self {
        Self::Subgame(e)
    }
}
impl From<EndgameError> for StopError {
    fn from(e: EndgameError) -> Self {
        Self::Subgame(e.into())
    }
}

impl From<koi_core::StateError> for StopError {
    fn from(e: koi_core::StateError) -> Self {
        Self::State(e)
    }
}

/// Both branches of the stop decision in the decider's margin units.
#[derive(Debug, Clone)]
pub struct StopEvaluation {
    /// Value of `Shōbu`: `+2 * apply_multipliers(base_score, …)` — the swing
    /// between banking and the opponent's loss is already in the margin.
    pub shobu: f64,
    /// Value of `Koi-Koi` as a solved residual subgame; `None` when Koi-Koi
    /// is not legal or the residual could not be solved (see
    /// `koi_koi_error` — the caller falls back to a heuristic).
    pub koi_koi: Option<SubgameSolution>,
    /// Why the Koi-Koi branch is absent: `None` when `koi_koi` is present or
    /// Koi-Koi is not a legal action here; otherwise the residual failure —
    /// provenance, not silence.
    pub koi_koi_error: Option<SubgameError>,
    /// The recommended action (argmax of the available branches — Shōbu when
    /// the Koi-Koi residual is unavailable: bank the sure thing).
    pub recommended: Action,
}

/// Evaluates the stop decision at `state` for `observer`.
///
/// `ctx` anchors the residual tree's infoset keys and bounds the solve (see
/// `SubgameContext`). Fails closed on a non-stop phase.
pub fn evaluate_stop(
    state: &KoiGameState,
    observer: Player,
    ctx: &SubgameContext<'_>,
) -> Result<StopEvaluation, StopError> {
    if !matches!(state.phase, TurnPhase::AwaitingStopDecision { .. }) {
        return Err(StopError::NotAStopDecision);
    }
    if state.active != observer {
        return Err(StopError::NotObserversTurn);
    }
    if state.phase == TurnPhase::Ended {
        return Err(StopError::NotAStopDecision);
    }

    // Shōbu branch: the kernel already encodes the multipliers — replay the
    // action on a kernel state and read the resulting leg margin. This keeps
    // the evaluator honest about opp-koi-koi doubling, 7+ doubling, etc.
    let shobu = state.apply_action(Action::Shobu)?;
    let shobu_value = shobu.leg_margin(observer) as f64;

    // Koi-Koi branch — only if the kernel offers it here (both hands
    // non-empty and enough stock for the continuation, per `can_call_koi_koi`).
    let koi_legal = state
        .legal_actions()
        .iter()
        .any(|action| matches!(action, Action::KoiKoi));
    let (koi_koi, koi_koi_error) = if !koi_legal {
        (None, None)
    } else {
        let continued = state.apply_action(Action::KoiKoi)?;
        if continued.is_ended() {
            // A koikoi that immediately ends the round: the realized
            // margin is the value — compare it against shobu directly.
            // Unreachable today (can_call_koi_koi requires both hands
            // non-empty + stock ≥ 2) but the comparison is honest if
            // the gate ever relaxes.
            let solution = SubgameSolution {
                value: continued.leg_margin(observer) as f64,
                profile: Vec::new(),
                root_infoset: 0,
                method: super::turn7_subgame::SubgameMethod::CfrPlus { iterations: 0 },
                tree_nodes: 0,
                worlds: 0,
            };
            (Some(solution), None)
        } else {
            match solve_subgame(&continued, observer, ctx) {
                Ok(solution) => (Some(solution), None),
                Err(error) => (None, Some(error)),
            }
        }
    };

    let koi_value = koi_koi.as_ref().map(|s| s.value);
    let recommended = match koi_value {
        Some(v) if v > shobu_value => Action::KoiKoi,
        _ => Action::Shobu,
    };
    Ok(StopEvaluation {
        shobu: shobu_value,
        koi_koi,
        koi_koi_error,
        recommended,
    })
}

#[cfg(test)]
mod tests {
    use koi_core::{Card, CardSet, PublicView, Ruleset};

    use crate::efg::KoiVariant;

    use super::*;

    /// A South stop decision with Koi-Koi *legal* but a tiny residual:
    /// South holds {12} (Apr), North one hidden card, field {44} (Dec), and
    /// three hidden cards total — unknowns {20,24,36} (Jun/Jul/Oct). The
    /// pile carries a completed Hanami plus inert filler; the phase's
    /// `base_score` and `last_yaku_score` are set to the pile's true value.
    fn stop_state() -> (KoiGameState, u32) {
        let rules = Ruleset::nintendo();
        let south_hand = CardSet::from_card(Card::new_unchecked(12));
        let field = CardSet::from_card(Card::new_unchecked(44));
        let hidden = CardSet::from_card(Card::new_unchecked(20))
            .insert(Card::new_unchecked(24))
            .insert(Card::new_unchecked(36));
        let taken = south_hand.union(field).union(hidden);
        // Hanami in South's pile; the remaining 41 cards split publicly.
        let mut pile_south = CardSet::from_card(Card::new_unchecked(8)).insert(Card::new_unchecked(32));
        for card in CardSet::ALL.difference(taken) {
            if pile_south.count() >= 21 {
                break;
            }
            pile_south = pile_south.insert(card);
        }
        let pile_north = CardSet::ALL.difference(taken).difference(pile_south);
        let base = koi_core::yaku::score_yaku(pile_south, &rules);
        let view = PublicView {
            observer: Player::South,
            rules,
            dealer: Player::South,
            active: Player::South,
            turn: 14,
            round: 0,
            score: [0, 0],
            phase: TurnPhase::AwaitingStopDecision { base_score: base },
            own_hand: south_hand,
            opponent_hand_count: 1,
            field,
            captured: [pile_south, pile_north],
            stock_count: 2,
            koi_koi_caller: None,
            koi_koi_calls: [0, 0],
            last_yaku_score: [base, 0],
        };
        (KoiGameState::from_public_view(&view).unwrap(), base)
    }

    /// Banking must read the multiplied margin straight out of the kernel —
    /// and with both hands live and two stock cards, the Koi-Koi residual is
    /// solved (6 worlds) rather than skipped.
    #[test]
    fn shobu_and_koi_koi_branches_both_report() {
        let (state, base) = stop_state();
        let ctx = SubgameContext {
            variant: &KoiVariant::NANO_6,
            initial_field: state.field,
            max_nodes: 1_000_000,
            cfr_iterations: 200,
            ..Default::default()
        };
        let evaluation = evaluate_stop(&state, Player::South, &ctx).unwrap();
        // Banking must equal the kernel's margin after Shōbu.
        let probe = state.apply_action(Action::Shobu).unwrap();
        assert_eq!(evaluation.shobu, probe.leg_margin(Player::South) as f64);
        assert_eq!(
            evaluation.shobu,
            2.0 * Ruleset::nintendo().apply_multipliers(base, false) as f64
        );
        assert!(evaluation.shobu > 0.0);
        // The Koi-Koi branch ran a real residual solve over 6 worlds.
        let koi = evaluation.koi_koi.expect("Koi-Koi is legal and the residual is small");
        assert!(koi.value.is_finite());
        assert_eq!(koi.worlds, 6);
    }

    /// When Koi-Koi is not legal (empty stock tail), only the Shōbu branch
    /// reports — and there is no fabricated residual.
    #[test]
    fn koikoi_illegal_skips_the_residual() {
        let (mut view_state, _) = stop_state();
        // Force the stock empty: the kernel then refuses Koi-Koi.
        view_state.stock = koi_core::Stock::from_slice(&[]).unwrap();
        let expected = {
            let probe = view_state.apply_action(Action::Shobu).unwrap();
            probe.leg_margin(Player::South) as f64
        };
        let ctx = SubgameContext {
            variant: &KoiVariant::NANO_6,
            initial_field: view_state.field,
            max_nodes: 1_000_000,
            cfr_iterations: 50,
            ..Default::default()
        };
        let evaluation = evaluate_stop(&view_state, Player::South, &ctx).unwrap();
        assert!(evaluation.koi_koi.is_none());
        assert!(evaluation.koi_koi_error.is_none());
        assert_eq!(evaluation.recommended, Action::Shobu);
        assert_eq!(evaluation.shobu, expected);
    }

    /// Non-stop phases reject the evaluator.
    #[test]
    fn non_stop_phases_fail_closed() {
        let stock = [Card::new_unchecked(40)];
        let (state, _) = KoiGameState::new_from_parts(
            &stock,
            [
                CardSet::from_card(Card::new_unchecked(0)),
                CardSet::from_card(Card::new_unchecked(4)),
            ],
            CardSet::from_card(Card::new_unchecked(44)),
            Player::South,
            Player::South,
            Ruleset::nintendo(),
        );
        let ctx = SubgameContext {
            variant: &KoiVariant::NANO_6,
            initial_field: state.field,
            max_worlds: 64,
            max_nodes: 1_000_000,
            cfr_iterations: 1,
            ..Default::default()
        };
        assert!(matches!(
            evaluate_stop(&state, Player::South, &ctx),
            Err(StopError::NotAStopDecision)
        ));
    }
}
