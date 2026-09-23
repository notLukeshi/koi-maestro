//! # Koi-Bench
//! Paired benchmark harness: manifest, worker protocol, referee runner,
//! checkpointed evidence, and sequential stopping.

pub mod artifact;
pub mod checkpoint;
pub mod framing;
pub mod manifest;
pub mod process_tree;
pub mod protocol;
pub mod ranking;
pub mod report;
pub mod runner;
pub mod sprt;
pub mod statistics;
pub mod worker;
