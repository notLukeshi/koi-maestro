//! Runtime leaf evaluation: the shared feature/action tensor contract and
//! the backends used by the resolver's depth-0 boundary. The offline data
//! pipeline (labels/generate/npy/manifest) lives in `koi-learn`.
//!
//! - [`features`] — the 384-float state encoding.
//! - [`actions`] — the canonical `MAX_ACTIONS = 16` legal-action ordering
//!   and mask layout.
//! - [`eval`] — the `LeafEvaluator` abstraction (a configured backend that
//!   cannot be built fails solver construction closed — never a silent
//!   rollout under leaf provenance).
//! - [`handcrafted`] — the zero-cost static evaluator: no model artifact,
//!   honest baseline, exercises the whole contract.
//! - [`ort_eval`] — the ONNX Runtime backend (`leaf-ort` feature).

pub mod actions;
pub mod eval;
pub mod features;
pub mod handcrafted;
#[cfg(feature = "leaf-ort")]
pub mod ort_eval;

pub use actions::{canonical_legal, legal_mask, spread_over_mask, MAX_ACTIONS};
pub use eval::{
    belief_marginals_for_state, leaf_eval_for_state, leaf_eval_micros, leaf_fallback_count, note_leaf_fallback,
    open_leaf_evaluator, LeafEval, LeafEvalContext, LeafEvalError, LeafEvalMut, LeafEvaluator, LeafQuery,
    BUILTIN_LEAF_MODEL,
};
pub use features::{card_marginals, encode, unseen_mask, unseen_mask_in, CARD_BLOCK, FEATURE_DIM};
pub use handcrafted::HandcraftedEvaluator;
#[cfg(feature = "leaf-ort")]
pub use ort_eval::OrtLeafEvaluator;
