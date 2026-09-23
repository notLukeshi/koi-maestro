//! Extensive-form game machinery: the compiler, the materialized tree, and
//! sequence-form extraction.
//!
//! The compiler enumerates a (possibly reduced) Koi-Koi round through the
//! validated koi-core state machine — never through a parallel transition
//! model. See `compiler` for the chance-node design (P2-D1) and `tree` for
//! the perfect-recall information-set key (P2-D3).

pub mod compiler;
pub mod sequence;
pub mod tree;

pub use compiler::{
    compile_gadget_subgame, compile_subgame, compile_variant, compile_variant_with_budget, EfgCompileError, KoiVariant,
    MarginOracle, MAX_EFG_NODES,
};
pub use sequence::{SequenceForm, SequenceFormError};
pub use tree::{
    draw_event_key, gadget_event_key, ChanceOutcome, EfgTree, InfoSet, InfoSetKey, InfosetId, NodeId, NodeType,
};
