//! Shared provenance: the compile-time git stamp + FNV workspace source
//! digest and the runtime digest recomputation. `build.rs` and this library
//! share `src/shared.rs` textually — the embedded stamp and the runtime
//! walk cannot drift apart.
//!
//! Koi-Maestro has a single repository era (workspace-only since the first
//! commit), so unlike a sibling research codebase's `sibling-build-info` there is no
//! `RepoEra`/`BenchHome` discrimination to maintain — `Cargo.lock` and
//! `crates/*` are always at the repository root.

use std::path::{Path, PathBuf};

mod shared;

/// Git commit the workspace was built from.
pub const BUILD_COMMIT: &str = env!("KOI_BUILD_COMMIT");
/// Git tree object id of that commit.
pub const BUILD_TREE: &str = env!("KOI_BUILD_TREE");
/// Whether the working tree was dirty at build time (`"true"`/`"false"`).
pub const BUILD_DIRTY: &str = env!("KOI_BUILD_DIRTY");
/// FNV-1a-64 digest over every source file the workspace hashes.
pub const BUILD_SOURCE_DIGEST: &str = env!("KOI_BUILD_SOURCE_DIGEST");
/// The encoded rustflags this build consumed (`CARGO_ENCODED_RUSTFLAGS`).
/// Empty for a plain build — codegen-affecting flags like `target-cpu` or
/// `profile-use` are provenance, not decoration.
pub const BUILD_RUSTFLAGS: &str = env!("KOI_BUILD_RUSTFLAGS");
/// FNV-1a-64 hash of the `-Cprofile-use` profdata file, when one steered
/// this build. `None` for unprofiled builds.
pub const BUILD_PGO_PROFILE_FX64: Option<&str> = option_env!("KOI_BUILD_PGO_PROFILE_FX64");
/// The cargo profile this binary was built under (`release`, `release-ci`,
/// `dev`, ...). Canonical evidence is gated on `--profile release`.
pub const BUILD_PROFILE: &str = env!("KOI_BUILD_PROFILE");

/// The repository root the building crate belongs to — the ancestor of this
/// crate's manifest directory holding `.git`.
pub fn repository_root() -> Result<PathBuf, String> {
    shared::repository_root(Path::new(env!("CARGO_MANIFEST_DIR")))
        .ok_or_else(|| "crate is not inside a git repository".to_string())
}

/// True when the build-time working tree was dirty.
pub fn build_dirty() -> bool {
    BUILD_DIRTY == "true"
}

/// Recomputes the workspace source digest over `root` at runtime — the same
/// file set and FNV-1a order `build.rs` embedded into
/// [`BUILD_SOURCE_DIGEST`].
pub fn hash_workspace_sources(root: &Path) -> Result<String, String> {
    let files = shared::workspace_source_files(root)?;
    shared::workspace_source_digest(root, &files)
}

/// FNV-1a-64 over a file's bytes — the hasher the source digest uses, so
/// `fnv1a64:`-prefixed provenance fields are honestly labeled.
pub fn hash_file(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|error| format!("failed to hash {}: {error}", path.display()))?;
    let mut hash = shared::FNV_OFFSET_BASIS;
    shared::hash_source_chunk(&mut hash, &bytes);
    Ok(format!("{hash:016x}"))
}

/// FNV-1a-64 of the workspace `Cargo.lock` — dataset/report provenance.
/// The lock always lives at the repository root in this workspace.
pub fn cargo_lock_hash() -> Result<String, String> {
    let root = repository_root()?;
    hash_file(&root.join("Cargo.lock"))
}

/// Recursive file collection used by the digest — exported so callers that
/// enumerate the same trees stay identical.
pub use shared::{collect_files, watched_source_dirs};

#[cfg(test)]
mod tests {
    use super::*;

    /// The compile-time stamp and the runtime recomputation must agree —
    /// this is the invariant the `include!("shared.rs")` pattern exists for.
    /// A dirty working tree changes file contents but not the file set, so
    /// the digest comparison is valid even with uncommitted edits.
    #[test]
    fn runtime_source_digest_matches_build_stamp() {
        let root = repository_root().expect("repository root");
        let digest = hash_workspace_sources(&root).expect("workspace digest");
        assert_eq!(
            digest, BUILD_SOURCE_DIGEST,
            "runtime digest drifted from the compile-time stamp — a source file \
             changed after this binary was built, or the two implementations diverged"
        );
    }

    #[test]
    fn cargo_lock_hash_is_stable_hex() {
        let hash = cargo_lock_hash().expect("cargo lock hash");
        assert_eq!(hash.len(), 16);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
