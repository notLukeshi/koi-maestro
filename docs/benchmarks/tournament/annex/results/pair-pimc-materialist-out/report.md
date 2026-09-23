# Koi-Koi paired benchmark — annex: pimc vs materialist

**verdict: `development`** — 20 of 256 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `63544d90e19be822` |
| referee | `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| baseline | `pimc` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| candidate | `materialist` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| stopping | `gsprt_accept_lower` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.4625`  CI99 `[0.3000, 0.6375]`
- **candidate margin**: `1.70`  CI99 `[-2.65, 7.95]`
- clusters `20`, legs `40` — wins `17`, draws `3`, losses `20`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 490 | 91.65 | 19.72 | 320.42 | 727.16 | 1122.72 | 0 |
| candidate | 487 | 0.18 | 0.14 | 0.20 | 19.84 | 2.24 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | 10.00 | valid (cand S) | -4.00 | valid (cand N) |
| `7619234134199958248` | -8.00 | valid (cand S) | 28.00 | valid (cand N) |
| `9318061777837526749` | 0.00 | valid (cand S) | 2.00 | valid (cand N) |
| `6001197866264162459` | -4.00 | valid (cand S) | 2.00 | valid (cand N) |
| `6044635964106635247` | 28.00 | valid (cand S) | -2.00 | valid (cand N) |
| `6903715913044025677` | -12.00 | valid (cand S) | 12.00 | valid (cand N) |
| `10814382527142015852` | -4.00 | valid (cand S) | -4.00 | valid (cand N) |
| `13535137607426657764` | 56.00 | valid (cand S) | 12.00 | valid (cand N) |
| `431149292466098373` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `14974509662690242321` | -4.00 | valid (cand S) | 4.00 | valid (cand N) |
| `15928830168549992736` | 0.00 | valid (cand S) | -10.00 | valid (cand N) |
| `4112377384228269587` | -20.00 | valid (cand S) | 10.00 | valid (cand N) |
| `18000172133288265584` | 0.00 | valid (cand S) | 4.00 | valid (cand N) |
| `17685270382932053218` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `9991098352591498860` | 6.00 | valid (cand S) | 10.00 | valid (cand N) |
| `13872560487504887217` | -10.00 | valid (cand S) | -2.00 | valid (cand N) |
| `5109558618046986729` | -2.00 | valid (cand S) | -10.00 | valid (cand N) |
| `18422795515743521302` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `13075277902204111389` | 6.00 | valid (cand S) | -4.00 | valid (cand N) |
| `15724539146521815599` | -20.00 | valid (cand S) | -2.00 | valid (cand N) |

