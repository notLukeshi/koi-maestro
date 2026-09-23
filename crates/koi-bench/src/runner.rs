//! The benchmark referee: plays mirrored paired legs over the deal-cluster
//! panel, enforces the worker protocol and time budgets, records traces,
//! and honors the manifest's stopping rule.
//!
//! A deal cluster is one seed played twice: the candidate deals (South) in
//! `candidate_deals` and the baseline deals in `baseline_deals`. Both legs
//! share the deck so deal difficulty cancels in the paired estimand — the
//! cluster, not the leg, is the unit of inference (E1/E4).
//!
//! Failure discipline: referee-side failures (spawn, IO on our own
//! handles, checkpoint errors) are hard errors; worker-side failures
//! (protocol violations, timeouts, illegal actions, crashes) forfeit the
//! leg for the offending artifact and the run continues — evidence is
//! intention-to-treat.

use std::path::Path;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use koi_core::{deal_from_seed, derive_named_seed, AnomalyResolution, KoiGameState, Player, TurnPhase};

use crate::artifact::{referee_identity, RefereeIdentity, ResolvedArtifact};
use crate::checkpoint::Checkpoint;
use crate::manifest::{
    fnv1a64_hex, ArtifactIdentity, Manifest, StoppingSpec, MAX_ACTIONS_PER_ROUND, MAX_TOTAL_TRACE_ENTRIES,
};
use crate::protocol::{DecisionRequest, WireAction, WireDealView, WireLedgerEntry, WireState};
use crate::statistics::margin_to_win_score;
use crate::worker::{DecideError, WorkerPool};

/// Which board seat an artifact occupies for one leg.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Seat {
    /// South deals and moves first.
    South,
    North,
}

impl Seat {
    fn of(player: Player) -> Self {
        match player {
            Player::South => Seat::South,
            Player::North => Seat::North,
        }
    }
}

/// Which manifest entrant a seat is played by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactRole {
    Baseline,
    Candidate,
}

/// How a leg ended.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LegStatus {
    /// Played to a natural or anomaly-resolved end.
    Valid,
    /// A worker-side protocol, framing, or legality failure ended the leg;
    /// the offender forfeits.
    Invalid { offender: ArtifactRole, reason: String },
    /// The offender's summed decision latency crossed its
    /// `time_hard_cap_ms` — too slow, not invalid.
    TimeForfeit {
        offender: ArtifactRole,
        spent_ms: f64,
        hard_cap_ms: u64,
    },
}

/// One recorded decision: what the referee sent and what came back.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEntry {
    /// Phase-action index within the round.
    pub action_index: usize,
    pub seat: Seat,
    pub artifact: ArtifactRole,
    /// The per-decision seed the worker replay contract pins.
    pub solver_seed: u64,
    /// FNV-1a/64 of the serialized `WireState` — binds the referee's
    /// canonical view of the decision point.
    pub referee_state_digest_fnv1a64: String,
    /// Retry ordinal within this decision (always 0 — retries are P2+).
    pub retry: u32,
    pub selected_action: WireAction,
    /// Referee wall-clock ms around the whole worker transaction.
    pub decision_latency_ms: f64,
    /// Worker-reported leaf-model ms inside the decision (0 today).
    #[serde(default)]
    pub leaf_eval_latency_ms: f64,
}

/// One completed (or forfeited) leg.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegResult {
    /// Which seat the candidate occupied this leg.
    pub candidate_seat: Seat,
    pub status: LegStatus,
    pub south_points: Option<i32>,
    pub north_points: Option<i32>,
    /// Candidate-relative margin: positive when the candidate led.
    pub candidate_margin: f64,
    /// `margin_to_win_score(candidate_margin)` — {0, 0.5, 1}.
    pub candidate_win_score: f64,
    /// Phase-actions actually played.
    pub actions: usize,
    /// Decks discarded by anomaly redeals before this leg started.
    #[serde(default)]
    pub redeals: u32,
    pub trace: Vec<TraceEntry>,
}

/// One deal cluster: the paired mirrored legs for a seed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedResult {
    pub seed: u64,
    /// The leg where the candidate sits South (deals first).
    pub candidate_deals: LegResult,
    /// The leg where the baseline sits South.
    pub baseline_deals: LegResult,
}

/// One unordered pairing's merged evidence inside a tournament: the
/// pairwise `BenchmarkRun`'s seeds plus the stop it recorded. `first`
/// played the manifest's baseline role, `second` the candidate role — so
/// `candidate_win_score` is `second`'s leg score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingRun {
    /// Index into `TournamentRun::entrants` mapped to the `Baseline` role.
    pub first: usize,
    /// Index into `TournamentRun::entrants` mapped to the `Candidate` role.
    pub second: usize,
    /// The stopping outcome the pairwise run recorded — ranking provenance
    /// requires it (a complete schedule is the only bias-free evidence).
    pub stopping: crate::sprt::StoppingOutcome,
    pub seeds: Vec<SeedResult>,
}

/// The merged tournament record — what `tournament_rank` assembles from
/// per-pairing `report.json` files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TournamentRun {
    pub referee: RefereeIdentity,
    /// Entrant identities in manifest order — the first attested build
    /// per entrant. Additional distinct builds seen across pairings are
    /// kept in `additional_entrant_builds` (the solver-relevant binding —
    /// `config_hash_fx64` — must still match or the merge refuses).
    pub entrants: Vec<ArtifactIdentity>,
    /// Non-primary artifact builds attested for entrants across pairings
    /// — e.g. a supplemental field run under a later binary whose
    /// incumbent solver code is provably invariant. Disclosed in the
    /// report's provenance section; never silently pooled.
    #[serde(default)]
    pub additional_entrant_builds: Vec<ArtifactIdentity>,
    pub pairings: Vec<PairingRun>,
    pub cargo_lock_hash_fx64: String,
}

/// The complete run record — the report's evidence payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkRun {
    pub referee: RefereeIdentity,
    pub baseline: ArtifactIdentity,
    pub candidate: ArtifactIdentity,
    pub manifest_hash_fx64: String,
    pub cargo_lock_hash_fx64: String,
    pub ruleset: String,
    /// How the run's evidence was selected: a complete fixed schedule, or a
    /// sequential boundary crossing. A sequential stop makes the win-score
    /// estimate selection-biased — the confirmation decision is the evidence.
    /// `default` keeps legacy evidence (recorded before sequential
    /// stopping existed) parseable as a complete schedule.
    #[serde(default)]
    pub stopping: crate::sprt::StoppingOutcome,
    /// The OS-entropy secret every deck was derived through:
    /// `deal_from_seed(derive_named_seed(run_secret, seed))`. Recorded
    /// post-run so evidence consumers can reproduce every dealt cluster;
    /// during the run it lives only in the referee's memory and the
    /// checkpoint header.
    #[serde(default)]
    pub run_secret: u64,
    pub seeds: Vec<SeedResult>,
}

/// Per-leg latency ledger: summed `decision_latency_ms` per artifact.
#[derive(Debug, Default)]
struct LegClock {
    baseline_ms: f64,
    candidate_ms: f64,
}

impl LegClock {
    fn charge(&mut self, artifact: ArtifactRole, latency_ms: f64) -> f64 {
        let spent = match artifact {
            ArtifactRole::Baseline => &mut self.baseline_ms,
            ArtifactRole::Candidate => &mut self.candidate_ms,
        };
        *spent += latency_ms;
        *spent
    }

    fn spent(&self, artifact: ArtifactRole) -> f64 {
        match artifact {
            ArtifactRole::Baseline => self.baseline_ms,
            ArtifactRole::Candidate => self.candidate_ms,
        }
    }
}

/// Why a leg ended early — drives which `LegStatus` the forfeit becomes.
enum LegForfeit {
    Invalid(String),
    TimeExceeded { spent_ms: f64, hard_cap_ms: u64 },
}

/// Stream tag for per-decision solver seeds: `derive_named_seed(seed, tag ^ index)`.
const DECISION_STREAM_TAG: u64 = 0x4b4f_4944_4543_4901;
/// Stream tag for anomaly redeals of a cluster seed.
const REDEAL_STREAM_TAG: u64 = 0x4b4f_4952_444c_0001;

/// The immutable leg context: the two resolved entrants plus the run's
/// rules, bounds, and deal secret. Bundled so `play_leg` stays under the
/// arity lint.
struct LegContext<'a> {
    pool: &'a mut WorkerPool,
    baseline: &'a ResolvedArtifact,
    candidate: &'a ResolvedArtifact,
    manifest: &'a Manifest,
    timeout: Duration,
    /// OS-entropy run secret adopted from the checkpoint header — every
    /// deck is `deal_from_seed(derive_named_seed(run_secret, seed))`, so a
    /// worker that knows the public panel seed (the `solver_seed`
    /// derivation is a bijection it can invert) still cannot reconstruct
    /// the deck it is playing against.
    run_secret: u64,
}

/// The artifact seated at `seat` for a leg where `candidate_seat` holds
/// the candidate. A free function so the returned borrow is tied to the
/// entrants, not to `ctx` (whose `pool` must stay mutable).
fn artifact_for<'a>(
    seat: Seat,
    candidate_seat: Seat,
    baseline: &'a ResolvedArtifact,
    candidate: &'a ResolvedArtifact,
) -> (&'a ResolvedArtifact, ArtifactRole) {
    if seat == candidate_seat {
        (candidate, ArtifactRole::Candidate)
    } else {
        (baseline, ArtifactRole::Baseline)
    }
}

/// Builds the `WireDealView` for `observer` from the freshly dealt state.
fn deal_view(state: &KoiGameState, observer: Player) -> WireDealView {
    WireDealView {
        own_hand: state.hands[observer.index()].bits(),
        opponent_hand_count: state.hands[observer.opponent().index()].count() as u8,
        field: state.field.bits(),
        stock_count: state.stock.len() as u8,
        dealer: match state.dealer {
            Player::South => 0,
            Player::North => 1,
        },
        deal_score: state.score,
    }
}

/// Builds the forfeit leg record — both forfeit kinds score through
/// `invalid_forfeit_margin`, but the status keeps the cause distinct so the
/// report separates protocol invalidity from hard-cap overruns.
fn forfeit_leg(
    candidate_seat: Seat,
    offender: ArtifactRole,
    forfeit: LegForfeit,
    forfeit_margin: f64,
    actions: usize,
    redeals: u32,
    trace: Vec<TraceEntry>,
) -> LegResult {
    let status = match forfeit {
        LegForfeit::Invalid(reason) => LegStatus::Invalid { offender, reason },
        LegForfeit::TimeExceeded { spent_ms, hard_cap_ms } => LegStatus::TimeForfeit {
            offender,
            spent_ms,
            hard_cap_ms,
        },
    };
    let candidate_margin = match offender {
        ArtifactRole::Candidate => -forfeit_margin,
        ArtifactRole::Baseline => forfeit_margin,
    };
    LegResult {
        candidate_seat,
        status,
        south_points: None,
        north_points: None,
        candidate_margin,
        candidate_win_score: margin_to_win_score(candidate_margin),
        actions,
        redeals,
        trace,
    }
}

/// Plays one leg: a single round for `seed` with the candidate at
/// `candidate_seat`. Worker failures fold into a forfeit; referee failures
/// propagate as hard errors.
fn play_leg(ctx: &mut LegContext, seed: u64, candidate_seat: Seat, request_ids: &mut u64) -> Result<LegResult> {
    let manifest = ctx.manifest;
    let timeout = ctx.timeout;
    let rules = manifest.ruleset();
    // Anomaly policy (E2): resolve in-engine. A lucky hand ends the leg
    // immediately with the configured win points; a void/null redeal draws
    // a derived deck — the cluster keeps its estimand either way.
    let mut redeals = 0u32;
    // The public panel seed is mixed with the run secret before it ever
    // touches a deck: `solver_seed` derivation is invertible by design
    // (decision replay needs it), so deal secrecy rests here instead.
    let deal_seed = derive_named_seed(ctx.run_secret, seed);
    let (mut state, initial_views) = loop {
        let deck = if redeals == 0 {
            deal_from_seed(deal_seed)
        } else {
            deal_from_seed(derive_named_seed(deal_seed, REDEAL_STREAM_TAG + u64::from(redeals)))
        };
        let (mut state, anomaly) = KoiGameState::new_deal(deck, rules);
        match anomaly {
            Some(anomaly) => match state.resolve_deal_anomaly(anomaly) {
                AnomalyResolution::InstantWin { .. } => {
                    let south = state.score[0];
                    let north = state.score[1];
                    let candidate_margin = match candidate_seat {
                        Seat::South => (south - north) as f64,
                        Seat::North => (north - south) as f64,
                    };
                    return Ok(LegResult {
                        candidate_seat,
                        status: LegStatus::Valid,
                        south_points: Some(south),
                        north_points: Some(north),
                        candidate_margin,
                        candidate_win_score: margin_to_win_score(candidate_margin),
                        actions: 0,
                        redeals,
                        trace: Vec::new(),
                    });
                }
                AnomalyResolution::Redeal => {
                    redeals += 1;
                    if redeals > 16 {
                        bail!("seed {seed}: deal anomaly persisted across 16 redeals");
                    }
                    continue;
                }
            },
            None => {
                let views = [deal_view(&state, Player::South), deal_view(&state, Player::North)];
                break (state, views);
            }
        }
    };

    let mut ledger: Vec<WireLedgerEntry> = Vec::with_capacity(MAX_ACTIONS_PER_ROUND);
    let mut trace: Vec<TraceEntry> = Vec::with_capacity(MAX_ACTIONS_PER_ROUND);
    let mut clock = LegClock::default();
    let mut actions = 0usize;

    while !state.is_ended() {
        if actions >= MAX_ACTIONS_PER_ROUND {
            bail!("leg exceeded the {MAX_ACTIONS_PER_ROUND}-action round bound");
        }
        let seat = Seat::of(state.active);
        let (resolved, role) = artifact_for(seat, candidate_seat, ctx.baseline, ctx.candidate);

        let mut frame = WireState::view_fields(&state, state.active);
        frame.initial = initial_views[state.active.index()].clone();
        frame.ledger = ledger.clone();
        let frame_bytes = serde_json::to_vec(&frame).context("failed to serialize wire state")?;
        let digest = fnv1a64_hex(&frame_bytes);

        // Nested derivation keeps the stream identifiers distinct: folding
        // seed⊕actions⊕seat into one mixer input collides across tuples —
        // (seed 1, action 0) and (seed 4, action 5) produced identical
        // solver streams in the P3 traces.
        let solver_seed = derive_named_seed(
            derive_named_seed(seed, DECISION_STREAM_TAG),
            actions as u64 ^ ((seat as u64) << 8),
        );
        *request_ids += 1;
        let request = DecisionRequest {
            protocol_version: crate::protocol::WORKER_PROTOCOL_VERSION,
            request_id: *request_ids,
            solver_seed,
            expected_worker_commit: resolved.expected_commit.clone(),
            expected_worker_tree: resolved.expected_tree.clone(),
            expected_worker_clean: resolved.expected_clean,
            config: resolved.config.clone(),
            state: frame,
            remaining_budget_ms: manifest
                .time_hard_cap_ms
                .map(|cap| cap.saturating_sub(clock.spent(role) as u64)),
            deadline_ms: Some(timeout.as_millis() as u64),
        };

        let client = ctx.pool.worker_for(resolved, timeout)?;
        let (response, latency_ms) = client.decide(
            &request,
            &resolved.expected_commit,
            &resolved.expected_tree,
            resolved.expected_clean,
        );
        let response = match response {
            Ok(response) => response,
            Err(error) => {
                let reason = match &error {
                    DecideError::Timeout => "transaction timeout".to_owned(),
                    DecideError::Crashed(detail) => format!("worker crashed: {detail}"),
                    DecideError::BadFrame(detail) => format!("invalid frame: {detail}"),
                    DecideError::Protocol(detail) => detail.clone(),
                };
                // A failed worker is untrusted for subsequent legs — any
                // DecideError retires the process, not just a dead one: a
                // live worker that already violated the protocol must not
                // keep serving legs (selective-forfeit channel). Retire
                // rather than restart here: the next `worker_for` lazily
                // respawns, so a failed respawn can never swallow this
                // leg's recorded forfeit.
                ctx.pool.retire(resolved);
                return Ok(forfeit_leg(
                    candidate_seat,
                    role,
                    LegForfeit::Invalid(reason),
                    manifest.invalid_forfeit_margin,
                    actions,
                    redeals,
                    trace,
                ));
            }
        };

        let spent = clock.charge(role, latency_ms);
        // Record the transaction before any forfeit path: the decision
        // happened and its latency is real measured evidence. Withholding
        // it on the killing decision would right-censor both the
        // per-decision latency distribution and the leg-time sums —
        // `LegStatus::TimeForfeit::spent_ms` then equals the trace sum.
        if let Some(wire_action) = response.selected_action.clone() {
            trace.push(TraceEntry {
                action_index: actions,
                seat,
                artifact: role,
                solver_seed,
                referee_state_digest_fnv1a64: digest,
                retry: 0,
                selected_action: wire_action,
                decision_latency_ms: latency_ms,
                leaf_eval_latency_ms: response.leaf_eval_latency_ms,
            });
        }
        if manifest.time_hard_cap_ms.is_some_and(|cap| spent > cap as f64) {
            let cap = manifest.time_hard_cap_ms.unwrap_or(u64::MAX);
            return Ok(forfeit_leg(
                candidate_seat,
                role,
                LegForfeit::TimeExceeded {
                    spent_ms: spent,
                    hard_cap_ms: cap,
                },
                manifest.invalid_forfeit_margin,
                actions,
                redeals,
                trace,
            ));
        }

        let Some(wire_action) = response.selected_action else {
            return Ok(forfeit_leg(
                candidate_seat,
                role,
                LegForfeit::Invalid("worker returned no action".to_owned()),
                manifest.invalid_forfeit_margin,
                actions,
                redeals,
                trace,
            ));
        };
        let action = match wire_action.to_action() {
            Ok(action) => action,
            Err(error) => {
                return Ok(forfeit_leg(
                    candidate_seat,
                    role,
                    LegForfeit::Invalid(format!("unparseable action: {error}")),
                    manifest.invalid_forfeit_margin,
                    actions,
                    redeals,
                    trace,
                ));
            }
        };
        if !state.legal_actions().contains(&action) {
            return Ok(forfeit_leg(
                candidate_seat,
                role,
                LegForfeit::Invalid(format!("illegal action {action:?}")),
                manifest.invalid_forfeit_margin,
                actions,
                redeals,
                trace,
            ));
        }

        // The ledger records only applied decisions — an unapplied action
        // (forfeit paths above) must never enter it or worker replay
        // diverges.
        let drawn = match state.phase {
            TurnPhase::AwaitingStockResolution { drawn } => Some(drawn.index()),
            _ => None,
        };
        ledger.push(WireLedgerEntry {
            player: match state.active {
                Player::South => 0,
                Player::North => 1,
            },
            action: wire_action,
            drawn,
        });

        state = state
            .apply_action(action)
            .context("referee legal replay failed on a validated action")?;
        actions += 1;
    }

    let south = state.score[0];
    let north = state.score[1];
    let candidate_margin = match candidate_seat {
        Seat::South => (south - north) as f64,
        Seat::North => (north - south) as f64,
    };
    Ok(LegResult {
        candidate_seat,
        status: LegStatus::Valid,
        south_points: Some(south),
        north_points: Some(north),
        candidate_margin,
        candidate_win_score: margin_to_win_score(candidate_margin),
        actions,
        redeals,
        trace,
    })
}

/// Runs the manifest's paired panel to completion (or its sequential
/// boundary), resuming from `out_dir/legs.jsonl` when the checkpoint binds
/// to this manifest and artifact pair. `manifest_dir` is the directory
/// containing the manifest file — relative `path` artifacts resolve
/// against it, never against the caller's working directory.
pub fn run(manifest: &Manifest, manifest_dir: &Path, out_dir: &Path) -> Result<BenchmarkRun> {
    manifest.validate().context("manifest failed validation")?;
    let manifest_hash = manifest.canonical_hash_fx64()?;
    let manifest_dir = manifest_dir
        .canonicalize()
        .context("failed to canonicalize the manifest directory")?;
    let mut baseline = crate::artifact::resolve(&manifest.baseline, &manifest_dir)?;
    let mut candidate = crate::artifact::resolve(&manifest.candidate, &manifest_dir)?;

    let timeout = Duration::from_secs(manifest.worker_timeout_seconds);
    let mut pool = WorkerPool::default();

    // Preflight: start every distinct worker and pin negotiated build
    // stamps onto the artifacts — a binding failure aborts before any leg.
    let negotiated = pool.preflight(&[&baseline, &candidate], timeout)?;
    for (executable, commit, tree, dirty) in &negotiated {
        for resolved in [&mut baseline, &mut candidate] {
            if resolved.executable == *executable {
                resolved.record_negotiated(commit, tree, *dirty)?;
            }
        }
    }

    let baseline_identity = baseline.identity()?;
    let candidate_identity = candidate.identity()?;
    let mut checkpoint = Checkpoint::open(
        &out_dir.join("legs.jsonl"),
        &manifest_hash,
        &baseline_identity,
        &candidate_identity,
        &manifest.seeds,
    )?;
    let run_secret = checkpoint.run_secret();

    // Resume: replayed clusters are a strict panel prefix — index them by
    // seed once rather than scanning per seed.
    let completed_index: std::collections::HashMap<u64, usize> = checkpoint
        .completed()
        .iter()
        .enumerate()
        .map(|(index, result)| (result.seed, index))
        .collect();
    let mut results: Vec<SeedResult> = Vec::new();
    let (mut evaluator, mut equivalence_evaluator) = match &manifest.stopping {
        StoppingSpec::Fixed => (None, None),
        StoppingSpec::Sequential {
            h0_elo,
            h1_elo,
            alpha,
            beta,
            min_clusters,
            equivalence,
        } => (
            Some(crate::sprt::GsprtEvaluator::new(
                crate::sprt::GsprtSpec {
                    elo0: *h0_elo,
                    elo1: *h1_elo,
                    alpha: *alpha,
                    beta: *beta,
                    min_clusters: *min_clusters,
                },
                manifest.seeds.len(),
            )),
            equivalence
                .clone()
                .map(|spec| crate::sprt::EquivalenceEvaluator::new(spec, manifest.seeds.len())),
        ),
    };
    let mut stopping: Option<crate::sprt::StoppingOutcome> = None;
    // Request ids continue past the replayed evidence so a resumed run
    // never recycles an id — deterministic enough for diagnostics even
    // though forfeit paths make exact replay counts unreconstructible.
    let mut request_ids: u64 = checkpoint
        .completed()
        .iter()
        .map(|result| result.candidate_deals.trace.len() + result.baseline_deals.trace.len())
        .sum::<usize>() as u64;
    let mut trace_total = 0usize;

    for &seed in &manifest.seeds {
        let result = if let Some(&index) = completed_index.get(&seed) {
            checkpoint.completed()[index].clone()
        } else {
            // Paired mirrored legs: candidate deals in leg A, baseline in B.
            let mut leg = LegContext {
                pool: &mut pool,
                baseline: &baseline,
                candidate: &candidate,
                manifest,
                timeout,
                run_secret,
            };
            let candidate_deals = play_leg(&mut leg, seed, Seat::South, &mut request_ids)?;
            // Mirrored legs share the deal: what leg A showed a worker as
            // its own hand is leg B's hidden opponent hand. Retiring both
            // processes between legs — and before the next cluster — keeps
            // a stateful worker from carrying that private knowledge across
            // the mirror. Process-tree termination also confines any
            // unmeasured background work to the leg that spawned it.
            leg.pool.retire(&baseline);
            leg.pool.retire(&candidate);
            let baseline_deals = play_leg(&mut leg, seed, Seat::North, &mut request_ids)?;
            leg.pool.retire(&baseline);
            leg.pool.retire(&candidate);
            let result = SeedResult {
                seed,
                candidate_deals,
                baseline_deals,
            };
            checkpoint.append(&result)?;
            result
        };
        trace_total += result.candidate_deals.trace.len() + result.baseline_deals.trace.len();
        if trace_total > MAX_TOTAL_TRACE_ENTRIES {
            bail!("run exceeded the {MAX_TOTAL_TRACE_ENTRIES}-entry trace bound");
        }
        if let Some(evaluator) = evaluator.as_mut() {
            evaluator.observe(&result);
        }
        if let Some(equivalence) = equivalence_evaluator.as_mut() {
            equivalence.observe(&result);
        }
        // GSPRT boundary first; the equivalence certification is the
        // fallback decision for near-tied pairs the GLLR cannot reach.
        let mut decision = match evaluator.as_ref() {
            Some(evaluator) => evaluator
                .decision()?
                .map(|outcome| outcome.with_equivalence_cs(equivalence_evaluator.as_ref().map(|e| e.interval()))),
            None => None,
        };
        if decision.is_none() {
            decision = equivalence_evaluator
                .as_ref()
                .and_then(crate::sprt::EquivalenceEvaluator::decision);
        }
        results.push(result);
        if decision.is_some() {
            stopping = decision;
            break;
        }
    }

    // A fixed schedule ends at CompleteSchedule; a sequential spec that
    // never crossed records the cap outcome — the LLR stays `None` when
    // the panel ended before `min_clusters`.
    let stopping = match stopping {
        Some(outcome) => outcome,
        None => match evaluator.as_ref() {
            Some(evaluator) => evaluator
                .exhausted()?
                .with_equivalence_cs(equivalence_evaluator.as_ref().map(|e| e.interval())),
            None => crate::sprt::StoppingOutcome::CompleteSchedule,
        },
    };

    pool.shutdown();

    let cargo_lock = koi_build_info::cargo_lock_hash().unwrap_or_else(|_| "unavailable".to_owned());

    Ok(BenchmarkRun {
        referee: referee_identity(),
        baseline: baseline_identity,
        candidate: candidate_identity,
        manifest_hash_fx64: manifest_hash,
        cargo_lock_hash_fx64: cargo_lock,
        ruleset: manifest.ruleset.clone(),
        stopping,
        run_secret,
        seeds: results,
    })
}
