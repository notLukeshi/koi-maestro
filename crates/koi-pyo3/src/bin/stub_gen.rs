//! Dev-only stub generator — emits `src/koi_maestro/_engine.pyi` from the
//! `#[gen_stub_*]`-annotated `koi_pyo3` surface. Not shipped in the wheel;
//! the committed stub is diffed by the freshness gate.
//!
//! Ported from a sibling research codebase's `crates/sibling-pyo3/src/bin/stub_gen.rs` —
//! same env-discovery contract, retargeted to `koi_maestro_rs::stub_info`.

use pyo3_stub_gen::Result;
use std::{backtrace::BacktraceStatus, env, fs, path::PathBuf};

// env::set_var calls run before any threads spawn (single-threaded main).
fn configure_python_environment() {
    if env::var_os("PYTHONHOME").is_some() {
        println!(
            "PYTHONHOME already set to {}",
            env::var("PYTHONHOME").unwrap_or_default()
        );
        return;
    }

    let mut candidates = Vec::new();
    if let Some(venv) = env::var_os("VIRTUAL_ENV") {
        candidates.push(PathBuf::from(venv));
    }
    // The venv sits at the repository root — the crate's parent in the
    // legacy layout, its grandparent under `crates/<member>`. Walk every
    // ancestor so both eras resolve it.
    let manifest_dir: PathBuf = env!("CARGO_MANIFEST_DIR").into();
    for ancestor in manifest_dir.ancestors() {
        candidates.push(ancestor.join(".venv"));
        candidates.push(ancestor.join("venv"));
    }

    for candidate in candidates {
        if !candidate.exists() {
            continue;
        }
        let mut python_home = candidate.clone();
        if let Ok(cfg) = fs::read_to_string(candidate.join("pyvenv.cfg")) {
            for line in cfg.lines() {
                if let Some(home) = line.strip_prefix("home =") {
                    let base = PathBuf::from(home.trim());
                    if base.exists() {
                        python_home = base;
                    }
                }
            }
        }
        let lib_dir = python_home.join("Lib");
        if !lib_dir.exists() {
            continue;
        }
        // Single-threaded at this point — safe.
        env::set_var("PYTHONHOME", &python_home);
        let mut path_entries = vec![lib_dir];
        let site = candidate.join("Lib").join("site-packages");
        if site.exists() {
            path_entries.push(site);
        }
        if let Some(existing) = env::var_os("PYTHONPATH") {
            path_entries.extend(env::split_paths(&existing));
        }
        if let Ok(joined) = env::join_paths(path_entries) {
            env::set_var("PYTHONPATH", joined);
        }
        return;
    }

    println!("Warning: no Python virtual environment located; proceeding without PYTHONHOME.");
}

fn main() -> Result<()> {
    println!("Generating stubs...");
    env::set_var("RUST_BACKTRACE", "1");
    env::set_var("RUST_LIB_BACKTRACE", "1");
    // pyo3-stub-gen 0.23 evaluates `env::var("CARGO_MANIFEST_DIR")` eagerly in
    // an `unwrap_or` even when `python-source` is set — export the
    // compile-time value so the eager arm cannot panic.
    env::set_var("CARGO_MANIFEST_DIR", env!("CARGO_MANIFEST_DIR"));

    configure_python_environment();

    let content = match koi_maestro_rs::render_stub_pyi() {
        Ok(content) => content,
        Err(error) => {
            println!("Failed to gather stub info: {error}");
            println!("Debug details: {error:?}");
            let backtrace = error.backtrace();
            if backtrace.status() == BacktraceStatus::Captured {
                println!("Backtrace:\n{backtrace}");
            }
            for cause in error.chain() {
                println!("Caused by: {cause}");
            }
            return Err(error);
        }
    };

    // The committed stub is a single file next to the extension
    // (`src/koi_maestro/_engine.pyi`), not the mixed-layout `_engine/`
    // package shape `StubInfo::generate` emits.
    let manifest_dir: &std::path::Path = env!("CARGO_MANIFEST_DIR").as_ref();
    let dest = manifest_dir
        .ancestors()
        .map(|dir| dir.join("src").join("koi_maestro"))
        .find(|dir| dir.exists())
        .unwrap_or_else(|| manifest_dir.join("src").join("koi_maestro"))
        .join("_engine.pyi");
    std::fs::write(&dest, content)?;
    println!("Wrote {}", dest.display());
    Ok(())
}
