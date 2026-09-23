//! The solver facade: one entry point that dispatches on the validated
//! configuration's solver method.
//!
//! The per-decision contract is deterministic: `solver_seed` is the entire
//! entropy source for a call. The benchmark worker replays decisions by
//! re-pinning the same seed, so this entry point must not read ambient
//! randomness — `Config`'s own random-seed resolution happens once at
//! construction and becomes part of the recorded provenance.

use koi_core::{derive_named_seed, Action, KoiGameState, Player, PublicObservation, TurnPhase};
use rand::rngs::SmallRng;
use rand::SeedableRng;

use crate::config::{Config, ConfigError, SolverMethod, ValidatedConfig};
use crate::error::SolverError;
use crate::resolving::{replay_ledger, resolve_decision, GadgetOracle, MarginAnchors, OpponentModel};
use crate::{heuristic, ismcts, pimc, random};

/// Named stream tag for the uniform-random baseline.
const RANDOM_STREAM_TAG: u64 = 0x4b4f_4952_414e_4401;
/// Named stream tag for the resolver's belief sampler and root draw.
const RESOLVE_STREAM_TAG: u64 = 0x4b4f_4952_4553_4f4c;
/// Named stream tag for the `Random` scripted archetype's uniform draws.
const ARCHETYPE_STREAM_TAG: u64 = 0x4b4f_4941_5243_4801;
/// Named stream tag for the leaf-policy entrant's per-decision belief
/// sampler — kept distinct from the resolve stream so a seed shared
/// across solvers cannot collide streams.
const LEAF_POLICY_STREAM_TAG: u64 = 0x4b4f_494c_4541_4601;

/// A configured solver. Cheap to construct — the leaf-policy entrant's
/// evaluator attaches lazily on its first decision (matching the
/// resolving path's per-decision open semantics: a configured model that
/// cannot be opened fails the decision closed as a config defect).
pub struct Solver {
    config: ValidatedConfig,
    /// The leaf-policy entrant's evaluator cache: opened once, reused for
    /// the rest of the process — an ONNX session per decision would
    /// dominate the one-call-per-decision cost model. `Mutex` keeps the
    /// `Solver` `Sync`; a poisoned lock is recovered, never fatal.
    leaf_evaluator: std::sync::Mutex<Option<Box<dyn crate::leaf::LeafEvaluator>>>,
}

impl Solver {
    /// Builds a solver from a raw config, validating every field.
    pub fn from_config(config: Config) -> Result<Self, ConfigError> {
        Ok(Self {
            config: config.validate()?,
            leaf_evaluator: std::sync::Mutex::new(None),
        })
    }

    /// Builds a solver from an already-validated config.
    pub fn from_validated_config(config: ValidatedConfig) -> Self {
        Self {
            config,
            leaf_evaluator: std::sync::Mutex::new(None),
        }
    }

    /// The validated configuration this solver runs.
    pub fn config(&self) -> &ValidatedConfig {
        &self.config
    }

    /// One decision under the per-decision seeding contract.
    ///
    /// Returns `Ok(None)` only at a terminal state or with no legal action;
    /// a search that cannot produce a candidate fails closed with
    /// `SolverError` rather than guessing.
    pub fn find_best_action(&self, state: &KoiGameState, solver_seed: u64) -> Result<Option<Action>, SolverError> {
        let legal = state.legal_actions();
        if state.is_ended() || legal.is_empty() {
            return Ok(None);
        }
        if legal.len() == 1 {
            return Ok(legal.into_iter().next());
        }
        let observer = state.active;
        match self.config.solver {
            SolverMethod::Random => {
                let mut rng = SmallRng::seed_from_u64(derive_named_seed(solver_seed, RANDOM_STREAM_TAG));
                Ok(random::find_random_action(state, &mut rng))
            }
            SolverMethod::Heuristic => Ok(heuristic::find_best_action(state)),
            SolverMethod::Ismcts => ismcts::find_best_action_ismcts(
                state,
                observer,
                self.config.ismcts.iterations,
                self.config.ismcts.score_norm,
                self.config.use_parallel,
                solver_seed,
            ),
            SolverMethod::Pimc => pimc::find_best_move_pimc(
                state,
                observer,
                self.config.pimc,
                self.config.use_parallel,
                self.config.ismcts.score_norm,
                solver_seed,
            ),
            SolverMethod::Endgame => Ok(endgame_action(state, observer)),
            SolverMethod::LeafPolicy => self.leaf_policy_action(state, solver_seed),
            // Resolving on a bare state would silently resolve the wrong
            // game (the public ledger is not derivable from zones) — the
            // worker must route through `find_best_action_observed`.
            SolverMethod::Resolving => Err(SolverError::MissingObservation),
            SolverMethod::Archetype => {
                let archetype = self.config.archetype.expect("validation guarantees an archetype");
                match archetype.pick(state) {
                    Some(action) if legal.contains(&action) => Ok(Some(action)),
                    Some(_) => Err(SolverError::ArchetypeViolation),
                    // `Random` has no deterministic pick — uniform over legal.
                    None => {
                        let mut rng = SmallRng::seed_from_u64(derive_named_seed(solver_seed, ARCHETYPE_STREAM_TAG));
                        Ok(random::find_random_action(state, &mut rng))
                    }
                }
            }
        }
    }

    /// One decision for a stateful solver that consumes the public ledger —
    /// the P3 resolving method. `observation` is the wire's attested
    /// deal-view + ordered ledger (already replay-validated by
    /// `into_state`); `gadget` selects the safety oracle the caller loaded
    /// (blueprint-certified or learned-empirical).
    ///
    /// Every other method ignores the observation and defers to
    /// [`Self::find_best_action`].
    pub fn find_best_action_observed(
        &self,
        state: &KoiGameState,
        observation: &PublicObservation,
        gadget: &GadgetOracle,
        solver_seed: u64,
    ) -> Result<Option<Action>, SolverError> {
        if self.config.solver != SolverMethod::Resolving {
            return self.find_best_action(state, solver_seed);
        }
        let legal = state.legal_actions();
        if state.is_ended() || legal.is_empty() {
            return Ok(None);
        }
        if legal.len() == 1 {
            return Ok(legal.into_iter().next());
        }
        let observer = state.active;
        if observation.initial.observer != observer {
            // The ledger must be the acting seat's own observation — an
            // observation anchored on the opponent would leak their view.
            return Err(SolverError::MissingObservation);
        }
        let replay = replay_ledger(observation).map_err(crate::resolving::ResolveError::from)?;
        let replay_state = match state.phase {
            // A pending draw is a public fact no ledger entry carries — the
            // resolution entry that names it is still in the future. Pin it
            // from the attested state, exactly as the wire's own ledger
            // validation does, before the views are compared.
            TurnPhase::AwaitingStockResolution { drawn } => {
                crate::resolving::pin_pending_draw(replay.state, observer.opponent(), drawn)
                    .map_err(crate::resolving::ResolveError::from)?
            }
            _ => replay.state,
        };
        if replay_state.public_view(observer) != state.public_view(observer) {
            // The observation must attest this exact decision — a replay
            // that lands on different public zones is a mis-anchored view
            // (the wire proves this too, but a direct caller can bypass
            // it), and a resolve against it would silently solve the
            // wrong game.
            return Err(SolverError::MissingObservation);
        }
        let anchors = MarginAnchors {
            initial_field: replay.initial_field,
            history_prefix: replay.history_prefix.clone(),
        };
        let variant = gadget.variant();
        let oracle = gadget.oracle(&anchors);
        let model = self.config.resolving.adaptive.then(OpponentModel::default);
        // A configured leaf model that cannot be opened fails the decision
        // closed — construction errors are config defects (missing file,
        // no leaf-ort build), unlike per-call inference failures which
        // degrade to the oracle inside the resolve.
        let leaf_evaluator = match &self.config.resolving.leaf_model {
            None => None,
            Some(spec) => Some(std::cell::RefCell::new(
                crate::leaf::open_leaf_evaluator(spec).map_err(SolverError::LeafModel)?,
            )),
        };
        let mut rng = SmallRng::seed_from_u64(derive_named_seed(solver_seed, RESOLVE_STREAM_TAG));
        let ctx = crate::resolving::ResolveContext {
            variant: &variant,
            anchors: &anchors,
            oracle: &oracle,
            spec: &self.config.resolving.spec,
            model: model.as_ref(),
            replay: model.as_ref().map(|_| &replay),
            leaf_evaluator: leaf_evaluator.as_ref(),
        };
        let resolved = resolve_decision(state, observer, &ctx, &mut rng)?;

        let mut action = resolved.action;
        if self.config.resolving.ox {
            if let Some(model) = &model {
                if let Some(posterior) = model.archetype_posterior(&resolved.belief, &replay, observer) {
                    if model.gate_passes(&posterior) {
                        if let Some(ox) = crate::resolving::ox_action(observer, &resolved.belief, &posterior, &resolved)
                        {
                            action = ox;
                        }
                    }
                }
            }
        }
        Ok(Some(action))
    }

    /// The P6 leaf-policy entrant: one encoded-state + belief-marginal
    /// model call per decision, then masked-argmax over the canonical
    /// action order — the trained policy head played as a standalone
    /// player, with no search and no ledger requirement.
    ///
    /// Contract: the belief block is a fresh `build_belief` marginal over
    /// the acting player's unseen set (the same labeler path the data
    /// generator uses — never the opponent's private assignment). The
    /// evaluator opens lazily once and is cached; a model that cannot be
    /// *opened* fails the decision closed (`SolverError::LeafModel`),
    /// while a per-call failure — a state exceeding the 16-slot head, an
    /// inference error, or a non-finite policy — degrades to the P1
    /// heuristic pick and is counted via `note_leaf_fallback`. Argmax
    /// ties break to the earliest canonical slot (smallest `action_key`),
    /// so decisions replay bit-for-bit under one seed.
    fn leaf_policy_action(&self, state: &KoiGameState, solver_seed: u64) -> Result<Option<Action>, SolverError> {
        let mut guard = self
            .leaf_evaluator
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if guard.is_none() {
            let spec = self
                .config
                .leaf_policy
                .leaf_model
                .as_deref()
                .unwrap_or(crate::leaf::BUILTIN_LEAF_MODEL);
            *guard = Some(crate::leaf::open_leaf_evaluator(spec).map_err(SolverError::LeafModel)?);
        }
        let evaluator = guard.as_mut().expect("the evaluator was opened above");
        let mut rng = SmallRng::seed_from_u64(derive_named_seed(solver_seed, LEAF_POLICY_STREAM_TAG));
        let belief =
            crate::leaf::belief_marginals_for_state(state, &mut rng).map_err(crate::resolving::ResolveError::from)?;
        match crate::leaf::leaf_eval_for_state(state, &belief, evaluator.as_mut()) {
            Ok(out) => {
                // A successful eval implies the state fits the head —
                // re-derive the canonical order for the slot mapping.
                let legal = crate::leaf::canonical_legal(state).expect("a passed eval implies the head fits");
                Ok(Some(legal[policy_argmax(&out.policy, legal.len())]))
            }
            Err(_) => {
                crate::leaf::note_leaf_fallback();
                Ok(heuristic::find_best_action(state))
            }
        }
    }
}

/// Masked-argmax over the canonical order: the highest-mass legal slot,
/// ties to the earliest (smallest `action_key`). Only slots below
/// `legal_len` compete — mass leaked onto a masked slot can never win.
fn policy_argmax(policy: &[f32; crate::leaf::MAX_ACTIONS], legal_len: usize) -> usize {
    let mut best = 0usize;
    for index in 1..legal_len {
        if policy[index] > policy[best] {
            best = index;
        }
    }
    best
}

/// The P2 entrant: heuristic backbone with the exact endgame stack where it
/// is honest — solved stop decisions and the exact last-turn solver.
///
/// - `AwaitingStopDecision`: `evaluate_stop` compares the known Shōbu
///   margin against the belief-weighted residual subgame; on any failure
///   the conservative action is Shōbu (bank the sure thing).
/// - Observer hand down to one card *and the opponent's hand empty*:
///   `turn8_exact::best_action` is *exact* — every later decision is a
///   public-information one (stock resolution, stop calls), so per-world
///   minimax carries no clairvoyance. With the opponent still holding a
///   card their last hand play is a hidden-info decision whose minimax EV
///   conditions on the unseen stock — a clairvoyant bound, not exact, so
///   the gate requires both hands to be spent after this play.
/// - Everything else: the P1 heuristic. Deeper imperfect-information
///   subgame calls are P3 resolving's job — this entrant deliberately does
///   not use the clairvoyant relaxation mid-game.
fn endgame_action(state: &KoiGameState, observer: Player) -> Option<Action> {
    if matches!(state.phase, koi_core::TurnPhase::AwaitingStopDecision { .. }) {
        let ctx = crate::endgame::SubgameContext {
            variant: &crate::efg::KoiVariant::FULL,
            initial_field: state.field,
            ..Default::default()
        };
        return match crate::endgame::evaluate_stop(state, observer, &ctx) {
            Ok(evaluation) => Some(evaluation.recommended),
            Err(_) => Some(Action::Shobu),
        };
    }
    if state.hands[observer.index()].count() <= 1 && state.hands[observer.opponent().index()].is_empty() {
        if let Ok(action) = crate::endgame::best_action(state, observer, crate::endgame::turn8_exact::MAX_EXACT_WORLDS)
        {
            return Some(action);
        }
    }
    heuristic::find_best_action(state)
}

impl std::fmt::Debug for Solver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Solver")
            .field("solver", &self.config.solver.as_str())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use koi_core::{deal_from_seed, Player, Ruleset};

    use super::*;

    fn base_state() -> KoiGameState {
        for seed in 0..u64::MAX {
            let (state, anomaly) = KoiGameState::new_deal(deal_from_seed(seed), Ruleset::nintendo());
            if anomaly.is_none() {
                return state;
            }
        }
        unreachable!()
    }

    #[test]
    fn every_backend_returns_a_legal_action() {
        let state = base_state();
        for solver in ["random", "heuristic", "ismcts", "pimc", "endgame", "leaf_policy"] {
            let engine = Solver::from_config(Config {
                solver: solver.to_owned(),
                ismcts: crate::config::IsmctsConfig {
                    iterations: 64,
                    ..Default::default()
                },
                pimc: crate::config::PimcConfig {
                    max_simulations: 8,
                    ..Default::default()
                },
                seed: Some(7),
                ..Config::default()
            })
            .unwrap();
            let action = engine.find_best_action(&state, 42).unwrap().unwrap();
            assert!(
                state.legal_actions().contains(&action),
                "{solver} returned an illegal action"
            );
        }
        // Scripted archetypes decide through the bare path.
        for archetype in [
            "heuristic",
            "materialist",
            "banker",
            "gambler",
            "timid",
            "yaku_chaser",
            "random",
        ] {
            let engine = Solver::from_config(Config {
                solver: "archetype".to_owned(),
                archetype: Some(archetype.to_owned()),
                seed: Some(7),
                ..Config::default()
            })
            .unwrap();
            let action = engine.find_best_action(&state, 42).unwrap().unwrap();
            assert!(
                state.legal_actions().contains(&action),
                "archetype {archetype} returned an illegal action"
            );
        }
    }

    /// A test-only injector: the leaf-policy entrant's evaluator cache is
    /// private, so stub backends are installed through this hook.
    fn inject_leaf_evaluator(engine: &Solver, evaluator: Box<dyn crate::leaf::LeafEvaluator>) {
        *engine.leaf_evaluator.lock().unwrap() = Some(evaluator);
    }

    /// The leaf-policy entrant picks a legal action and replays
    /// bit-for-bit under one seed — with the handcrafted evaluator the
    /// uniform policy argmaxes to the first canonical action.
    #[test]
    fn leaf_policy_decides_legally_and_replays() {
        let state = base_state();
        let engine = Solver::from_config(Config {
            solver: "leaf_policy".to_owned(),
            seed: Some(7),
            ..Config::default()
        })
        .unwrap();
        let legal = crate::leaf::canonical_legal(&state).unwrap();
        let first = engine.find_best_action(&state, 4242).unwrap().unwrap();
        assert_eq!(
            first, legal[0],
            "the uniform handcrafted policy must argmax to canonical slot 0"
        );
        let second = engine.find_best_action(&state, 4242).unwrap();
        assert_eq!(
            Some(first),
            second,
            "leaf_policy must replay identically under one seed"
        );
    }

    /// A stub backend: concentrated policy on `pick` (or a hard failure).
    struct PolicyStub {
        pick: usize,
        fail: bool,
    }

    impl crate::leaf::LeafEvaluator for PolicyStub {
        fn evaluate(
            &mut self,
            query: &crate::leaf::LeafQuery,
        ) -> Result<crate::leaf::LeafEval, crate::leaf::LeafEvalError> {
            if self.fail {
                return Err(crate::leaf::LeafEvalError::Inference("stub failure".to_owned()));
            }
            let mut policy = [0.0_f32; crate::leaf::MAX_ACTIONS];
            policy[self.pick] = 0.9;
            // Leak residual mass onto a masked slot — masked-argmax must
            // ignore it.
            for (index, slot) in policy.iter_mut().enumerate() {
                if query.legal_mask[index] == 0.0 {
                    *slot = 0.99;
                    break;
                }
            }
            Ok(crate::leaf::LeafEval { ev: 0.0, policy })
        }
    }

    /// Masked-argmax: the highest-mass *legal* slot wins; mass on a
    /// masked slot can never be selected even when it dominates.
    #[test]
    fn leaf_policy_argmax_respects_the_legal_mask() {
        let state = base_state();
        let legal = crate::leaf::canonical_legal(&state).unwrap();
        let target = legal.len() - 1;
        let engine = Solver::from_config(Config {
            solver: "leaf_policy".to_owned(),
            seed: Some(7),
            ..Config::default()
        })
        .unwrap();
        inject_leaf_evaluator(
            &engine,
            Box::new(PolicyStub {
                pick: target,
                fail: false,
            }),
        );
        let action = engine.find_best_action(&state, 99).unwrap().unwrap();
        assert_eq!(action, legal[target], "the highest-mass legal slot must win");
        // The masked slot carries 0.99 > 0.9 — the pick proves the mask held.
    }

    /// Eval failure degrades to a legal action (the heuristic pick) and
    /// increments the fallback counter — never silent.
    #[test]
    fn leaf_policy_eval_failure_falls_back_legally_and_counts() {
        crate::leaf::eval::reset_leaf_fallback_count();
        let state = base_state();
        let engine = Solver::from_config(Config {
            solver: "leaf_policy".to_owned(),
            seed: Some(7),
            ..Config::default()
        })
        .unwrap();
        inject_leaf_evaluator(&engine, Box::new(PolicyStub { pick: 0, fail: true }));
        let action = engine.find_best_action(&state, 5).unwrap().unwrap();
        assert!(state.legal_actions().contains(&action), "the fallback must be legal");
        assert_eq!(crate::leaf::leaf_fallback_count(), 1, "the degradation must be counted");
    }

    /// policy_argmax: ties break to the earliest canonical slot; masked
    /// slots never compete.
    #[test]
    fn policy_argmax_tie_breaks_to_the_earliest_legal_slot() {
        let mut policy = [0.0_f32; crate::leaf::MAX_ACTIONS];
        policy[0] = 0.5;
        policy[2] = 0.5;
        policy[5] = 1.0; // masked — beyond legal_len
        assert_eq!(policy_argmax(&policy, 3), 0);
        policy[0] = 0.0;
        assert_eq!(policy_argmax(&policy, 3), 2);
    }

    /// Resolving on a bare state must fail closed — the public ledger is
    /// not derivable from zones, so a silent resolve would solve the
    /// wrong game.
    #[test]
    fn resolving_without_the_observation_fails_closed() {
        let state = base_state();
        let engine = Solver::from_config(Config {
            solver: "resolving".to_owned(),
            seed: Some(7),
            ..Config::default()
        })
        .unwrap();
        assert!(matches!(
            engine.find_best_action(&state, 42),
            Err(SolverError::MissingObservation)
        ));
    }

    /// The observed path resolves a real decision to a legal action and
    /// replays identically under one seed.
    #[test]
    fn resolving_decides_a_legal_action_from_the_ledger() {
        let state = base_state();
        let observer = state.active;
        let observation = koi_core::PublicObservation {
            initial: state.public_view(observer),
            entries: Vec::new(),
        };
        let engine = Solver::from_config(Config {
            solver: "resolving".to_owned(),
            resolving: crate::config::ResolvingConfig {
                max_worlds: 8,
                max_nodes: 200_000,
                cfr_iterations: 20,
                max_decision_depth: 2,
                adaptive: false,
                ox: false,
                blueprint_artifact: None,
                leaf_model: None,
            },
            seed: Some(7),
            ..Config::default()
        })
        .unwrap();
        let gadget = crate::resolving::GadgetOracle::Learned;
        let first = engine
            .find_best_action_observed(&state, &observation, &gadget, 4242)
            .unwrap()
            .expect("a live decision resolves");
        assert!(
            state.legal_actions().contains(&first),
            "resolve returned an illegal action"
        );
        let second = engine
            .find_best_action_observed(&state, &observation, &gadget, 4242)
            .unwrap();
        assert_eq!(Some(first), second, "the resolve must replay under one seed");
    }

    #[test]
    fn decisions_replay_bit_for_bit() {
        let state = base_state();
        for solver in ["random", "ismcts", "pimc"] {
            let engine = Solver::from_config(Config {
                solver: solver.to_owned(),
                ismcts: crate::config::IsmctsConfig {
                    iterations: 32,
                    ..Default::default()
                },
                pimc: crate::config::PimcConfig {
                    max_simulations: 8,
                    ..Default::default()
                },
                use_parallel: true,
                seed: Some(9),
                ..Config::default()
            })
            .unwrap();
            let a = engine.find_best_action(&state, 1234).unwrap();
            let b = engine.find_best_action(&state, 1234).unwrap();
            assert_eq!(a, b, "{solver} must replay identically under one seed");
        }
    }

    #[test]
    fn terminal_state_returns_none() {
        let mut state = base_state();
        state.phase = koi_core::TurnPhase::Ended;
        let engine = Solver::from_validated_config(ValidatedConfig::default());
        assert_eq!(engine.find_best_action(&state, 1).unwrap(), None);
    }

    #[test]
    fn invalid_solver_config_fails_construction() {
        assert!(
            Solver::from_config(Config {
                solver: "ismcts".to_owned(),
                ismcts: crate::config::IsmctsConfig {
                    iterations: 0,
                    ..Default::default()
                },
                seed: Some(3),
                ..Config::default()
            })
            .is_err(),
            "a zero-iteration budget must be rejected at construction"
        );
        let _ = Player::South;
    }
}
