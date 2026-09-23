# koi-maestro

Hanafuda Koi-Koi engine and game-theoretic solver suite: a Rust workspace
implementing the complete game semantics (Nintendo ruleset, yaku scoring,
deal anomalies), a solver stack spanning baselines through continual
resolving with a learned leaf oracle, a paired benchmark harness with
sequential stopping and tournament ranking, and a Python extension module
with a training pipeline for the leaf model.

Evidence labels used throughout: **Certified** = reduced-domain mathematical
certificates; **Measured** = timed observations under a named protocol;
**Empirical** = solver strength from played benchmark legs; **Boundary** =
known limits / development-only evidence.

## Status

Complete milestone build — engine, solver stack, benchmark harness, Python
bindings, and leaf-model pipeline are implemented and gate-tested. A
9-entrant field tournament and a champion/runner-up annex have been played
under the frozen protocol (see *Results*). Build: `main` is stable; the
pinned toolchain is Rust 1.93.1.

## Solver roster

| `solver` | What it is |
|---|---|
| `random` | Uniform legal action |
| `heuristic` | Deterministic yaku/material policy |
| `ismcts` | Information-set MCTS (determinized) |
| `pimc` | Perfect-information Monte Carlo (field spec: 512 sims, batch 64) |
| `endgame` | Heuristic midgame + solved stop decisions + exact last-turn play |
| `resolving` | Continual resolving: belief ledger, safety gadget, CFR subgame solving; `adaptive`/`ox` flags, `blueprint_artifact`, `leaf_model` |
| `leaf_policy` | Direct policy entrant: 384-feature encode + belief marginals, masked argmax over canonical actions, legal fallback |
| `archetype` | Scripted opponents for annex/robustness probes: `heuristic`, `materialist`, `yaku_chaser`, `banker`, `gambler`, `timid`, `random` |

`resolving_leaf` is a manifest-level name for `solver: "resolving"` with
`resolving.leaf_model` set — the resolver prices depth-capped leaves with
the trained ONNX evaluator instead of the rollout oracle. `leaf_policy`
without a `leaf_model` falls back to a deterministic handcrafted evaluator.

## Requirements

- Rust **1.93.1** (`rust-toolchain.toml` pins channel + rustfmt + clippy).
- Python **>=3.10,<3.14** and **maturin >=1.10,<2.0** for the extension.
- Runtime dep: `numpy >=1.26,<3`. Optional `[ml]` extra: torch,
  safetensors, onnx, onnxruntime.
- Optional leaf inference: ONNX Runtime shared library reachable via
  `ORT_DYLIB_PATH` + the `leaf-ort` cargo feature (`ort` is loaded
  dynamically — no vendored binaries). On aarch64 CUDA hosts (e.g.
  NVIDIA DGX Spark) the runtime wheel must be fetched and extracted
  manually; see `docs/ARCHITECTURE.md` for the required setup steps.
- Targets: Linux x86_64, Linux aarch64 (Armv9.2), Windows x86_64. macOS is
  out of scope.

## Quickstart

```powershell
# Windows — full gate: fmt, clippy -D warnings, tests (profile release-ci),
# doctests, cross-target cargo check (aarch64/x86_64-linux + windows-msvc)
.\scripts\check.ps1
```

```bash
# Linux — same pipeline
./scripts/check.sh
```

Env knobs: `KOI_PROFILE=release` reruns tests under the certification
profile; `KOI_SKIP_CROSS=1` skips the cross-target checks.

```bash
# Paired benchmark (manifest schema: paired legs on shared deal clusters,
# GSPRT/EB-CS stopping, bootstrap CIs, forfeit-as-loss)
cargo run -p koi-bench --release -- run \
  --manifest docs/benchmarks/manifests/p5-smoke-ismcts-vs-heuristic.json \
  --out out/smoke
# -> out/smoke/{legs.jsonl,report.json,report.md}
```

```bash
# Python wheel (profile release-wheel: panics unwind into PanicException)
pip install maturin
maturin build --profile release-wheel
pip install target/wheels/koi_maestro-*.whl
python -m pytest src/tests   # wheel API smoke + dataset contract
```

Test boundaries — suites outside the default `cargo test` gate:

| Suite | Why it's opt-in |
|-------|-----------------|
| ORT golden parity (`cargo test -p koi-solver --features leaf-ort`) | Needs the ONNX Runtime dynamic library on the loader path plus the committed fixture model. |
| `src/tests` (pytest) | Runs against the built wheel — `maturin build` + `pip install` first; not part of `cargo test`. |
| `koi-build-info` digest test | Recomputes the workspace source digest and compares it to the build-time stamp; requires a real git checkout — without `.git` the build itself still works but stamps `unstamped` provenance (dirty), and this test's comparison no longer applies. |
| Worker process-tree tests (koi-bench, Windows) | Spawn/kill real child processes; asserted only on `cfg(windows)`. |

A minimal entrant config inside a pair manifest's `candidate`/`baseline`
`config` block:

```json
{ "solver": "pimc", "pimc": { "max_simulations": 512, "batch_size": 64, "scoring_method": "mean" } }
{ "solver": "resolving", "resolving": { "adaptive": true, "ox": false } }
{ "solver": "leaf_policy", "leaf_policy": { "leaf_model": "path/to/model.onnx" } }
```

Leaf-model entrants additionally need the release binary built with the
ORT feature (`cargo build --release -p koi-bench --features leaf-ort`)
and `ORT_DYLIB_PATH` pointing at the ONNX Runtime library.

## Results

All empirical numbers below are paired-leg win scores on mirrored deal
clusters (dealer swapped), intention-to-treat (forfeits score as losses),
measured under the frozen protocol: Nintendo ruleset, GSPRT(0, +100 Elo,
α=β=.05) floor 8/cap 256 clusters, EB-CS equivalence δ=.05 armed ≥16,
bootstrap 10,000 @ 0.99, 30 s per-leg latency cap, forfeit margin 256.

### Certified — reduced-domain solver certificates (P2)

- `nano_6` exact LP equilibrium value ±1.777778, joint-profile
  exploitability 0.000e0 at tol 1e-6.
- `micro_8` CFR-family convergence to South ≈ 0.4286: CFR+ ~0 exploitability
  @1000+ iters, DCFR/PCFR+ ~0 @100+, MCCFR 0.0069 / CCS-MCCFR 0.0059 @40k.
- Evidence: `docs/benchmarks/p2-solver-certificates/` — these certify the
  solver machinery on reduced domains, never full-game optimality.

### Empirical — frozen-protocol field (9-entrant merged tournament, 36 pairs)

873 clusters played of the 256-declared panel (≤77 per pairing after
sequential stopping), 1746 valid legs, 0 forfeits — release binary,
dirty-tree builds (two attested binaries, all disclosed), binary-hash bound
(verdict `development`).

| Rank | Entrant | BT Elo (CI99) | Win score |
|---|---|---|---|
| 1 | `resolving_leaf` | +72.5 [+10.5, +145.7] | 0.6076 |
| 2 | `endgame` | +56.3 [+36.7, +83.2] | 0.5833 |
| 3 | `pimc` | +42.5 [−17.8, +94.5] | 0.5625 |
| 4 | `heuristic` | +19.8 [−25.9, +67.2] | 0.5278 |
| 5 | `leaf_policy` | +15.2 [−29.1, +59.8] | 0.5208 |
| 6 | `resolving_fixed` | +3.9 [−68.4, +60.9] | 0.5035 |
| 7 | `resolving_adaptive` | −0.6 [−53.9, +45.7] | 0.4965 |
| 8 | `ismcts` | −21.0 [−63.2, +12.4] | 0.4653 |
| 9 | `random` | −188.7 [−260.0, −121.9] | 0.2326 |

Nash mixture: `resolving_leaf` 0.5936 / `endgame` 0.4010 / others ≈ 0.
Six intransitive triples recorded (7.1% of all triples) — the Nash
mixture, not the scalar Elo, is the reliable summary. `resolving_leaf`
reached the top at ~10× smaller resolve budget than the field's other
resolvers (12 worlds / 60k nodes / 80 CFR / depth 4 vs 24w/600k/300cfr/d6).
Annex (earlier 7-entrant field's champion + runner-up vs 7 scripted
archetypes, 14 pairs): `pimc` beats all seven archetypes over both seats;
`endgame` beats five, ties `banker`/`timid` at 0.500.
Evidence: `docs/benchmarks/tournament/` (`tournament-report.md` is the
merged ranking) + `annex/annex-report.md`.

### Measured — relaxed-budget control (exploratory, different time contract)

Deep resolving (24 worlds / 600k nodes / 300 CFR / depth 6, adaptive+OX off)
vs `pimc` at a 120 s leg cap on the tuning panel: GSPRT `accept_lower` at
9 clusters, resolving win score 0.472 — **pimc's field result is a strength
result, not a time-contract artifact**. This evidence is never pooled with
the canonical field.

### Boundary — leaf model (development evidence)

- 384→(value, 16-policy) residual MLP, 768×4, trained 30 epochs on a
  48-shard corpus (12 blueprint + 36 resolve labels; 4 resolve shards lost
  to a generator crash — documented boundary). Validation combined loss
  1.9634; ONNX opset 20 / IR 9, Rust↔Python parity ~1.2e-5 value / ~2e-7
  policy (committed fixture `crates/koi-solver/tests/leaf/`).
- Tuning evidence: `resolving_leaf` ≈ rollout-oracle resolving at equal spec
  (ws 0.5000 vs `resolving_adaptive`); outright loss to `heuristic`
  (ws 0.4062) — the learned oracle is accepted on validation parity, not
  demonstrated dominance.
- Field: `leaf_policy` completed all seven supplemental pairs clean
  (0 forfeits, sub-second decisions); `resolving_leaf` required a reduced
  spec to fit the cumulative 30 s leg cap — then topped the merged
  9-entrant field (see the table above).

## Repository layout

| Path | Contents |
|---|---|
| `crates/koi-core` | Pure game domain: 48-card bitboards, rulesets, 3-phase state machine, yaku scoring, seed derivation |
| `crates/koi-solver` | Decision engines: baselines, endgame stack (exact turn-8, turn-7 subgame LP, optimal stopping, two-phase simplex), EFG/sequence-form + CFR family + exploitability, continual resolving, leaf evaluator + `leaf_policy` |
| `crates/koi-bench` | Paired benchmark harness: manifest schema, JSONL worker protocol, referee (mirrored legs, checkpoints, GSPRT/EB-CS), bootstrap stats, Davidson-ties BT + Holm + Nash mixture, provenance. Bins: `koi-bench`, `tournament_emit`, `tournament_rank` |
| `crates/koi-learn` | Leaf-dataset pipeline: `.npy` writer, blueprint/resolve labelers, shard manifests |
| `crates/koi-pyo3` | `koi_maestro._engine` extension + `stub_gen` (committed `.pyi`, freshness-gated) |
| `crates/koi-build-info` | Compile-time git stamp + FNV source digest + profile identity |
| `src/koi_maestro` | Python package: thin `__init__`, `learn/` (dataset, model, train, export, calibrate) |
| `src/tests` | pytest suite: wheel smoke (all solvers, determinism) + dataset contract |
| `docs/` | `RULES.md`, `ARCHITECTURE.md`, `BENCHMARK-JOURNAL.md`, `benchmarks/` evidence base |
| `scripts/` | `check.{ps1,sh}` gates, `tournament/` ops scripts |

## Documentation

- `docs/RULES.md` — canonical engine-semantics rules (phases, yaku, scoring).
- `docs/ARCHITECTURE.md` — system architecture and platform hardening notes.
- `docs/BENCHMARK-JOURNAL.md` — intervention log; one row per measured run.
- `docs/benchmarks/` — the append-only evidence base; `INDEX.md` maps
  directories to journal rows.

## License

Dual-licensed under MIT OR Apache-2.0 — see `LICENSE-MIT`, `LICENSE-APACHE`.
