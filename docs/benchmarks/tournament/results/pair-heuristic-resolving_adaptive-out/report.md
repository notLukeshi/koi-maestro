# Koi-Koi paired benchmark — P5 field: 7-entrant definitive tournament: heuristic vs resolving_adaptive

**verdict: `development`** — 22 of 256 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `80ebf4e9c2021a8c` |
| referee | `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| baseline | `heuristic` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| candidate | `resolving_adaptive` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| stopping | `gsprt_accept_lower` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.4659`  CI99 `[0.3068, 0.6250]`
- **candidate margin**: `-0.27`  CI99 `[-4.86, 3.91]`
- clusters `22`, legs `44` — wins `20`, draws `1`, losses `23`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 551 | 0.18 | 0.14 | 0.21 | 16.01 | 2.21 | 0 |
| candidate | 542 | 864.14 | 12.54 | 3695.61 | 5347.23 | 10644.59 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | 2.00 | valid (cand S) | 20.00 | valid (cand N) |
| `7619234134199958248` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `9318061777837526749` | 2.00 | valid (cand S) | -4.00 | valid (cand N) |
| `6001197866264162459` | -10.00 | valid (cand S) | 20.00 | valid (cand N) |
| `6044635964106635247` | -10.00 | valid (cand S) | 0.00 | valid (cand N) |
| `6903715913044025677` | 32.00 | valid (cand S) | -12.00 | valid (cand N) |
| `10814382527142015852` | 4.00 | valid (cand S) | -12.00 | valid (cand N) |
| `13535137607426657764` | 2.00 | valid (cand S) | -2.00 | valid (cand N) |
| `431149292466098373` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `14974509662690242321` | -10.00 | valid (cand S) | 28.00 | valid (cand N) |
| `15928830168549992736` | -28.00 | valid (cand S) | -10.00 | valid (cand N) |
| `4112377384228269587` | -8.00 | valid (cand S) | -4.00 | valid (cand N) |
| `18000172133288265584` | -28.00 | valid (cand S) | 36.00 | valid (cand N) |
| `17685270382932053218` | 4.00 | valid (cand S) | 20.00 | valid (cand N) |
| `9991098352591498860` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `13872560487504887217` | -10.00 | valid (cand S) | -12.00 | valid (cand N) |
| `5109558618046986729` | 12.00 | valid (cand S) | 4.00 | valid (cand N) |
| `18422795515743521302` | -10.00 | valid (cand S) | 20.00 | valid (cand N) |
| `13075277902204111389` | -6.00 | valid (cand S) | 2.00 | valid (cand N) |
| `15724539146521815599` | -28.00 | valid (cand S) | 10.00 | valid (cand N) |
| `5644602462366436724` | -28.00 | valid (cand S) | 32.00 | valid (cand N) |
| `5285643395559536116` | -10.00 | valid (cand S) | -20.00 | valid (cand N) |

