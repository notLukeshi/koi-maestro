//! The decision-time continual resolver.
//!
//! At a decision the resolver builds the belief over compatible opponent
//! hands, optionally reweights it by the opponent model's ledger
//! likelihood, compiles the gadget-wrapped subgame, and solves it with
//! CFR+. The observer's root infoset strategy is *sampled* — never the
//! argmax: a resolved equilibrium mixture is genuinely mixed, and
//! collapsing it to a pure action makes the entrant exploitable (P3-D6).
//!
//! The resolve is depth-capped with oracle-valued leaves (P3-D3) so the
//! canonical 48-card residual stays tractable: the retry ladder halves the
//! world budget and then the depth before conceding. On the reduced
//! variants a blueprint gadget makes the resolve *certified*; on the
//! canonical game the learned rollout oracle makes it *empirically
//! guarded* — an honest distinction the evidence reports.
//!
//! The OX arm (P3-D5): when the shrunk archetype posterior clears the
//! gate, each root action is scored by its expected margin under
//! posterior-mixture opponent rollouts, and the argmax replaces the
//! resolved action only when it strictly improves on the resolved row's
//! model-EV. This is empirical exploitation — the gadget bounds the
//! *resolve* arm, the shrunk posterior floor is the only OX safety net.

use koi_core::{Action, KoiGameState, Player};
use rand::{Rng, RngExt};

use crate::cfr::train_cfr_plus;
use crate::efg::compiler::{compile_gadget_subgame, EfgCompileError, MarginOracle};
use crate::efg::tree::{EfgTree, NodeType};
use crate::efg::KoiVariant;
use crate::heuristic;

use super::belief::{build_belief, BeliefError, World};
use super::gadget::MarginAnchors;
use super::model::{Archetype, OpponentModel};
use super::reconstruct::{LedgerReplay, ReconstructError};

/// The resolve budget.
#[derive(Debug, Clone, Copy)]
pub struct ResolveSpec {
    /// Belief cap: maximum opponent-hand worlds per resolve.
    pub max_worlds: usize,
    /// Compiled gadget-tree node budget.
    pub max_nodes: usize,
    /// CFR+ iterations over the gadget tree.
    pub cfr_iterations: usize,
    /// Decision-ply cap below each world root.
    pub max_decision_depth: usize,
}

impl Default for ResolveSpec {
    fn default() -> Self {
        Self {
            max_worlds: 24,
            max_nodes: 600_000,
            cfr_iterations: 300,
            max_decision_depth: 6,
        }
    }
}

/// Why a decision could not be resolved.
#[derive(Debug)]
pub enum ResolveError {
    /// The public zones cannot partition into a consistent hidden pool.
    Belief(BeliefError),
    /// The public ledger could not be replayed — the observation is
    /// malformed or contradicts the kernel.
    Replay(ReconstructError),
    /// The gadget subgame could not be compiled within budget.
    Compile(EfgCompileError),
    /// The tree trained no usable observer root strategy.
    Unresolved,
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Belief(error) => write!(f, "belief construction failed: {error}"),
            Self::Replay(error) => write!(f, "ledger replay failed: {error}"),
            Self::Compile(error) => write!(f, "gadget compilation failed: {error}"),
            Self::Unresolved => f.write_str("the resolve produced no observer root strategy"),
        }
    }
}

impl std::error::Error for ResolveError {}

impl From<BeliefError> for ResolveError {
    fn from(error: BeliefError) -> Self {
        Self::Belief(error)
    }
}

impl From<ReconstructError> for ResolveError {
    fn from(error: ReconstructError) -> Self {
        Self::Replay(error)
    }
}

/// The resolved decision plus its evidence.
#[derive(Debug, Clone)]
pub struct ResolvedDecision {
    /// The sampled root action.
    pub action: Action,
    /// The averaged strategy row at the observer's root infoset.
    pub root_strategy: Vec<f64>,
    /// The actions the row distributes over (the state's legal order).
    pub root_actions: Vec<Action>,
    /// The resolved value for the observer in observer-margin units.
    pub value: f64,
    /// Compiled gadget-tree size.
    pub tree_nodes: usize,
    /// Worlds in the belief.
    pub worlds: usize,
    /// The (possibly model-reweighted) belief the compile ran on — the OX
    /// arm scores actions against these concrete worlds.
    pub belief: Vec<World>,
    /// CFR+ iterations actually run.
    pub cfr_iterations: usize,
}

/// The resolve's domain bundle: which subgame domain to compile, the
/// public anchors, the safety oracle, the budget, and the optional
/// opponent-model inputs (`model` and `replay` engage the reweight
/// together or not at all).
pub struct ResolveContext<'a> {
    /// The subgame domain: the blueprint variant when certified, FULL for
    /// the learned oracle.
    pub variant: &'a KoiVariant,
    /// The public anchors the gadget oracle replays from.
    pub anchors: &'a MarginAnchors,
    /// The oracle pricing gadget opt-outs and depth-capped leaves.
    pub oracle: &'a dyn MarginOracle,
    /// The resolve budget.
    pub spec: &'a ResolveSpec,
    /// The opponent model — `Some` engages the belief reweight.
    pub model: Option<&'a OpponentModel>,
    /// The ledger replay the model scores — required when `model` is set.
    pub replay: Option<&'a LedgerReplay>,
    /// The configured leaf evaluator — `Some` prices oracle leaves through
    /// the model under each rung's posterior; per-call failures degrade to
    /// the fallback oracle (and count) rather than failing the resolve.
    pub leaf_evaluator: Option<&'a std::cell::RefCell<Box<dyn crate::leaf::LeafEvaluator>>>,
}

/// A zero-margin oracle for the ladder's pre-flight check: the gadget's
/// *structure* never depends on the oracle's values, so a probe compile
/// hits the node budget exactly where the real compile would — without
/// paying the learned oracle's per-leaf rollouts on rungs that provably
/// cannot fit.
struct ProbeOracle;

impl MarginOracle for ProbeOracle {
    fn margin_south(&self, _state: &KoiGameState, _history: &[u64]) -> Option<f64> {
        Some(0.0)
    }
}

/// Resolves the observer's decision at `state`.
///
/// The retry ladder halves the world budget then the depth on
/// `NodeBudgetExceeded`, rebuilding the belief at each rung so sampled
/// worlds stay unbiased. Each rung is pre-flighted with a zero-margin
/// probe compile: identical tree shape, a fraction of the cost of a real
/// compile, so futile rungs are skipped instead of burning a capped
/// expansion.
pub fn resolve_decision(
    state: &KoiGameState,
    observer: Player,
    ctx: &ResolveContext<'_>,
    rng: &mut impl Rng,
) -> Result<ResolvedDecision, ResolveError> {
    if state.active != observer || state.is_ended() {
        // A resolve anchors on the acting seat's observation — anything
        // else would solve a game the caller is not actually in.
        return Err(ResolveError::Unresolved);
    }
    let ladder = [
        (ctx.spec.max_worlds, ctx.spec.max_decision_depth),
        (ctx.spec.max_worlds / 2, ctx.spec.max_decision_depth),
        (ctx.spec.max_worlds / 4, ctx.spec.max_decision_depth),
        (4, 2),
        (2, 2),
        (2, 1),
    ];
    let mut last_error = None;
    for (world_cap, depth) in ladder {
        if world_cap == 0 || depth == 0 {
            break;
        }
        let mut worlds = match build_belief(state, observer, ctx.variant.cards, world_cap, rng) {
            Ok(worlds) => worlds,
            Err(error) => return Err(ResolveError::Belief(error)),
        };
        if let (Some(model), Some(replay)) = (ctx.model, ctx.replay) {
            model.reweight(&mut worlds, replay, observer);
        }
        match attempt_resolve(state, observer, &worlds, depth, ctx, rng) {
            Ok(decision) => return Ok(decision),
            Err(error @ ResolveError::Compile(EfgCompileError::NodeBudgetExceeded { .. })) => {
                last_error = Some(error);
                continue;
            }
            Err(error) => return Err(error),
        }
    }
    Err(last_error.unwrap_or(ResolveError::Unresolved))
}

/// Resolves `state` under a caller-supplied world set — the data-labeler
/// path. The resolve, the emitted belief, and the EV all live under the
/// exact posterior `worlds` carries (natural or a perturbed random range —
/// the random-range regularization needs the solve itself to live under
/// the perturbed weights, which an internally rebuilt belief cannot honor).
///
/// The retry ladder reduces depth only: the caller fixed the belief, so
/// the world budget cannot be retried without re-sampling.
pub fn resolve_decision_with_worlds(
    state: &KoiGameState,
    observer: Player,
    worlds: &[World],
    ctx: &ResolveContext<'_>,
    rng: &mut impl Rng,
) -> Result<ResolvedDecision, ResolveError> {
    if state.active != observer || state.is_ended() || worlds.is_empty() {
        return Err(ResolveError::Unresolved);
    }
    let mut worlds = worlds.to_vec();
    if let (Some(model), Some(replay)) = (ctx.model, ctx.replay) {
        model.reweight(&mut worlds, replay, observer);
    }
    let mut depths = vec![ctx.spec.max_decision_depth];
    for rung in [2, 1] {
        if rung < ctx.spec.max_decision_depth {
            depths.push(rung);
        }
    }
    let mut last_error = None;
    for depth in depths {
        match attempt_resolve(state, observer, &worlds, depth, ctx, rng) {
            Ok(decision) => return Ok(decision),
            Err(error @ ResolveError::Compile(EfgCompileError::NodeBudgetExceeded { .. })) => {
                last_error = Some(error);
                continue;
            }
            Err(error) => return Err(error),
        }
    }
    Err(last_error.unwrap_or(ResolveError::Unresolved))
}

/// One gadget compile + solve at `depth` over `worlds` — the shared body of
/// the belief-rebuilding ladder and the explicit-worlds path. The probe
/// pre-flight and the per-rung leaf belief block live here so both paths
/// pay exactly the same checks.
fn attempt_resolve(
    state: &KoiGameState,
    observer: Player,
    worlds: &[World],
    depth: usize,
    ctx: &ResolveContext<'_>,
    rng: &mut impl Rng,
) -> Result<ResolvedDecision, ResolveError> {
    let weighted: Vec<(KoiGameState, f64)> = worlds.iter().map(|world| (world.state, world.weight)).collect();
    if compile_gadget_subgame(
        ctx.variant,
        &weighted,
        ctx.anchors.initial_field,
        ctx.anchors.history_prefix.clone(),
        &ProbeOracle,
        depth,
        ctx.spec.max_nodes,
        None,
    )
    .is_err_and(|error| matches!(error, EfgCompileError::NodeBudgetExceeded { .. }))
    {
        // This rung provably cannot fit — skip the expensive compile.
        return Err(ResolveError::Compile(EfgCompileError::NodeBudgetExceeded {
            budget: ctx.spec.max_nodes,
        }));
    }
    // With a leaf evaluator configured, oracle leaves price through the
    // model under THIS rung's posterior — the belief block is rebuilt per
    // rung because the reweight moves mass between worlds. Depth-capped
    // leaves are collected and batch-evaluated after expansion; the root
    // opt-out calls still price individually through the margin oracle.
    let leaf_oracle;
    let leaf_collector;
    let oracle: &dyn crate::efg::compiler::MarginOracle = match &ctx.leaf_evaluator {
        Some(evaluator) => {
            let unseen = crate::leaf::unseen_mask_in(state, ctx.variant.cards);
            let weights: Vec<f64> = worlds.iter().map(|world| world.weight.max(0.0)).collect();
            let belief = crate::leaf::card_marginals(unseen, worlds, &weights);
            leaf_collector = std::cell::RefCell::new(Vec::new());
            leaf_oracle = crate::resolving::gadget::LeafMarginOracle::new(evaluator, belief, ctx.oracle);
            &leaf_oracle
        }
        None => {
            leaf_collector = std::cell::RefCell::new(Vec::new());
            ctx.oracle
        }
    };
    let mut tree = compile_gadget_subgame(
        ctx.variant,
        &weighted,
        ctx.anchors.initial_field,
        ctx.anchors.history_prefix.clone(),
        oracle,
        depth,
        ctx.spec.max_nodes,
        ctx.leaf_evaluator.as_ref().map(|_| &leaf_collector),
    )
    .map_err(ResolveError::Compile)?;
    // Batch-evaluate collected leaves and patch the placeholder terminals.
    if let Some(evaluator) = &ctx.leaf_evaluator {
        let collected = leaf_collector.into_inner();
        if !collected.is_empty() {
            let mut evaluator = evaluator.borrow_mut();
            let unseen = crate::leaf::unseen_mask_in(state, ctx.variant.cards);
            let weights: Vec<f64> = worlds.iter().map(|world| world.weight.max(0.0)).collect();
            let belief = crate::leaf::card_marginals(unseen, worlds, &weights);
            let mut queries = Vec::with_capacity(collected.len());
            let mut fallback_values: Vec<Option<f64>> = Vec::with_capacity(collected.len());
            for (_, leaf_state, history) in collected.iter() {
                match crate::leaf::canonical_legal(leaf_state) {
                    Some(legal) => {
                        queries.push(crate::leaf::LeafQuery {
                            features: crate::leaf::encode(leaf_state, &belief, &legal),
                            legal_mask: crate::leaf::legal_mask(&legal).map(|bit| bit as u8 as f32),
                        });
                        fallback_values.push(None);
                    }
                    None => {
                        // Leaf exceeds the policy head — degrade to the
                        // rollout oracle exactly like leaf_eval_for_state.
                        crate::leaf::note_leaf_fallback();
                        let value = ctx.oracle.margin_south(leaf_state, history);
                        fallback_values.push(value);
                    }
                }
            }
            let eval_started = std::time::Instant::now();
            let batch_result = evaluator.evaluate_batch(&queries);
            crate::leaf::eval::charge_leaf_eval_micros(eval_started.elapsed());
            match batch_result {
                Ok(evals) => {
                    let mut eval_iter = evals.into_iter();
                    for (index, (node_id, leaf_state, _)) in collected.iter().enumerate() {
                        let value = match fallback_values[index] {
                            Some(fallback) => fallback,
                            None => {
                                let eval = eval_iter.next().expect("one eval per collected query");
                                match leaf_state.active {
                                    Player::South => f64::from(eval.ev),
                                    Player::North => f64::from(-eval.ev),
                                }
                            }
                        };
                        tree.set_terminal_value(*node_id, value);
                    }
                }
                Err(_) => {
                    // Batch inference failed — degrade every collected leaf
                    // to the rollout oracle, matching the per-call path.
                    for (index, (node_id, leaf_state, history)) in collected.iter().enumerate() {
                        let value = match fallback_values[index] {
                            Some(fallback) => fallback,
                            None => {
                                crate::leaf::note_leaf_fallback();
                                ctx.oracle.margin_south(leaf_state, history).unwrap_or(0.0)
                            }
                        };
                        tree.set_terminal_value(*node_id, value);
                    }
                }
            }
        }
    }
    finish_resolve(state, observer, &tree, ctx.spec.cfr_iterations, worlds.to_vec(), rng)
}

/// Trains the gadget tree and samples the observer's root strategy.
fn finish_resolve(
    state: &KoiGameState,
    observer: Player,
    tree: &EfgTree,
    cfr_iterations: usize,
    worlds: Vec<World>,
    rng: &mut impl Rng,
) -> Result<ResolvedDecision, ResolveError> {
    let blueprint = train_cfr_plus(tree, cfr_iterations);
    let profile = blueprint.averaged_profile();

    // The observer's root infoset: under the root chance, each world's
    // enter-branch child shares one observer infoset (identical
    // observations). Find the first enter child that is a decision node.
    let root_infoset = match tree.node(tree.root()) {
        NodeType::Chance { outcomes } => outcomes
            .iter()
            .filter_map(|outcome| match tree.node(outcome.child) {
                NodeType::Decision { children, .. } => children.get(1).copied(),
                _ => None,
            })
            .find_map(|enter| match tree.node(enter) {
                NodeType::Decision { infoset, .. } => Some(*infoset),
                _ => None,
            }),
        _ => None,
    }
    .ok_or(ResolveError::Unresolved)?;

    let infoset = &tree.infosets()[root_infoset];
    let row = profile.get(root_infoset).ok_or(ResolveError::Unresolved)?;
    if row.len() != infoset.actions.len() || row.iter().any(|p| !p.is_finite() || *p < 0.0) {
        return Err(ResolveError::Unresolved);
    }
    let legal = state.legal_actions();
    if infoset.actions.as_slice() != legal.as_slice() {
        // The resolved row must distribute over the observer's real legal
        // set — a mismatch means the anchors leaked hidden state.
        return Err(ResolveError::Unresolved);
    }

    // Sample from the resolved mixture — never argmax (P3-D6).
    let draw: f64 = rng.random_range(0.0..1.0);
    let mut cumulative = 0.0;
    let mut picked = infoset.actions.len() - 1;
    for (index, weight) in row.iter().enumerate() {
        cumulative += weight;
        if draw < cumulative {
            picked = index;
            break;
        }
    }
    let action = infoset.actions[picked];

    // The resolved value at the root infoset: belief×enter-weighted over
    // the members — each world contributes P(world)·P(enter|gadget) times
    // its subtree value under the profile. Members share the strategy row
    // but not the value (worlds differ), so a single member would bias the
    // diagnostic. When the opponent opts out everywhere the subtree never
    // plays — fall back to the unweighted member average.
    let mut weighted = 0.0;
    let mut weight_total = 0.0;
    let mut member_values: Vec<f64> = Vec::with_capacity(infoset.members.len());
    if let NodeType::Chance { outcomes } = tree.node(tree.root()) {
        for outcome in outcomes {
            if let NodeType::Decision {
                infoset: gadget_infoset,
                children,
                ..
            } = tree.node(outcome.child)
            {
                let enter = children.get(1).copied();
                if let Some(member) = enter.filter(
                    |node| matches!(tree.node(*node), NodeType::Decision { infoset: id, .. } if *id == root_infoset),
                ) {
                    let member_value = match tree.node(member) {
                        NodeType::Decision { children, .. } => children
                            .iter()
                            .enumerate()
                            .map(|(i, child)| row[i] * node_value(tree, *child, &profile, observer))
                            .sum(),
                        _ => return Err(ResolveError::Unresolved),
                    };
                    member_values.push(member_value);
                    let w = outcome.probability * profile[*gadget_infoset].get(1).copied().unwrap_or(0.0);
                    weighted += w * member_value;
                    weight_total += w;
                }
            }
        }
    }
    let value = if weight_total > 0.0 {
        weighted / weight_total
    } else if member_values.is_empty() {
        return Err(ResolveError::Unresolved);
    } else {
        member_values.iter().sum::<f64>() / member_values.len() as f64
    };

    Ok(ResolvedDecision {
        action,
        root_strategy: row.clone(),
        root_actions: infoset.actions.clone(),
        value,
        tree_nodes: tree.node_count(),
        worlds: worlds.len(),
        belief: worlds,
        cfr_iterations,
    })
}

/// The expected `player` margin under the solved profile below `node`.
fn node_value(tree: &EfgTree, node: usize, profile: &[Vec<f64>], player: Player) -> f64 {
    match tree.node(node) {
        NodeType::Terminal { utility_south } => match player {
            Player::South => *utility_south,
            Player::North => -*utility_south,
        },
        NodeType::Chance { outcomes } => outcomes
            .iter()
            .map(|outcome| outcome.probability * node_value(tree, outcome.child, profile, player))
            .sum(),
        NodeType::Decision { infoset, children, .. } => {
            let row = &profile[*infoset];
            children
                .iter()
                .enumerate()
                .map(|(i, child)| row.get(i).copied().unwrap_or(0.0) * node_value(tree, *child, profile, player))
                .sum()
        }
    }
}

/// The OX arm: the exploitative root action under the shrunk archetype
/// posterior, or `None` when no action strictly improves on the resolved
/// row's model-EV.
///
/// `EV(a) = Σ_w w_w · Σ_a' p_a' · rollout(w after a, opp = a')` — the
/// opponent plays each posterior archetype deterministically (the `Random`
/// archetype plays its canonical first legal action), the observer plays
/// the P1 heuristic; rollouts run to the round's end inside each world
/// (deterministic — a world's stock is fixed).
pub fn ox_action(
    observer: Player,
    worlds: &[World],
    posterior: &[(Archetype, f64)],
    resolved: &ResolvedDecision,
) -> Option<Action> {
    let support: Vec<(Archetype, f64)> = posterior.iter().filter(|(_, p)| *p > 1e-4).copied().collect();
    if support.is_empty() || worlds.is_empty() {
        return None;
    }
    // One EV evaluation per root action — the applied transition is
    // archetype-independent, so it hoists out of the support loop.
    let evs: Vec<f64> = resolved
        .root_actions
        .iter()
        .map(|action| {
            worlds
                .iter()
                .map(|world| {
                    let next = world.state.apply_action(*action);
                    let inner: f64 = support
                        .iter()
                        .map(|(archetype, mass)| {
                            let v = next.map_or(0.0, |s| rollout_margin(&s, observer, *archetype));
                            mass * v
                        })
                        .sum();
                    world.weight * inner
                })
                .sum()
        })
        .collect();
    let resolved_ev: f64 = resolved
        .root_strategy
        .iter()
        .zip(&evs)
        .map(|(weight, ev)| weight * ev)
        .sum();
    let mut best: Option<(f64, u64, Action)> = None;
    for (action, &ev) in resolved.root_actions.iter().zip(&evs) {
        let key = action.action_key();
        best = match best {
            Some((best_ev, best_key, _)) if ev <= best_ev || (ev == best_ev && key >= best_key) => best,
            _ => Some((ev, key, *action)),
        };
    }
    let (ox_ev, _, ox) = best?;
    if ox_ev > resolved_ev + 1e-9 {
        Some(ox)
    } else {
        None
    }
}

/// The deterministic in-world rollout: observer by the P1 heuristic, the
/// opponent by `archetype` (`Random` takes the canonical first legal
/// action — a deterministic surrogate, not a sampled uniform). The stock
/// tail is permuted per world first — the canonical order would feed every
/// rollout the same card-index-correlated draw sequence. Margin in
/// observer units.
fn rollout_margin(state: &KoiGameState, observer: Player, archetype: Archetype) -> f64 {
    let mut state = *state;
    super::gadget::permute_stock_for_rollout(&mut state);
    let mut guard = 0usize;
    while !state.is_ended() && guard < 96 {
        let action = if state.active == observer {
            match heuristic::find_best_action(&state) {
                Some(action) => action,
                None => break,
            }
        } else {
            archetype.pick(&state).unwrap_or_else(|| state.legal_actions()[0])
        };
        state = match state.apply_action(action) {
            Ok(next) => next,
            Err(_) => break,
        };
        guard += 1;
    }
    state.leg_margin(observer) as f64
}

#[cfg(test)]
mod tests {
    use rand::rngs::SmallRng;
    use rand::SeedableRng;

    use koi_core::{Card, CardSet, TurnPhase};

    use super::*;
    use crate::resolving::gadget::{learned_margin_south, GadgetOracle, MarginAnchors};

    fn live_state() -> KoiGameState {
        for seed in 0..u64::MAX {
            let (state, anomaly) =
                KoiGameState::new_deal(koi_core::deal_from_seed(seed), koi_core::Ruleset::nintendo());
            if anomaly.is_none() {
                return state;
            }
        }
        unreachable!()
    }

    /// A dealt NANO_6 state drawn from the compiled variant's own chance
    /// root — the same partition the blueprint was trained on.
    fn nano_deal() -> KoiGameState {
        use crate::efg::compiler::compile_variant;
        use crate::efg::tree::NodeType;

        let variant = &KoiVariant::NANO_6;
        let tree = compile_variant(variant, Player::South).unwrap();
        let NodeType::Chance { outcomes } = tree.node(tree.root()) else {
            panic!("variant root must be a chance node");
        };
        let hand = usize::from(variant.hand_size);
        let field_n = usize::from(variant.field_size);
        let to_set = |indices: &[u8]| {
            indices
                .iter()
                .fold(CardSet::EMPTY, |set, i| set.insert(Card::new(*i).unwrap()))
        };
        let dealt = &outcomes[0].dealt;
        let south = to_set(&dealt[..hand]);
        let north = to_set(&dealt[hand..hand * 2]);
        let field = to_set(&dealt[hand * 2..hand * 2 + field_n]);
        let stock: Vec<Card> = variant
            .cards
            .difference(south.union(north).union(field))
            .into_iter()
            .collect();
        let (state, anomaly) = KoiGameState::new_from_parts(
            &stock,
            [south, north],
            field,
            Player::South,
            Player::South,
            variant.rules,
        );
        assert!(anomaly.is_none());
        state
    }

    /// The certified lane must resolve under a decision-depth cap and at a
    /// stock-resolution root — blueprint lookups key on the true history
    /// (prefix + pending draw + path), not the resolve-root prefix alone
    /// (correctness review DEFECT 1: previously every capped leaf and
    /// every ResolveStock opt-out missed the blueprint and the resolve
    /// failed OracleUnavailable).
    #[test]
    fn blueprint_oracle_resolves_depth_capped_and_stock_resolution() {
        use crate::cfr::cfr_plus::train_cfr_plus;
        use crate::efg::compiler::compile_variant;
        use crate::resolving::gadget::SafetyGadget;
        use std::sync::Arc;

        let variant = &KoiVariant::NANO_6;
        let tree = compile_variant(variant, Player::South).unwrap();
        let blueprint = train_cfr_plus(&tree, 300);
        let gadget = SafetyGadget::new(Arc::new(tree), &blueprint);
        let oracle = GadgetOracle::Blueprint(Arc::new(gadget));
        let spec = ResolveSpec {
            max_worlds: 8,
            max_nodes: 50_000,
            cfr_iterations: 40,
            max_decision_depth: 1,
        };
        let mut rng = SmallRng::seed_from_u64(9);

        // Deal-root resolve: every depth-1 leaf is an oracle lookup.
        let state = nano_deal();
        let observer = state.active;
        let anchors = MarginAnchors {
            initial_field: state.field,
            history_prefix: Vec::new(),
        };
        let bound = oracle.oracle(&anchors);
        let ctx = ResolveContext {
            variant,
            anchors: &anchors,
            oracle: &bound,
            spec: &spec,
            model: None,
            replay: None,
            leaf_evaluator: None,
        };
        let decision = resolve_decision(&state, observer, &ctx, &mut rng).expect("depth-capped certified resolve");
        assert!(state.legal_actions().contains(&decision.action));

        // Stock-resolution root: the pending draw belongs in the oracle's
        // lookup stream — the resolve enters mid-resolution.
        let Action::PlayFromHand { .. } = decision.action else {
            panic!("a deal-root decision is always a hand play");
        };
        let mid = state.apply_action(decision.action).unwrap();
        let TurnPhase::AwaitingStockResolution { .. } = mid.phase else {
            panic!("a hand play must enter stock resolution");
        };
        let anchors = MarginAnchors {
            initial_field: state.field,
            history_prefix: vec![decision.action.action_key()],
        };
        let bound = oracle.oracle(&anchors);
        let ctx = ResolveContext {
            variant,
            anchors: &anchors,
            oracle: &bound,
            spec: &spec,
            model: None,
            replay: None,
            leaf_evaluator: None,
        };
        let decision = resolve_decision(&mid, mid.active, &ctx, &mut rng).expect("stock-resolution certified resolve");
        assert!(mid.legal_actions().contains(&decision.action));
    }

    /// The learned-oracle resolve produces a legal action and a valid
    /// strategy row on the canonical game.
    #[test]
    fn resolve_returns_a_legal_sampled_action() {
        let state = live_state();
        let observer = state.active;
        let anchors = MarginAnchors {
            initial_field: state.field,
            history_prefix: Vec::new(),
        };
        let oracle = GadgetOracle::Learned.oracle(&anchors);
        let spec = ResolveSpec {
            max_worlds: 8,
            cfr_iterations: 60,
            max_decision_depth: 4,
            ..Default::default()
        };
        let ctx = ResolveContext {
            variant: &KoiVariant::FULL,
            anchors: &anchors,
            oracle: &oracle,
            spec: &spec,
            model: None,
            replay: None,
            leaf_evaluator: None,
        };
        let mut rng = SmallRng::seed_from_u64(5);
        let decision = resolve_decision(&state, observer, &ctx, &mut rng).unwrap();
        assert!(state.legal_actions().contains(&decision.action));
        assert_eq!(decision.root_actions.len(), decision.root_strategy.len());
        let mass: f64 = decision.root_strategy.iter().sum();
        assert!((mass - 1.0).abs() < 1e-6, "root row must be a distribution: {mass}");
    }

    /// The resolved row is a genuine mixture: sampling must replay under
    /// the same seed (per-decision determinism contract).
    #[test]
    fn resolve_replays_under_the_same_seed() {
        let state = live_state();
        let observer = state.active;
        let anchors = MarginAnchors {
            initial_field: state.field,
            history_prefix: Vec::new(),
        };
        let oracle = GadgetOracle::Learned.oracle(&anchors);
        let spec = ResolveSpec {
            max_worlds: 8,
            cfr_iterations: 40,
            max_decision_depth: 4,
            ..Default::default()
        };
        let ctx = ResolveContext {
            variant: &KoiVariant::FULL,
            anchors: &anchors,
            oracle: &oracle,
            spec: &spec,
            model: None,
            replay: None,
            leaf_evaluator: None,
        };
        let run = |seed: u64| {
            let mut rng = SmallRng::seed_from_u64(seed);
            resolve_decision(&state, observer, &ctx, &mut rng).unwrap().action
        };
        assert_eq!(run(7), run(7));
    }

    /// A configured leaf evaluator prices the gadget's oracle leaves — the
    /// resolve completes through `LeafMarginOracle` and returns a legal
    /// action. `builtin:handcrafted` exercises the whole contract without
    /// an ONNX artifact.
    #[test]
    fn resolve_prices_leaves_through_the_leaf_evaluator() {
        let state = live_state();
        let observer = state.active;
        let anchors = MarginAnchors {
            initial_field: state.field,
            history_prefix: Vec::new(),
        };
        let oracle = GadgetOracle::Learned.oracle(&anchors);
        let spec = ResolveSpec {
            max_worlds: 8,
            cfr_iterations: 40,
            max_decision_depth: 4,
            ..Default::default()
        };
        let evaluator =
            std::cell::RefCell::new(crate::leaf::open_leaf_evaluator(crate::leaf::BUILTIN_LEAF_MODEL).unwrap());
        let ctx = ResolveContext {
            variant: &KoiVariant::FULL,
            anchors: &anchors,
            oracle: &oracle,
            spec: &spec,
            model: None,
            replay: None,
            leaf_evaluator: Some(&evaluator),
        };
        let mut rng = SmallRng::seed_from_u64(5);
        let decision = resolve_decision(&state, observer, &ctx, &mut rng).unwrap();
        assert!(state.legal_actions().contains(&decision.action));
    }

    /// A leaf evaluator that fails every call degrades to the fallback
    /// oracle per call — the resolve still completes and the failure is
    /// counted, never silent.
    #[test]
    fn a_failing_leaf_evaluator_degrades_to_the_oracle_and_counts() {
        use crate::leaf::{LeafEval, LeafEvalError, LeafEvaluator, LeafQuery};

        struct AlwaysFail;
        impl LeafEvaluator for AlwaysFail {
            fn evaluate(&mut self, _query: &LeafQuery) -> Result<LeafEval, LeafEvalError> {
                Err(LeafEvalError::Inference("synthetic failure".to_owned()))
            }
        }

        let state = live_state();
        let observer = state.active;
        let anchors = MarginAnchors {
            initial_field: state.field,
            history_prefix: Vec::new(),
        };
        let oracle = GadgetOracle::Learned.oracle(&anchors);
        let spec = ResolveSpec {
            max_worlds: 4,
            cfr_iterations: 20,
            max_decision_depth: 2,
            ..Default::default()
        };
        let evaluator: std::cell::RefCell<Box<dyn LeafEvaluator>> = std::cell::RefCell::new(Box::new(AlwaysFail));
        let ctx = ResolveContext {
            variant: &KoiVariant::FULL,
            anchors: &anchors,
            oracle: &oracle,
            spec: &spec,
            model: None,
            replay: None,
            leaf_evaluator: Some(&evaluator),
        };
        crate::leaf::eval::reset_leaf_fallback_count();
        let mut rng = SmallRng::seed_from_u64(5);
        let decision = resolve_decision(&state, observer, &ctx, &mut rng).unwrap();
        assert!(state.legal_actions().contains(&decision.action));
        assert!(
            crate::leaf::leaf_fallback_count() > 0,
            "every leaf query failed — the oracle path must have degraded at least once"
        );
    }

    /// OX never fires below the strict-improvement bar: a resolved row
    /// already on the model-optimal action returns None.
    #[test]
    fn ox_respects_the_improvement_floor() {
        let state = live_state();
        let observer = state.active;
        let worlds = vec![World {
            opponent_hand: koi_core::CardSet::EMPTY,
            stock_tail: Vec::new(),
            weight: 1.0,
            state,
        }];
        let resolved = ResolvedDecision {
            action: state.legal_actions()[0],
            root_strategy: vec![1.0],
            root_actions: vec![state.legal_actions()[0]],
            value: 0.0,
            tree_nodes: 0,
            worlds: 1,
            belief: worlds.clone(),
            cfr_iterations: 0,
        };
        let posterior = vec![(Archetype::Heuristic, 1.0)];
        // Single-action root: OX cannot improve on the only legal action.
        assert!(ox_action(observer, &worlds, &posterior, &resolved).is_none());
    }

    /// The learned oracle's margin is finite and bounded.
    #[test]
    fn learned_margin_is_bounded() {
        let state = live_state();
        let margin = learned_margin_south(&state);
        assert!(margin.is_finite() && margin.abs() <= 64.0);
    }
}
