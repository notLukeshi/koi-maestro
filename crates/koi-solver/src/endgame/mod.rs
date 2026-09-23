//! Endgame solvers and LP optimization routines.
//!
//! - `lp`: the dense two-phase simplex the sequence-form encoder feeds.
//! - `solve`: exact equilibrium of a materialized EFG via the sequence-form
//!   LP — the LP certificate path for reduced domains.
//! - `turn8_exact`: exhaustive-worlds per-world minimax for last-decision
//!   positions (P2-D6).
//! - `turn7_subgame`: belief-weighted residual EFGs solved by LP or CFR+ —
//!   imperfect-information subgame glue.
//! - `optimal_stopping`: the Shōbu/Koi-Koi decision evaluator.
pub mod lp;
pub mod optimal_stopping;
pub mod solve;
pub mod turn7_subgame;
pub mod turn8_exact;

pub use optimal_stopping::{evaluate_stop, StopError, StopEvaluation};
pub use solve::{solve_form, solve_tree, SolveError, SolvedStrategy};
pub use turn7_subgame::{solve_subgame, SubgameContext, SubgameError, SubgameMethod, SubgameSolution};
pub use turn8_exact::{best_action, exhaustive_action_values, EndgameError};
