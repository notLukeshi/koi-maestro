//! Uniform-random legal-action baseline — the floor every real solver is
//! benchmarked against.

use koi_core::{Action, KoiGameState};
use rand::{Rng, RngExt};

/// A uniformly random legal action, or `None` at a terminal state.
pub fn find_random_action(state: &KoiGameState, rng: &mut impl Rng) -> Option<Action> {
    let legal = state.legal_actions();
    if legal.is_empty() {
        None
    } else {
        Some(legal[rng.random_range(0..legal.len())])
    }
}
