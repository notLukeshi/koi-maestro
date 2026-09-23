# Koi-Koi paired benchmark — P5 field: 7-entrant definitive tournament: ismcts vs pimc

**verdict: `development`** — 12 of 256 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `813fb4eabffe5590` |
| referee | `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| baseline | `ismcts` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| candidate | `pimc` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| stopping | `gsprt_accept_upper` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.7083`  CI99 `[0.5417, 0.8750]`
- **candidate margin**: `2.00`  CI99 `[-5.75, 9.17]`
- clusters `12`, legs `24` — wins `17`, draws `0`, losses `7`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 294 | 44.92 | 26.07 | 163.94 | 672.27 | 550.23 | 0 |
| candidate | 314 | 76.88 | 17.86 | 331.11 | 739.77 | 1005.85 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | 12.00 | valid (cand S) | -52.00 | valid (cand N) |
| `7619234134199958248` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `9318061777837526749` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `6001197866264162459` | 4.00 | valid (cand S) | 40.00 | valid (cand N) |
| `6044635964106635247` | -8.00 | valid (cand S) | 2.00 | valid (cand N) |
| `6903715913044025677` | -12.00 | valid (cand S) | 12.00 | valid (cand N) |
| `10814382527142015852` | 2.00 | valid (cand S) | 10.00 | valid (cand N) |
| `13535137607426657764` | -32.00 | valid (cand S) | 12.00 | valid (cand N) |
| `431149292466098373` | 12.00 | valid (cand S) | 2.00 | valid (cand N) |
| `14974509662690242321` | 4.00 | valid (cand S) | 4.00 | valid (cand N) |
| `15928830168549992736` | -12.00 | valid (cand S) | 32.00 | valid (cand N) |
| `4112377384228269587` | 10.00 | valid (cand S) | 6.00 | valid (cand N) |

