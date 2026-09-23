//! Perfect-information Monte Carlo (PIMC): sample determinized worlds,
//! evaluate every root action inside each, and aggregate per action.
//!
//! The determinization list is the sole work unit: when the exact world
//! count fits `max_simulations` the evaluation is *exhaustive* — every
//! world consistent with the public view — and otherwise the worlds are
//! independent shuffled samples from the decision stream. Each (action,
//! world) pair produces one greedy-playout margin; aggregation is either
//! the mean normalized margin or the win rate (fraction of worlds where the
//! margin beats the opponent, ties counting half).
//!
//! Parallel mode chunks the world list across rayon; because each world's
//! outcome is computed independently and merged in index order, the result
//! is bit-identical to the sequential pass.

use koi_core::{derive_named_seed, Action, KoiGameState, Player};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;

use crate::config::{ScoringMethod, ValidatedPimcConfig};
use crate::determinization::{generate_determinizations, planned_determinization_count};
use crate::error::SolverError;
use crate::simulation::rollout;

/// The normalized margin for `observer` after taking `action` in `world`.
fn evaluate_in_world(world: &KoiGameState, action: Action, observer: Player, score_norm: f32) -> f32 {
    match world.apply_action(action) {
        Ok(next) => rollout(next, observer, score_norm),
        Err(_) => 0.0,
    }
}

/// Aggregates one action's per-world outcomes into a scalar.
fn aggregate(outcomes: &[f32], method: ScoringMethod) -> f32 {
    if outcomes.is_empty() {
        return f32::NEG_INFINITY;
    }
    match method {
        ScoringMethod::Mean => outcomes.iter().sum::<f32>() / outcomes.len() as f32,
        ScoringMethod::WinRate => {
            let wins: f32 = outcomes
                .iter()
                .map(|value| {
                    if *value > 0.5 {
                        1.0
                    } else if *value == 0.5 {
                        0.5
                    } else {
                        0.0
                    }
                })
                .sum();
            wins / outcomes.len() as f32
        }
    }
}

/// PIMC root decision. `score_norm` normalizes rollout margins; the same
/// constant as ISMCTS' `score_norm` config field is applied.
pub(crate) fn find_best_move_pimc(
    state: &KoiGameState,
    observer: Player,
    config: ValidatedPimcConfig,
    use_parallel: bool,
    score_norm: f32,
    seed: u64,
) -> Result<Option<Action>, SolverError> {
    if state.is_ended() {
        return Ok(None);
    }
    let legal: Vec<Action> = state.legal_actions().into_iter().collect();
    if legal.is_empty() {
        return Ok(None);
    }
    if legal.len() == 1 {
        return Ok(Some(legal[0]));
    }

    let planned = planned_determinization_count(state, observer, config.max_simulations);
    log::debug!(
        "pimc decision: {} legal actions x {} worlds (cap {})",
        legal.len(),
        planned,
        config.max_simulations
    );
    let mut rng = ChaCha8Rng::seed_from_u64(derive_named_seed(seed, 0x91_4c_50));
    let worlds =
        generate_determinizations(state, observer, config.max_simulations, &mut rng).map_err(SolverError::State)?;

    // scores[action][world] — the flat per-world margins, world order kept.
    let score_for = |action: Action| -> Vec<f32> {
        if use_parallel {
            worlds
                .par_chunks(config.batch_size.max(1))
                .flat_map_iter(|chunk| {
                    chunk
                        .iter()
                        .map(|world| evaluate_in_world(world, action, observer, score_norm))
                        .collect::<Vec<f32>>()
                })
                .collect()
        } else {
            worlds
                .iter()
                .map(|world| evaluate_in_world(world, action, observer, score_norm))
                .collect()
        }
    };

    legal
        .iter()
        .map(|action| (*action, aggregate(&score_for(*action), config.scoring_method)))
        .max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.action_key().cmp(&a.0.action_key())))
        .map(|(action, _)| Some(action))
        .ok_or(SolverError::NoCompleteEvaluation)
}
