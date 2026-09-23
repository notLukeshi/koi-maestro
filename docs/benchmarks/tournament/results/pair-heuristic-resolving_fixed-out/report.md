# Koi-Koi paired benchmark — P5 field: 7-entrant definitive tournament: heuristic vs resolving_fixed

**verdict: `development`** — 15 of 256 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `38d0c5f7264a829b` |
| referee | `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| baseline | `heuristic` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| candidate | `resolving_fixed` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| stopping | `gsprt_accept_lower` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.4667`  CI99 `[0.3333, 0.6000]`
- **candidate margin**: `5.27`  CI99 `[-1.73, 15.87]`
- clusters `15`, legs `30` — wins `14`, draws `0`, losses `16`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 335 | 0.15 | 0.14 | 0.21 | 4.49 | 1.73 | 0 |
| candidate | 329 | 996.01 | 42.66 | 3881.80 | 5628.61 | 10922.95 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | -12.00 | valid (cand S) | -12.00 | valid (cand N) |
| `7619234134199958248` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `9318061777837526749` | 28.00 | valid (cand S) | -28.00 | valid (cand N) |
| `6001197866264162459` | -4.00 | valid (cand S) | 24.00 | valid (cand N) |
| `6044635964106635247` | 32.00 | valid (cand S) | -40.00 | valid (cand N) |
| `6903715913044025677` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `10814382527142015852` | 96.00 | valid (cand S) | 2.00 | valid (cand N) |
| `13535137607426657764` | -20.00 | valid (cand S) | 10.00 | valid (cand N) |
| `431149292466098373` | -10.00 | valid (cand S) | 32.00 | valid (cand N) |
| `14974509662690242321` | 10.00 | valid (cand S) | -2.00 | valid (cand N) |
| `15928830168549992736` | 28.00 | valid (cand S) | -6.00 | valid (cand N) |
| `4112377384228269587` | -4.00 | valid (cand S) | 28.00 | valid (cand N) |
| `18000172133288265584` | 20.00 | valid (cand S) | -10.00 | valid (cand N) |
| `17685270382932053218` | -4.00 | valid (cand S) | 8.00 | valid (cand N) |
| `9991098352591498860` | -4.00 | valid (cand S) | -4.00 | valid (cand N) |

