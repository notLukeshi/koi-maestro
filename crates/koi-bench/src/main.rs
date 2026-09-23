//! koi-bench: the paired benchmark referee and worker binary.
//!
//! `koi-bench run --manifest m.json --out dir/` plays the manifest's deal
//! panel as mirrored candidate/baseline legs and writes `legs.jsonl`,
//! `report.json`, `report.md`. `koi-bench worker` is the spawned-side
//! protocol endpoint: newline-delimited `DecisionRequest` frames on stdin,
//! `@@KOI_BENCH@@`-prefixed `DecisionResponse` frames on stdout.
//! `--worker-launcher` is the Windows containment shim (see process_tree).

use std::io::{BufReader, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::Parser;

use koi_bench::framing::read_bounded_frame;
use koi_bench::manifest::Manifest;
use koi_bench::protocol::{
    DecisionRequest, DecisionResponse, NegotiationResponse, NEGOTIATION_REQUEST_KIND, RESPONSE_PREFIX,
    WORKER_PROTOCOL_VERSION,
};

#[derive(Debug, Parser)]
#[command(name = "koi-bench")]
#[command(about = "Paired benchmark harness for Koi-Maestro")]
enum Cli {
    /// Run a paired benchmark from a manifest.
    Run {
        /// Path to the benchmark manifest (JSON, schema 1).
        #[arg(long, short)]
        manifest: PathBuf,
        /// Output directory for legs.jsonl + report.{json,md}.
        #[arg(long, short)]
        out: PathBuf,
    },
    /// Serve decision requests on stdin/stdout (spawned by the referee).
    Worker,
    /// Print an example manifest to stdout.
    ExampleManifest,
}

fn main() -> ExitCode {
    // The Windows containment shim must run before clap — it is launched
    // with `--worker-launcher` plus env-carried program/arguments.
    if std::env::args().any(|arg| arg == "--worker-launcher") {
        return match koi_bench::process_tree::run_worker_launcher() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("worker launcher failed: {error:?}");
                ExitCode::FAILURE
            }
        };
    }
    match try_main() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("koi-bench failed: {error:?}");
            ExitCode::FAILURE
        }
    }
}

fn try_main() -> Result<()> {
    env_logger::init();
    match Cli::parse() {
        Cli::Run { manifest, out } => {
            let manifest_dir = manifest
                .parent()
                .map(std::path::Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from("."));
            let manifest = Manifest::load(&manifest)?;
            let run = koi_bench::runner::run(&manifest, &manifest_dir, &out)?;
            let paths = koi_bench::report::write_reports(&out, &run, &manifest)?;
            println!("wrote {} and {}", paths.json.display(), paths.markdown.display());
            // Evidence discipline: a run whose every leg forfeited carries
            // no played evidence — its "result" is synthetic forfeit
            // margins. The report is written for diagnosis, then the CLI
            // fails so scripts cannot consume it as a successful run.
            let played = run
                .seeds
                .iter()
                .flat_map(|seed| [&seed.candidate_deals.status, &seed.baseline_deals.status])
                .filter(|status| matches!(status, koi_bench::runner::LegStatus::Valid))
                .count();
            anyhow::ensure!(
                played > 0,
                "vacuous run: zero of {} legs produced played evidence — report is diagnostic only",
                run.seeds.len() * 2
            );
            Ok(())
        }
        Cli::Worker => run_worker(),
        Cli::ExampleManifest => {
            print!("{}", example_manifest());
            Ok(())
        }
    }
}

/// Worker-side validated-config cache: entrants hold one config per run,
/// so validation — and any heavy resource `Solver` construction grows —
/// pays once per distinct config, never per decision. The per-request
/// `solver_seed` still pins entropy at call time; the cache only skips
/// revalidation.
struct WorkerState {
    configs: std::collections::HashMap<String, koi_solver::ValidatedConfig>,
    /// Blueprint gadgets keyed by artifact path: a resolving entrant loads
    /// its certified oracle once per worker, never per decision — the load
    /// recompiles the variant tree and re-verifies every row digest.
    gadgets: std::collections::HashMap<String, std::sync::Arc<koi_solver::resolving::SafetyGadget>>,
}

/// The worker protocol loop: one JSON request per stdin line, one prefixed
/// JSON response per stdout line. Never exits early on a bad request — a
/// parseable request_id always earns a `DecisionResponse` failure frame so
/// the referee can attribute the forfeit.
fn run_worker() -> Result<()> {
    let stdin = std::io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let mut state = WorkerState {
        configs: std::collections::HashMap::new(),
        gadgets: std::collections::HashMap::new(),
    };

    while let Some(line) = read_bounded_frame(&mut reader).map_err(|error| anyhow::anyhow!("stdin: {error}"))? {
        let reply = handle_request(&mut state, &line);
        writeln!(out, "{RESPONSE_PREFIX}{reply}")
            .and_then(|()| out.flush())
            .context("failed to write worker response")?;
    }
    Ok(())
}

fn handle_request(state: &mut WorkerState, line: &str) -> String {
    let peek: serde_json::Value = match serde_json::from_str(line) {
        Ok(value) => value,
        Err(error) => {
            return serde_json::to_string(&DecisionResponse::failure(
                0,
                WORKER_PROTOCOL_VERSION,
                format!("unparseable request: {error}"),
            ))
            .expect("response serialization is infallible");
        }
    };
    let request_id = peek.get("request_id").and_then(serde_json::Value::as_u64).unwrap_or(0);

    if peek.get("kind").and_then(|kind| kind.as_str()) == Some(NEGOTIATION_REQUEST_KIND) {
        return serde_json::to_string(&NegotiationResponse::new(request_id))
            .expect("response serialization is infallible");
    }

    let request: DecisionRequest = match serde_json::from_value(peek) {
        Ok(request) => request,
        Err(error) => {
            return serde_json::to_string(&DecisionResponse::failure(
                request_id,
                WORKER_PROTOCOL_VERSION,
                format!("invalid decision request: {error}"),
            ))
            .expect("response serialization is infallible");
        }
    };
    let version = request.protocol_version;
    if version != WORKER_PROTOCOL_VERSION {
        return serde_json::to_string(&DecisionResponse::failure(
            request_id,
            version,
            format!("unsupported protocol version {version}"),
        ))
        .expect("response serialization is infallible");
    }

    let response = match request.state.into_state() {
        Err(error) => DecisionResponse::failure(request_id, version, format!("state rejected: {error}")),
        Ok(game_state) => {
            let key = match serde_json::to_string(&request.config) {
                Ok(key) => key,
                Err(error) => {
                    return serde_json::to_string(&DecisionResponse::failure(
                        request_id,
                        version,
                        format!("config serialization failed: {error}"),
                    ))
                    .expect("response serialization is infallible");
                }
            };
            let validated = match state.configs.entry(key) {
                std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
                std::collections::hash_map::Entry::Vacant(entry) => match request.config.validate() {
                    Ok(config) => entry.insert(config),
                    Err(error) => {
                        return serde_json::to_string(&DecisionResponse::failure(
                            request_id,
                            version,
                            format!("config rejected: {error}"),
                        ))
                        .expect("response serialization is infallible");
                    }
                },
            };
            let solver = koi_solver::Solver::from_validated_config(validated.with_effective_seed(request.solver_seed));
            // Attribute this decision's model time: the counter is
            // thread-local and cumulative — the before/after delta is the
            // leaf-eval share of the deciding call (0 for non-leaf solvers).
            let leaf_before = koi_solver::leaf::leaf_eval_micros();
            if validated.solver != koi_solver::SolverMethod::Resolving {
                match solver.find_best_action(&game_state, request.solver_seed) {
                    Ok(action) => {
                        let mut response = DecisionResponse::success(request_id, version, action);
                        response.leaf_eval_latency_ms =
                            koi_solver::leaf::leaf_eval_micros().saturating_sub(leaf_before) as f64 / 1000.0;
                        response
                    }
                    Err(error) => DecisionResponse::failure(request_id, version, format!("solver failed: {error}")),
                }
            } else {
                // The resolving method consumes the public ledger — the
                // same observation `into_state` just validated by replay.
                let observation = match request.state.to_observation() {
                    Ok(observation) => observation,
                    Err(error) => {
                        return serde_json::to_string(&DecisionResponse::failure(
                            request_id,
                            version,
                            format!("observation rejected: {error}"),
                        ))
                        .expect("response serialization is infallible");
                    }
                };
                let gadget = match resolving_gadget(&mut state.gadgets, &validated.resolving) {
                    Ok(gadget) => gadget,
                    Err(error) => {
                        return serde_json::to_string(&DecisionResponse::failure(
                            request_id,
                            version,
                            format!("gadget unavailable: {error}"),
                        ))
                        .expect("response serialization is infallible");
                    }
                };
                match solver.find_best_action_observed(&game_state, &observation, &gadget, request.solver_seed) {
                    Ok(action) => {
                        let mut response = DecisionResponse::success(request_id, version, action);
                        response.leaf_eval_latency_ms =
                            koi_solver::leaf::leaf_eval_micros().saturating_sub(leaf_before) as f64 / 1000.0;
                        response
                    }
                    Err(error) => DecisionResponse::failure(request_id, version, format!("solver failed: {error}")),
                }
            }
        }
    };
    serde_json::to_string(&response).expect("response serialization is infallible")
}

/// The gadget oracle for one resolving decision: the cached certified
/// blueprint when the config names an artifact (loaded and re-verified
/// once per worker, never per decision), the learned rollout otherwise —
/// the empirical guard.
fn resolving_gadget(
    gadgets: &mut std::collections::HashMap<String, std::sync::Arc<koi_solver::resolving::SafetyGadget>>,
    config: &koi_solver::config::ValidatedResolvingConfig,
) -> Result<koi_solver::resolving::GadgetOracle> {
    match &config.blueprint_artifact {
        None => Ok(koi_solver::resolving::GadgetOracle::Learned),
        Some(path) => {
            if !gadgets.contains_key(path) {
                let gadget = koi_solver::resolving::load_gadget(std::path::Path::new(path))
                    .map_err(|error| anyhow::anyhow!("blueprint artifact load failed: {error}"))?;
                gadgets.insert(path.clone(), std::sync::Arc::new(gadget));
            }
            Ok(koi_solver::resolving::GadgetOracle::Blueprint(
                gadgets.get(path).expect("inserted").clone(),
            ))
        }
    }
}

fn example_manifest() -> String {
    let manifest = Manifest {
        schema_version: koi_bench::manifest::BENCHMARK_SCHEMA_VERSION,
        run_label: "example: heuristic vs random".to_owned(),
        ruleset: "nintendo".to_owned(),
        seeds: (1..=64).collect(),
        stopping: koi_bench::manifest::StoppingSpec::Sequential {
            h0_elo: 0.0,
            h1_elo: 100.0,
            alpha: 0.05,
            beta: 0.05,
            min_clusters: 8,
            equivalence: None,
        },
        worker_timeout_seconds: 60,
        time_hard_cap_ms: Some(30_000),
        invalid_forfeit_margin: koi_bench::manifest::DEFAULT_INVALID_FORFEIT_MARGIN,
        bootstrap: koi_bench::statistics::BootstrapSpec {
            repetitions: 10_000,
            seed: 7,
            confidence_level: 0.99,
        },
        baseline: koi_bench::manifest::Entrant {
            label: "random".to_owned(),
            artifact: koi_bench::manifest::ArtifactSpec::SelfBinary,
            config: koi_solver::Config {
                solver: "random".to_owned(),
                seed: Some(1),
                ..koi_solver::Config::default()
            },
        },
        candidate: koi_bench::manifest::Entrant {
            label: "heuristic".to_owned(),
            artifact: koi_bench::manifest::ArtifactSpec::SelfBinary,
            config: koi_solver::Config {
                solver: "heuristic".to_owned(),
                seed: Some(2),
                ..koi_solver::Config::default()
            },
        },
    };
    serde_json::to_string_pretty(&manifest).expect("example manifest serializes")
}
