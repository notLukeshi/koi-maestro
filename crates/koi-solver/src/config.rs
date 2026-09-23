//! Two-stage solver configuration: a raw serde-facing `Config` validates
//! into a `ValidatedConfig` whose every field is a checked value.
//!
//! The benchmark worker receives wire configs and must never see a field it
//! does not own — `Config` keeps `deny_unknown_fields` at every level and
//! validation rejects anything outside the documented ranges, so a
//! malformed manifest fails at build time instead of degrading silently.

use serde::{Deserialize, Serialize};

use koi_core::random_nonzero_seed;

/// Hard bound on ISMCTS iterations per decision.
pub const MAX_ISMCTS_ITERATIONS: usize = 4_000_000;
/// Hard bound on the tree size grown inside one ISMCTS decision.
pub const MAX_ISMCTS_TREE_NODES_PER_DECISION: usize = 2_000_000;
/// Hard bound on PIMC determinizations per decision.
pub const MAX_PIMC_DETERMINIZATIONS: usize = 4_096;
/// Hard bound on the PIMC parallel batch size.
pub const MAX_PIMC_BATCH: usize = 1_024;
/// Bound on `score_norm`: a normalization bound larger than any reachable
/// score makes every reward identical and is a config bug.
pub const MAX_SCORE_NORM: f32 = 1_000.0;
/// Hard bound on the resolving belief support per decision.
pub const MAX_RESOLVE_WORLDS: usize = 512;
/// Hard bound on CFR+ iterations inside one resolve.
pub const MAX_RESOLVE_CFR: usize = 100_000;
/// Hard bound on the resolve's decision-ply cap.
pub const MAX_RESOLVE_DEPTH: usize = 32;

/// Errors from configuration validation or solver construction.
#[derive(Debug)]
pub enum ConfigError {
    /// A field held an out-of-range or inconsistent value.
    Invalid { field: &'static str, detail: String },
    /// A construction-time resource (solver, evaluator) could not be built.
    Construction { field: &'static str, detail: String },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid { field, detail } => write!(f, "invalid config field {field}: {detail}"),
            Self::Construction { field, detail } => {
                write!(f, "failed to construct {field}: {detail}")
            }
        }
    }
}

impl std::error::Error for ConfigError {}

fn invalid(field: &'static str, detail: impl Into<String>) -> ConfigError {
    ConfigError::Invalid {
        field,
        detail: detail.into(),
    }
}

/// The search backend a solver dispatches to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum SolverMethod {
    /// Uniform random legal action — the floor baseline.
    Random,
    /// Greedy Yaku-aware heuristic — deterministic, no sampling.
    Heuristic,
    /// Information-set MCTS with determinization and greedy rollouts.
    Ismcts,
    /// Perfect-information Monte Carlo over sampled worlds.
    Pimc,
    /// Heuristic backbone plus the P2 endgame stack: solved stop decisions
    /// (`optimal_stopping` → belief-weighted residual subgame) and the exact
    /// turn-8 solver once the observer's hand is down to one card (the only
    /// point where per-world minimax is exact rather than clairvoyant).
    Endgame,
    /// The P3 continual resolver: belief + safety gadget + CFR+ subgame
    /// solve at every decision. Requires the public ledger — the bare
    /// `find_best_action` path fails closed on it.
    Resolving,
    /// The trained leaf policy head played directly — masked-argmax over
    /// the canonical action order, one model call per decision, no
    /// search. The belief block is sampled fresh per decision from the
    /// acting player's unseen set (the labeler path, not the ledger).
    LeafPolicy,
    /// A scripted archetype baseline — the opponent model's portfolio
    /// members as standalone entrants for the P3 ablation panel.
    Archetype,
}

impl SolverMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Random => "random",
            Self::Heuristic => "heuristic",
            Self::Ismcts => "ismcts",
            Self::Pimc => "pimc",
            Self::Endgame => "endgame",
            Self::Resolving => "resolving",
            Self::LeafPolicy => "leaf_policy",
            Self::Archetype => "archetype",
        }
    }
}

impl std::str::FromStr for SolverMethod {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "random" => Ok(Self::Random),
            "heuristic" => Ok(Self::Heuristic),
            "ismcts" => Ok(Self::Ismcts),
            "pimc" | "perfect_play" => Ok(Self::Pimc),
            "endgame" => Ok(Self::Endgame),
            "resolving" | "resolving_adaptive" => Ok(Self::Resolving),
            "leaf_policy" | "leafpolicy" => Ok(Self::LeafPolicy),
            "archetype" => Ok(Self::Archetype),
            other => Err(format!("unknown solver method '{other}'")),
        }
    }
}

/// How PIMC aggregates a root action's per-world outcomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ScoringMethod {
    /// Mean normalized margin across worlds.
    Mean,
    /// Fraction of worlds where the action leads the margin.
    WinRate,
}

impl ScoringMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mean => "mean",
            Self::WinRate => "win_rate",
        }
    }
}

impl std::str::FromStr for ScoringMethod {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "mean" => Ok(Self::Mean),
            "win_rate" | "frequency" => Ok(Self::WinRate),
            other => Err(format!("unknown pimc scoring method '{other}'")),
        }
    }
}

/// ISMCTS-specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IsmctsConfig {
    /// Tree-search iterations per decision.
    #[serde(default = "defaults::ismcts_iterations")]
    pub iterations: usize,
    /// Normalization bound for rollout margins (≈ the largest plausible
    /// round margin; values beyond it clamp to the extremes).
    #[serde(default = "defaults::score_norm")]
    pub score_norm: f32,
}

impl Default for IsmctsConfig {
    fn default() -> Self {
        Self {
            iterations: defaults::ismcts_iterations(),
            score_norm: defaults::score_norm(),
        }
    }
}

/// PIMC-specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PimcConfig {
    /// Determinizations sampled per decision.
    #[serde(default = "defaults::pimc_simulations")]
    pub max_simulations: usize,
    /// Rayon chunk size over the determinization list.
    #[serde(default = "defaults::pimc_batch")]
    pub batch_size: usize,
    /// Outcome aggregation: "mean" or "win_rate".
    #[serde(default = "defaults::pimc_scoring")]
    pub scoring_method: String,
}

impl Default for PimcConfig {
    fn default() -> Self {
        Self {
            max_simulations: defaults::pimc_simulations(),
            batch_size: defaults::pimc_batch(),
            scoring_method: defaults::pimc_scoring(),
        }
    }
}

/// Resolving-specific configuration (P3).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvingConfig {
    /// Belief cap: maximum opponent-hand worlds per resolve.
    #[serde(default = "defaults::resolve_worlds")]
    pub max_worlds: usize,
    /// Compiled gadget-tree node budget.
    #[serde(default = "defaults::resolve_nodes")]
    pub max_nodes: usize,
    /// CFR+ iterations over the gadget tree.
    #[serde(default = "defaults::resolve_cfr_iterations")]
    pub cfr_iterations: usize,
    /// Decision-ply cap below each world root.
    #[serde(default = "defaults::resolve_depth")]
    pub max_decision_depth: usize,
    /// Opponent-model belief reweight + archetype posterior (the
    /// "adaptive" in `resolving_adaptive`). `false` resolves on the
    /// uniform-chance prior — the ablation arm.
    #[serde(default = "defaults::true_val")]
    pub adaptive: bool,
    /// OX exploitation arm. Only meaningful with `adaptive`; gated behind
    /// the shrunk posterior's activation threshold. Off by default — an
    /// OX claim additionally requires the measured baseline the P3 plan
    /// demands, which is an evidence discipline, not a config knob.
    #[serde(default)]
    pub ox: bool,
    /// Blueprint artifact path for the certified gadget oracle. `None`
    /// selects the learned rollout oracle — empirical guard only.
    #[serde(default)]
    pub blueprint_artifact: Option<String>,
    /// Learned leaf evaluator for oracle leaves: `"builtin:handcrafted"`
    /// selects the deterministic fallback evaluator; any other value is an
    /// ONNX model path (requires the `leaf-ort` build feature). `None`
    /// prices leaves through the margin oracle as before. Per-call model
    /// failures degrade to the oracle — counted, never silent.
    #[serde(default)]
    pub leaf_model: Option<String>,
}

impl Default for ResolvingConfig {
    fn default() -> Self {
        Self {
            max_worlds: defaults::resolve_worlds(),
            max_nodes: defaults::resolve_nodes(),
            cfr_iterations: defaults::resolve_cfr_iterations(),
            max_decision_depth: defaults::resolve_depth(),
            adaptive: defaults::true_val(),
            ox: false,
            blueprint_artifact: None,
            leaf_model: None,
        }
    }
}

/// Leaf-policy configuration (P6): the trained policy head played
/// directly — masked-argmax over the canonical action order, no search.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LeafPolicyConfig {
    /// ONNX model path or `"builtin:handcrafted"`; `None` selects the
    /// handcrafted evaluator — the deterministic zero-cost baseline whose
    /// uniform-over-legal policy argmaxes to the first canonical action.
    /// Any other value is an ONNX model path (requires the `leaf-ort`
    /// build feature). Per-call model failures degrade to the P1
    /// heuristic pick — counted via `leaf_fallback_count`, never silent.
    #[serde(default)]
    pub leaf_model: Option<String>,
}

/// Raw, serde-facing solver configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Solver backend: "random", "heuristic", "ismcts", "pimc",
    /// "endgame", "resolving" (alias "resolving_adaptive"), "leaf_policy",
    /// or "archetype".
    #[serde(default = "defaults::solver")]
    pub solver: String,
    /// The scripted archetype when `solver == "archetype"`.
    #[serde(default)]
    pub archetype: Option<String>,
    #[serde(default)]
    pub ismcts: IsmctsConfig,
    #[serde(default)]
    pub pimc: PimcConfig,
    #[serde(default)]
    pub resolving: ResolvingConfig,
    #[serde(default)]
    pub leaf_policy: LeafPolicyConfig,
    /// Root-parallel search: schedule-invariant for `ismcts` (fixed tree
    /// count) and `pimc` (per-world outcomes are independent), a provenance
    /// lie elsewhere.
    #[serde(default = "defaults::true_val")]
    pub use_parallel: bool,
    /// Process logging level: error|warn|info|debug|trace.
    #[serde(default = "defaults::log_level")]
    pub log_level: String,
    /// Master seed. `None`/`0` resolves to a fresh nonzero seed.
    pub seed: Option<u64>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            solver: defaults::solver(),
            archetype: None,
            ismcts: IsmctsConfig::default(),
            pimc: PimcConfig::default(),
            resolving: ResolvingConfig::default(),
            leaf_policy: LeafPolicyConfig::default(),
            use_parallel: defaults::true_val(),
            log_level: defaults::log_level(),
            seed: None,
        }
    }
}

impl Config {
    pub fn validate(self) -> Result<ValidatedConfig, ConfigError> {
        let solver: SolverMethod = self
            .solver
            .parse()
            .map_err(|detail: String| invalid("solver", detail))?;
        if self.ismcts.iterations == 0 || self.ismcts.iterations > MAX_ISMCTS_ITERATIONS {
            return Err(invalid(
                "ismcts.iterations",
                format!("must be in 1..={MAX_ISMCTS_ITERATIONS}"),
            ));
        }
        if !self.ismcts.score_norm.is_finite()
            || self.ismcts.score_norm <= 0.0
            || self.ismcts.score_norm > MAX_SCORE_NORM
        {
            return Err(invalid(
                "ismcts.score_norm",
                format!("must be finite and in (0, {MAX_SCORE_NORM}]"),
            ));
        }
        if self.pimc.max_simulations == 0 || self.pimc.max_simulations > MAX_PIMC_DETERMINIZATIONS {
            return Err(invalid(
                "pimc.max_simulations",
                format!("must be in 1..={MAX_PIMC_DETERMINIZATIONS}"),
            ));
        }
        if self.pimc.batch_size == 0 || self.pimc.batch_size > MAX_PIMC_BATCH {
            return Err(invalid("pimc.batch_size", format!("must be in 1..={MAX_PIMC_BATCH}")));
        }
        let scoring_method: ScoringMethod = self
            .pimc
            .scoring_method
            .parse()
            .map_err(|detail: String| invalid("pimc.scoring_method", detail))?;
        let archetype = match (solver, &self.archetype) {
            (SolverMethod::Archetype, Some(name)) => Some(
                name.parse::<crate::resolving::Archetype>()
                    .map_err(|detail: String| invalid("archetype", detail))?,
            ),
            (SolverMethod::Archetype, None) => {
                return Err(invalid("archetype", "solver \"archetype\" needs an archetype name"));
            }
            (_, Some(_)) => {
                return Err(invalid(
                    "archetype",
                    "archetype is only valid with solver \"archetype\"",
                ));
            }
            (_, None) => None,
        };
        if self.resolving.max_worlds == 0 || self.resolving.max_worlds > MAX_RESOLVE_WORLDS {
            return Err(invalid(
                "resolving.max_worlds",
                format!("must be in 1..={MAX_RESOLVE_WORLDS}"),
            ));
        }
        if self.resolving.max_nodes == 0 || self.resolving.max_nodes > crate::efg::MAX_EFG_NODES {
            return Err(invalid(
                "resolving.max_nodes",
                format!("must be in 1..={}", crate::efg::MAX_EFG_NODES),
            ));
        }
        if self.resolving.cfr_iterations == 0 || self.resolving.cfr_iterations > MAX_RESOLVE_CFR {
            return Err(invalid(
                "resolving.cfr_iterations",
                format!("must be in 1..={MAX_RESOLVE_CFR}"),
            ));
        }
        if self.resolving.max_decision_depth == 0 || self.resolving.max_decision_depth > MAX_RESOLVE_DEPTH {
            return Err(invalid(
                "resolving.max_decision_depth",
                format!("must be in 1..={MAX_RESOLVE_DEPTH}"),
            ));
        }
        if self.resolving.ox && !self.resolving.adaptive {
            return Err(invalid("resolving.ox", "ox requires the adaptive opponent model"));
        }
        if let Some(leaf_model) = &self.resolving.leaf_model {
            if leaf_model.trim().is_empty() {
                return Err(invalid(
                    "resolving.leaf_model",
                    "must name a model path or \"builtin:handcrafted\"",
                ));
            }
        }
        if let Some(leaf_model) = &self.leaf_policy.leaf_model {
            if leaf_model.trim().is_empty() {
                return Err(invalid(
                    "leaf_policy.leaf_model",
                    "must name a model path or \"builtin:handcrafted\"",
                ));
            }
        }
        let log_level = self.log_level.trim().to_ascii_lowercase();
        if !matches!(log_level.as_str(), "error" | "warn" | "info" | "debug" | "trace") {
            return Err(invalid("log_level", "must be one of error|warn|info|debug|trace"));
        }
        let seed = self.seed.unwrap_or_else(random_nonzero_seed);
        let seed = if seed == 0 { random_nonzero_seed() } else { seed };
        Ok(ValidatedConfig {
            solver,
            archetype,
            ismcts: ValidatedIsmctsConfig {
                iterations: self.ismcts.iterations,
                score_norm: self.ismcts.score_norm,
            },
            pimc: ValidatedPimcConfig {
                max_simulations: self.pimc.max_simulations,
                batch_size: self.pimc.batch_size,
                scoring_method,
            },
            resolving: ValidatedResolvingConfig {
                spec: crate::resolving::ResolveSpec {
                    max_worlds: self.resolving.max_worlds,
                    max_nodes: self.resolving.max_nodes,
                    cfr_iterations: self.resolving.cfr_iterations,
                    max_decision_depth: self.resolving.max_decision_depth,
                },
                adaptive: self.resolving.adaptive,
                ox: self.resolving.ox,
                blueprint_artifact: self.resolving.blueprint_artifact.clone(),
                leaf_model: self.resolving.leaf_model.clone(),
            },
            leaf_policy: ValidatedLeafPolicyConfig {
                leaf_model: self.leaf_policy.leaf_model.clone(),
            },
            use_parallel: self.use_parallel,
            log_level,
            seed,
        })
    }
}

/// Fully validated configuration — the only shape the solver consumes.
#[derive(Debug, Clone)]
pub struct ValidatedConfig {
    pub solver: SolverMethod,
    /// The scripted archetype when `solver == SolverMethod::Archetype`.
    pub archetype: Option<crate::resolving::Archetype>,
    pub ismcts: ValidatedIsmctsConfig,
    pub pimc: ValidatedPimcConfig,
    pub resolving: ValidatedResolvingConfig,
    pub leaf_policy: ValidatedLeafPolicyConfig,
    pub use_parallel: bool,
    pub log_level: String,
    seed: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct ValidatedIsmctsConfig {
    pub iterations: usize,
    pub score_norm: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct ValidatedPimcConfig {
    pub max_simulations: usize,
    pub batch_size: usize,
    pub scoring_method: ScoringMethod,
}

/// Validated resolving knobs: the resolve spec plus the arm switches.
#[derive(Debug, Clone)]
pub struct ValidatedResolvingConfig {
    /// The resolve budget.
    pub spec: crate::resolving::ResolveSpec,
    /// Opponent-model belief reweight + archetype posterior.
    pub adaptive: bool,
    /// OX arm (gated on the shrunk posterior threshold at use time).
    pub ox: bool,
    /// Blueprint artifact path for the certified gadget oracle.
    pub blueprint_artifact: Option<String>,
    /// Learned leaf evaluator path or `"builtin:handcrafted"`, else `None`.
    pub leaf_model: Option<String>,
}

/// Validated leaf-policy knobs (P6): the model spec only — there are no
/// budget fields because the entrant makes exactly one model call per
/// decision.
#[derive(Debug, Clone)]
pub struct ValidatedLeafPolicyConfig {
    /// ONNX model path or `"builtin:handcrafted"`; `None` → handcrafted.
    pub leaf_model: Option<String>,
}

impl Default for ValidatedConfig {
    fn default() -> Self {
        Config::default().validate().expect("the default config validates")
    }
}

impl ValidatedConfig {
    /// The resolved master seed — always nonzero.
    pub fn resolved_seed(&self) -> u64 {
        self.seed
    }

    /// Returns a copy pinned to `seed` — the worker's per-decision replay
    /// contract injects `solver_seed` as the sole RNG source.
    pub fn with_effective_seed(&self, seed: u64) -> Self {
        let mut config = self.clone();
        config.seed = seed;
        config
    }

    /// The canonical, serialization-stable provenance snapshot: every knob
    /// that can change solver behavior, nothing else.
    pub fn effective_solver(&self) -> EffectiveSolverConfig {
        EffectiveSolverConfig {
            schema_version: 3,
            solver: self.solver.as_str().to_owned(),
            settings: EffectiveSolverSettings {
                ismcts: EffectiveIsmctsConfig {
                    iterations: self.ismcts.iterations,
                    score_norm: self.ismcts.score_norm,
                    max_tree_nodes_per_decision: MAX_ISMCTS_TREE_NODES_PER_DECISION,
                },
                pimc: EffectivePimcConfig {
                    max_simulations: self.pimc.max_simulations,
                    batch_size: self.pimc.batch_size,
                    scoring_method: self.pimc.scoring_method.as_str().to_owned(),
                },
                resolving: EffectiveResolvingConfig {
                    archetype: self.archetype.map(|archetype| archetype.as_str().to_owned()),
                    max_worlds: self.resolving.spec.max_worlds,
                    max_nodes: self.resolving.spec.max_nodes,
                    cfr_iterations: self.resolving.spec.cfr_iterations,
                    max_decision_depth: self.resolving.spec.max_decision_depth,
                    adaptive: self.resolving.adaptive,
                    ox: self.resolving.ox,
                    blueprint_artifact: self.resolving.blueprint_artifact.clone(),
                    leaf_model: self.resolving.leaf_model.clone(),
                },
                leaf_policy: EffectiveLeafPolicyConfig {
                    leaf_model: self.leaf_policy.leaf_model.clone(),
                },
            },
            seed: self.seed,
        }
    }
}

/// Serializable effective-config snapshot for provenance hashing.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EffectiveSolverConfig {
    pub schema_version: u32,
    pub solver: String,
    pub settings: EffectiveSolverSettings,
    pub seed: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EffectiveSolverSettings {
    pub ismcts: EffectiveIsmctsConfig,
    pub pimc: EffectivePimcConfig,
    pub resolving: EffectiveResolvingConfig,
    pub leaf_policy: EffectiveLeafPolicyConfig,
}

/// Provenance snapshot of the resolving knobs (P3).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EffectiveResolvingConfig {
    /// The scripted archetype for `solver == "archetype"`, else `None`.
    pub archetype: Option<String>,
    pub max_worlds: usize,
    pub max_nodes: usize,
    pub cfr_iterations: usize,
    pub max_decision_depth: usize,
    pub adaptive: bool,
    pub ox: bool,
    pub blueprint_artifact: Option<String>,
    pub leaf_model: Option<String>,
}

/// Provenance snapshot of the leaf-policy model binding (P6).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EffectiveLeafPolicyConfig {
    /// ONNX model path or `"builtin:handcrafted"`, else `None`.
    pub leaf_model: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EffectiveIsmctsConfig {
    pub iterations: usize,
    pub score_norm: f32,
    pub max_tree_nodes_per_decision: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EffectivePimcConfig {
    pub max_simulations: usize,
    pub batch_size: usize,
    pub scoring_method: String,
}

mod defaults {
    pub fn ismcts_iterations() -> usize {
        10_000
    }
    pub fn score_norm() -> f32 {
        20.0
    }
    pub fn pimc_simulations() -> usize {
        64
    }
    pub fn pimc_batch() -> usize {
        32
    }
    pub fn pimc_scoring() -> String {
        "mean".to_owned()
    }
    pub fn solver() -> String {
        "heuristic".to_owned()
    }
    pub fn log_level() -> String {
        "warn".to_owned()
    }
    pub fn true_val() -> bool {
        true
    }
    pub fn resolve_worlds() -> usize {
        24
    }
    pub fn resolve_nodes() -> usize {
        600_000
    }
    pub fn resolve_cfr_iterations() -> usize {
        300
    }
    pub fn resolve_depth() -> usize {
        6
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_documented_solver_name_parses() {
        for (name, expected) in [
            ("random", SolverMethod::Random),
            ("heuristic", SolverMethod::Heuristic),
            ("ISMCTS", SolverMethod::Ismcts),
            ("pimc", SolverMethod::Pimc),
            ("endgame", SolverMethod::Endgame),
            ("resolving", SolverMethod::Resolving),
            ("leaf_policy", SolverMethod::LeafPolicy),
        ] {
            let config = Config {
                solver: name.to_owned(),
                ..Config::default()
            }
            .validate()
            .unwrap();
            assert_eq!(config.solver, expected);
        }
        let archetype = Config {
            solver: "archetype".to_owned(),
            archetype: Some("banker".to_owned()),
            ..Config::default()
        }
        .validate()
        .unwrap();
        assert_eq!(archetype.solver, SolverMethod::Archetype);
        assert_eq!(archetype.archetype, Some(crate::resolving::Archetype::Banker));
        assert!(Config {
            solver: "monte-carlo".to_owned(),
            ..Config::default()
        }
        .validate()
        .is_err());
    }

    #[test]
    fn archetype_and_resolving_validation_is_fail_closed() {
        // An archetype solver without a name is rejected.
        assert!(Config {
            solver: "archetype".to_owned(),
            ..Config::default()
        }
        .validate()
        .is_err());
        // A stray archetype name under another solver is rejected.
        assert!(Config {
            solver: "heuristic".to_owned(),
            archetype: Some("banker".to_owned()),
            ..Config::default()
        }
        .validate()
        .is_err());
        // An unknown archetype name is rejected.
        assert!(Config {
            solver: "archetype".to_owned(),
            archetype: Some("cheater".to_owned()),
            ..Config::default()
        }
        .validate()
        .is_err());
        // OX without the adaptive model is contradictory — rejected.
        assert!(Config {
            solver: "resolving".to_owned(),
            resolving: ResolvingConfig {
                adaptive: false,
                ox: true,
                ..ResolvingConfig::default()
            },
            ..Config::default()
        }
        .validate()
        .is_err());
        // Zero budgets are rejected.
        assert!(Config {
            solver: "resolving".to_owned(),
            resolving: ResolvingConfig {
                max_worlds: 0,
                ..ResolvingConfig::default()
            },
            ..Config::default()
        }
        .validate()
        .is_err());
    }

    #[test]
    fn rejects_out_of_range_budgets() {
        for config in [
            Config {
                ismcts: IsmctsConfig {
                    iterations: 0,
                    ..IsmctsConfig::default()
                },
                ..Config::default()
            },
            Config {
                ismcts: IsmctsConfig {
                    iterations: MAX_ISMCTS_ITERATIONS + 1,
                    ..IsmctsConfig::default()
                },
                ..Config::default()
            },
            Config {
                pimc: PimcConfig {
                    max_simulations: 0,
                    ..PimcConfig::default()
                },
                ..Config::default()
            },
            Config {
                pimc: PimcConfig {
                    batch_size: 0,
                    ..PimcConfig::default()
                },
                ..Config::default()
            },
            Config {
                ismcts: IsmctsConfig {
                    score_norm: f32::NAN,
                    ..IsmctsConfig::default()
                },
                ..Config::default()
            },
        ] {
            assert!(config.validate().is_err());
        }
    }

    #[test]
    fn a_random_seed_resolves_once_and_is_nonzero() {
        let config = Config::default().validate().unwrap();
        assert_ne!(config.resolved_seed(), 0);
        assert_eq!(config.resolved_seed(), config.resolved_seed());
        let pinned = config.with_effective_seed(42);
        assert_eq!(pinned.resolved_seed(), 42);
        assert_eq!(pinned.effective_solver().seed, 42);
    }

    #[test]
    fn unknown_fields_are_rejected() {
        assert!(serde_json::from_str::<Config>(r#"{"solver":"heuristic","solvre":"x"}"#).is_err());
        assert!(serde_json::from_str::<IsmctsConfig>(r#"{"iterationz":10}"#).is_err());
    }
}
