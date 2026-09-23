# Koi-Koi paired benchmark — P5 field: 7-entrant definitive tournament: random vs ismcts

**verdict: `development`** — 31 of 256 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `e48c0d96e4c7730e` |
| referee | `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| baseline | `random` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| candidate | `ismcts` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| stopping | `gsprt_accept_upper` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.6452`  CI99 `[0.5161, 0.7742]`
- **candidate margin**: `8.13`  CI99 `[2.23, 15.68]`
- clusters `31`, legs `62` — wins `40`, draws `0`, losses `22`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 789 | 0.69 | 0.14 | 0.21 | 50.26 | 8.77 | 0 |
| candidate | 827 | 48.40 | 27.41 | 152.56 | 1127.11 | 645.59 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | -10.00 | valid (cand S) | 40.00 | valid (cand N) |
| `7619234134199958248` | 28.00 | valid (cand S) | 2.00 | valid (cand N) |
| `9318061777837526749` | 4.00 | valid (cand S) | 4.00 | valid (cand N) |
| `6001197866264162459` | 2.00 | valid (cand S) | 2.00 | valid (cand N) |
| `6044635964106635247` | 48.00 | valid (cand S) | -2.00 | valid (cand N) |
| `6903715913044025677` | 44.00 | valid (cand S) | -2.00 | valid (cand N) |
| `10814382527142015852` | 10.00 | valid (cand S) | -4.00 | valid (cand N) |
| `13535137607426657764` | 10.00 | valid (cand S) | -6.00 | valid (cand N) |
| `431149292466098373` | 2.00 | valid (cand S) | -2.00 | valid (cand N) |
| `14974509662690242321` | -4.00 | valid (cand S) | 2.00 | valid (cand N) |
| `15928830168549992736` | 68.00 | valid (cand S) | -28.00 | valid (cand N) |
| `4112377384228269587` | -4.00 | valid (cand S) | 4.00 | valid (cand N) |
| `18000172133288265584` | -4.00 | valid (cand S) | -12.00 | valid (cand N) |
| `17685270382932053218` | 4.00 | valid (cand S) | 10.00 | valid (cand N) |
| `9991098352591498860` | 10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `13872560487504887217` | 56.00 | valid (cand S) | 72.00 | valid (cand N) |
| `5109558618046986729` | -4.00 | valid (cand S) | 4.00 | valid (cand N) |
| `18422795515743521302` | 32.00 | valid (cand S) | 4.00 | valid (cand N) |
| `13075277902204111389` | -2.00 | valid (cand S) | 4.00 | valid (cand N) |
| `15724539146521815599` | 4.00 | valid (cand S) | -12.00 | valid (cand N) |
| `5644602462366436724` | 76.00 | valid (cand S) | -10.00 | valid (cand N) |
| `5285643395559536116` | -4.00 | valid (cand S) | 4.00 | valid (cand N) |
| `17028051217783541004` | -8.00 | valid (cand S) | -2.00 | valid (cand N) |
| `11245501377626014972` | 28.00 | valid (cand S) | 4.00 | valid (cand N) |
| `4707493349660855478` | 2.00 | valid (cand S) | 2.00 | valid (cand N) |
| `89351670316142122` | -10.00 | valid (cand S) | 12.00 | valid (cand N) |
| `3625370522854471219` | -2.00 | valid (cand S) | 12.00 | valid (cand N) |
| `5331815729948895851` | 2.00 | valid (cand S) | -36.00 | valid (cand N) |
| `7901749859059848878` | 4.00 | valid (cand S) | 10.00 | valid (cand N) |
| `2193579961990494438` | -2.00 | valid (cand S) | 4.00 | valid (cand N) |
| `7877213756195515994` | 6.00 | valid (cand S) | 28.00 | valid (cand N) |

