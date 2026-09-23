# Koi-Koi paired benchmark — annex: endgame vs heuristic

**verdict: `development`** — 9 of 256 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `3e54901724aa4395` |
| referee | `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| baseline | `endgame` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| candidate | `heuristic` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| stopping | `gsprt_accept_lower` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.4444`  CI99 `[0.2778, 0.5000]`
- **candidate margin**: `-1.44`  CI99 `[-5.78, 0.00]`
- clusters `9`, legs `18` — wins `8`, draws `0`, losses `10`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 180 | 0.11 | 0.11 | 0.16 | 0.22 | 1.14 | 0 |
| candidate | 177 | 0.26 | 0.11 | 0.17 | 26.12 | 2.57 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | -2.00 | valid (cand S) | -24.00 | valid (cand N) |
| `7619234134199958248` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `9318061777837526749` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `6001197866264162459` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `6044635964106635247` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `6903715913044025677` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `10814382527142015852` | -12.00 | valid (cand S) | 12.00 | valid (cand N) |
| `13535137607426657764` | 6.00 | valid (cand S) | -6.00 | valid (cand N) |
| `431149292466098373` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |

