# Koi-Koi paired benchmark — P5 field: 7-entrant definitive tournament: ismcts vs resolving_adaptive

**verdict: `development`** — 18 of 256 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `eb49922458b43905` |
| referee | `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| baseline | `ismcts` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| candidate | `resolving_adaptive` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| stopping | `gsprt_accept_lower` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.4444`  CI99 `[0.2778, 0.6111]`
- **candidate margin**: `-0.72`  CI99 `[-9.72, 6.89]`
- clusters `18`, legs `36` — wins `16`, draws `0`, losses `20`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 432 | 83.48 | 35.02 | 642.62 | 1109.30 | 1001.74 | 0 |
| candidate | 431 | 878.71 | 30.28 | 3750.40 | 5368.72 | 10520.07 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | -10.00 | valid (cand S) | 4.00 | valid (cand N) |
| `7619234134199958248` | 4.00 | valid (cand S) | 2.00 | valid (cand N) |
| `9318061777837526749` | -4.00 | valid (cand S) | 4.00 | valid (cand N) |
| `6001197866264162459` | 2.00 | valid (cand S) | -2.00 | valid (cand N) |
| `6044635964106635247` | -28.00 | valid (cand S) | -6.00 | valid (cand N) |
| `6903715913044025677` | 60.00 | valid (cand S) | -10.00 | valid (cand N) |
| `10814382527142015852` | -2.00 | valid (cand S) | 6.00 | valid (cand N) |
| `13535137607426657764` | 10.00 | valid (cand S) | -2.00 | valid (cand N) |
| `431149292466098373` | -68.00 | valid (cand S) | -8.00 | valid (cand N) |
| `14974509662690242321` | 60.00 | valid (cand S) | -28.00 | valid (cand N) |
| `15928830168549992736` | 4.00 | valid (cand S) | 28.00 | valid (cand N) |
| `4112377384228269587` | 28.00 | valid (cand S) | -12.00 | valid (cand N) |
| `18000172133288265584` | -4.00 | valid (cand S) | 4.00 | valid (cand N) |
| `17685270382932053218` | 2.00 | valid (cand S) | -8.00 | valid (cand N) |
| `9991098352591498860` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `13872560487504887217` | -12.00 | valid (cand S) | 12.00 | valid (cand N) |
| `5109558618046986729` | -4.00 | valid (cand S) | -20.00 | valid (cand N) |
| `18422795515743521302` | -8.00 | valid (cand S) | -20.00 | valid (cand N) |

