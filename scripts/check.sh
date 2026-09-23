#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

# Native crates that must stay warning-free on every supported target.
# koi-pyo3 is intentionally excluded: its build script requires a Python
# interpreter for the *target* triple, so it is only validated natively.
NATIVE_CRATES=(-p koi-maestro-core -p koi-maestro-solver -p koi-maestro-bench)
CROSS_TARGETS=(aarch64-unknown-linux-gnu x86_64-unknown-linux-gnu x86_64-pc-windows-msvc)

run() {
    local label="$1"
    shift
    echo "=== $label ==="
    "$@"
}

# PyO3 test binaries load libpython at runtime — when a project `.venv`
# exists, put its base prefix's lib dirs on the loader path so `cargo test`
# covers koi-pyo3 instead of aborting at test-list time.
if [ -x .venv/bin/python ]; then
    base_prefix="$(.venv/bin/python -c 'import sys; print(sys.base_prefix)')"
    export LD_LIBRARY_PATH="${base_prefix}/lib:${base_prefix}:${LD_LIBRARY_PATH:-}"
fi

run 'cargo fmt --check' cargo fmt --check
run 'cargo clippy' cargo clippy --workspace --all-targets -- -D warnings

# Gate profile: release-ci (thin LTO, multi-CGU) keeps gate wall-clock low
# while preserving release semantics (panic=abort, overflow-checks off).
# KOI_PROFILE=release re-runs the certification profile — that is also the
# only profile whose binaries may produce official performance numbers.
profile="${KOI_PROFILE:-release-ci}"

if cargo nextest --version >/dev/null 2>&1; then
    run "cargo nextest run ($profile)" cargo nextest run --cargo-profile "$profile" --workspace
else
    run "cargo test ($profile)" cargo test --profile "$profile" --workspace
fi

run "cargo test --doc ($profile)" cargo test --doc --profile "$profile" --workspace

# Cross-target compile gate: `cargo check` needs no linker, so it verifies that
# the native crates build for every supported OS/ISA from any host. Toolchain
# setup failures (no rustup, offline) are skipped, but a failed check on an
# installed target IS a portability regression and fails.
# --all-targets also cross-compiles tests/examples: dev-deps must stay
# pure-Rust (the criterion C toolchain was the blocker removed in P6).
if [ -n "${KOI_SKIP_CROSS:-}" ] && [ "${KOI_SKIP_CROSS}" != "0" ]; then
    echo ""
    echo "=== cross-target checks skipped (KOI_SKIP_CROSS) ==="
elif ! command -v rustup >/dev/null 2>&1; then
    echo ""
    echo "=== cross-target checks skipped (rustup not found) ==="
else
    host_triple="$(rustc -vV | sed -n 's/^host: //p')"
    for target in "${CROSS_TARGETS[@]}"; do
        if [ "$target" = "$host_triple" ]; then
            echo "=== cargo check ($target) === skipped (host target)"
            continue
        fi
        if ! rustup target list --installed | grep -qx "$target"; then
            echo "=== installing Rust target $target ==="
            if ! rustup target add "$target" >/dev/null; then
                echo "warning: could not install target $target (offline or unavailable); skipping cross-check" >&2
                continue
            fi
        fi
        run "cargo check ($target)" cargo check --all-targets "${NATIVE_CRATES[@]}" --target "$target"
    done
    echo ""
    echo "NOTE: koi-pyo3 excluded from cross-checks (needs a target Python); build with maturin on the host."
fi

echo ""
echo "=== ALL CHECKS PASSED ==="
