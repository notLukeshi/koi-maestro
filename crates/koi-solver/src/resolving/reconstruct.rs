//! Ledger replay: rebuilds the decision trajectory a public observation
//! attests, producing the per-decision frames the opponent model scores and
//! the `InfoSetKey`-compatible anchors the subgame compiler needs.
//!
//! Richer than the sibling codebase's replay because Koi-Koi's drawn cards are *semi-public
//! during the turn*: a `ResolveStock` ledger entry names the card that was
//! face-up for that resolution, so the replay pins it at the stock head
//! before applying the action — the same lazy hidden-card materialization
//! the benchmark's wire validation performs (ported here so the solver
//! stack owns it: `koi-bench` validates; `koi-solver` consumes).
//!
//! Hidden-card handling: the observer's placeholder materialization keeps
//! every hidden card unidentified until the ledger names it. An opponent
//! `PlayFromHand` naming a card still in the stock swaps it into the
//! opponent hand for an unidentified placeholder; a named draw pins the
//! stock head. Every action then passes the real `apply_action` legality —
//! a legal-looking but unreachable ledger fails closed.

use koi_core::{
    Action, Card, CardSet, KoiGameState, LedgerEntry, Player, PublicObservation, PublicView, StateError, TurnPhase,
};

use crate::efg::tree::draw_event_key;

/// One replayed decision: the state before the action (hidden zones are
/// placeholders), who acted, what they chose, and the resolved draw.
#[derive(Debug, Clone)]
pub struct DecisionFrame {
    /// The state as replayed just before `action` was applied.
    pub state: KoiGameState,
    /// The acting player.
    pub player: Player,
    /// The action applied.
    pub action: Action,
    /// The publicly drawn card a `ResolveStock` action resolved.
    pub drawn: Option<Card>,
}

/// The replayed observation: per-decision frames plus the public anchors
/// every downstream subgame shares.
#[derive(Debug, Clone)]
pub struct LedgerReplay {
    /// One frame per ledger entry, in order.
    pub frames: Vec<DecisionFrame>,
    /// The `InfoSetKey::public_history` event stream this ledger produces:
    /// `action_key`s for plays/resolves/decisions and `draw_event_key`s for
    /// each publicly drawn card (emitted between the play and its
    /// resolution, matching the compiler's event ordering).
    pub history_prefix: Vec<u64>,
    /// The dealt field — the shared public anchor of every consistent world.
    pub initial_field: CardSet,
    /// The replayed terminal state.
    pub state: KoiGameState,
}

/// Why an observation cannot be replayed.
#[derive(Debug, Clone)]
pub enum ReconstructError {
    /// The deal-time view is inconsistent.
    BadDeal(StateError),
    /// A ledger entry is structurally invalid (wrong seat, missing or
    /// spurious draw, illegal action, hidden card out of pool).
    Malformed { index: usize, detail: String },
    /// A transition the kernel rejected.
    Transition { index: usize, error: StateError },
}

impl std::fmt::Display for ReconstructError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadDeal(error) => write!(f, "the deal-time view is inconsistent: {error}"),
            Self::Malformed { index, detail } => write!(f, "ledger entry {index}: {detail}"),
            Self::Transition { index, error } => write!(f, "ledger entry {index} transition failed: {error}"),
        }
    }
}

impl std::error::Error for ReconstructError {}

fn malformed(index: usize, detail: impl Into<String>) -> ReconstructError {
    ReconstructError::Malformed {
        index,
        detail: detail.into(),
    }
}

/// Replays `observation` through the kernel. The deal-time view must be
/// deal-shaped — `active == dealer`, `turn == 0`, empty piles, no koi-koi
/// state, `AwaitingHandAction` — or the observation attests a round it
/// is not, and the replay is rejected rather than silently re-anchored.
pub fn replay_ledger(observation: &PublicObservation) -> Result<LedgerReplay, ReconstructError> {
    let observer = observation.initial.observer;
    let opponent = observer.opponent();
    let initial = &observation.initial;
    if initial.active != initial.dealer
        || initial.turn != 0
        || initial.phase != TurnPhase::AwaitingHandAction
        || initial.captured.iter().any(|pile| *pile != CardSet::EMPTY)
        || initial.koi_koi_caller.is_some()
        || initial.koi_koi_calls != [0; 2]
        || initial.last_yaku_score != [0; 2]
    {
        return Err(malformed(
            usize::MAX,
            "the deal-time view attests mid-round state (turn, phase, piles, or koi-koi flags)",
        ));
    }
    let mut state = KoiGameState::from_public_view(&deal_view(initial, observer)).map_err(ReconstructError::BadDeal)?;

    let mut frames = Vec::with_capacity(observation.entries.len());
    let mut history_prefix = Vec::with_capacity(observation.entries.len() * 2);
    for (index, entry) in observation.entries.iter().enumerate() {
        if entry.player != state.active {
            return Err(malformed(index, "entry attributes the action to the wrong seat"));
        }
        state = materialize_for_action(state, opponent, entry, index)?;
        if !state.legal_actions().contains(&entry.action) {
            return Err(malformed(index, "entry is not a legal action"));
        }
        // Event ordering matches the compiler: play, then the draw reveal,
        // then the resolution; decisions stand alone.
        match entry.action {
            Action::ResolveStock { .. } => {
                let drawn = entry
                    .drawn
                    .ok_or_else(|| malformed(index, "stock resolution must name the drawn card"))?;
                history_prefix.push(draw_event_key(drawn));
                history_prefix.push(entry.action.action_key());
            }
            _ => {
                if entry.drawn.is_some() {
                    return Err(malformed(index, "drawn card named outside a stock resolution"));
                }
                history_prefix.push(entry.action.action_key());
            }
        }
        let before = state;
        state = state
            .apply_action(entry.action)
            .map_err(|error| ReconstructError::Transition { index, error })?;
        frames.push(DecisionFrame {
            state: before,
            player: entry.player,
            action: entry.action,
            drawn: entry.drawn,
        });
    }

    Ok(LedgerReplay {
        frames,
        history_prefix,
        initial_field: observation.initial.field,
        state,
    })
}

/// The deal-time view the replay starts from: piles empty, turn 0, the
/// dealer to move — the observation's `initial` re-anchored.
fn deal_view(initial: &PublicView, observer: Player) -> PublicView {
    PublicView {
        observer,
        rules: initial.rules,
        dealer: initial.dealer,
        active: initial.dealer,
        turn: 0,
        round: initial.round,
        score: initial.score,
        phase: TurnPhase::AwaitingHandAction,
        own_hand: initial.own_hand,
        opponent_hand_count: initial.opponent_hand_count,
        field: initial.field,
        captured: [CardSet::EMPTY, CardSet::EMPTY],
        stock_count: initial.stock_count,
        koi_koi_caller: None,
        koi_koi_calls: [0; 2],
        last_yaku_score: [0; 2],
    }
}

/// Lazily materializes the hidden cards an entry names: the opponent's
/// played card swaps in from the stock, a named draw pins the stock head.
fn materialize_for_action(
    mut state: KoiGameState,
    opponent: Player,
    entry: &LedgerEntry,
    index: usize,
) -> Result<KoiGameState, ReconstructError> {
    if let TurnPhase::AwaitingStockResolution { drawn: current } = state.phase {
        let named = entry
            .drawn
            .ok_or_else(|| malformed(index, "stock ledger entry must name the drawn card"))?;
        if current == named {
            return Ok(state);
        }
        return pin_stock_head(state, opponent, named).map_err(|error| malformed(index, error));
    }

    if let Action::PlayFromHand { card, .. } = entry.action {
        if state.active == opponent {
            let opponent_hand = state.hands[opponent.index()];
            if !opponent_hand.contains(card) {
                let Some(position) = state.stock.iter().position(|stock_card| *stock_card == card) else {
                    return Err(malformed(index, "names a card outside the hidden pool"));
                };
                let placeholder = opponent_hand
                    .into_iter()
                    .next()
                    .ok_or_else(|| malformed(index, "opponent hand is empty during replay"))?;
                let mut stock = state.stock;
                stock.as_mut_slice()[position] = placeholder;
                let new_hand = opponent_hand.remove(placeholder).insert(card);
                state = state
                    .rebuild_hidden(opponent, new_hand, stock.as_slice())
                    .map_err(|error| malformed(index, format!("opponent-card assignment failed: {error}")))?;
            }
        }
    }
    Ok(state)
}

/// Pins a pending drawn card — the public fact of the *current* decision,
/// which no ledger entry carries — onto a replayed state. The caller
/// compares the result against the snapshot's declared zones.
pub fn pin_pending_draw(state: KoiGameState, opponent: Player, drawn: Card) -> Result<KoiGameState, ReconstructError> {
    pin_stock_head(state, opponent, drawn).map_err(|detail| ReconstructError::Malformed {
        index: usize::MAX,
        detail,
    })
}

/// Pins `named` to the stock head of an `AwaitingStockResolution` state,
/// swapping it in from the opponent's placeholder hand or the stock tail.
fn pin_stock_head(mut state: KoiGameState, opponent: Player, named: Card) -> Result<KoiGameState, String> {
    let TurnPhase::AwaitingStockResolution { drawn: current } = state.phase else {
        return Err("drawn-card pinning requires an AwaitingStockResolution phase".to_owned());
    };
    let opponent_hand = state.hands[opponent.index()];
    let mut stock = state.stock;
    state.phase = TurnPhase::AwaitingStockResolution { drawn: named };
    if opponent_hand.contains(named) {
        stock.as_mut_slice()[0] = named;
        let new_hand = opponent_hand.remove(named).insert(current);
        state
            .rebuild_hidden(opponent, new_hand, stock.as_slice())
            .map_err(|error| format!("hidden reassignment rejected the pinned draw: {error}"))
    } else {
        let Some(position) = stock.iter().position(|card| *card == named) else {
            return Err("named drawn card is outside the hidden pool".to_owned());
        };
        stock.as_mut_slice().swap(0, position);
        state
            .rebuild_hidden(opponent, opponent_hand, stock.as_slice())
            .map_err(|error| format!("hidden reassignment rejected the pinned draw: {error}"))
    }
}

#[cfg(test)]
mod tests {
    use koi_core::{deal_from_seed, Ruleset};

    use super::*;

    /// Plays a leg under the kernel while recording the public observation
    /// the referee would ship, then replays it — the round-trip must land
    /// on the identical trajectory.
    #[test]
    fn recorded_ledger_replays_the_true_trajectory() {
        let mut rng_state = None;
        for seed in 0..u64::MAX {
            let (state, anomaly) = KoiGameState::new_deal(deal_from_seed(seed), Ruleset::nintendo());
            if anomaly.is_none() {
                rng_state = Some(state);
                break;
            }
        }
        let mut state = rng_state.unwrap();
        let observer = state.dealer;
        let initial = state.public_view(observer);
        let mut entries = Vec::new();
        let mut expected_prefix = Vec::new();

        let mut turns = 0;
        while !state.is_ended() && turns < 40 {
            let action = state.legal_actions()[0];
            let drawn = match state.phase {
                TurnPhase::AwaitingStockResolution { drawn } => Some(drawn),
                _ => None,
            };
            entries.push(LedgerEntry {
                player: state.active,
                action,
                drawn,
            });
            match action {
                Action::ResolveStock { .. } => {
                    expected_prefix.push(draw_event_key(drawn.unwrap()));
                    expected_prefix.push(action.action_key());
                }
                _ => expected_prefix.push(action.action_key()),
            }
            state = state.apply_action(action).unwrap();
            turns += 1;
        }

        let observation = PublicObservation {
            initial: initial.clone(),
            entries,
        };
        let replay = replay_ledger(&observation).unwrap();
        assert_eq!(replay.frames.len(), observation.entries.len());
        assert_eq!(replay.history_prefix, expected_prefix);
        assert_eq!(replay.initial_field, initial.field);
        // The replayed terminal must match the true trajectory on every
        // public zone.
        assert_eq!(replay.state.field, state.field);
        assert_eq!(replay.state.captured, state.captured);
        assert_eq!(replay.state.score, state.score);
        assert_eq!(replay.state.hands[observer.index()], state.hands[observer.index()]);
    }

    #[test]
    fn wrong_seat_and_unnamed_draws_fail_closed() {
        let mut state = None;
        for seed in 0..u64::MAX {
            let (s, anomaly) = KoiGameState::new_deal(deal_from_seed(seed), Ruleset::nintendo());
            if anomaly.is_none() {
                state = Some(s);
                break;
            }
        }
        let state = state.unwrap();
        let observer = state.dealer;
        let initial = state.public_view(observer);

        // Wrong seat: attribute the opener to the non-dealer.
        let bad = PublicObservation {
            initial: initial.clone(),
            entries: vec![LedgerEntry {
                player: observer.opponent(),
                action: state.legal_actions()[0],
                drawn: None,
            }],
        };
        assert!(matches!(replay_ledger(&bad), Err(ReconstructError::Malformed { .. })));

        // An illegal action: a card the observer does not hold.
        let foreign = CardSet::ALL
            .difference(state.hands[observer.index()])
            .into_iter()
            .next()
            .unwrap();
        let bad = PublicObservation {
            initial,
            entries: vec![LedgerEntry {
                player: observer,
                action: Action::PlayFromHand {
                    card: foreign,
                    capture: koi_core::CaptureChoice::NoMatch,
                },
                drawn: None,
            }],
        };
        assert!(matches!(replay_ledger(&bad), Err(ReconstructError::Malformed { .. })));
    }
}
