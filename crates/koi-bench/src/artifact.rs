//! Artifact resolution and identity binding.
//!
//! An entrant's artifact is either the referee binary re-spawned in worker
//! mode (`self_binary`) or an explicit executable path. Resolution produces
//! the `ArtifactIdentity` recorded in the run's provenance: label,
//! executable, build stamps, and a content hash of the binary image.
//!
//! `path` artifacts are opaque binaries — their build stamps are learned at
//! negotiation time and recorded as negotiated, never assumed.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use crate::manifest::{fnv1a64_hex, ArtifactIdentity, ArtifactSpec, Entrant};
pub use koi_build_info::{
    build_dirty, cargo_lock_hash, repository_root, BUILD_COMMIT, BUILD_DIRTY, BUILD_PGO_PROFILE_FX64, BUILD_RUSTFLAGS,
    BUILD_SOURCE_DIGEST, BUILD_TREE,
};

/// The referee's own identity, from build-time stamps.
pub fn referee_identity() -> RefereeIdentity {
    RefereeIdentity {
        build_commit: BUILD_COMMIT.to_string(),
        build_tree: BUILD_TREE.to_string(),
        build_dirty: build_dirty(),
        source_digest_fnv1a64: BUILD_SOURCE_DIGEST.to_string(),
        rustflags: BUILD_RUSTFLAGS.to_string(),
        pgo_profile_fx64: BUILD_PGO_PROFILE_FX64.map(str::to_string),
    }
}

/// The referee's build provenance — stamped into every run record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefereeIdentity {
    pub build_commit: String,
    pub build_tree: String,
    pub build_dirty: bool,
    pub source_digest_fnv1a64: String,
    pub rustflags: String,
    pub pgo_profile_fx64: Option<String>,
}

use serde::{Deserialize, Serialize};

/// A resolved artifact: spec + identity + the executable to spawn.
#[derive(Debug, Clone)]
pub struct ResolvedArtifact {
    pub label: String,
    pub spec: ArtifactSpec,
    pub executable: PathBuf,
    /// The solver config this entrant plays under (cloned into each
    /// decision request — the worker binary itself is config-agnostic).
    pub config: koi_solver::Config,
    /// FNV-1a/64 of the executable image, hashed once at resolve time.
    /// `WorkerPool::restart` re-hashes and fails closed on drift — a binary
    /// swapped mid-run must not ride the recorded identity.
    pub binary_hash: String,
    /// FNV-1a/64 of the blueprint artifact's bytes when the config names
    /// one — the oracle is part of the entrant's identity exactly like the
    /// binary image, and drift is re-verified on respawn.
    pub blueprint_hash: Option<String>,
    /// FNV-1a/64 of the leaf model's bytes when `resolving.leaf_model`
    /// names an ONNX path — the model is entrant identity too. The
    /// `"builtin:handcrafted"` sentinel names no file and carries no hash.
    pub leaf_model_hash: Option<String>,
    /// Build identity the referee expects responses to attest. For
    /// `self_binary` artifacts these are this binary's own stamps; for
    /// `path` artifacts they are learned at negotiation and filled by
    /// `record_negotiated`.
    pub expected_commit: String,
    pub expected_tree: String,
    pub expected_clean: bool,
}

impl ResolvedArtifact {
    /// The provenance identity recorded for this entrant.
    pub fn identity(&self) -> Result<ArtifactIdentity> {
        let mut config_bytes = serde_json::to_vec(&self.config).context("entrant config serialization failed")?;
        if let Some(hash) = &self.blueprint_hash {
            config_bytes.extend_from_slice(hash.as_bytes());
        }
        if let Some(hash) = &self.leaf_model_hash {
            config_bytes.extend_from_slice(hash.as_bytes());
        }
        Ok(ArtifactIdentity {
            label: self.label.clone(),
            executable: self.executable.clone(),
            build_commit: self.expected_commit.clone(),
            build_tree: self.expected_tree.clone(),
            build_dirty: !self.expected_clean,
            binary_hash_fnv1a64: self.binary_hash.clone(),
            config_hash_fx64: fnv1a64_hex(&config_bytes),
        })
    }

    /// Re-hashes the executable — and any named blueprint artifact — and
    /// fails closed on drift, called before any respawn so a mid-run swap
    /// cannot ride the recorded identity.
    pub fn verify_image(&self) -> Result<()> {
        let bytes = std::fs::read(&self.executable)
            .with_context(|| format!("failed to re-read worker binary {}", self.executable.display()))?;
        let current = fnv1a64_hex(&bytes);
        if current != self.binary_hash {
            bail!(
                "worker binary {} changed mid-run: {} -> {}",
                self.executable.display(),
                self.binary_hash,
                current
            );
        }
        if let (Some(path), Some(recorded)) = (&self.config.resolving.blueprint_artifact, &self.blueprint_hash) {
            let bytes = std::fs::read(path).with_context(|| format!("failed to re-read blueprint artifact {path}"))?;
            let current = fnv1a64_hex(&bytes);
            if &current != recorded {
                bail!("blueprint artifact {path} changed mid-run: {recorded} -> {current}");
            }
        }
        if let (Some(path), Some(recorded)) = (&self.config.resolving.leaf_model, &self.leaf_model_hash) {
            let bytes = std::fs::read(path).with_context(|| format!("failed to re-read leaf model {path}"))?;
            let current = fnv1a64_hex(&bytes);
            if &current != recorded {
                bail!("leaf model {path} changed mid-run: {recorded} -> {current}");
            }
        }
        Ok(())
    }

    /// After negotiation, pin the worker's attested build stamps so every
    /// decision response is checked against them. `self_binary` artifacts
    /// must attest exactly this binary's stamps — anything else means the
    /// launcher spawned the wrong artifact.
    pub fn record_negotiated(&mut self, commit: &str, tree: &str, dirty: bool) -> Result<()> {
        match self.spec {
            ArtifactSpec::SelfBinary => {
                if commit != BUILD_COMMIT || tree != BUILD_TREE || dirty != build_dirty() {
                    bail!("self-spawned worker attests a foreign build: {commit} {tree} dirty={dirty}");
                }
            }
            ArtifactSpec::Path { .. } => {
                self.expected_commit = commit.to_owned();
                self.expected_tree = tree.to_owned();
                self.expected_clean = !dirty;
            }
        }
        Ok(())
    }
}

/// The arguments a worker binary is spawned with.
fn worker_arguments(spec: &ArtifactSpec) -> Vec<String> {
    match spec {
        ArtifactSpec::SelfBinary => vec!["worker".to_owned()],
        ArtifactSpec::Path { .. } => vec!["worker".to_owned()],
    }
}

/// Resolves an entrant to a spawnable artifact. `manifest_dir` is the
/// directory containing the manifest file — relative `path` artifacts are
/// resolved against it.
pub fn resolve(entrant: &Entrant, manifest_dir: &Path) -> Result<ResolvedArtifact> {
    entrant
        .config
        .clone()
        .validate()
        .map_err(|error| anyhow::anyhow!("entrant '{}' config invalid: {error}", entrant.label))?;
    let (executable, commit, tree, clean) = match &entrant.artifact {
        ArtifactSpec::SelfBinary => (
            std::env::current_exe().context("failed to locate the running executable")?,
            BUILD_COMMIT.to_string(),
            BUILD_TREE.to_string(),
            !build_dirty(),
        ),
        ArtifactSpec::Path { path } => {
            let resolved = if path.is_absolute() {
                path.clone()
            } else {
                manifest_dir.join(path)
            };
            if !resolved.is_file() {
                bail!(
                    "entrant '{}' artifact does not exist: {}",
                    entrant.label,
                    resolved.display()
                );
            }
            (
                resolved
                    .canonicalize()
                    .with_context(|| format!("failed to canonicalize {}", resolved.display()))?,
                // Learned at negotiation; decision responses before that
                // are impossible, so these placeholders never validate.
                "pending-negotiation".to_string(),
                "pending-negotiation".to_string(),
                false,
            )
        }
    };
    let bytes =
        std::fs::read(&executable).with_context(|| format!("failed to read worker binary {}", executable.display()))?;
    let binary_hash = fnv1a64_hex(&bytes);
    // The blueprint artifact is part of the entrant's identity like the
    // binary: a relative path resolves against `manifest_dir` (workers
    // inherit the referee's CWD — left alone it would resolve against
    // wherever koi-bench was invoked), and its bytes enter the config
    // hash so a swapped oracle cannot ride the recorded provenance.
    let mut config = entrant.config.clone();
    let blueprint_hash = match &config.resolving.blueprint_artifact {
        None => None,
        Some(path) => {
            let raw = PathBuf::from(path);
            let candidate = if raw.is_absolute() {
                raw
            } else {
                manifest_dir.join(&raw)
            };
            let canonical = candidate.canonicalize().with_context(|| {
                format!(
                    "entrant '{}' blueprint artifact does not exist: {}",
                    entrant.label,
                    candidate.display()
                )
            })?;
            let bytes = std::fs::read(&canonical)
                .with_context(|| format!("failed to read blueprint artifact {}", canonical.display()))?;
            config.resolving.blueprint_artifact = Some(canonical.to_string_lossy().into_owned());
            Some(fnv1a64_hex(&bytes))
        }
    };
    // An ONNX leaf model is provenance exactly like the blueprint — except
    // the `"builtin:handcrafted"` sentinel, which names no file. The
    // sentinel is the only non-path value the engine accepts.
    let leaf_model_hash = match &config.resolving.leaf_model {
        None => None,
        Some(spec) if spec == koi_solver::leaf::BUILTIN_LEAF_MODEL => None,
        Some(path) => {
            let raw = PathBuf::from(path);
            let candidate = if raw.is_absolute() {
                raw
            } else {
                manifest_dir.join(&raw)
            };
            let canonical = candidate.canonicalize().with_context(|| {
                format!(
                    "entrant '{}' leaf model does not exist: {}",
                    entrant.label,
                    candidate.display()
                )
            })?;
            let bytes = std::fs::read(&canonical)
                .with_context(|| format!("failed to read leaf model {}", canonical.display()))?;
            config.resolving.leaf_model = Some(canonical.to_string_lossy().into_owned());
            Some(fnv1a64_hex(&bytes))
        }
    };
    Ok(ResolvedArtifact {
        label: entrant.label.clone(),
        spec: entrant.artifact.clone(),
        executable,
        config,
        binary_hash,
        blueprint_hash,
        leaf_model_hash,
        expected_commit: commit,
        expected_tree: tree,
        expected_clean: clean,
    })
}

/// The spawn arguments for `resolved`'s executable.
pub fn spawn_arguments(resolved: &ResolvedArtifact) -> Vec<String> {
    worker_arguments(&resolved.spec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn self_binary_resolves_to_current_exe() {
        let entrant = Entrant {
            label: "self".to_owned(),
            artifact: ArtifactSpec::SelfBinary,
            config: koi_solver::Config::default(),
        };
        let mut resolved = resolve(&entrant, Path::new(".")).unwrap();
        assert_eq!(resolved.executable, std::env::current_exe().unwrap());
        assert_eq!(resolved.expected_commit, BUILD_COMMIT);
        // A foreign attestation on a self-spawned worker fails closed.
        assert!(resolved.record_negotiated("abc", "tree", false).is_err() || BUILD_COMMIT == "abc");
    }

    #[test]
    fn missing_path_artifact_fails() {
        let entrant = Entrant {
            label: "ext".to_owned(),
            artifact: ArtifactSpec::Path {
                path: PathBuf::from("no/such/worker.exe"),
            },
            config: koi_solver::Config::default(),
        };
        assert!(resolve(&entrant, Path::new(".")).is_err());
    }
}
