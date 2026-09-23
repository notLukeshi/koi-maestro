//! Regret matching: the behavioral strategy induced by cumulative regrets.
//!
//! Ported verbatim from a sibling research codebase's `solver/cfr/regret.rs` — pure
//! game-agnostic math.

/// The behavioral strategy from cumulative per-action regrets:
/// proportional to the positive regret part, uniform when no action has
/// positive regret (regret matching; CFR+ reuses this on clamped tables).
///
/// One allocation: the clipped vector is normalized in place and returned.
pub fn regret_matching(regrets: &[f64]) -> Vec<f64> {
    let mut total = 0.0;
    let mut positive = Vec::with_capacity(regrets.len());
    for &regret in regrets {
        let clipped = regret.max(0.0);
        positive.push(clipped);
        total += clipped;
    }
    if total > 0.0 {
        for value in &mut positive {
            *value /= total;
        }
    } else {
        let uniform = 1.0 / regrets.len() as f64;
        positive.clear();
        positive.resize(regrets.len(), uniform);
    }
    positive
}

/// The shared stack buffer for strategy/value scratch in CFR traversals —
/// koi action lists are bounded by two entries per hand card (play +
/// pair variants, <= 16), so 16 slots keep every infoset on the stack.
pub type ActionBuf = smallvec::SmallVec<[f64; 16]>;

/// `regret_matching` into a caller-provided stack buffer. The shared CFR
/// traversal calls this once per visited decision node — at 2-3 allocs per
/// node that was the dominant hot-path allocation, so the buffer stays on
/// the stack.
pub fn regret_matching_into(regrets: &[f64], out: &mut ActionBuf) {
    out.clear();
    let mut total = 0.0;
    for &regret in regrets {
        let clipped = regret.max(0.0);
        out.push(clipped);
        total += clipped;
    }
    if total > 0.0 {
        for value in out.iter_mut() {
            *value /= total;
        }
    } else {
        let uniform = 1.0 / regrets.len() as f64;
        for value in out.iter_mut() {
            *value = uniform;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positive_regrets_normalize_proportionally() {
        let strategy = regret_matching(&[1.0, 3.0, 0.0]);
        assert!((strategy[0] - 0.25).abs() < 1e-12);
        assert!((strategy[1] - 0.75).abs() < 1e-12);
        assert_eq!(strategy[2], 0.0);
    }

    #[test]
    fn negative_regrets_are_excluded() {
        let strategy = regret_matching(&[-2.0, 4.0, -1.0]);
        assert_eq!(strategy[0], 0.0);
        assert_eq!(strategy[1], 1.0);
        assert_eq!(strategy[2], 0.0);
    }

    #[test]
    fn all_nonpositive_regrets_fall_back_to_uniform() {
        for regrets in [vec![0.0; 4], vec![-1.0, -2.0, -3.0]] {
            let strategy = regret_matching(&regrets);
            let uniform = 1.0 / regrets.len() as f64;
            assert!(
                strategy.iter().all(|&p| (p - uniform).abs() < 1e-12),
                "regrets {regrets:?} must fall back to uniform, got {strategy:?}"
            );
        }
    }

    #[test]
    fn negative_regret_mass_never_leaks_into_the_strategy() {
        let strategy = regret_matching(&[-5.0, 2.0, 5.0]);
        assert_eq!(strategy[0], 0.0);
        assert!((strategy[1] - 2.0 / 7.0).abs() < 1e-12);
        assert!((strategy[2] - 5.0 / 7.0).abs() < 1e-12);
    }
}
