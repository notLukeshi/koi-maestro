//! Exact equilibrium solving of a materialized EFG via the sequence-form
//! linear program (Koller, Megiddo & von Stengel 1996).
//!
//! For the row player's treeplex `E x = e, x >= 0` and the column player's
//! `F y = f, y >= 0` with dense payoff `A` (rows = row sequences, columns =
//! column sequences), the equilibrium realization plan solves
//!
//! ```text
//! maximize    fᵀu
//! subject to  E x = e,  x >= 0,
//!             Fᵀu <= Aᵀx          (one row per column sequence)
//! ```
//!
//! dualizing the opponent's best-response program. In `lp::minimize`'s
//! standard form (`min c.z`, `Az = b`, `z >= 0`, `b >= 0`) the free dual
//! variables split as `u = u+ - u-` and each inequality gets a slack `s_t`.
//!
//! This is the LP certificate path: it is exact up to `lp` tolerance and
//! therefore only intended for the reduced domains whose sequence counts
//! keep the dense tableau small (P2-D5).

use koi_core::Player;

use crate::efg::sequence::{PlayerSequences, SequenceForm};
use crate::efg::tree::EfgTree;
use crate::efg::SequenceFormError;

use super::lp::{self, LpError};

/// The dense simplex tableau cell cap: `(1 + I1 + n2) * (n1 + 2(1+I2) + n2)`
/// — roughly 4-6x the payoff cell count, and the binding constraint for
/// LP solving (a payoff form that fits `MAX_PAYOFF_CELLS` can still blow
/// this). 64M f64 cells ≈ 0.5 GiB of tableau.
pub const MAX_LP_CELLS: usize = 64_000_000;

/// Why a tree could not be solved exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolveError {
    /// Sequence-form extraction failed (imperfect recall or payoff cap).
    SequenceForm(SequenceFormError),
    /// The constraint tableau exceeds `MAX_LP_CELLS` — callers with an
    /// approximate fallback (CFR+) should route here, same as
    /// `SequenceFormError::TooLarge`.
    TableauTooLarge,
    /// The simplex reported the program infeasible/unbounded/numerically
    /// broken — unreachable for a well-formed game tree, so this failing
    /// means the encoder is broken. Fail closed.
    Lp(LpError),
}

impl std::fmt::Display for SolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SequenceForm(e) => write!(f, "sequence-form extraction failed: {e}"),
            Self::TableauTooLarge => write!(f, "the LP tableau exceeds MAX_LP_CELLS"),
            Self::Lp(e) => write!(f, "the equilibrium LP failed: {e}"),
        }
    }
}

impl std::error::Error for SolveError {}

impl From<SequenceFormError> for SolveError {
    fn from(e: SequenceFormError) -> Self {
        Self::SequenceForm(e)
    }
}

impl From<LpError> for SolveError {
    fn from(e: LpError) -> Self {
        Self::Lp(e)
    }
}

/// The LP solution for one player of a compiled game.
#[derive(Debug, Clone, PartialEq)]
pub struct SolvedStrategy {
    /// The game value guaranteed to `player` (that player's utility units).
    pub value: f64,
    /// The player's realization plan over their sequence set.
    pub plan: Vec<f64>,
    /// Behavioral strategy per the player's local infoset (parallel to
    /// `global_infosets`). Unreachable infosets (parent plan ~0) get the
    /// uniform fallback.
    pub strategy: Vec<Vec<f64>>,
    /// The tree's global infoset id behind each `strategy` row — lifts the
    /// per-player solution back onto the materialized tree.
    pub global_infosets: Vec<usize>,
}

/// Solves `tree` exactly for `player`'s equilibrium strategy and game value.
///
/// For North the mirrored form is solved — North becomes the row player of a
/// negated-transposed game, and `value` is expressed in North's utility.
pub fn solve_tree(tree: &EfgTree, player: Player) -> Result<SolvedStrategy, SolveError> {
    let base = SequenceForm::compile(tree)?;
    let form = match player {
        Player::South => base,
        Player::North => base.mirrored(),
    };
    solve_form(&form)
}

/// Solves an already-extracted sequence form for its row player.
pub fn solve_form(form: &SequenceForm) -> Result<SolvedStrategy, SolveError> {
    let row = &form.south; // row player of this form
    let col = &form.north;

    let n1 = row.sequence_count;
    let n2 = col.sequence_count;
    let i2 = col.infosets.len();

    // Column player's treeplex F: (1 + I2) rows x n2 cols.
    //   row 0:      y[0] = 1
    //   row 1 + j:  y[parent_j] - sum_a y[seq_a] = 0
    let f = |k: usize, t: usize| -> f64 {
        if k == 0 {
            if t == 0 {
                1.0
            } else {
                0.0
            }
        } else {
            let record = &col.infosets[k - 1];
            if record.parent == t {
                1.0
            } else if record.actions.contains(&t) {
                -1.0
            } else {
                0.0
            }
        }
    };

    // Variables: [x: n1][u+: 1+I2][u-: 1+I2][s: n2].
    let m2 = 1 + i2;
    let variables = n1 + 2 * m2 + n2;
    let u_plus = |k: usize| n1 + k;
    let u_minus = |k: usize| n1 + m2 + k;
    let slack = |t: usize| n1 + 2 * m2 + t;

    // Rows: (1 + I1) treeplex equalities for x, then n2 inequality rows.
    let i1 = row.infosets.len();
    let rows = (1 + i1) + n2;
    // The tableau is the binding size constraint — a payoff form inside
    // MAX_PAYOFF_CELLS can still blow this, so guard before allocating.
    let cells = rows
        .checked_mul(variables)
        .filter(|&cells| cells <= MAX_LP_CELLS)
        .ok_or(SolveError::TableauTooLarge)?;
    let mut a = vec![0.0; cells];
    let mut b = vec![0.0; rows];
    let mut c = vec![0.0; variables];

    // E x = e: row 0 pins x[0] = 1; row 1+i: x[parent_i] - sum_a x[seq_a] = 0.
    a[0] = 1.0;
    b[0] = 1.0;
    for (i, infoset) in row.infosets.iter().enumerate() {
        let r = 1 + i;
        a[r * variables + infoset.parent] = 1.0;
        for &sequence in &infoset.actions {
            a[r * variables + sequence] = -1.0;
        }
    }

    // For each column sequence t: sum_k F[k,t] u_k - sum_s A[s,t] x_s + s_t = 0.
    for t in 0..n2 {
        let r = (1 + i1) + t;
        for k in 0..m2 {
            let coef = f(k, t);
            if coef != 0.0 {
                a[r * variables + u_plus(k)] = coef;
                a[r * variables + u_minus(k)] = -coef;
            }
        }
        for s in 0..n1 {
            let payoff = form.payoff(s, t);
            if payoff != 0.0 {
                a[r * variables + s] = -payoff;
            }
        }
        a[r * variables + slack(t)] = 1.0;
    }

    // Objective: maximize fᵀu = u_0 → minimize -u+_0 + u-_0.
    c[u_plus(0)] = -1.0;
    c[u_minus(0)] = 1.0;

    let (objective, z) = lp::minimize(&c, &a, &b)?;
    let value = -objective;

    let plan: Vec<f64> = z[..n1].to_vec();
    let strategy = behavioral_strategy(row, &plan);
    Ok(SolvedStrategy {
        value,
        plan,
        strategy,
        global_infosets: row.global_infosets.clone(),
    })
}

/// Lifts a realization plan to behavioral strategies: `sigma(a|I) =
/// x[seq_a] / x[parent]` where the parent's mass is positive; unreachable
/// infosets get the uniform fallback (their choice is payoff-irrelevant).
fn behavioral_strategy(sequences: &PlayerSequences, plan: &[f64]) -> Vec<Vec<f64>> {
    const PLAN_EPS: f64 = 1e-9;
    sequences
        .infosets
        .iter()
        .map(|infoset| {
            let parent_mass = plan[infoset.parent];
            if parent_mass > PLAN_EPS {
                let mut row: Vec<f64> = infoset
                    .actions
                    .iter()
                    .map(|&sequence| (plan[sequence] / parent_mass).clamp(0.0, 1.0))
                    .collect();
                // Clamping can push the row off the simplex; a feasible plan
                // drifts by at most LP tolerance, so rescale back to mass 1
                // (a degenerate row gets the uniform fallback like an
                // unreachable infoset).
                let mass: f64 = row.iter().sum();
                if mass > PLAN_EPS && mass.is_finite() {
                    for p in row.iter_mut() {
                        *p /= mass;
                    }
                    row
                } else {
                    vec![1.0 / infoset.actions.len() as f64; infoset.actions.len()]
                }
            } else {
                vec![1.0 / infoset.actions.len() as f64; infoset.actions.len()]
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::efg::{compile_variant, KoiVariant};

    /// The LP equilibrium's game value must dominate every pure response:
    /// South's LP value must lie within the saddle-point band — at least the
    /// uniform-profile value and at most the uniform best-response value.
    #[test]
    fn equilibrium_value_dominates_the_uniform_profile() {
        let tree = compile_variant(&KoiVariant::NANO_6, Player::South).unwrap();
        let solved = solve_tree(&tree, Player::South).unwrap();

        let uniform: Vec<Vec<f64>> = tree
            .infosets()
            .iter()
            .map(|infoset| vec![1.0 / infoset.actions.len() as f64; infoset.actions.len()])
            .collect();
        let uniform_value = crate::cfr::profile_value_south(&tree, &uniform);
        let south_br = crate::cfr::best_response_value(&tree, &uniform, Player::South);
        let north_br = crate::cfr::best_response_value(&tree, &uniform, Player::North);

        // The game value is sandwiched between what the row player gets
        // under uniform play and what a best-responding row player earns.
        assert!(
            solved.value >= uniform_value - 1e-6 && solved.value <= south_br + 1e-6,
            "value {} outside [{}, {}]",
            solved.value,
            uniform_value,
            south_br
        );
        // And the column player's symmetric bound: -north_br <= value.
        assert!(solved.value >= -north_br - 1e-6);
    }

    /// The two players' LP values must negate: zero-sum consistency.
    #[test]
    fn mirrored_solve_negates_the_value() {
        let tree = compile_variant(&KoiVariant::NANO_6, Player::South).unwrap();
        let south = solve_tree(&tree, Player::South).unwrap();
        let north = solve_tree(&tree, Player::North).unwrap();
        assert!(
            (south.value + north.value).abs() < 1e-6,
            "south {} + north {} must cancel",
            south.value,
            north.value
        );
    }

    /// LP plans are feasible: probabilities normalize at reachable infosets.
    #[test]
    fn strategies_are_normalized() {
        let tree = compile_variant(&KoiVariant::NANO_6, Player::South).unwrap();
        let solved = solve_tree(&tree, Player::South).unwrap();
        for strategy in &solved.strategy {
            let total: f64 = strategy.iter().sum();
            assert!((total - 1.0).abs() < 1e-6, "strategy {strategy:?} sums to {total}");
            assert!(strategy.iter().all(|p| *p >= -1e-9));
        }
    }
}
