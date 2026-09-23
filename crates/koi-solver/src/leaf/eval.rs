//! The leaf-evaluator abstraction: the resolver's depth-0 boundary delegates
//! continuation values to a trained model when one is configured, and falls
//! back to `rollout_margin` otherwise (and on evaluator error — a model that
//! fails must degrade the search, never corrupt it).
//!
//! Contract: `ev` is the acting player's expected margin at the leaf (the
//! same actor-relative convention as the data labeler), and `policy` is a
//! masked softmax over the canonical action order.
//!
//! Ported from a sibling research codebase's `leaf/eval.rs` — the sibling version takes
//! `&dyn GameInterface`; Koi's kernel state is concrete `KoiGameState`, and
//! the belief block arrives as caller-computed marginals so the same
//! evaluator serves the resolver's root-posterior leaves and the data
//! labeler's freshly sampled beliefs.

use std::fmt;

use koi_core::{KoiGameState, Player};

use super::{actions::MAX_ACTIONS, features::FEATURE_DIM};

/// One leaf evaluation request: the actor-relative feature vector and the
/// legal-action mask in canonical order.
#[derive(Debug, Clone)]
pub struct LeafQuery {
    pub features: [f32; FEATURE_DIM],
    pub legal_mask: [f32; MAX_ACTIONS],
}

/// Model output for one leaf.
#[derive(Debug, Clone, PartialEq)]
pub struct LeafEval {
    /// Acting-player expected margin (points), actor-relative.
    pub ev: f32,
    /// Masked policy over the canonical action order.
    pub policy: [f32; MAX_ACTIONS],
}

#[derive(Debug)]
pub enum LeafEvalError {
    /// The ONNX Runtime shared library could not be located/loaded.
    RuntimeUnavailable(String),
    /// The model file could not be read or is not a valid graph.
    ModelLoad(String),
    /// Inference itself failed (shape/dtype/kernel errors).
    Inference(String),
}

impl fmt::Display for LeafEvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RuntimeUnavailable(msg) => write!(f, "onnx runtime unavailable: {msg}"),
            Self::ModelLoad(msg) => write!(f, "leaf model failed to load: {msg}"),
            Self::Inference(msg) => write!(f, "leaf inference failed: {msg}"),
        }
    }
}

impl std::error::Error for LeafEvalError {}

/// A leaf evaluator. `Send` so one evaluator can sit behind the solver's
/// shared mutex and serve whichever worker is resolving. `evaluate_batch`
/// is the fast path the resolver targets once traversal is vectorized;
/// until then the default implementation loops the single-leaf call so
/// every backend already supports the contract.
pub trait LeafEvaluator: Send {
    fn evaluate(&mut self, query: &LeafQuery) -> Result<LeafEval, LeafEvalError>;

    fn evaluate_batch(&mut self, queries: &[LeafQuery]) -> Result<Vec<LeafEval>, LeafEvalError> {
        queries.iter().map(|query| self.evaluate(query)).collect()
    }
}

/// The `resolving.leaf_model` sentinel selecting the deterministic
/// handcrafted evaluator — no file, no ONNX Runtime. Every other value is
/// an ONNX model path.
pub const BUILTIN_LEAF_MODEL: &str = "builtin:handcrafted";

/// Opens the configured leaf evaluator. A construction error is fatal to
/// the caller — a configured-but-unopenable model is a config defect, not
/// the per-call transient failure the resolver degrades from. Without the
/// `leaf-ort` build feature, model paths fail closed as
/// `RuntimeUnavailable`.
pub fn open_leaf_evaluator(spec: &str) -> Result<Box<dyn LeafEvaluator>, LeafEvalError> {
    if spec == BUILTIN_LEAF_MODEL {
        return Ok(Box::new(super::handcrafted::HandcraftedEvaluator));
    }
    #[cfg(feature = "leaf-ort")]
    {
        Ok(Box::new(super::ort_eval::OrtLeafEvaluator::new(spec, 1)?))
    }
    #[cfg(not(feature = "leaf-ort"))]
    {
        Err(LeafEvalError::RuntimeUnavailable(
            "a leaf model path requires the leaf-ort build feature".to_owned(),
        ))
    }
}

/// The resolver's borrowed handle on an evaluator. The `'static` bound is
/// written out explicitly: `&mut dyn Trait` in argument position would
/// otherwise default the object bound to the borrow lifetime, which breaks
/// reborrowing a `Box<dyn LeafEvaluator + 'static>` down the recursion.
pub type LeafEvalMut<'a> = &'a mut (dyn LeafEvaluator + 'static);

/// Belief-world budget for a standalone leaf evaluation: cheaper than the
/// resolve root's — the leaf only needs a posterior sketch over the acting
/// player's unseen cards, not the resolve root's full range.
const LEAF_BELIEF_WORLDS: usize = 32;

/// Count of leaf evaluations that degraded to the rollout estimate since
/// process start. Observability only — a nonzero value means leaf quality
/// dropped silently, never that the resolve was wrong.
static LEAF_FALLBACKS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Number of evaluator-failure → rollout degradations since process start.
pub fn leaf_fallback_count() -> usize {
    LEAF_FALLBACKS.load(std::sync::atomic::Ordering::Relaxed)
}

/// Records one evaluator-failure → rollout degradation. Called by the
/// resolver's leaf boundary, not by the evaluator itself.
pub fn note_leaf_fallback() {
    LEAF_FALLBACKS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

thread_local! {
    /// Microseconds this thread has spent inside `LeafEvaluator::evaluate`
    /// — cumulative, never reset. Decisions are single-threaded (no rayon
    /// inside the resolver), so a read-before/read-after delta attributes
    /// a whole decision's leaf cost to the deciding worker without any
    /// cross-thread contamination.
    static LEAF_EVAL_MICROS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// This thread's cumulative microseconds inside model `evaluate` calls —
/// the decision-attribution counter. Read it twice across a decision and
/// take the delta; do not reset it (overlapping users on one thread would
/// lose each other's time).
pub fn leaf_eval_micros() -> u64 {
    LEAF_EVAL_MICROS.with(|cell| cell.get())
}

/// Charges an elapsed model-call duration into the decision-attribution
/// counter — shared by the per-call path (timed inside
/// `leaf_eval_for_state`) and the batched resolver path (timed around
/// `evaluate_batch`), so both report the same quantity.
pub(crate) fn charge_leaf_eval_micros(elapsed: std::time::Duration) {
    LEAF_EVAL_MICROS.with(|cell| cell.set(cell.get() + elapsed.as_micros() as u64));
}

/// Test-only reset so counter assertions are not order-dependent.
#[cfg(test)]
pub fn reset_leaf_eval_micros() {
    LEAF_EVAL_MICROS.with(|cell| cell.set(0));
}

/// Test-only reset so counter assertions are not order-dependent.
#[cfg(test)]
pub fn reset_leaf_fallback_count() {
    LEAF_FALLBACKS.store(0, std::sync::atomic::Ordering::Relaxed);
}

/// Evaluates `state` through `evaluator` and returns the model's full
/// output: the actor-relative margin and the masked policy over the
/// canonical action order. `belief` is the marginal opponent-hand
/// posterior over the state's unseen set — the caller chooses its
/// provenance (resolve-root posterior inside a resolve, a fresh
/// `build_belief` sample for labeling), so world-private assignments can
/// never leak into the feature block.
pub fn leaf_eval_for_state(
    state: &KoiGameState,
    belief: &[f32; super::features::CARD_BLOCK],
    evaluator: LeafEvalMut<'_>,
) -> Result<LeafEval, LeafEvalError> {
    // canonical_legal fails closed above MAX_ACTIONS: a wider position would
    // silently produce an all-ones legal_mask and lie to the model, so the
    // leaf degrades to the resolver's rollout estimate via an Inference error.
    let legal = super::actions::canonical_legal(state)
        .ok_or_else(|| LeafEvalError::Inference(format!("leaf exceeds the {MAX_ACTIONS}-action policy head")))?;
    let query = LeafQuery {
        features: super::features::encode(state, belief, &legal),
        legal_mask: super::actions::legal_mask(&legal).map(|bit| bit as u8 as f32),
    };
    // Time the model call itself — the quantity `leaf_bench` measures and
    // the `leaf_eval_latency_ms` trace field reports. Belief construction
    // and feature encoding are ordinary CPU work priced inside the
    // decision, not inside the model.
    let eval_started = std::time::Instant::now();
    let out = evaluator.evaluate(&query);
    charge_leaf_eval_micros(eval_started.elapsed());
    let out = out?;
    if !out.ev.is_finite() || out.policy.iter().any(|p| !p.is_finite()) {
        // A NaN/infinite value would propagate through every regret update
        // or sample on the path — treat it as evaluator failure so the
        // caller degrades (rollout leaf / uniform pick) and the fallback
        // counter records it.
        return Err(LeafEvalError::Inference(format!(
            "leaf model returned non-finite output: ev {}",
            out.ev
        )));
    }
    Ok(out)
}

/// Builds the standalone belief block for `state`: a fresh `build_belief`
/// posterior over the acting player's unseen set, reduced to per-card
/// marginals. This is the data-labeler / ad-hoc-eval path — the resolver
/// passes its own root posterior instead.
pub fn belief_marginals_for_state(
    state: &KoiGameState,
    rng: &mut impl rand::Rng,
) -> Result<[f32; super::features::CARD_BLOCK], crate::resolving::BeliefError> {
    let observer = state.active;
    let worlds = crate::resolving::build_belief(state, observer, koi_core::CardSet::ALL, LEAF_BELIEF_WORLDS, rng)?;
    let weights: Vec<f64> = {
        let total: f64 = worlds.iter().map(|world| world.weight.max(0.0)).sum();
        if total <= 0.0 {
            vec![1.0 / worlds.len() as f64; worlds.len()]
        } else {
            worlds.iter().map(|world| world.weight.max(0.0) / total).collect()
        }
    };
    Ok(super::features::card_marginals(
        super::features::unseen_mask(state),
        &worlds,
        &weights,
    ))
}

/// The resolve-time leaf context: one evaluator plus the resolve root's
/// posterior belief block. Constructed once per resolve so every leaf
/// consults the model under the *observer's* posterior — never a resolved
/// world's private assignment.
pub struct LeafEvalContext {
    evaluator: Box<dyn LeafEvaluator>,
    belief: [f32; super::features::CARD_BLOCK],
}

impl LeafEvalContext {
    pub fn new(evaluator: Box<dyn LeafEvaluator>, belief: [f32; super::features::CARD_BLOCK]) -> Self {
        Self { evaluator, belief }
    }

    /// The evaluator's south-relative margin at `state`. Actor-relative
    /// `ev` converts by seat — the same convention the gadget oracle uses.
    pub fn margin_south(&mut self, state: &KoiGameState) -> Result<f64, LeafEvalError> {
        let out = leaf_eval_for_state(state, &self.belief, self.evaluator.as_mut())?;
        Ok(match state.active {
            Player::South => f64::from(out.ev),
            Player::North => f64::from(-out.ev),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use koi_core::{deal_from_seed, Ruleset};

    /// A zero-cost stand-in: heuristic-shaped margin, uniform-over-legal policy.
    struct StubEvaluator {
        fail: bool,
        calls: usize,
    }

    impl LeafEvaluator for StubEvaluator {
        fn evaluate(&mut self, query: &LeafQuery) -> Result<LeafEval, LeafEvalError> {
            self.calls += 1;
            if self.fail {
                return Err(LeafEvalError::Inference("stub failure".to_owned()));
            }
            let mut policy = [0.0_f32; MAX_ACTIONS];
            for (slot, mask) in policy.iter_mut().zip(query.legal_mask.iter()) {
                *slot = *mask;
            }
            let mass: f32 = policy.iter().sum();
            if mass > 0.0 {
                for slot in policy.iter_mut() {
                    *slot /= mass;
                }
            }
            Ok(LeafEval { ev: 1.5, policy })
        }
    }

    #[test]
    fn evaluate_batch_default_loops_the_single_call() {
        let mut evaluator = StubEvaluator { fail: false, calls: 0 };
        let queries = vec![
            LeafQuery {
                features: [0.0; FEATURE_DIM],
                legal_mask: [1.0; MAX_ACTIONS],
            },
            LeafQuery {
                features: [1.0; FEATURE_DIM],
                legal_mask: [0.5; MAX_ACTIONS],
            },
        ];
        let out = evaluator.evaluate_batch(&queries).unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(evaluator.calls, 2);
    }

    #[test]
    fn a_nonfinite_ev_is_rejected_as_inference_failure() {
        struct NanEval;
        impl LeafEvaluator for NanEval {
            fn evaluate(&mut self, _query: &LeafQuery) -> Result<LeafEval, LeafEvalError> {
                Ok(LeafEval {
                    ev: f32::NAN,
                    policy: [0.0; MAX_ACTIONS],
                })
            }
        }
        let (state, _) = KoiGameState::new_deal(deal_from_seed(3), Ruleset::nintendo());
        let mut evaluator = NanEval;
        let err = leaf_eval_for_state(&state, &[0.0; super::super::features::CARD_BLOCK], &mut evaluator).unwrap_err();
        assert!(matches!(err, LeafEvalError::Inference(_)));
    }

    #[test]
    fn the_context_converts_actor_relative_ev_to_south_margin() {
        let (state, _) = KoiGameState::new_deal(deal_from_seed(8), Ruleset::nintendo());
        let mut ctx = LeafEvalContext::new(
            Box::new(StubEvaluator { fail: false, calls: 0 }),
            [0.0; super::super::features::CARD_BLOCK],
        );
        let margin = ctx.margin_south(&state).unwrap();
        // South acts first at a fresh deal — actor-relative ev IS the south margin.
        assert_eq!(state.active, Player::South);
        assert!((margin - 1.5).abs() < 1e-6);
    }
}
