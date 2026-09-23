//! # Koi-Core
//! High-performance, validated domain model and bitboards for Hanafuda Koi-Koi.

pub mod action;
pub mod card;
pub mod rules;
pub mod seed;
pub mod state;
pub mod yaku;

/// Re-exported legacy helpers, kept for backwards compatibility with the
/// original scaffold. New code should use the newtypes on `Card`, `Month`, and
/// `CardSet`.
pub use action::{Action, CaptureChoice};
pub use card::{Card, CardSet, Month};
pub use rules::{HouseRules, NullRoundResolution, Ruleset, YakuPoints};
pub use seed::{deal_from_seed, derive_named_seed, random_nonzero_seed};
pub use state::{
    AnomalyResolution, DealAnomaly, KoiGameState, LedgerEntry, Player, PublicObservation, PublicView, StateError,
    Stock, TurnPhase,
};
