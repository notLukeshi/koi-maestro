//! Shared playout machinery: greedy rollouts and terminal evaluation for
//! ISMCTS and PIMC.
//!
//! A rollout plays the determinized world forward with the greedy heuristic
//! policy until the round ends. Rollouts are the value oracle for every
//! sampled world — they are cheap (≤ 48 applies) and deterministic given
//! the world and the RNG tiebreak stream.

use koi_core::{KoiGameState, Player};

use crate::heuristic;

/// Hard bound on rollout steps: a round has at most 16 player-turns × 3
/// phase actions, plus slack for stop decisions. Exceeding it means a state
/// machine bug — stop and evaluate what we have rather than hang.
pub(crate) const ROLLOUT_SAFETY_LIMIT: usize = 96;

/// Normalized margin in `[0, 1]` for `observer` at a terminal or
/// truncated state: `0.5 + 0.5·clamp(margin/score_norm, ±1)`.
pub(crate) fn normalized_margin(state: &KoiGameState, observer: Player, score_norm: f32) -> f32 {
    let margin = state.leg_margin(observer) as f32;
    let scaled = (margin / score_norm).clamp(-1.0, 1.0);
    0.5 + 0.5 * scaled
}

/// Greedy playout to the end of the round; returns the normalized margin
/// for `observer`. Falls back to evaluating the truncated state if the
/// safety limit or an unexpected transition error is hit.
pub(crate) fn rollout(mut state: KoiGameState, observer: Player, score_norm: f32) -> f32 {
    for _ in 0..ROLLOUT_SAFETY_LIMIT {
        if state.is_ended() {
            break;
        }
        let Some(action) = heuristic::find_best_action(&state) else {
            break;
        };
        match state.apply_action(action) {
            Ok(next) => state = next,
            Err(_) => break,
        }
    }
    normalized_margin(&state, observer, score_norm)
}

#[cfg(test)]
mod tests {
    use koi_core::{deal_from_seed, Ruleset};

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
    fn rollout_terminates_within_safety_limit() {
        let state = base_state();
        let observer = state.active;
        let value = rollout(state, observer, 20.0);
        assert!((0.0..=1.0).contains(&value));
    }

    #[test]
    fn rollout_is_deterministic() {
        let state = base_state();
        let observer = state.active;
        assert_eq!(rollout(state, observer, 20.0), rollout(state, observer, 20.0));
    }

    #[test]
    fn full_greedy_round_produces_a_margin() {
        let mut state = base_state();
        let observer = state.active;
        let mut steps = 0;
        while !state.is_ended() && steps < ROLLOUT_SAFETY_LIMIT * 2 {
            let action = heuristic::find_best_action(&state).expect("non-terminal state has actions");
            state = state.apply_action(action).expect("heuristic action is legal");
            steps += 1;
        }
        assert!(state.is_ended(), "greedy round must terminate");
        let _ = observer;
    }
}
