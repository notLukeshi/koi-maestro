# Benchmark evidence index

Append-only evidence base. Every row of `docs/BENCHMARK-JOURNAL.md`'s
intervention log maps to a directory or artifact here. Verdict vocabulary:
`canonical` (clean-tree release evidence), `development` (measured evidence
with recorded provenance caveats), `vacuous` (no played legs).

## Layout

| Path | Contents | Journal row(s) |
|---|---|---|
| `manifests/` | Phase manifests (p2–p5; p1's manifest ships inside `p1-heuristic-vs-random/`, tuning manifests live in `tuning/`) | P2, P3, P5-smoke |
| `p1-heuristic-vs-random/` | P1 canonical pair evidence (manifest + legs + report) | `engine-v0.2-p1-bench` |
| `p1-smoke/` | P1 pipeline smoke (dirty-tree, informational) | P1 row (smoke note) |
| `p2-endgame-vs-heuristic/` | P2 paired legs: endgame entrant vs heuristic | `545bcf2..8974354` |
| `p2-solver-certificates/` | Reduced-domain certificates (nano_6/micro_8 LP + CFR convergence) | `bb487b5` P2 solver stack |
| `p2-smoke/` | P2 pipeline smoke (certificate schema; label says p2-solver-certificates) | P2 row (smoke note) |
| `p3-resolving-vs-{banker,gambler,heuristic,materialist}/` | P3 resolving vs scripted archetypes, 64-seed CRN panel | `engine-v1.1-p3-resolving` |
| `p3-ox-vs-noox/` | OX ablation pair | P3 row |
| `p3-smoke/` | P3 smoke | P3 row |
| `p5-smoke-ismcts-vs-heuristic/` | P5 harness smoke | P5 machinery |
| `tournament/` | Canonical tournament tree (below) | P5 field + annex, P6 supplemental |
| `tuning/` | Tuning-panel development evidence — never canonical, never pooled | A5 leaf selection, A3 relaxed-budget control, spec probes |

## `tournament/` detail

| Path | Contents |
|---|---|
| `manifest.json` | v1 7-entrant roster manifest (P5 field) |
| `manifest-9.json` | 9-entrant roster manifest (P6 field: + resolving_leaf + leaf_policy) |
| `pilot-manifest.json` | 3-entrant C-2 pilot roster |
| `panel.json` | Shared 256-seed canonical panel (hash `cd42c48457b48eec`) |
| `freeze.json` | Current freeze record (P6 9-entrant, commit `9d0894e`) — the live binding file `freeze.ps1` regenerates on every re-freeze |
| `freeze-p5-7entrant.json` | Archived v1 freeze record (commit `bc6adfe`, binary `890228ce`) |
| `manifests/` | 36 emitted pair manifests (complete 9-entrant round robin) |
| `results/` | Per-pair output dirs: `legs.jsonl` + `report.json` + `report.md` |
| `results-superseded/` | Preserved superseded evidence (3 `vacuous` + 5 `development` verdicts): resolving_leaf at 16w/200k/80cfr/d4 (768×4 model) forfeited ~90% of legs under the cumulative 30s leg cap — superseded by the 12w/60k/80cfr/d4 spec |
| `pilot/` | C-2 pilot pair manifests + results |
| `annex/` | Champion/runner-up vs 7 archetypes: `manifests/`, `results/`, `annex-report.md` aggregate |
| `tournament-results.json` / `tournament-report.md` | Merged 9-entrant ranking report (`tournament_rank` output; resolving_leaf champion +72.5 Elo) |

## Integrity tooling

- `scripts/tournament/verify.ps1 [-All]` — legs.jsonl schema + protocol +
  provenance per pair, first-cluster gate, panel-prefix check.
- `scripts/tournament/freeze.ps1` — re-binds commit/tree/binary/manifest/
  panel/leaf-model hashes; required after any config or model change.
- `scripts/tournament/rank.ps1` — merges all pair reports into the
  tournament ranking report.

## Transform log

Mechanical evidence transforms are recorded here (append-only):

- **2026-09-22 (P6-D14a)** `report.json` `seeds[].*deals.trace` → `[]` in all
  77 files sharing the `run.seeds` schema (38.2MB → 1.3MB). `legs.jsonl`
  remains the canonical trace store — zero unique data lost. The two
  LP-certificate reports carry no legs and were untouched.
- **2026-09-22 (P6-D14b)** absolute dev paths scrubbed in 181 tracked files:
  repo-local absolute paths → `<repo-root>` (incl. `executable`/`leaf_model`
  fields — hashes bind identity, not paths), sibling-repo absolute paths →
  `<sibling-repo>`. Residual personal-path hits: zero.
- **2026-09-23 (publication audit)** `tournament-report.md` header corrected:
  "clusters per pairing (declared panel)" 77 → **256** (the generator had
  printed the max-played count; fixed in `report.rs` — `declared_clusters`
  is now a report field). `tournament-results.json` carries no such field
  and needed no edit.
- **2026-09-23 (P6-D14a follow-up)** latency-skeleton restore: the trace
  strip above had also removed `decision_latency_ms`, which the merged
  ranking reads — the merged latency columns degenerated to `0.0 [NaN]`.
  Restored all trace entries as skeletons (every real field preserved;
  `selected_action` replaced by a minimal valid `WireAction`, `"stripped":
  true` marker added) from the pre-strip blob `b11bbd6^` across the same
  77 `run.seeds` files (61,831 entries), then re-ran `tournament_rank`
  (rebuilt binary carrying the `declared_clusters` fix). Verified: all
  latency means/p95 finite and plausible, ranking/Elo/win-scores
  bit-identical to the pre-restore merge — an evidence-preserving
  reporting repair, not a new tournament run. `legs.jsonl` remains the
  canonical full-fidelity trace store.
