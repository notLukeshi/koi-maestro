# Koi-Koi paired benchmark — annex: pimc vs heuristic

**verdict: `development`** — 13 of 256 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `f8b1d576519fff35` |
| referee | `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| baseline | `pimc` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| candidate | `heuristic` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| stopping | `gsprt_accept_lower` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.4231`  CI99 `[0.2692, 0.5962]`
- **candidate margin**: `-2.00`  CI99 `[-6.38, 2.08]`
- clusters `13`, legs `26` — wins `9`, draws `4`, losses `13`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 315 | 83.13 | 7.66 | 313.19 | 866.86 | 1007.11 | 0 |
| candidate | 308 | 0.14 | 0.14 | 0.21 | 0.51 | 1.70 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | 0.00 | valid (cand S) | -8.00 | valid (cand N) |
| `7619234134199958248` | 0.00 | valid (cand S) | -4.00 | valid (cand N) |
| `9318061777837526749` | 12.00 | valid (cand S) | -12.00 | valid (cand N) |
| `6001197866264162459` | -28.00 | valid (cand S) | 0.00 | valid (cand N) |
| `6044635964106635247` | 4.00 | valid (cand S) | -6.00 | valid (cand N) |
| `6903715913044025677` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `10814382527142015852` | -12.00 | valid (cand S) | 28.00 | valid (cand N) |
| `13535137607426657764` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `431149292466098373` | 10.00 | valid (cand S) | 4.00 | valid (cand N) |
| `14974509662690242321` | -20.00 | valid (cand S) | -4.00 | valid (cand N) |
| `15928830168549992736` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `4112377384228269587` | -20.00 | valid (cand S) | 10.00 | valid (cand N) |
| `18000172133288265584` | 0.00 | valid (cand S) | -6.00 | valid (cand N) |

