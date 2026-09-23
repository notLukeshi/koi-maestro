# Koi-Koi paired benchmark — P5 field: 7-entrant definitive tournament: pimc vs resolving_adaptive

**verdict: `development`** — 9 of 256 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `6efe1a9d31b586a2` |
| referee | `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| baseline | `pimc` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| candidate | `resolving_adaptive` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| stopping | `gsprt_accept_lower` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.5000`  CI99 `[0.5000, 0.5000]`
- **candidate margin**: `-4.22`  CI99 `[-10.56, -0.11]`
- clusters `9`, legs `18` — wins `9`, draws `0`, losses `9`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 203 | 94.13 | 41.83 | 329.10 | 549.02 | 1061.58 | 0 |
| candidate | 202 | 960.01 | 129.72 | 3862.01 | 5156.16 | 10773.48 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | 10.00 | valid (cand S) | -20.00 | valid (cand N) |
| `7619234134199958248` | 6.00 | valid (cand S) | -2.00 | valid (cand N) |
| `9318061777837526749` | -10.00 | valid (cand S) | 4.00 | valid (cand N) |
| `6001197866264162459` | -28.00 | valid (cand S) | 10.00 | valid (cand N) |
| `6044635964106635247` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `6903715913044025677` | 12.00 | valid (cand S) | -12.00 | valid (cand N) |
| `10814382527142015852` | 4.00 | valid (cand S) | -44.00 | valid (cand N) |
| `13535137607426657764` | -44.00 | valid (cand S) | 40.00 | valid (cand N) |
| `431149292466098373` | -12.00 | valid (cand S) | 10.00 | valid (cand N) |

