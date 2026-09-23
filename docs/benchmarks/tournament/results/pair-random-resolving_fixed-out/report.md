# Koi-Koi paired benchmark — P5 field: 7-entrant definitive tournament: random vs resolving_fixed

**verdict: `development`** — 18 of 256 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `e51285ea91a7dee5` |
| referee | `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| baseline | `random` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| candidate | `resolving_fixed` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| stopping | `gsprt_accept_upper` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.7083`  CI99 `[0.5278, 0.8750]`
- **candidate margin**: `10.06`  CI99 `[1.94, 18.33]`
- clusters `18`, legs `36` — wins `25`, draws `1`, losses `10`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 445 | 0.27 | 0.14 | 0.21 | 30.08 | 3.37 | 0 |
| candidate | 483 | 873.01 | 28.34 | 3722.20 | 5407.41 | 11712.91 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | -4.00 | valid (cand S) | 6.00 | valid (cand N) |
| `7619234134199958248` | 28.00 | valid (cand S) | 32.00 | valid (cand N) |
| `9318061777837526749` | 28.00 | valid (cand S) | -2.00 | valid (cand N) |
| `6001197866264162459` | 4.00 | valid (cand S) | 4.00 | valid (cand N) |
| `6044635964106635247` | -2.00 | valid (cand S) | -28.00 | valid (cand N) |
| `6903715913044025677` | 2.00 | valid (cand S) | -4.00 | valid (cand N) |
| `10814382527142015852` | 28.00 | valid (cand S) | 10.00 | valid (cand N) |
| `13535137607426657764` | -12.00 | valid (cand S) | 64.00 | valid (cand N) |
| `431149292466098373` | 12.00 | valid (cand S) | 10.00 | valid (cand N) |
| `14974509662690242321` | 4.00 | valid (cand S) | -2.00 | valid (cand N) |
| `15928830168549992736` | 0.00 | valid (cand S) | 10.00 | valid (cand N) |
| `4112377384228269587` | 28.00 | valid (cand S) | 12.00 | valid (cand N) |
| `18000172133288265584` | 4.00 | valid (cand S) | -2.00 | valid (cand N) |
| `17685270382932053218` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `9991098352591498860` | 12.00 | valid (cand S) | 10.00 | valid (cand N) |
| `13872560487504887217` | 12.00 | valid (cand S) | -28.00 | valid (cand N) |
| `5109558618046986729` | 12.00 | valid (cand S) | 60.00 | valid (cand N) |
| `18422795515743521302` | 48.00 | valid (cand S) | 6.00 | valid (cand N) |

