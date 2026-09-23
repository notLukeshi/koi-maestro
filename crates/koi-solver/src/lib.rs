//! # Koi-Solver
//! Decision engines for Hanafuda Koi-Koi: baselines, ISMCTS, and PIMC over
//! determinized worlds, plus the two-stage configuration contract the
//! benchmark worker validates against.

pub mod cfr;
pub mod config;
pub mod efg;
pub mod endgame;
pub mod engine;
pub mod error;
pub mod heuristic;
pub mod leaf;
pub mod random;
pub mod resolving;

mod determinization;
mod ismcts;
mod pimc;
mod simulation;

pub use config::{Config, ConfigError, EffectiveSolverConfig, SolverMethod, ValidatedConfig};
pub use engine::Solver;
pub use error::SolverError;
