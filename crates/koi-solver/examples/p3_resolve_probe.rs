//! P3 resolve-latency probe: times `resolve_decision` across a simulated
//! canonical leg so the manifest spec can be sized against the leg budget.
//!
//! NOT a canonical-evidence artifact — a sizing instrument. It reports
//! per-decision wall time under several specs on the FULL domain with the
//! learned oracle, plus the belief size each rung actually used.
//!
//! Run: `cargo run -p koi-solver --release --example p3_resolve_probe`

use koi_core::{deal_from_seed, KoiGameState, LedgerEntry, PublicObservation, Ruleset};
use koi_solver::efg::KoiVariant;
use koi_solver::resolving::{learned_margin_south, replay_ledger, resolve_decision, MarginAnchors, ResolveSpec};
use rand::rngs::SmallRng;
use rand::SeedableRng;

/// A margin oracle backed by `learned_margin_south` for direct probing.
struct LearnedProbe;

impl koi_solver::efg::MarginOracle for LearnedProbe {
    fn margin_south(&self, state: &KoiGameState, _history: &[u64]) -> Option<f64> {
        Some(learned_margin_south(state))
    }
}

fn main() {
    let specs = [
        (
            "deep",
            ResolveSpec {
                max_worlds: 24,
                max_nodes: 600_000,
                cfr_iterations: 300,
                max_decision_depth: 6,
            },
        ),
        (
            "manifest",
            ResolveSpec {
                max_worlds: 16,
                max_nodes: 300_000,
                cfr_iterations: 120,
                max_decision_depth: 4,
            },
        ),
        (
            "middle",
            ResolveSpec {
                max_worlds: 16,
                max_nodes: 200_000,
                cfr_iterations: 80,
                max_decision_depth: 4,
            },
        ),
        (
            "shallow",
            ResolveSpec {
                max_worlds: 12,
                max_nodes: 150_000,
                cfr_iterations: 60,
                max_decision_depth: 3,
            },
        ),
    ];

    for (label, spec) in &specs {
        let mut total_ms = 0.0;
        let mut decisions = 0usize;
        let mut max_ms: f64 = 0.0;
        let mut failures = 0usize;
        for seed in 0..4u64 {
            let (mut state, anomaly) = KoiGameState::new_deal(deal_from_seed(1000 + seed), Ruleset::nintendo());
            assert!(anomaly.is_none());
            let observer = state.dealer;
            let initial = state.public_view(observer);
            let mut entries = Vec::new();
            let mut turns = 0usize;
            while !state.is_ended() && turns < 24 {
                if state.active == observer {
                    // Decide only on the observer's own turns — the entrant
                    // sees decisions for its own seat.
                    let observation = PublicObservation {
                        initial: initial.clone(),
                        entries: entries.clone(),
                    };
                    let replay = replay_ledger(&observation).expect("live ledger replays");
                    let anchors = MarginAnchors {
                        initial_field: replay.initial_field,
                        history_prefix: replay.history_prefix.clone(),
                    };
                    let mut rng = SmallRng::seed_from_u64(7);
                    let start = std::time::Instant::now();
                    let ctx = koi_solver::resolving::ResolveContext {
                        variant: &KoiVariant::FULL,
                        anchors: &anchors,
                        oracle: &LearnedProbe,
                        spec,
                        model: None,
                        replay: None,
                        leaf_evaluator: None,
                    };
                    let resolved = resolve_decision(&state, observer, &ctx, &mut rng);
                    let ms = start.elapsed().as_secs_f64() * 1000.0;
                    decisions += 1;
                    total_ms += ms;
                    max_ms = max_ms.max(ms);
                    // A ResolveStock entry must carry the public draw it
                    // resolves — the same shape the referee's ledger uses.
                    let drawn = match state.phase {
                        koi_core::TurnPhase::AwaitingStockResolution { drawn } => Some(drawn),
                        _ => None,
                    };
                    match resolved {
                        Ok(resolved) => {
                            entries.push(LedgerEntry {
                                player: observer,
                                action: resolved.action,
                                drawn,
                            });
                            state = state.apply_action(resolved.action).expect("resolved action applies");
                        }
                        Err(error) => {
                            failures += 1;
                            // Fall back to a legal action so the leg continues.
                            let action = state.legal_actions()[0];
                            entries.push(LedgerEntry {
                                player: observer,
                                action,
                                drawn,
                            });
                            state = state.apply_action(action).expect("legal action applies");
                            let _ = error;
                        }
                    }
                } else {
                    // Opponent + stock resolutions advance by first-legal.
                    let drawn = match state.phase {
                        koi_core::TurnPhase::AwaitingStockResolution { drawn } => Some(drawn),
                        _ => None,
                    };
                    let action = state.legal_actions()[0];
                    entries.push(LedgerEntry {
                        player: state.active,
                        action,
                        drawn,
                    });
                    state = state.apply_action(action).expect("legal action applies");
                }
                turns += 1;
            }
        }
        println!(
            "{label}: {decisions} decisions, mean {:.0} ms, max {:.0} ms, failures {failures}",
            total_ms / decisions.max(1) as f64,
            max_ms,
        );
    }
}
