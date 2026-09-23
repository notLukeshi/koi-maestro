//! Solver error type.

use koi_core::StateError;

use crate::resolving::ResolveError;

/// Errors a solver can return. Construction failures live in
/// `crate::config::ConfigError`; a solver that cannot answer a decision
/// fails closed — callers forfeit the decision rather than guess.
#[derive(Debug)]
pub enum SolverError {
    /// The state offered no legal action at a phase that requires one.
    NoLegalAction,
    /// Search finished without any completely evaluated candidate.
    NoCompleteEvaluation,
    /// A requested work budget overflows the platform's counters.
    WorkCountOverflow,
    /// A requested work budget exceeds the solver's documented bound.
    WorkLimitExceeded { requested: usize, maximum: usize },
    /// The underlying state transition or reconstruction failed.
    State(StateError),
    /// The resolving method was asked for a decision without the public
    /// ledger it requires — resolving on a bare state would silently
    /// resolve the wrong game, so this fails closed.
    MissingObservation,
    /// The continual resolver could not produce a decision.
    Resolve(ResolveError),
    /// A scripted archetype returned an action outside the legal set —
    /// an internal-policy bug, never an input error.
    ArchetypeViolation,
    /// The configured leaf model could not be constructed — a missing
    /// file, a bad graph, or a build without `leaf-ort`. Construction
    /// failure is fatal, unlike per-call inference failure which degrades
    /// inside the resolve.
    LeafModel(crate::leaf::LeafEvalError),
}

impl std::fmt::Display for SolverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoLegalAction => write!(f, "no legal action"),
            Self::NoCompleteEvaluation => write!(f, "search produced no evaluated candidate"),
            Self::WorkCountOverflow => write!(f, "requested work overflows the platform"),
            Self::WorkLimitExceeded { requested, maximum } => {
                write!(f, "requested work {requested} exceeds the {maximum} limit")
            }
            Self::State(error) => write!(f, "state error: {error}"),
            Self::MissingObservation => {
                write!(f, "resolving requires the public ledger observation")
            }
            Self::Resolve(error) => write!(f, "resolve failed: {error}"),
            Self::ArchetypeViolation => {
                write!(f, "scripted archetype produced an illegal action")
            }
            Self::LeafModel(error) => write!(f, "leaf evaluator unavailable: {error}"),
        }
    }
}

impl std::error::Error for SolverError {}

impl From<StateError> for SolverError {
    fn from(error: StateError) -> Self {
        Self::State(error)
    }
}

impl From<ResolveError> for SolverError {
    fn from(error: ResolveError) -> Self {
        Self::Resolve(error)
    }
}
