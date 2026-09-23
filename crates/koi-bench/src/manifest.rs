//! Benchmark manifest: the declarative contract a paired run validates
//! against (schema 1).
//!
//! Every field that changes evidence is bound here — artifact identities,
//! solver configs, the deal-seed panel, stopping rule, ruleset, and the
//! bootstrap/timeout budgets — and the manifest's canonical hash rides in
//! every checkpoint header so a foreign checkpoint fails closed instead of
//! silently resuming.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::statistics::BootstrapSpec;

/// The newest manifest schema this build serves.
pub const BENCHMARK_SCHEMA_VERSION: u32 = 1;
/// The oldest manifest schema still accepted.
pub const MIN_BENCHMARK_SCHEMA_VERSION: u32 = 1;
/// Hard bound on a worker's per-transaction liveness window.
pub const MAX_WORKER_TIMEOUT_SECONDS: u64 = 600;
/// Hard bound on the deal-cluster panel size.
pub const MAX_CLUSTERS: usize = 10_000;
/// Hard bound on bootstrap repetitions.
pub const MAX_BOOTSTRAP_REPETITIONS: usize = 1_000_000;
/// Hard bound on total bootstrap draws (repetitions × clusters).
pub const MAX_BOOTSTRAP_DRAWS: u64 = 100_000_000;
/// Hard bound on trace entries across the whole run — the checkpoint file
/// is append-only and must stay bounded.
pub const MAX_TOTAL_TRACE_ENTRIES: usize = 10_000_000;
/// Hard bound on phase-actions inside one leg's round: 16 player-turns ×
/// 3 phases, with slack for stop decisions.
pub const MAX_ACTIONS_PER_ROUND: usize = 64;
/// The default margin a forfeited leg records for the offender.
/// Forfeit margin must exceed the largest leg margin the game can produce
/// (max stacked yaku ≈ 50 base × 4 multiplier ceiling ≈ 200) so a forfeit
/// can never improve on a played-out loss.
pub const DEFAULT_INVALID_FORFEIT_MARGIN: f64 = 256.0;

/// How an entrant's worker executable is resolved.
///
/// - `self`: the running referee binary is re-spawned as `koi-bench worker`
///   — the canonical paired-evidence path, since one binary can serve two
///   different configs.
/// - `path`: an existing executable is spawned directly.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ArtifactSpec {
    /// The referee binary re-spawned in worker mode.
    SelfBinary,
    /// An explicit executable path (relative to the manifest directory).
    Path { path: PathBuf },
}

/// A resolved artifact's stable identity for provenance binding.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactIdentity {
    pub label: String,
    pub executable: PathBuf,
    /// `self`-spawned artifacts carry the referee's own build stamps.
    pub build_commit: String,
    pub build_tree: String,
    pub build_dirty: bool,
    /// FNV-1a/64 over the executable image — binds the artifact bytes.
    pub binary_hash_fnv1a64: String,
    /// FNV-1a/64 of the entrant's serialized solver config — attests which
    /// settings the artifact played under without the manifest in hand.
    #[serde(default)]
    pub config_hash_fx64: String,
}

/// One tournament entrant: an artifact plus the config it plays under.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entrant {
    pub label: String,
    pub artifact: ArtifactSpec,
    /// The worker's solver config — schema identical to
    /// `koi_solver::Config`; `deny_unknown_fields` is inherited.
    pub config: koi_solver::Config,
}

/// How the run decides when to stop consuming deal clusters.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum StoppingSpec {
    /// Fixed schedule: consume the whole panel.
    Fixed,
    /// GSPRT sequential test on the paired cluster stream.
    Sequential {
        /// Elo of the null hypothesis (usually 0 — no difference).
        h0_elo: f64,
        /// Elo of the alternative the test is powered for.
        h1_elo: f64,
        /// Type-I error bound.
        alpha: f64,
        /// Type-II error bound.
        beta: f64,
        /// Clusters that must complete before the sequential rule may fire
        /// — an LLR computed on fewer clusters is not evidence.
        min_clusters: usize,
        /// Indifference-band equivalence certification, armed alongside
        /// the GSPRT: when the empirical-Bernstein CS fits inside
        /// `0.5 ± delta` the pair is certified equivalent and stops.
        /// Absent leaves the run GSPRT-only.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        equivalence: Option<crate::sprt::EquivalenceSpec>,
    },
}

/// The paired-benchmark manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// Must equal `BENCHMARK_SCHEMA_VERSION` exactly.
    pub schema_version: u32,
    /// Free-form run label for the report header.
    pub run_label: String,
    /// The ruleset both entrants play under: "nintendo" or "fuda_wiki".
    /// House rules need a wire format that does not exist yet — declaring
    /// one fails validation rather than silently picking a default.
    pub ruleset: String,
    /// Deal-cluster seed panel — the resampling unit (E1).
    pub seeds: Vec<u64>,
    pub stopping: StoppingSpec,
    /// Per-transaction worker liveness bound.
    pub worker_timeout_seconds: u64,
    /// Optional per-leg latency bank: an artifact whose summed
    /// `decision_latency_ms` crosses this cap forfeits the leg as
    /// `TimeForfeit`. `None` disables the clock.
    #[serde(default)]
    pub time_hard_cap_ms: Option<u64>,
    /// Margin recorded against a forfeiting artifact.
    #[serde(default = "default_forfeit_margin")]
    pub invalid_forfeit_margin: f64,
    /// Cluster-bootstrap interval parameters (must be ≥ 0.99 confidence).
    pub bootstrap: BootstrapSpec,
    pub baseline: Entrant,
    pub candidate: Entrant,
}

fn default_forfeit_margin() -> f64 {
    DEFAULT_INVALID_FORFEIT_MARGIN
}

/// How the tournament's shared deal panel is specified. `Generated`
/// expands deterministically — a candidate stream `mix_seed(seed, i)`
/// filtered to anomaly-free deals — so the panel content is derivable
/// from the spec while the expanded hash still binds it in `freeze.json`.
/// `Explicit` carries the seed list verbatim (imported/pilot panels).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PanelSpec {
    /// Expand `mix_seed(seed, i)` for i = 0.., keeping deals with no
    /// `DealAnomaly` (Teshi / FourPairs / FieldVoid) until `size` survive.
    Generated { seed: u64, size: usize },
    /// A verbatim seed list — must itself be anomaly-free; verified at
    /// emit/validate time, never silently filtered.
    Explicit { seeds: Vec<u64> },
}

/// The tournament manifest: one shared protocol block plus the entrant
/// roster. `tournament_emit` expands it into pairwise [`Manifest`]s on the
/// shared panel; `tournament_rank` merges the pairings' evidence back.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TournamentManifest {
    /// Must equal `BENCHMARK_SCHEMA_VERSION` exactly.
    pub schema_version: u32,
    pub run_label: String,
    /// "nintendo" for the canonical field; "fuda_wiki" cross-checks only.
    pub ruleset: String,
    /// The shared deal panel — the field runs every pairing on the same
    /// anomaly-free seeds so deal difficulty cancels globally (R-02).
    pub panel: PanelSpec,
    /// Sequential stopping applied per pairing, verbatim into each
    /// emitted pairwise manifest.
    pub stopping: StoppingSpec,
    pub worker_timeout_seconds: u64,
    #[serde(default)]
    pub time_hard_cap_ms: Option<u64>,
    #[serde(default = "default_forfeit_margin")]
    pub invalid_forfeit_margin: f64,
    /// Bootstrap spec for both the per-pair reports and the ranking fit.
    pub bootstrap: BootstrapSpec,
    /// The roster — every unordered pair plays one pairing.
    pub entrants: Vec<Entrant>,
}

/// Expands a `Generated` panel: deterministic anomaly-free seeds. `mix`
/// folds a fixed domain tag into splitmix64 so the stream is stable and
/// disjoint from solver/deal streams.
pub fn expand_panel(panel: &PanelSpec, ruleset: koi_core::Ruleset) -> Result<Vec<u64>> {
    match panel {
        PanelSpec::Explicit { seeds } => {
            for &seed in seeds {
                let (state, anomaly) = koi_core::KoiGameState::new_deal(koi_core::deal_from_seed(seed), ruleset);
                let _ = state;
                if anomaly.is_some() {
                    bail!("explicit panel seed {seed} deals an anomaly — the panel must be anomaly-free");
                }
            }
            Ok(seeds.clone())
        }
        PanelSpec::Generated { seed, size } => {
            const PANEL_TAG: u64 = 0x4b4f_4950_414e_454c; // "KOIPANEL"
            let mut seeds = Vec::with_capacity(*size);
            let mut index = 0u64;
            while seeds.len() < *size {
                let candidate = mix_seed(*seed ^ PANEL_TAG, index);
                index += 1;
                let (state, anomaly) = koi_core::KoiGameState::new_deal(koi_core::deal_from_seed(candidate), ruleset);
                let _ = state;
                if anomaly.is_none() {
                    seeds.push(candidate);
                }
                if index > 1_000_000 {
                    bail!("panel expansion failed to find {size} anomaly-free seeds within 1M candidates");
                }
            }
            Ok(seeds)
        }
    }
}

/// splitmix64 over (global, index) — the panel stream's mixer.
fn mix_seed(global: u64, index: u64) -> u64 {
    let mut z = global.wrapping_add(index).wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

impl TournamentManifest {
    pub fn load(path: &Path) -> Result<Self> {
        let text =
            std::fs::read_to_string(path).with_context(|| format!("failed to read manifest {}", path.display()))?;
        let manifest: Self =
            serde_json::from_str(&text).with_context(|| format!("failed to parse manifest {}", path.display()))?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> Result<()> {
        if self.schema_version != BENCHMARK_SCHEMA_VERSION {
            bail!(
                "unsupported tournament schema {}: this build serves {}..={}",
                self.schema_version,
                MIN_BENCHMARK_SCHEMA_VERSION,
                BENCHMARK_SCHEMA_VERSION
            );
        }
        if self.run_label.trim().is_empty() || self.run_label.chars().count() > 128 {
            bail!("run_label must be non-empty and at most 128 characters");
        }
        match self.ruleset.as_str() {
            "nintendo" | "fuda_wiki" => {}
            other => bail!("unsupported ruleset '{other}': expected nintendo|fuda_wiki"),
        }
        match &self.panel {
            PanelSpec::Generated { size, .. } => {
                if *size == 0 || *size > MAX_CLUSTERS {
                    bail!("panel size must be in 1..={MAX_CLUSTERS}");
                }
            }
            PanelSpec::Explicit { seeds } => {
                if seeds.is_empty() || seeds.len() > MAX_CLUSTERS {
                    bail!("panel seeds must contain 1..={MAX_CLUSTERS} deal clusters");
                }
                let mut sorted = seeds.clone();
                sorted.sort_unstable();
                sorted.dedup();
                if sorted.len() != seeds.len() {
                    bail!("panel seeds must be distinct");
                }
            }
        }
        if self.worker_timeout_seconds == 0 || self.worker_timeout_seconds > MAX_WORKER_TIMEOUT_SECONDS {
            bail!("worker_timeout_seconds must be in 1..={MAX_WORKER_TIMEOUT_SECONDS}");
        }
        if let Some(cap) = self.time_hard_cap_ms {
            if cap == 0 {
                bail!("time_hard_cap_ms must be positive when present");
            }
        }
        if !self.invalid_forfeit_margin.is_finite() || self.invalid_forfeit_margin <= 0.0 {
            bail!("invalid_forfeit_margin must be finite and positive");
        }
        if self.bootstrap.repetitions == 0 || self.bootstrap.repetitions > MAX_BOOTSTRAP_REPETITIONS {
            bail!("bootstrap.repetitions must be in 1..={MAX_BOOTSTRAP_REPETITIONS}");
        }
        if !self.bootstrap.confidence_level.is_finite()
            || self.bootstrap.confidence_level < 0.99
            || self.bootstrap.confidence_level >= 1.0
        {
            bail!("bootstrap.confidence_level must be in [0.99, 1.0)");
        }
        if self.entrants.len() < 2 || self.entrants.len() > 16 {
            bail!("a tournament needs 2..=16 entrants");
        }
        for (index, entrant) in self.entrants.iter().enumerate() {
            validate_entrant(entrant, &format!("entrants[{index}]"))?;
            if self.entrants[..index].iter().any(|other| other.label == entrant.label) {
                bail!("entrant labels must be distinct: '{}'", entrant.label);
            }
        }
        // The stopping spec is validated against the *expanded* panel —
        // min_clusters bounds are relative to the cluster cap, and the
        // equivalence block arms on the same basis.
        let seeds = expand_panel(&self.panel, self.ruleset())?;
        let probe = Manifest {
            schema_version: self.schema_version,
            run_label: self.run_label.clone(),
            ruleset: self.ruleset.clone(),
            seeds,
            stopping: self.stopping.clone(),
            worker_timeout_seconds: self.worker_timeout_seconds,
            time_hard_cap_ms: self.time_hard_cap_ms,
            invalid_forfeit_margin: self.invalid_forfeit_margin,
            bootstrap: self.bootstrap.clone(),
            baseline: self.entrants[0].clone(),
            candidate: self.entrants[1].clone(),
        };
        probe.validate()
    }

    /// The parsed ruleset every entrant plays under.
    pub fn ruleset(&self) -> koi_core::Ruleset {
        match self.ruleset.as_str() {
            "fuda_wiki" => koi_core::Ruleset::fuda_wiki(),
            _ => koi_core::Ruleset::nintendo(),
        }
    }

    /// The pairwise manifest for one unordered pair: `first` takes the
    /// baseline role, `second` the candidate role — matching the ranking's
    /// `candidate_win_score = second's score` convention.
    pub fn pair_manifest(&self, first: usize, second: usize, seeds: &[u64]) -> Result<Manifest> {
        if first >= second || second >= self.entrants.len() {
            bail!("pair_manifest requires first < second < {}", self.entrants.len());
        }
        Ok(Manifest {
            schema_version: self.schema_version,
            run_label: format!(
                "{}: {} vs {}",
                self.run_label, self.entrants[first].label, self.entrants[second].label
            ),
            ruleset: self.ruleset.clone(),
            seeds: seeds.to_vec(),
            stopping: self.stopping.clone(),
            worker_timeout_seconds: self.worker_timeout_seconds,
            time_hard_cap_ms: self.time_hard_cap_ms,
            invalid_forfeit_margin: self.invalid_forfeit_margin,
            bootstrap: self.bootstrap.clone(),
            baseline: self.entrants[first].clone(),
            candidate: self.entrants[second].clone(),
        })
    }

    /// Canonical manifest hash — FNV-1a/64 over the reserialized manifest.
    pub fn canonical_hash_fx64(&self) -> Result<String> {
        let canonical = serde_json::to_vec(self).context("tournament manifest reserialization failed")?;
        Ok(fnv1a64_hex(&canonical))
    }
}

impl Manifest {
    /// Loads and validates a manifest file. Fails closed on any violation —
    /// an invalid manifest is never partially honored.
    pub fn load(path: &Path) -> Result<Self> {
        let text =
            std::fs::read_to_string(path).with_context(|| format!("failed to read manifest {}", path.display()))?;
        let manifest: Self =
            serde_json::from_str(&text).with_context(|| format!("failed to parse manifest {}", path.display()))?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Validates every bound and cross-field invariant.
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != BENCHMARK_SCHEMA_VERSION {
            bail!(
                "unsupported manifest schema {}: this build serves {}..={}",
                self.schema_version,
                MIN_BENCHMARK_SCHEMA_VERSION,
                BENCHMARK_SCHEMA_VERSION
            );
        }
        if self.run_label.trim().is_empty() || self.run_label.chars().count() > 128 {
            bail!("run_label must be non-empty and at most 128 characters");
        }
        match self.ruleset.as_str() {
            "nintendo" | "fuda_wiki" => {}
            other => bail!("unsupported ruleset '{other}': expected nintendo|fuda_wiki"),
        }
        if self.seeds.is_empty() || self.seeds.len() > MAX_CLUSTERS {
            bail!("seeds must contain 1..={MAX_CLUSTERS} deal clusters");
        }
        let mut sorted = self.seeds.clone();
        sorted.sort_unstable();
        sorted.dedup();
        if sorted.len() != self.seeds.len() {
            bail!("seeds must be distinct — duplicate deal clusters bias the estimand");
        }
        if self.worker_timeout_seconds == 0 || self.worker_timeout_seconds > MAX_WORKER_TIMEOUT_SECONDS {
            bail!("worker_timeout_seconds must be in 1..={MAX_WORKER_TIMEOUT_SECONDS}");
        }
        if let Some(cap) = self.time_hard_cap_ms {
            if cap == 0 {
                bail!("time_hard_cap_ms must be positive when present");
            }
        }
        if !self.invalid_forfeit_margin.is_finite() || self.invalid_forfeit_margin <= 0.0 {
            bail!("invalid_forfeit_margin must be finite and positive");
        }
        if self.bootstrap.repetitions == 0 || self.bootstrap.repetitions > MAX_BOOTSTRAP_REPETITIONS {
            bail!("bootstrap.repetitions must be in 1..={MAX_BOOTSTRAP_REPETITIONS}");
        }
        if !self.bootstrap.confidence_level.is_finite()
            || self.bootstrap.confidence_level < 0.99
            || self.bootstrap.confidence_level >= 1.0
        {
            bail!("bootstrap.confidence_level must be in [0.99, 1.0)");
        }
        let draws = (self.bootstrap.repetitions as u64)
            .checked_mul(self.seeds.len() as u64)
            .context("bootstrap workload overflows")?;
        if draws > MAX_BOOTSTRAP_DRAWS {
            bail!(
                "bootstrap workload {draws} draws exceeds the {MAX_BOOTSTRAP_DRAWS} limit \
                 (repetitions x clusters)"
            );
        }
        match &self.stopping {
            StoppingSpec::Fixed => {}
            StoppingSpec::Sequential {
                h0_elo,
                h1_elo,
                alpha,
                beta,
                min_clusters,
                equivalence,
            } => {
                for (name, value) in [
                    ("h0_elo", *h0_elo),
                    ("h1_elo", *h1_elo),
                    ("alpha", *alpha),
                    ("beta", *beta),
                ] {
                    if !value.is_finite() {
                        bail!("stopping.{name} must be finite");
                    }
                }
                if !(0.0..1.0).contains(alpha) || !(0.0..1.0).contains(beta) {
                    bail!("stopping alpha and beta must be in (0, 1)");
                }
                if *h1_elo <= *h0_elo {
                    bail!("stopping requires h1_elo > h0_elo (one-sided superiority test)");
                }
                if *min_clusters == 0 || *min_clusters > self.seeds.len() {
                    bail!("stopping.min_clusters must be in 1..=panel size");
                }
                if let Some(equivalence) = equivalence {
                    equivalence.validate()?;
                    if equivalence.min_clusters < *min_clusters {
                        bail!(
                            "stopping.equivalence.min_clusters {} must be at least the gsprt floor {}",
                            equivalence.min_clusters,
                            min_clusters
                        );
                    }
                    if equivalence.min_clusters > self.seeds.len() {
                        bail!("stopping.equivalence.min_clusters exceeds the cluster cap");
                    }
                }
            }
        }
        validate_entrant(&self.baseline, "baseline")?;
        validate_entrant(&self.candidate, "candidate")?;
        if self.baseline.label == self.candidate.label {
            bail!("entrant labels must be distinct");
        }
        Ok(())
    }

    /// The parsed ruleset both entrants play under.
    pub fn ruleset(&self) -> koi_core::Ruleset {
        match self.ruleset.as_str() {
            "fuda_wiki" => koi_core::Ruleset::fuda_wiki(),
            _ => koi_core::Ruleset::nintendo(),
        }
    }

    /// Canonical manifest hash — FNV-1a/64 over the reserialized manifest.
    /// Bind this into checkpoint headers; a checkpoint whose recorded hash
    /// differs belongs to a different run and must fail closed.
    pub fn canonical_hash_fx64(&self) -> Result<String> {
        let canonical = serde_json::to_vec(self).context("manifest reserialization failed")?;
        Ok(fnv1a64_hex(&canonical))
    }
}

fn validate_entrant(entrant: &Entrant, role: &str) -> Result<()> {
    if entrant.label.trim().is_empty() || entrant.label.chars().count() > 64 {
        bail!("{role}.label must be non-empty and at most 64 characters");
    }
    match &entrant.artifact {
        ArtifactSpec::SelfBinary => {}
        ArtifactSpec::Path { path } => {
            if path.as_os_str().is_empty() {
                bail!("{role}.artifact.path must be non-empty");
            }
        }
    }
    // Solver config validates eagerly — a malformed entrant fails at
    // manifest time, not mid-leg.
    entrant
        .config
        .clone()
        .validate()
        .map_err(|error| anyhow::anyhow!("{role}.config invalid: {error}"))?;
    Ok(())
}

/// FNV-1a/64, hex-encoded — the project's lightweight non-crypto digest
/// for content binding (manifest, binaries, Cargo.lock).
pub fn fnv1a64_hex(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    let mut out = String::with_capacity(16);
    let _ = write!(out, "{hash:016x}");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_manifest() -> Manifest {
        Manifest {
            schema_version: BENCHMARK_SCHEMA_VERSION,
            run_label: "smoke".to_owned(),
            ruleset: "nintendo".to_owned(),
            seeds: vec![11, 22, 33],
            stopping: StoppingSpec::Fixed,
            worker_timeout_seconds: 60,
            time_hard_cap_ms: None,
            invalid_forfeit_margin: DEFAULT_INVALID_FORFEIT_MARGIN,
            bootstrap: BootstrapSpec {
                repetitions: 1_000,
                seed: 5,
                confidence_level: 0.99,
            },
            baseline: Entrant {
                label: "base".to_owned(),
                artifact: ArtifactSpec::SelfBinary,
                config: koi_solver::Config::default(),
            },
            candidate: Entrant {
                label: "cand".to_owned(),
                artifact: ArtifactSpec::SelfBinary,
                config: koi_solver::Config::default(),
            },
        }
    }

    #[test]
    fn a_valid_manifest_passes() {
        valid_manifest().validate().unwrap();
    }

    #[test]
    fn rejects_every_bound_violation() {
        for mutate in [
            (|m: &mut Manifest| m.schema_version = 9) as fn(&mut Manifest),
            |m| m.seeds.clear(),
            |m| m.seeds = vec![1, 1],
            |m| m.worker_timeout_seconds = 0,
            |m| m.worker_timeout_seconds = MAX_WORKER_TIMEOUT_SECONDS + 1,
            |m| m.time_hard_cap_ms = Some(0),
            |m| m.invalid_forfeit_margin = 0.0,
            |m| m.bootstrap.repetitions = 0,
            |m| m.bootstrap.confidence_level = 0.5,
            |m| m.ruleset = "house".to_owned(),
            |m| m.candidate.label = m.baseline.label.clone(),
        ] {
            let mut manifest = valid_manifest();
            mutate(&mut manifest);
            assert!(manifest.validate().is_err());
        }
    }

    #[test]
    fn sequential_stopping_validates_hypotheses() {
        let mut manifest = valid_manifest();
        manifest.stopping = StoppingSpec::Sequential {
            h0_elo: 0.0,
            h1_elo: 100.0,
            alpha: 0.05,
            beta: 0.05,
            min_clusters: 2,
            equivalence: None,
        };
        manifest.validate().unwrap();
        manifest.stopping = StoppingSpec::Sequential {
            h0_elo: 100.0,
            h1_elo: 0.0,
            alpha: 0.05,
            beta: 0.05,
            min_clusters: 2,
            equivalence: None,
        };
        assert!(manifest.validate().is_err());
        manifest.stopping = StoppingSpec::Sequential {
            h0_elo: 0.0,
            h1_elo: 100.0,
            alpha: 0.05,
            beta: 0.05,
            min_clusters: 0,
            equivalence: None,
        };
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn canonical_hash_is_stable() {
        let manifest = valid_manifest();
        assert_eq!(
            manifest.canonical_hash_fx64().unwrap(),
            manifest.canonical_hash_fx64().unwrap()
        );
        let mut other = valid_manifest();
        other.run_label = "different".to_owned();
        assert_ne!(
            manifest.canonical_hash_fx64().unwrap(),
            other.canonical_hash_fx64().unwrap()
        );
    }

    #[test]
    fn deny_unknown_fields() {
        let json = serde_json::to_string(&valid_manifest()).unwrap();
        let with_extra = json.replacen(r#""run_label":"smoke""#, r#""run_label":"smoke","bogus":1"#, 1);
        assert!(serde_json::from_str::<Manifest>(&with_extra).is_err());
    }
}
