# Koi-Maestro: Official Architecture & Systems Specification

**Target:** `koi-maestro` (Game-Theoretic Engine & AI Suite for Hanafuda Koi-Koi)  
**Standards:** PyO3 0.29+ (Bound API), Maturin 1.9+ (PEP 621 / PEP 735), Rust 2021 Workspace, `uv` (Astral), PyTorch 2.x  
**Status:** the full target architecture is implemented and gate-tested — `koi-core` (48-card kernel, rules, Yaku), `koi-solver` (baselines, endgame stack, EFG/sequence-form + CFR family, continual resolving, leaf evaluator), `koi-bench` (paired tournament harness), `koi-learn` (dataset pipeline), and `koi-pyo3` (Python extension) all exist in the workspace.

---

## 1. Architectural Foundations: The Hybrid Facade Pattern

To achieve maximum game-theoretic search throughput while providing a clean, strictly-typed public Python API, `koi-maestro` adopts a **Decoupled Multi-Crate Workspace with Hybrid Python Facade**:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                      Python Public API (`src/koi_maestro/`)             │
│   • 100% Typed (PEP 561 py.typed + .pyi stubs)                          │
│   • Ergonomic Pythonic Facade, Domain Exceptions, ML Pipeline           │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │ Direct In-Process Dynamic Linking
                                     ▼
┌─────────────────────────────────────────────────────────────────────────┐
│              Native Extension Module (`koi_maestro._engine`)            │
│   • Compiled by Maturin from `crates/koi-pyo3` (`cdylib`)               │
│   • PyO3 0.29 Bound API (`Bound<'py, T>`)                               │
│   • Zero-copy memory buffer exchange with NumPy / PyTorch               │
│   • Explicit GIL Detachment (`py.detach(|| ...)`) for parallel search   │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │ Pure Rust Internal Crates
                  ┌──────────────────┴──────────────────┐
                  ▼                                     ▼
┌───────────────────────────────────┐ ┌───────────────────────────────────┐
│    `crates/koi-solver` (`rlib`)   │ │     `crates/koi-core` (`rlib`)    │
│  • Continual Resolving (CFR+)     │ │  • 48-bit `CardSet = u64`         │
│  • Turn 8 Exact Expectimax (<50µs)│ │  • AVX2/FMA SIMD Yaku Evaluation  │
│  • Turn 7 Subgame LP / Resolving  │ │  • 3-Phase Turn State Machine     │
│  • OX-Search (ICML 2024)          │ │  • Zero-allocation ArrayVec stack │
│  • ONNX Runtime Leaf Inference    │ │  • Zobrist 64-bit hashing         │
└───────────────────────────────────┘ └───────────────────────────────────┘
```

> **Performance numbers in this document** (`<5 ns` Yaku, `<50 µs` exact endgame,
> `>1M` Python steps/s, etc.) are **acceptance targets and hypotheses**, not
> measured facts. They are validated (or refuted) by the benchmark protocol
> recorded in `docs/BENCHMARK-JOURNAL.md` before being treated as
> architectural guarantees.

---

## 2. Multi-Crate Workspace Architecture

### Why Single-Crate with Features is an Anti-Pattern
In hybrid engines, mixing `pyo3` (`extension-module`), native CLI targets, benchmarks, and math kernels in a single crate leads to:
1. **Windows Linker Traps:** Enabling `pyo3/extension-module` suppresses linking against `python3.lib`. This causes `cargo test` and standalone binaries to fail on Windows with `unresolved external symbol __imp_Py...`.
2. **Feature Poisoning:** Dev-dependencies and test flags infect the crate globally due to Cargo's additive feature resolution.
3. **Slow Iteration:** Any modification to a core domain rule forces re-linking of the entire Python bridge and CLI.

### Multi-Crate Crate Structure
* **`crates/koi-core` (`rlib`)**: Pure domain model (cards, 48-bit bitboards, rules, scoring). Compiles in < 1s. Zero Python or heavy dependencies.
* **`crates/koi-solver` (`rlib`)**: Game-theoretic solvers (CFR+, Turn 8 expectimax, Turn 7 resolving, OX-search, Rayon, ONNX Runtime leaf inference behind `leaf-ort`).
* **`crates/koi-pyo3` (`cdylib`)**: The **only** crate that depends on `pyo3` and `numpy`. Produces the Python C-extension `_engine.pyd` / `_engine.so`.
* **`crates/koi-bench` (`bin`)**: Paired tournament harness, cluster bootstrapping, Win32 Job Object containment, and SPRT gating.
* **`crates/koi-build-info` (`rlib`)**: Shared build provenance — compile-time git stamp, FNV workspace source digest, rustflags/PGO identity. Single implementation shared textually between `build.rs` and the runtime so the two can never drift (pattern proven by `sibling-build-info`).

> **Descoped:** the `koi-cli` terminal-UI crate sketched in earlier drafts is
> **not** a workspace member and is not scheduled by the master plan — CLI
> surfaces live in `koi-bench` binaries. Add it back only via an explicit
> scope change.

---

## 3. Maturin & PyO3 Modern Standards (0.26+)

### 3.1 `extension-module` scoping & `generate-import-lib`
- `extension-module` must **not** appear in any `Cargo.toml`: enabled
  unconditionally it suppresses linking against `python3.lib`, so
  `cargo test` and standalone binaries fail on Windows. It is enabled
  **only** for wheel builds via `[tool.maturin] features`
  (`pyo3/extension-module`), so Rust-side builds link Python normally.
- The `generate-import-lib` feature is **deprecated and a no-op** in modern PyO3 on Windows MSVC because PyO3 now uses `raw-dylib` linking.

### 3.2 Root `pyproject.toml` (PEP 621 + PEP 735)

The committed `pyproject.toml` is the single source of truth — the
architecturally significant parts are:

```toml
[build-system]
requires = ["maturin>=1.10.0,<2.0.0"]   # >=1.10: --profile wins over
build-backend = "maturin"                #  pyproject profile + editable-profile

[project]
license = "MIT OR Apache-2.0"
license-files = ["LICENSE-MIT", "LICENSE-APACHE"]
requires-python = ">=3.10,<3.14"
dependencies = ["numpy>=1.26.0,<3.0.0"]

[project.optional-dependencies]
ml = ["torch>=2.2.0", "safetensors>=0.4.0", "onnx>=1.16.0", "onnxruntime>=1.17.0"]
dev = ["pytest>=8.1.0", "ruff>=0.9.0", "mypy>=1.14.0"]   # mirrors [dependency-groups].dev

[tool.maturin]
manifest-path = "crates/koi-pyo3/Cargo.toml"
features = ["pyo3/extension-module"]    # maturin-scoped only — never a Cargo feature
module-name = "koi_maestro._engine"
python-source = "src"
strip = true
profile = "release-wheel"               # panic=unwind wheel profile (§3.1)
editable-profile = "dev"                # fast editable installs (maturin >=1.10)
include = ["src/koi_maestro/py.typed", "src/koi_maestro/_engine.pyi"]
```

---

## 4. PyO3 Best Practices & GIL Management

### 4.1 The Bound API (`Bound<'py, T>`)
All PyO3 interactions use `Bound<'py, T>`, ensuring exact lifetime binding with the Python GIL:
- Pass `Bound<'py, PyAny>` or `&Bound<'_, PyAny>` in function signatures.
- Use `obj.cast::<PyList>()` for downcasting.
- Use `obj.unbind()` to store a GIL-independent `Py<T>` across threads, and `handle.bind(py)` to reacquire access.

### 4.2 Safe GIL Release (`py.detach(|| ...)`)
**State Isolation Rule**: Always clone or snapshot the lightweight Rust state (`GameState`) **before** releasing the GIL. Passing Python references or accessing Python memory inside `py.detach` causes data races and memory corruption.

### 4.3 Zero-Copy Pre-Allocated Buffer Pattern (RL Throughput > 1M steps/s)
Instead of allocating NumPy arrays per step, Python pre-allocates contiguous buffers, and Rust writes directly into the memory slices without allocations during the search/step loop.

---

## 5. Asset Migration Inventory from a Sibling Research Codebase

> **Provenance note:** this inventory is retained as an attribution record —
> every sanctioned import has landed (see §8 status column). The staged
> snapshot `templates/sibling_staged/` (re-staged 2026-09-19 from
> the sibling codebase's `crates/` @ `31208b1`, post-split v2-campaign code) was
> deleted in the P6 cleanup after the last import landed; the sibling
> repository remains the reference on any discrepancy.

### IMPORTED AND LANDED
1. `sibling-core/src/helpers/seed.rs` → `crates/koi-core/src/seed.rs` — SplitMix64 `derive_named_seed`. Verbatim.
2. `sibling/scripts/check.{ps1,sh}` → `scripts/` — ported + `KOI_PROFILE=release-ci` gate wiring.
3. `sibling-bench/src/benchmark/framing.rs` → `crates/koi-bench/src/framing.rs` — verbatim bounded reader.
4. `sibling-bench/src/benchmark/process_tree.rs` → `crates/koi-bench/src/process_tree.rs` — Job Object + Unix process groups, `KOI_*` env, `koi-bench` stem.
5. `sibling-bench/src/benchmark/statistics.rs` → `crates/koi-bench/src/statistics.rs` — v1 flattening; needs TimeForfeit + leg-time + EB-CS when P1 lands the runner (see §8).
6. `sibling-solver/src/solver/endgame/lp.rs` → `crates/koi-solver/src/endgame/lp.rs` — two-phase simplex, AVX2+NEON kernels, 27 tests.
7. `sibling-build-info/` → `crates/koi-build-info/` — provenance substrate, simplified to the single workspace era.

### LANDED IN LATER PHASES (formerly "defer to P1/P2" — all items below have landed)
1. `sibling-bench/src/benchmark/{manifest,protocol,runner,worker,report,artifact,checkpoint}.rs` — wire DTOs + referee mapped to Koi-Koi (48 cards, dealer-mirrored legs, yaku scoring). Landed in `crates/koi-bench/src/`.
2. `sibling-bench/src/benchmark/{sprt,ranking}.rs` + `bin/tournament_rank.rs` — sequential inference + tournament merge. Landed (`sprt.rs`, `ranking.rs`, `bin/tournament_rank.rs`); `gsprt_calibrate` deliberately unported.
3. `sibling-pyo3/src/bin/stub_gen.rs` + `pyo3-stub-gen` — `.pyi` generator. Landed.
4. `sibling-core/src/helpers/config.rs` — two-stage `Config`→`ValidatedConfig` validation pattern. Landed in `crates/koi-solver/src/config.rs`.
5. `sibling-solver/src/solver/{cfr,efg,resolving,determinization,leaf,heuristic,ismcts,pimc}` — solver machinery, remodeled for Koi-Koi semantics. Landed under `crates/koi-solver/src/`.

### DO NOT IMPORT (Obsolete / Sibling-specific / Bevy / Selenium)
1. `crates/sibling-ui/` & Bevy dependencies (GUI bloat — koi is headless).
2. Selenium & Python browser infrastructure.
3. `crates/sibling-core/src/{game,states}/` (the sibling codebase's 40-card logic), `src/sibling_maestro/` game-facing Python.

---

## 6. Cross-Platform & OS Hardening

The engine targets three platforms natively, with zero lowest-common-denominator fallbacks on hot paths:

| Platform | ISA | OS-specific mechanisms |
|----------|-----|------------------------|
| Linux `x86_64` | AVX2/FMA (`_mm256_*`, `vfmadd213pd`, `POPCNT`/`TZCNT`) | POSIX process groups (`setpgid(0,0)` + `kill(-pgid, SIGKILL)`), `prctl(PR_SET_PDEATHSIG)`, cgroups v2 |
| Linux `aarch64` (DGX Spark, GB10 Grace Blackwell, Armv9.2) | NEON `float64x2` (`vfmaq_f64`), SVE2/SME2 optional (Cortex-X925/A725), `CNT`+`RBIT`/`CLZ` | Same POSIX containment; coherent unified memory (128 GB LPDDR5x over NVLink-C2C) — no CPU↔VRAM staging |
| Windows `x86_64` | AVX2/FMA, `POPCNT`/`TZCNT` | Win32 Job Objects (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`), `ReadFile` activation channel |

### Windows-specific notes

1. **File Locking Mitigation**: Windows kernel locks active `.pyd` files (`ERROR_SHARING_VIOLATION` / os error 32). In PowerShell, run `Get-Process python -ErrorAction SilentlyContinue | Stop-Process -Force` prior to rebuilds.
2. **Fast Linking**: Use `lld-link.exe` from LLVM instead of MSVC `link.exe` for 4x faster link times.
3. **Compiler Cache**: Enable `sccache` in `.cargo/config.toml`.
4. **Defender Exclusions**: Exclude repository path from Windows Defender real-time scanning to reduce build times by ~40%.

### Linux / DGX Spark notes

1. **Validation**: `scripts/check.sh` is the POSIX equivalent of `scripts/check.ps1` (fmt → clippy → nextest/test → doctests → cross-target gate for all three triples).
2. **Native toolchain**: `rustup` is Tier-1 on `aarch64-unknown-linux-gnu`; `cargo build`/`test` additionally need a C toolchain/linker (`sudo apt install build-essential`) — `cargo check` does not. Criterion benches need it for `alloca`'s `alloca.c`.
3. **PyO3**: native builds need `python3-dev`/`libpython3.x-dev` + `maturin>=1.9`; cross-builds need `PYO3_CROSS=1`, `PYO3_CROSS_LIB_DIR`, `PYO3_CROSS_PYTHON_VERSION`, plus a target linker — prefer building natively on the DGX.
4. **Accelerator providers (Post-P3)**: `ort`'s default binary download is **CPU-only** for `aarch64-unknown-linux-gnu`, but official aarch64 CUDA-13 wheels now exist (Microsoft merged aarch64 into the CUDA-13 packaging pipeline, PR onnxruntime#27760) — on PyPI stable since ~1.25 and on the `ort-cuda-13-nightly` Azure feed. **Required pattern**: `ort = { features = ["load-dynamic"] }` + `ort::init_from(path)` / `ORT_DYLIB_PATH`, never the `cuda`/`tensorrt` cargo features (they fail at link time on aarch64). Setup is: download the official aarch64 CUDA-13 wheel → extract the `.so` → create the SONAME symlinks. EP selection at runtime tries CUDA → CPU fallback. Constraints: GB10 is `sm_121` → FP32/FP16 models only (no INT8 kernels); driver **≥580.142** (580.173.02 recommended) — earlier 580.x crash under batch load (`CUDNN_FE` failure 11); TensorRT EP requires `--use_tensorrt` source builds and is optional.
5. **PyTorch**: official stable wheels cap at `sm_120` (PTX-JITs cleanly to `sm_121` via `compute_120`), and CUDA aarch64 nightly wheels are on `download.pytorch.org/whl/nightly/cu130`. The `torch>=2.2.0` `[ml]` extra can still resolve CPU-only on aarch64 — on DGX prefer the **NGC container** `nvcr.io/nvidia/pytorch:25.09-py3` or newer (verified GB10 support, CUDA 13.0, Triton/cuDNN/NCCL pre-matched) or pin the nightly index. Caveats: `TRITON_PTXAS_PATH=/usr/local/cuda/bin/ptxas` for Triton; flash-attention has no `sm_121` kernels.
6. **`panic = "abort"` + cdylib**: `release` applies `panic="abort"` workspace-wide, so a panic escaping a `#[pyfunction]` aborts the Python process instead of raising. Wheel builds therefore use the dedicated **`release-wheel`** profile (`panic="unwind"`, inherits release) — wired in `pyproject.toml` `[tool.maturin]`. Keep the FFI boundary panic-free anyway; the profile is the safety net, not the contract.
---

## 7. Project Structure (Actual)

```text
koi-maestro/
├── Cargo.toml                     # Workspace manifest: 6 crates, profiles
│                                  #   (release panic=abort; release-wheel
│                                  #   panic=unwind for the FFI boundary)
├── Cargo.lock                     # Locked dependency graph
├── pyproject.toml                 # PEP 621 + maturin (release-wheel profile),
│                                  #   ruff/mypy config, [ml] extra
├── rust-toolchain.toml            # channel 1.93.1 + rustfmt/clippy
├── README.md                      # Product overview + quickstart
├── LICENSE-MIT / LICENSE-APACHE   # Dual license texts
│
├── .github/workflows/ci.yml       # fmt, clippy -D warnings, tests, doctests,
│                                  #   cross-target checks, wheel builds
│
├── crates/
│   ├── koi-core/                  # Pure game domain: Card/Month/CardSet
│   │   └── src/                   #   bitboards, rulesets, 3-phase state
│   │                              #   machine, yaku scoring, seed derivation
│   ├── koi-solver/                # Decision engines: baselines (random,
│   │   ├── src/                   #   heuristic, ISMCTS, PIMC), endgame stack
│   │   │                          #   (exact turn-8, turn-7 subgame LP,
│   │   │                          #   optimal stopping, 2-phase simplex),
│   │   │                          #   EFG/sequence-form + CFR family
│   │   │                          #   (CFR+/DCFR/PCFR+/MCCFR/CCS-MCCFR +
│   │   │                          #   exploitability), continual resolving
│   │   │                          #   (belief, safety gadget, opponent model),
│   │   │                          #   leaf evaluator (384-feature, handcrafted
│   │   │                          #   + optional ORT), leaf_policy entrant
│   │   └── examples/              #   p2_certificates, p3_resolve_probe,
│   │                              #   efg_size_probe, seqform_size_probe
│   ├── koi-bench/                 # Paired benchmark harness: manifest schema,
│   │   ├── src/                   #   JSONL worker protocol, referee runner
│   │   │                          #   (mirrored legs, checkpoints, GSPRT+EB-CS),
│   │   │                          #   bootstrap stats, Davidson-ties BT + Holm +
│   │   │                          #   Nash mixture, provenance, containment
│   │   └── src/bin/               #   tournament_emit, tournament_rank
│   ├── koi-learn/                 # Leaf-dataset pipeline: .npy writer,
│   │   └── src/                   #   blueprint/resolve labelers, shard
│   │                              #   manifests (bin generate_leaf_data)
│   ├── koi-pyo3/                  # koi_maestro._engine cdylib + stub_gen
│   └── koi-build-info/            # Compile-time git stamp + FNV source digest
│
├── src/koi_maestro/               # Python package (maturin, src layout)
│   ├── __init__.py                # Thin re-export of the native module
│   ├── _engine.pyi / py.typed     # Committed stub (freshness-gated) + marker
│   └── learn/                     # dataset.py, model.py, train.py,
│                                  #   export.py, calibrate.py
│
├── src/tests/                     # pytest: wheel smoke + dataset contract
│
├── docs/                           
│   ├── RULES.md                   # Canonical engine-semantics rules
│   ├── ARCHITECTURE.md            # This document
│   ├── BENCHMARK-JOURNAL.md       # Intervention log (one row per run)
│   └── benchmarks/                # Append-only evidence base:
│                                  #   manifests/, phase dirs (p1..p5),
│                                  #   tournament/{manifests,results,pilot,
│                                  #   annex}, tuning/
│
└── scripts/
    ├── check.ps1 / check.sh       # Gate pipeline (platform pair)
    └── tournament/                # emit_manifests, freeze, launch_field,
                                   # launch_annex, rank, verify, watchdog
```

---
## 8. Rust File Import Inventory from a Sibling Research Codebase

Sources are the **post-split `crates/` paths** (a sibling research codebase @ `31208b1`,
v2-era). The staged offline snapshot was deleted in P6 — the sibling
repository wins on any discrepancy.

| Source in the sibling codebase | Destination in `koi-maestro` | Phase | Technical Role & Required Adaptations | Status |
| :--- | :--- | :--- | :--- | :--- |
| `sibling-core/src/helpers/seed.rs` | `crates/koi-core/src/seed.rs` | P0 | SplitMix64 `derive_named_seed`; pure `const fn`. | **LANDED** |
| `sibling/scripts/check.{ps1,sh}` | `scripts/check.{ps1,sh}` | P0 | Gate pipeline; added `KOI_PROFILE=release-ci` wiring + cross-gate. | **LANDED** |
| `sibling-bench/src/benchmark/framing.rs` | `crates/koi-bench/src/framing.rs` | P0 | 1 MiB bounded reader. | **LANDED** |
| `sibling-bench/src/benchmark/process_tree.rs` | `crates/koi-bench/src/process_tree.rs` | P0 | Job Object / process-group containment; `KOI_*` env, `koi-bench` stem. | **LANDED** |
| `sibling-bench/src/benchmark/statistics.rs` | `crates/koi-bench/src/statistics.rs` | P1 | ChaCha8 cluster bootstrap + `TimeForfeit` handling + EB-CS (`CsInterval`/`EmpiricalBernsteinCs`). | **LANDED** |
| `sibling-solver/src/solver/endgame/lp.rs` | `crates/koi-solver/src/endgame/lp.rs` | P0 | Two-phase simplex, AVX2+NEON, 27 tests. | **LANDED** |
| `sibling-build-info/` | `crates/koi-build-info/` | P0 | Git stamp + FNV workspace digest + rustflags/PGO identity; `include!("shared.rs")` single-impl trick. Single-era simplification (no `RepoEra`/`BenchHome`). | **LANDED** |
| `sibling-pyo3/src/bin/stub_gen.rs` | `crates/koi-pyo3/src/bin/stub_gen.rs` | Post-P3 | `.pyi` generation + ancestor pyproject resolver; shared `render_stub_pyi` + committed-file freshness gate. | **LANDED** |
| `sibling-core/src/helpers/config.rs` | `crates/koi-solver/src/config.rs` | P1 | Two-stage `Config`→`ValidatedConfig` pattern. | landed |
| `sibling-bench/src/benchmark/checkpoint.rs` | `crates/koi-bench/src/checkpoint.rs` | P1 | Bound append-only JSONL resume: torn-tail truncate, foreign-binding hard error, manifest-order evaluator fold. | landed |
| `sibling-bench/src/benchmark/artifact.rs` | `crates/koi-bench/src/artifact.rs` | P1 | Worktree builds + 10-component `ArtifactIdentity` provenance; consumes `koi-build-info` (single era). | landed |
| `sibling-bench/src/benchmark/manifest.rs` | `crates/koi-bench/src/manifest.rs` | P1 | Manifest schema + fail-closed validation + `deny_unknown_fields`. Redesign: `DealSpec{seed,dealer,deck}` 48-card, anomaly policy, ruleset pinning, paired-XOR-entrants. | landed |
| `sibling-bench/src/benchmark/protocol.rs` | `crates/koi-bench/src/protocol.rs` | P1 | Negotiation + typed frames + ledger-replay validation. Redesign: `WireState` for koi zones (field/captured/koi-koi flags/phase), `WireAction` 4-variant tagged enum, hidden = counts only. | landed |
| `sibling-bench/src/benchmark/runner.rs` | `crates/koi-bench/src/runner.rs` | P1 | Referee: resolve→preflight→checkpoint→mirrored legs→stopping. Redesign: dealer-swap mirroring, match-across-rounds legs, `LegClock` Koi-calibrated. | landed |
| `sibling-bench/src/benchmark/worker.rs` | `crates/koi-bench/src/worker.rs` | P1 | Solver cache keyed by effective config + per-decision reseed (`derive_named_seed`). | landed |
| `sibling-bench/src/benchmark/report.rs` | `crates/koi-bench/src/report.rs` | P1 | Atomic evidence publication (tmp-dir rename, refuse-overwrite), Markdown+JSON, era-aware repro instructions (single era here). | landed |
| `sibling-bench/src/benchmark/sprt.rs` | `crates/koi-bench/src/sprt.rs` | P2 | GSPRT + EB-CS equivalence. Port **after** the estimand decision — pentanomial bins assume W/D/L leg pairs. | landed |
| `sibling-bench/src/benchmark/ranking.rs` + `bin/tournament_rank.rs` | `crates/koi-bench/` | P5 | Davidson-ties BT + Holm + intransitivity; per-pairing merge with consistency validation + multi-build provenance disclosure. | landed |
| `sibling-bench/src/bin/gsprt_calibrate.rs` | — | — | GSPRT Monte-Carlo calibration. | not ported — unused |
| `sibling-bench/src/bin/paired_benchmark.rs` | `crates/koi-bench/src/main.rs` | P1 | CLI + hidden `--worker`/`--worker-launcher` flags (the containment re-exec entry — wire it when runner lands). | landed |
| `sibling-solver/src/solver/cfr/` | `crates/koi-solver/src/cfr/` | P2 | CFR+/MCCFR/CCS-MCCFR/DCFR/PCFR+ + regret tables + blueprint profile. | landed |
| `sibling-solver/src/solver/efg/` | `crates/koi-solver/src/efg/` | P2 | EFG compiler + sequence form + info-set tree. Remodel chance nodes for stock draws. | landed |
| `sibling-solver/src/solver/resolving/` | `crates/koi-solver/src/resolving/` | P3 | Continual resolving: belief, safety gadget, opponent model, ledger reconstruction. Remodel: koi draws go to field not hand (no un-deal); gadget as compiled root wrap with oracle-valued depth-capped leaves; sampled root action. | landed |
| `sibling-solver/src/solver/leaf/` | `crates/koi-solver/src/leaf/` | Post-P3 | `LeafEvaluator` trait contract (Send, batch default, fail-closed construction, counted degrade). 384-feature/16-action contract, handcrafted + `leaf-ort` backends, `LeafMarginOracle` resolver wiring. | **LANDED** |
| `sibling-learn/src/learn/` + `bin/generate_leaf_data.rs` | `crates/koi-learn/` | Post-P3 | npy v1.0 writer + labels (blueprint on reduced variants, per-row resolve on canonical via `resolve_decision_with_worlds`) + deterministic `mix_seed` generation + shard manifests. Bucket taxonomy remapped to koi's positional classes (stop calls, yaku pressure, final hand cycle). | **LANDED** |
| the sibling codebase's `src/sibling_maestro/learn/` | `src/koi_maestro/learn/` | Post-P3 | Dataset loader + residual-MLP model + train loop + ONNX export (in-graph norm, masked softmax) + calibration + golden parity fixtures. Koi dims: 384/16/48, scalar block 288..384. | **LANDED** |
| `sibling-solver/src/solver/{heuristic,ismcts,pimc}/` | `crates/koi-solver/src/{heuristic,ismcts,pimc}/` | P1 | Baseline solvers for the first paired panel. | landed |
| `sibling-solver/src/solver/determinization.rs` | `crates/koi-solver/src/determinization.rs` | P2 | World sampling/deck materialization; remodel hidden zones (opponent hand + stock order). | landed |
