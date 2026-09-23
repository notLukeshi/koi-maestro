//! The zero-cost evaluator: a static margin estimate and a uniform-over-
//! legal policy, requiring no model artifact. It exists so the leaf path is
//! exercisable end-to-end — encoder, mask layout, counters, resolver wiring
//! — before any ONNX backend or trained weights ship, and as the honest
//! baseline a trained model must beat.

use koi_core::{yaku::score_yaku, KoiGameState};

use super::eval::{LeafEval, LeafEvalError, LeafEvaluator, LeafQuery};
use super::features::CARD_BLOCK;
use super::{actions::MAX_ACTIONS, LeafEvalContext};

/// The built-in heuristic evaluator: `ev` is the actor's static margin —
/// formed-yaku differential plus the match-score lead — and `policy` is
/// uniform over the legal mask. Deterministic, allocation-free past the
/// query contract, and cheap enough to sit inside any decision budget.
#[derive(Debug, Default, Clone, Copy)]
pub struct HandcraftedEvaluator;

impl HandcraftedEvaluator {
    /// The actor-relative static margin the evaluator reports — exposed so
    /// parity tests can pin the estimator independent of the plumbing.
    pub fn margin(state: &KoiGameState) -> f32 {
        let actor = state.active;
        let opponent = actor.opponent();
        let yaku_delta = score_yaku(state.captured[actor.index()], &state.rules) as f32
            - score_yaku(state.captured[opponent.index()], &state.rules) as f32;
        let match_delta = (state.score[actor.index()] - state.score[opponent.index()]) as f32;
        yaku_delta + match_delta
    }
}

impl LeafEvaluator for HandcraftedEvaluator {
    fn evaluate(&mut self, query: &LeafQuery) -> Result<LeafEval, LeafEvalError> {
        // The query contract carries no state pointer — the static margin is
        // recovered from the encoded scalars the producer already computed.
        let yaku_delta = query.features[super::features::scalar::OWN_YAKU_SCORE]
            - query.features[super::features::scalar::OPP_YAKU_SCORE];
        let match_delta = query.features[super::features::scalar::OWN_MATCH_SCORE]
            - query.features[super::features::scalar::OPP_MATCH_SCORE];
        let mass: f32 = query.legal_mask.iter().sum();
        let mut policy = [0.0_f32; MAX_ACTIONS];
        if mass > 0.0 {
            for (slot, mask) in policy.iter_mut().zip(query.legal_mask.iter()) {
                *slot = mask / mass;
            }
        }
        Ok(LeafEval {
            ev: yaku_delta + match_delta,
            policy,
        })
    }
}

/// Builds a standalone handcrafted-eval context for `state`: the belief
/// block is sampled fresh (the labeler path — the resolver passes its own
/// root posterior to [`LeafEvalContext::new`]).
pub fn standalone_context(
    state: &KoiGameState,
    rng: &mut impl rand::Rng,
) -> Result<LeafEvalContext, crate::resolving::BeliefError> {
    let belief = super::eval::belief_marginals_for_state(state, rng)?;
    debug_assert_eq!(belief.len(), CARD_BLOCK);
    Ok(LeafEvalContext::new(Box::new(HandcraftedEvaluator), belief))
}

#[cfg(test)]
mod tests {
    use super::*;
    use koi_core::{deal_from_seed, Player, Ruleset};

    #[test]
    fn the_static_margin_is_actor_relative() {
        let (state, _) = KoiGameState::new_deal(deal_from_seed(21), Ruleset::nintendo());
        assert_eq!(state.active, Player::South);
        // At a fresh deal both piles and match scores are empty — margin 0.
        assert_eq!(HandcraftedEvaluator::margin(&state), 0.0);
    }

    #[test]
    fn the_policy_is_uniform_over_the_legal_mask() {
        let (state, _) = KoiGameState::new_deal(deal_from_seed(22), Ruleset::nintendo());
        let legal = super::super::actions::canonical_legal(&state).unwrap();
        let mask = super::super::actions::legal_mask(&legal).map(|bit| bit as u8 as f32);
        let query = LeafQuery {
            features: super::super::features::encode(&state, &[0.0; CARD_BLOCK], &legal),
            legal_mask: mask,
        };
        let mut evaluator = HandcraftedEvaluator;
        let out = evaluator.evaluate(&query).unwrap();
        let n = legal.len() as f32;
        for (slot, p) in out.policy.iter().enumerate() {
            let expected = if slot < legal.len() { 1.0 / n } else { 0.0 };
            assert!((p - expected).abs() < 1e-7, "slot {slot}");
        }
    }
}
