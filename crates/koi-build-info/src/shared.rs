// Single implementation of the workspace source digest — included by both
// `build.rs` (compile-time stamp) and `lib.rs` (runtime recomputation), so
// the byte-identical invariant between the embedded stamp and the runtime
// walk holds by construction rather than by two hand-mirrored copies.
// (Pattern proven in a sibling research codebase's sibling-build-info crate.)
//
// Constraint: std-only, no `crate::` paths (this file is textually included
// into two different compilation contexts).

use std::fs;
use std::path::{Path, PathBuf};

pub(crate) const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
pub(crate) const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

pub(crate) fn hash_source_chunk(hash: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *hash = (*hash ^ u64::from(*byte)).wrapping_mul(FNV_PRIME);
    }
}

/// The repository root is the ancestor of `start` that holds `.git` — the
/// crate lives under `crates/<member>` in this workspace-only layout.
pub(crate) fn repository_root(start: &Path) -> Option<PathBuf> {
    let mut dir = start;
    loop {
        if dir.join(".git").exists() {
            return Some(dir.to_path_buf());
        }
        dir = dir.parent()?;
    }
}

/// Workspace member directories: every `crates/*` dir with a Cargo.toml.
/// Koi-Maestro has only ever had the workspace layout — unlike the sibling codebase there
/// is no legacy `rust/` era to discriminate.
pub(crate) fn workspace_member_dirs(repository_root: &Path) -> Vec<PathBuf> {
    let mut members = Vec::new();
    let crates = repository_root.join("crates");
    if crates.is_dir() {
        if let Ok(entries) = fs::read_dir(&crates) {
            for entry in entries.flatten() {
                let member = entry.path();
                if member.join("Cargo.toml").is_file() {
                    members.push(member);
                }
            }
        }
    }
    members
}

/// Directories watched for additions/removals — every hashed collect root,
/// so files created *after* the build script ran still retrigger it.
/// `examples`/`tests`/`benches` are hashed too: a certificate generator or
/// gate test edit must flip the digest, not ride on a stale `dirty:false`
/// stamp (reliability review).
pub fn watched_source_dirs(repository_root: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    for member in workspace_member_dirs(repository_root) {
        dirs.push(member.join("src"));
        dirs.push(member.join("examples"));
        dirs.push(member.join("tests"));
        dirs.push(member.join("benches"));
        dirs.push(member.join(".cargo"));
    }
    dirs.push(repository_root.join(".cargo"));
    dirs
}

/// Source set hashed into the digest. The list is sorted on repo-relative
/// `/`-normalized paths so directory iteration order never leaks into the
/// digest; required manifests fail loudly rather than silently dropping out.
pub(crate) fn workspace_source_files(repository_root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    for name in ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml"] {
        let path = repository_root.join(name);
        if path.is_file() {
            files.push(path);
        } else if matches!(name, "Cargo.toml" | "Cargo.lock") {
            return Err(format!("expected workspace file {} is missing", path.display()));
        }
    }
    for member in workspace_member_dirs(repository_root) {
        files.push(member.join("Cargo.toml"));
        for extra in ["build.rs", "rust-toolchain.toml"] {
            let path = member.join(extra);
            if path.is_file() {
                files.push(path);
            }
        }
        collect_files(&member.join("src"), &mut files)?;
        collect_files(&member.join("examples"), &mut files)?;
        collect_files(&member.join("tests"), &mut files)?;
        collect_files(&member.join("benches"), &mut files)?;
        collect_files(&member.join(".cargo"), &mut files)?;
    }
    collect_files(&repository_root.join(".cargo"), &mut files)?;
    files.sort_by_key(|path| {
        path.strip_prefix(repository_root)
            .map(|relative| relative.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| path.to_string_lossy().replace('\\', "/"))
    });
    Ok(files)
}

pub(crate) fn workspace_source_digest(repository_root: &Path, files: &[PathBuf]) -> Result<String, String> {
    let mut hash = FNV_OFFSET_BASIS;
    for path in files {
        let relative = path
            .strip_prefix(repository_root)
            .map_err(|_| format!("{} is outside {}", path.display(), repository_root.display()))?;
        let name = relative.to_string_lossy().replace('\\', "/");
        let bytes = fs::read(path).map_err(|error| format!("failed to hash {}: {error}", path.display()))?;
        hash_source_chunk(&mut hash, &(name.len() as u64).to_le_bytes());
        hash_source_chunk(&mut hash, name.as_bytes());
        hash_source_chunk(&mut hash, &(bytes.len() as u64).to_le_bytes());
        hash_source_chunk(&mut hash, &bytes);
    }
    Ok(format!("{hash:016x}"))
}

/// Recursive file collection — symlinks are never hashed (they can point
/// outside the workspace or create cycles; none are watched sources).
pub fn collect_files(directory: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    if !directory.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(directory).map_err(|error| format!("failed to read {}: {error}", directory.display()))? {
        let entry = entry.map_err(|error| format!("failed to iterate {}: {error}", directory.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| format!("failed to stat {}: {error}", path.display()))?;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            collect_files(&path, files)?;
        } else if file_type.is_file() {
            files.push(path);
        }
    }
    Ok(())
}
