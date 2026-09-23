//! Canonical action ordering for the fixed `MAX_ACTIONS` policy head.
//!
//! Every consumer — the data generator, the training loader, and the runtime
//! leaf evaluator — must map a state's legal actions onto the same positional
//! slots. `Action::action_key` is a total order over distinct legal actions,
//! so sorting by it is the canonical ordering. Slot `i` of `legal_mask`/
//! `policy_target` corresponds to the `i`-th legal action in that order;
//! slots at or beyond the legal count are masked off.
//!
//! Ported from a sibling research codebase's `leaf/actions.rs` — the sibling head was
//! 32-wide; Koi's cap is 16: at most two capture options exist per played
//! card (`Pair(a)|Pair(b)` when the field holds exactly two of a month),
//! over at most eight hand cards. `resolve_stock` admits two and a stop
//! decision exactly two — 16 covers every phase.

use koi_core::{Action, KoiGameState};

/// Fixed policy-head width — the tensor contract for `legal_mask` and
/// `policy_target`. Pinned: the shipped ONNX policy head is 16-wide; a
/// drift silently misaligns mask and logits.
pub const MAX_ACTIONS: usize = 16;

const _: () = assert!(MAX_ACTIONS == 16, "the leaf policy head is 16-wide by model contract");

/// Returns the legal actions sorted by `action_key`, or `None` when the
/// state exceeds the policy-head width. Callers must fail closed on
/// `None` — a truncated action list would emit a policy target with
/// missing mass.
pub fn canonical_legal(state: &KoiGameState) -> Option<Vec<Action>> {
    let mut legal: Vec<Action> = state.legal_actions().to_vec();
    legal.sort_by_key(Action::action_key);
    (legal.len() <= MAX_ACTIONS).then_some(legal)
}

/// The `[MAX_ACTIONS]` mask for a canonical-ordered legal list.
pub fn legal_mask(legal: &[Action]) -> [bool; MAX_ACTIONS] {
    let mut mask = [false; MAX_ACTIONS];
    for (index, slot) in mask.iter_mut().enumerate() {
        *slot = index < legal.len();
    }
    mask
}

/// Distributes `weights` (aligned to `legal`) into the `[MAX_ACTIONS]`
/// target layout and renormalizes over the legal mask — blueprint policies
/// leave no mass outside the legal set, but averaging noise can make the
/// sum drift from exactly 1.
pub fn spread_over_mask(weights: &[f64], legal: &[Action]) -> [f32; MAX_ACTIONS] {
    debug_assert_eq!(weights.len(), legal.len());
    let mut out = [0.0_f32; MAX_ACTIONS];
    let total: f64 = weights.iter().sum();
    for (index, w) in weights.iter().enumerate() {
        out[index] = if total > 0.0 { (w / total) as f32 } else { 0.0 };
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use koi_core::{deal_from_seed, Ruleset};

    #[test]
    fn canonical_ordering_is_stable_across_calls() {
        let (state, _) = KoiGameState::new_deal(deal_from_seed(9), Ruleset::nintendo());
        let a = canonical_legal(&state).unwrap();
        let b = canonical_legal(&state).unwrap();
        let keys_a: Vec<u64> = a.iter().map(Action::action_key).collect();
        let keys_b: Vec<u64> = b.iter().map(Action::action_key).collect();
        assert_eq!(keys_a, keys_b);
        let mut sorted = keys_a.clone();
        sorted.sort_unstable();
        assert_eq!(keys_a, sorted, "action_key order must be sorted");
    }

    #[test]
    fn the_mask_marks_exactly_the_legal_prefix() {
        let (state, _) = KoiGameState::new_deal(deal_from_seed(10), Ruleset::nintendo());
        let legal = canonical_legal(&state).unwrap();
        let mask = legal_mask(&legal);
        assert_eq!(mask.iter().filter(|m| **m).count(), legal.len());
        assert!(mask[legal.len()..].iter().all(|m| !*m));
    }

    #[test]
    fn spread_renormalizes_over_the_legal_mask() {
        let (state, _) = KoiGameState::new_deal(deal_from_seed(11), Ruleset::nintendo());
        let legal = canonical_legal(&state).unwrap();
        let weights = vec![0.5_f64; legal.len()];
        let out = spread_over_mask(&weights, &legal);
        let sum: f32 = out.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6);
        assert!(out[legal.len()..].iter().all(|p| *p == 0.0));
    }

    #[test]
    fn sixteen_covers_every_phase() {
        // Fuzz the cap across deals and phases: no reachable state may
        // exceed MAX_ACTIONS.
        let mut covered = 0usize;
        for seed in 0..32u64 {
            let (mut state, _) = KoiGameState::new_deal(deal_from_seed(seed), Ruleset::nintendo());
            let mut guard = 0;
            while !state.is_ended() && guard < 64 {
                let legal = canonical_legal(&state).expect("state fits the policy head");
                covered += 1;
                state = state.apply_action(legal[0]).unwrap();
                guard += 1;
            }
        }
        assert!(covered > 200, "expected deep coverage across 32 deals");
    }
}
