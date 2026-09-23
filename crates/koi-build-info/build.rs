use std::process::Command;

// The digest implementation is shared with the library — the compile-time
// stamp and the runtime recomputation are the same code, included textually
// so the byte-identical invariant cannot drift.
include!("src/shared.rs");

fn git(root: &std::path::Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(root)
        .args(arguments)
        .output()
        .unwrap_or_else(|error| panic!("failed to run git {}: {error}", arguments.join(" ")));
    assert!(
        output.status.success(),
        "git {} failed: {}",
        arguments.join(" "),
        String::from_utf8_lossy(&output.stderr).trim()
    );
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

fn watch_git_path(root: &std::path::Path, path: &str) {
    let path = std::path::PathBuf::from(path);
    let absolute = if path.is_absolute() { path } else { root.join(path) };
    println!("cargo:rerun-if-changed={}", absolute.display());
}

fn main() {
    let crate_root =
        std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set"));
    let repository_root = repository_root(&crate_root);
    // A source distribution (crate tarball, vendored tree, archive download)
    // has no `.git` — stamp honestly rather than panic so those builds work.
    // `dirty: true` marks the stamp as unable-to-prove-clean.
    let (commit, tree, dirty) = match &repository_root {
        Some(root) => (
            git(root, &["rev-parse", "HEAD"]),
            git(root, &["rev-parse", "HEAD^{tree}"]),
            !git(root, &["status", "--porcelain"]).is_empty(),
        ),
        None => ("unstamped".to_string(), "unstamped".to_string(), true),
    };
    // The source digest walks the `crates/*/Cargo.toml` layout, not `.git`,
    // so a tarball still yields a real digest when the layout is intact.
    let workspace_root = repository_root.clone().or_else(|| {
        crate_root
            .ancestors()
            .find(|ancestor| {
                ancestor
                    .join("crates")
                    .join("koi-build-info")
                    .join("Cargo.toml")
                    .is_file()
            })
            .map(std::path::Path::to_path_buf)
    });
    let source_files = workspace_root
        .as_deref()
        .and_then(|root| workspace_source_files(root).ok())
        .unwrap_or_default();
    let source_digest = workspace_root
        .as_deref()
        .and_then(|root| workspace_source_digest(root, &source_files).ok())
        .unwrap_or_else(|| "unstamped".to_string());

    // Codegen-affecting flags (target-cpu, profile-use, ...) are invisible to
    // the source digest but change the binary — they must be bound into
    // artifact provenance or a locally tuned build would be indistinguishable
    // from a canonical one.
    let rustflags = std::env::var("CARGO_ENCODED_RUSTFLAGS").unwrap_or_default();
    println!("cargo:rustc-env=KOI_BUILD_RUSTFLAGS={rustflags}");
    // When profile-use is among the flags, hash the .profdata file itself: the
    // path alone does not identify the profile content steering codegen.
    let pgo_hash = rustflags
        .split('\x1f')
        .collect::<Vec<_>>()
        .windows(2)
        .find_map(|pair| {
            if pair[0] == "-C" {
                pair[1].strip_prefix("profile-use=")
            } else {
                None
            }
        })
        .or_else(|| {
            rustflags
                .split('\x1f')
                .find_map(|flag| flag.strip_prefix("-Cprofile-use="))
        })
        .and_then(|path| {
            // Absolute paths resolve unambiguously; relative ones are read
            // from this crate's directory, which is where cargo runs build
            // scripts — profile-use should always be passed absolute.
            let resolved = crate_root.join(path);
            let resolved = if resolved.exists() {
                resolved
            } else {
                std::path::PathBuf::from(path)
            };
            println!("cargo:rerun-if-changed={}", resolved.display());
            std::fs::read(&resolved).ok().map(|bytes| {
                let mut hash = FNV_OFFSET_BASIS;
                hash_source_chunk(&mut hash, &bytes);
                format!("{hash:016x}")
            })
        });
    if let Some(pgo_hash) = pgo_hash {
        println!("cargo:rustc-env=KOI_BUILD_PGO_PROFILE_FX64={pgo_hash}");
    }
    println!("cargo:rustc-env=KOI_BUILD_COMMIT={commit}");
    println!("cargo:rustc-env=KOI_BUILD_TREE={tree}");
    println!("cargo:rustc-env=KOI_BUILD_DIRTY={dirty}");
    println!("cargo:rustc-env=KOI_BUILD_SOURCE_DIGEST={source_digest}");
    // The cargo profile name (release / release-ci / release-wheel / dev)
    // is provenance: canonical evidence is gated on --profile release, so
    // the stamp must record which profile produced the binary.
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "unknown".to_string());
    println!("cargo:rustc-env=KOI_BUILD_PROFILE={profile}");
    for path in &source_files {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    if let Some(root) = &workspace_root {
        for dir in watched_source_dirs(root) {
            println!("cargo:rerun-if-changed={}", dir.display());
        }
    }
    if let Some(root) = &repository_root {
        watch_git_path(root, &git(root, &["rev-parse", "--git-path", "HEAD"]));
        watch_git_path(root, &git(root, &["rev-parse", "--git-path", "index"]));
        if let Ok(output) = Command::new("git")
            .current_dir(root)
            .args(["symbolic-ref", "HEAD"])
            .output()
        {
            if output.status.success() {
                let reference = String::from_utf8(output.stdout).unwrap().trim().to_string();
                watch_git_path(root, &git(root, &["rev-parse", "--git-path", &reference]));
            }
        }
    }
}
