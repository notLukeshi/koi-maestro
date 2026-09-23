//! Deterministic per-row generation for the synthetic data pipeline.
//!
//! Each row is a pure function of `(global_seed, shard_id, row_index,
//! attempt)` through splitmix64 — the shard content is byte-identical
//! regardless of the Rayon thread count or row evaluation order. A row that
//! cannot be labelled (terminal state, oversized action list, or a state
//! outside the compiled tree) retries with the next attempt seed and is
//! reported as skipped after [`MAX_ROW_ATTEMPTS`].
//!
//! Ported from a sibling research codebase's `learn/generate.rs`. The koi replay tracks
//! the public event stream itself — hand-play keys, pending-draw keys,
//! resolution keys — because the blueprint anchors and the resolve
//! context's `MarginAnchors` both key on it, and a synthetic trajectory
//! has no ledger to reconstruct it from.

use koi_core::{deal_from_seed, Card, CardSet, KoiGameState, Player, Ruleset, TurnPhase};
use koi_solver::{
    efg::{
        tree::{draw_event_key, EfgTree},
        KoiVariant,
    },
    leaf::{
        actions::{canonical_legal, legal_mask, MAX_ACTIONS},
        features::{encode, CARD_BLOCK, FEATURE_DIM},
    },
    resolving::{build_belief, MarginAnchors, SafetyGadget},
};
use rand::{rngs::SmallRng, Rng, RngExt, SeedableRng};

use super::labels::{label_state, label_state_resolved, random_member_weights, BlueprintTarget};

/// Bounded retries per row before it is counted as skipped.
pub const MAX_ROW_ATTEMPTS: u32 = 8;

/// splitmix64 — the deterministic seed splitter (same schedule as the sibling codebase).
pub fn mix_seed(global: u64, shard: usize, row: usize, attempt: u32) -> u64 {
    let mut z = global
        .wrapping_add((shard as u64) << 40)
        .wrapping_add(row as u64)
        .wrapping_add((attempt as u64) << 56);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// Deals a state on `variant`'s card universe: shuffled subset dealt into
/// hands/field/stock via `new_from_parts`. `None` on a deal anomaly —
/// anomalous layouts are excluded from the corpus exactly as
/// `compile_variant` excludes them from the tree's chance root.
pub fn deal_variant(variant: &KoiVariant, rng: &mut impl Rng) -> Option<KoiGameState> {
    if *variant == KoiVariant::FULL {
        let (state, anomaly) = KoiGameState::new_deal(deal_from_seed(rng.random()), Ruleset::nintendo());
        return anomaly.is_none().then_some(state);
    }
    let mut deck: Vec<Card> = variant.cards.into_iter().collect();
    // Fisher–Yates on the variant's universe.
    for i in (1..deck.len()).rev() {
        let j = rng.random_range(0..=i);
        deck.swap(i, j);
    }
    let hand = usize::from(variant.hand_size);
    let field_n = usize::from(variant.field_size);
    let to_set = |cards: &[Card]| cards.iter().fold(CardSet::EMPTY, |set, c| set.insert(*c));
    let south = to_set(&deck[..hand]);
    let north = to_set(&deck[hand..hand * 2]);
    let field = to_set(&deck[hand * 2..hand * 2 + field_n]);
    let stock = &deck[hand * 2 + field_n..];
    let (state, anomaly) = KoiGameState::new_from_parts(
        stock,
        [south, north],
        field,
        Player::South,
        Player::South,
        variant.rules,
    );
    anomaly.is_none().then_some(state)
}

/// Replays `state` to a uniformly stratified decision ply, tracking the
/// public event stream into `history` and leaving the state's initial
/// field in `initial_field`. The replay mix is `guided_sample`-fraction
/// heuristic-guided, uniform otherwise — stratified coverage of both the
/// realistic distribution and off-policy branches. Landing on the terminal
/// keeps the pre-move decision state (the final-move bucket stays at its
/// natural frequency instead of absorbing every overshoot).
fn replay_to_ply(
    ctx_seed_rng: &mut impl Rng,
    variant: &KoiVariant,
    guided_sample: f64,
    state: &mut KoiGameState,
    history: &mut Vec<u64>,
) -> usize {
    let total_plies = usize::from(variant.hand_size) * 2;
    let target_ply = ctx_seed_rng.random_range(0..total_plies.max(1));
    let mut ply = 0_usize;
    while ply < target_ply && !state.is_ended() {
        let legal = state.legal_actions();
        if legal.is_empty() {
            break;
        }
        let action = if ctx_seed_rng.random::<f64>() < guided_sample {
            koi_solver::heuristic::find_best_action(state)
                .unwrap_or_else(|| legal[ctx_seed_rng.random_range(0..legal.len())])
        } else {
            legal[ctx_seed_rng.random_range(0..legal.len())]
        };
        let Ok(next) = state.apply_action(action) else {
            break;
        };
        history.push(action.action_key());
        // A pending draw is a public fact of the stream — keyed like the
        // compiler's chance events so blueprint anchors see the true prefix.
        if let TurnPhase::AwaitingStockResolution { drawn } = next.phase {
            history.push(draw_event_key(drawn));
        }
        if next.is_ended() {
            break;
        }
        *state = next;
        ply += 1;
    }
    ply
}

/// The label context shared by every row of a blueprint-method run.
pub struct LabelCtx<'a> {
    pub variant: &'a KoiVariant,
    pub tree: &'a EfgTree,
    pub profile: &'a [Vec<f64>],
    pub gadget: &'a SafetyGadget,
    pub seed: u64,
    pub max_worlds: usize,
    pub belief_perturb: f64,
    /// Probability of heuristic-guided (vs uniform) action sampling during
    /// state replay.
    pub guided_sample: f64,
    /// Self-play generation epoch, packed into the provenance key.
    pub generation_iter: u32,
    /// Solver budget recorded in provenance keys (CFR iterations or 0 for LP).
    pub solver_budget: u64,
}

/// One fully labelled dataset row.
pub struct Row {
    pub features: [f32; FEATURE_DIM],
    pub mask: [bool; MAX_ACTIONS],
    pub policy: [f32; MAX_ACTIONS],
    pub ev: f32,
    pub belief: [f32; CARD_BLOCK],
    pub provenance: u64,
    pub perturbed: bool,
}

/// Rare-state bucket flags — the coverage taxonomy the corpus must visibly
/// contain. A row can carry several flags; the manifest counts each
/// membership. Koi's rare classes are positional: the strategic weight sits
/// in stop calls, formed-yaku pressure, and the exhausted-stock endgame.
pub mod bucket {
    /// The actor faces a Koi-Koi/Shobu stop decision — the risk call the
    /// leaf evaluator most needs to price.
    pub const STOP_CALL: u8 = 1;
    /// The opponent already holds a formed yaku (score_yaku > 0) — the
    /// position is under live stop pressure.
    pub const YAKU_PRESSURE: u8 = 2;
    /// The deciding tail of the leg: both seats are down to their last two
    /// hand cards. (The stock never empties — 24 cards vs 16 draws — so a
    /// "deck exhausted" bucket would be unreachable on the real game.)
    pub const ENDGAME: u8 = 4;
}

/// Classifies a state's bucket membership. Positional only — reads the
/// phase, the opponent's captured pile, and the unseen partition.
pub fn classify_buckets(state: &KoiGameState) -> u8 {
    let mut flags = 0_u8;
    if matches!(state.phase, TurnPhase::AwaitingStopDecision { .. }) {
        flags |= bucket::STOP_CALL;
    }
    let opponent = state.active.opponent();
    if koi_core::yaku::score_yaku(state.captured[opponent.index()], &state.rules) > 0 {
        flags |= bucket::YAKU_PRESSURE;
    }
    if state.hands[0].count() + state.hands[1].count() <= 4 {
        flags |= bucket::ENDGAME;
    }
    flags
}

/// Deterministically assigns a row's bucket target: `frac` of rows are
/// targeted at one of the three rare-state classes (equal shares), the
/// rest sample naturally. The draw folds a fixed domain tag into
/// `mix_seed` so a row's target is stable across attempts and thread counts.
fn bucket_target(seed: u64, shard: usize, row: usize, frac: f64) -> u8 {
    if frac <= 0.0 {
        return 0;
    }
    let mut rng = SmallRng::seed_from_u64(mix_seed(seed ^ 0xB0C4_E719, shard, row, 0));
    let draw: f64 = rng.random();
    if draw < frac / 3.0 {
        bucket::STOP_CALL
    } else if draw < 2.0 * frac / 3.0 {
        bucket::YAKU_PRESSURE
    } else if draw < frac {
        bucket::ENDGAME
    } else {
        0
    }
}

/// The provenance key packed into every row: generation epoch, solver
/// budget, landing ply, and the row's mixed seed — enough to replay the
/// exact sample that produced the row.
fn provenance_key(generation_iter: u32, solver_budget: u64, ply: usize, seed: u64) -> u64 {
    ((generation_iter as u64 & 0xFF) << 56)
        | ((solver_budget.min(0xFFFF)) << 40)
        | ((ply.min(0xFF) as u64) << 32)
        | (seed & 0xFFFF_FFFF)
}

/// One deterministic row attempt under the blueprint labeler: deal a fresh
/// variant leg, replay to a stratified ply, then label by blueprint lookup.
/// Returns `None` when the sampled state is terminal, exceeds the action
/// cap, or falls outside the compiled tree.
fn try_row(ctx: &LabelCtx, shard: usize, row: usize, attempt: u32, target: u8) -> Option<(Row, u8)> {
    let seed = mix_seed(ctx.seed, shard, row, attempt);
    let mut rng = SmallRng::seed_from_u64(seed);

    let mut state = deal_variant(ctx.variant, &mut rng)?;
    let initial_field = state.field;
    let mut history = Vec::new();
    let ply = replay_to_ply(&mut rng, ctx.variant, ctx.guided_sample, &mut state, &mut history);
    if state.is_ended() {
        return None;
    }

    let legal = canonical_legal(&state)?;
    let flags = classify_buckets(&state);
    if target != 0 && flags & target == 0 {
        return None;
    }
    let observer = state.active;
    let worlds = build_belief(&state, observer, ctx.variant.cards, ctx.max_worlds, &mut rng).ok()?;
    if worlds.is_empty() {
        return None;
    }
    let anchors = MarginAnchors {
        initial_field,
        history_prefix: history,
    };
    let bp = BlueprintTarget {
        tree: ctx.tree,
        profile: ctx.profile,
        gadget: ctx.gadget,
    };
    let labels = label_state(&state, &anchors, &bp, &worlds, ctx.belief_perturb, &mut rng)?;

    let mask = legal_mask(&legal);
    let features = encode(&state, &labels.belief, &legal);
    let provenance = provenance_key(ctx.generation_iter, ctx.solver_budget, ply, seed);

    Some((
        Row {
            features,
            mask,
            policy: labels.policy,
            ev: labels.ev,
            belief: labels.belief,
            provenance,
            perturbed: labels.perturbed,
        },
        flags,
    ))
}

/// Generates row `row` of shard `shard` under the blueprint labeler.
/// Bucket-targeted rows retry harder (sampling is cheap, labels are not)
/// and fall back to a natural row when the target never materializes.
pub fn generate_row(ctx: &LabelCtx, shard: usize, row: usize, bucket_frac: f64) -> Option<(Row, u8)> {
    let target = bucket_target(ctx.seed, shard, row, bucket_frac);
    if target == 0 {
        for attempt in 0..MAX_ROW_ATTEMPTS {
            if let Some(outcome) = try_row(ctx, shard, row, attempt, 0) {
                return Some(outcome);
            }
        }
        return None;
    }
    for attempt in 0..MAX_ROW_ATTEMPTS.saturating_mul(8) {
        if let Some(outcome) = try_row(ctx, shard, row, attempt, target) {
            return Some(outcome);
        }
    }
    for attempt in 0..MAX_ROW_ATTEMPTS {
        if let Some(outcome) = try_row(ctx, shard, row, attempt | 0x8000_0000, 0) {
            return Some(outcome);
        }
    }
    None
}

/// The label context for `--method resolve` runs — no compiled tree: the
/// labels come from a root resolve per row, so this pathway also serves
/// the canonical variant where no blueprint exists.
pub struct ResolveCtx<'a> {
    pub variant: &'a KoiVariant,
    pub seed: u64,
    pub max_worlds: usize,
    pub belief_perturb: f64,
    /// Probability of heuristic-guided action sampling during replay.
    pub guided_sample: f64,
    /// Fraction of rows targeted at each rare-state bucket (equal thirds).
    pub bucket_frac: f64,
    /// The resolve budget per row label.
    pub spec: koi_solver::resolving::ResolveSpec,
    /// Self-play generation epoch, packed into the provenance key.
    pub generation_iter: u32,
}

/// One deterministic row attempt under the resolve labeler: deal, replay
/// to a stratified ply, then resolve the position for its policy/EV
/// targets. `target != 0` gates on the rare-state bucket before the resolve
/// runs — a mismatched state returns `None` having paid only for sampling.
fn try_row_resolved(
    ctx: &ResolveCtx,
    leaf: Option<&std::cell::RefCell<Box<dyn koi_solver::leaf::LeafEvaluator>>>,
    shard: usize,
    row: usize,
    attempt: u32,
    target: u8,
) -> Option<(Row, u8)> {
    let seed = mix_seed(ctx.seed, shard, row, attempt);
    let mut rng = SmallRng::seed_from_u64(seed);

    let mut state = deal_variant(ctx.variant, &mut rng)?;
    let initial_field = state.field;
    let mut history = Vec::new();
    let ply = replay_to_ply(&mut rng, ctx.variant, ctx.guided_sample, &mut state, &mut history);
    if state.is_ended() {
        return None;
    }

    let legal = canonical_legal(&state)?;
    let flags = classify_buckets(&state);
    if target != 0 && flags & target == 0 {
        return None;
    }
    let observer = state.active;
    let mut worlds = build_belief(&state, observer, ctx.variant.cards, ctx.max_worlds, &mut rng).ok()?;
    if worlds.is_empty() {
        return None;
    }

    // Perturbed rows overwrite the world weights with a random member
    // range: the resolve, the emitted belief vector and the EV target all
    // live under that range — see `label_state_resolved`.
    let perturbed = rng.random::<f64>() < ctx.belief_perturb;
    if perturbed {
        let range = random_member_weights(worlds.len(), &mut rng);
        for (world, weight) in worlds.iter_mut().zip(range) {
            world.weight = weight;
        }
    }
    let anchors = MarginAnchors {
        initial_field,
        history_prefix: history,
    };
    let labels = label_state_resolved(
        &state,
        &anchors,
        ctx.variant,
        &worlds,
        perturbed,
        &ctx.spec,
        leaf,
        &mut rng,
    )?;

    let mask = legal_mask(&legal);
    let features = encode(&state, &labels.belief, &legal);
    let provenance = provenance_key(ctx.generation_iter, ctx.spec.cfr_iterations as u64, ply, seed);

    Some((
        Row {
            features,
            mask,
            policy: labels.policy,
            ev: labels.ev,
            belief: labels.belief,
            provenance,
            perturbed: labels.perturbed,
        },
        flags,
    ))
}

/// Generates row `row` of shard `shard` under the resolve labeler. Rows
/// targeted at a rare-state bucket retry harder ([`MAX_ROW_ATTEMPTS`] × 8)
/// and fall back to a natural row when the target never materializes — the
/// coverage report records what the corpus actually contains.
pub fn generate_row_resolved(
    ctx: &ResolveCtx,
    leaf: Option<&std::cell::RefCell<Box<dyn koi_solver::leaf::LeafEvaluator>>>,
    shard: usize,
    row: usize,
) -> Option<(Row, u8)> {
    let target = bucket_target(ctx.seed, shard, row, ctx.bucket_frac);
    if target == 0 {
        for attempt in 0..MAX_ROW_ATTEMPTS {
            if let Some(outcome) = try_row_resolved(ctx, leaf, shard, row, attempt, 0) {
                return Some(outcome);
            }
        }
        return None;
    }
    // The bucket gate lives inside `try_row_resolved` — these retries only
    // pay for state sampling, not a resolve. One resolve still runs per
    // emitted row, on a state that already matches the target bucket.
    for attempt in 0..MAX_ROW_ATTEMPTS.saturating_mul(8) {
        if let Some(outcome) = try_row_resolved(ctx, leaf, shard, row, attempt, target) {
            return Some(outcome);
        }
    }
    // Fallback: resolve one natural, untargeted state — the same coverage
    // guarantee as before, without burning resolves on bucket misses.
    for attempt in 0..MAX_ROW_ATTEMPTS {
        if let Some(outcome) = try_row_resolved(ctx, leaf, shard, row, attempt | 0x8000_0000, 0) {
            return Some(outcome);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use koi_solver::cfr::cfr_plus::train_cfr_plus;
    use koi_solver::efg::compiler::compile_variant;
    use rayon::prelude::*;
    use std::sync::Arc;

    fn resolve_ctx() -> ResolveCtx<'static> {
        ResolveCtx {
            variant: &KoiVariant::FULL,
            seed: 0x5c0a,
            max_worlds: 16,
            belief_perturb: 0.3,
            guided_sample: 0.7,
            bucket_frac: 0.0,
            spec: koi_solver::resolving::ResolveSpec {
                max_worlds: 8,
                max_nodes: 50_000,
                cfr_iterations: 20,
                max_decision_depth: 2,
            },
            generation_iter: 0,
        }
    }

    /// The canonical pathway: a resolved row carries a normalized policy
    /// over the legal mask, a bounded EV, and a belief summing to the
    /// opponent's hand size.
    #[test]
    fn canonical_rows_are_labelled_by_the_resolve() {
        let ctx = resolve_ctx();
        let mut emitted = 0;
        for row_index in 0..8 {
            let Some((row, _flags)) = generate_row_resolved(&ctx, None, 0, row_index) else {
                continue;
            };
            emitted += 1;
            let policy_sum: f32 = row.policy.iter().sum();
            assert!((policy_sum - 1.0).abs() < 1e-5, "policy sums to one");
            let legal = row.mask.iter().filter(|m| **m).count();
            assert!(row.policy[legal..].iter().all(|p| *p == 0.0));
            assert!(row.ev.abs() <= 512.0, "leg-margin bound");
        }
        assert!(emitted > 0, "canonical generation must emit rows");
    }

    /// Determinism: the resolve pathway is thread-independent — provenance
    /// keys fold the row seed, so equality on them proves byte-identical
    /// generation under any rayon schedule.
    #[test]
    fn resolve_generation_is_thread_independent() {
        let ctx = resolve_ctx();
        let sequential: Vec<Option<u64>> = (0..8)
            .map(|r| generate_row_resolved(&ctx, None, 1, r).map(|(row, _)| row.provenance))
            .collect();
        let parallel: Vec<Option<u64>> = (0..8)
            .into_par_iter()
            .map(|r| generate_row_resolved(&ctx, None, 1, r).map(|(row, _)| row.provenance))
            .collect();
        assert_eq!(sequential, parallel);
    }

    /// Bucket classification: a stop-decision phase flags STOP_CALL; the
    /// final hand cycle flags ENDGAME; a formed opponent yaku flags
    /// YAKU_PRESSURE.
    #[test]
    fn bucket_flags_track_the_state() {
        let mut rng = SmallRng::seed_from_u64(21);
        let mut state = deal_variant(&KoiVariant::FULL, &mut rng).unwrap();
        let mut saw_endgame = false;
        while !state.is_ended() {
            let flags = classify_buckets(&state);
            assert_eq!(
                flags & bucket::STOP_CALL != 0,
                matches!(state.phase, TurnPhase::AwaitingStopDecision { .. })
            );
            let hand_total = state.hands[0].count() + state.hands[1].count();
            assert_eq!(flags & bucket::ENDGAME != 0, hand_total <= 4);
            if flags & bucket::ENDGAME != 0 {
                saw_endgame = true;
            }
            let legal = state.legal_actions();
            if legal.is_empty() {
                break;
            }
            // Never bank early — the leg must play out to its tail.
            let action = legal
                .iter()
                .copied()
                .find(|a| matches!(a, koi_core::Action::KoiKoi))
                .unwrap_or(legal[0]);
            state = state.apply_action(action).unwrap();
        }
        assert!(saw_endgame, "a played-out leg must reach the final hand cycle");
    }

    /// The blueprint path end-to-end on NANO_6: dealt, replayed, labelled —
    /// rows carry the certified profile's policy.
    #[test]
    fn blueprint_rows_are_labelled_on_the_reduced_domain() {
        let variant = &KoiVariant::NANO_6;
        let tree = Arc::new(compile_variant(variant, Player::South).unwrap());
        let blueprint = train_cfr_plus(&tree, 300);
        let profile = blueprint.averaged_profile();
        let gadget = SafetyGadget::new(tree.clone(), &blueprint);
        let ctx = LabelCtx {
            variant,
            tree: &tree,
            profile: &profile,
            gadget: &gadget,
            seed: 0x5c0a,
            max_worlds: 32,
            belief_perturb: 0.3,
            guided_sample: 0.7,
            generation_iter: 0,
            solver_budget: 300,
        };
        let mut emitted = 0;
        for row_index in 0..8 {
            let Some((row, _)) = generate_row(&ctx, 0, row_index, 0.0) else {
                continue;
            };
            emitted += 1;
            let sum: f32 = row.policy.iter().sum();
            assert!((sum - 1.0).abs() < 1e-5);
        }
        assert!(emitted > 0, "blueprint generation must emit rows");
    }
}
