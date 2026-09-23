# Koi-Koi paired benchmark — P5 field: 7-entrant definitive tournament: heuristic vs ismcts

**verdict: `development`** — 12 of 256 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `11429cee8fbd6726` |
| referee | `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| baseline | `heuristic` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| candidate | `ismcts` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| stopping | `gsprt_accept_lower` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.5000`  CI99 `[0.4167, 0.5833]`
- **candidate margin**: `3.17`  CI99 `[-3.67, 10.58]`
- clusters `12`, legs `24` — wins `11`, draws `2`, losses `11`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 350 | 0.78 | 0.15 | 0.23 | 47.96 | 11.37 | 0 |
| candidate | 343 | 38.20 | 13.81 | 125.17 | 677.23 | 545.97 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | -28.00 | valid (cand S) | 4.00 | valid (cand N) |
| `7619234134199958248` | -4.00 | valid (cand S) | 2.00 | valid (cand N) |
| `9318061777837526749` | -10.00 | valid (cand S) | 28.00 | valid (cand N) |
| `6001197866264162459` | -6.00 | valid (cand S) | 40.00 | valid (cand N) |
| `6044635964106635247` | 0.00 | valid (cand S) | 2.00 | valid (cand N) |
| `6903715913044025677` | -10.00 | valid (cand S) | 28.00 | valid (cand N) |
| `10814382527142015852` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `13535137607426657764` | 56.00 | valid (cand S) | -10.00 | valid (cand N) |
| `431149292466098373` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `14974509662690242321` | -8.00 | valid (cand S) | 12.00 | valid (cand N) |
| `15928830168549992736` | -28.00 | valid (cand S) | 12.00 | valid (cand N) |
| `4112377384228269587` | -4.00 | valid (cand S) | 0.00 | valid (cand N) |

