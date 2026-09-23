//! The learn pipeline: `.npy` shard writer, blueprint/resolve labelers,
//! deterministic row generation, and shard provenance manifests.

pub mod generate;
pub mod labels;
pub mod manifest;
pub mod npy;
