# Koi-Koi paired benchmark — P6 field: 9-entrant definitive tournament: heuristic vs leaf_policy

**verdict: `development`** — 27 of 256 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `8a3d86222e3df0ed` |
| referee | `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| baseline | `heuristic` @ `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| candidate | `leaf_policy` @ `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| stopping | `gsprt_accept_lower` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.5093`  CI99 `[0.3796, 0.6389]`
- **candidate margin**: `-1.22`  CI99 `[-5.74, 3.00]`
- clusters `27`, legs `54` — wins `27`, draws `1`, losses `26`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 579 | 0.22 | 0.14 | 0.21 | 16.22 | 2.40 | 0 |
| candidate | 570 | 34.08 | 41.65 | 107.21 | 333.79 | 359.75 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `7619234134199958248` | 2.00 | valid (cand S) | 28.00 | valid (cand N) |
| `9318061777837526749` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `6001197866264162459` | -12.00 | valid (cand S) | 20.00 | valid (cand N) |
| `6044635964106635247` | -28.00 | valid (cand S) | 4.00 | valid (cand N) |
| `6903715913044025677` | -52.00 | valid (cand S) | -4.00 | valid (cand N) |
| `10814382527142015852` | -12.00 | valid (cand S) | 10.00 | valid (cand N) |
| `13535137607426657764` | 10.00 | valid (cand S) | -32.00 | valid (cand N) |
| `431149292466098373` | -4.00 | valid (cand S) | 2.00 | valid (cand N) |
| `14974509662690242321` | -8.00 | valid (cand S) | 2.00 | valid (cand N) |
| `15928830168549992736` | 2.00 | valid (cand S) | 10.00 | valid (cand N) |
| `4112377384228269587` | 12.00 | valid (cand S) | -28.00 | valid (cand N) |
| `18000172133288265584` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `17685270382932053218` | 2.00 | valid (cand S) | 2.00 | valid (cand N) |
| `9991098352591498860` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `13872560487504887217` | 0.00 | valid (cand S) | -4.00 | valid (cand N) |
| `5109558618046986729` | -8.00 | valid (cand S) | -10.00 | valid (cand N) |
| `18422795515743521302` | 2.00 | valid (cand S) | 28.00 | valid (cand N) |
| `13075277902204111389` | -6.00 | valid (cand S) | 2.00 | valid (cand N) |
| `15724539146521815599` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `5644602462366436724` | 28.00 | valid (cand S) | -28.00 | valid (cand N) |
| `5285643395559536116` | -10.00 | valid (cand S) | 20.00 | valid (cand N) |
| `17028051217783541004` | -10.00 | valid (cand S) | 44.00 | valid (cand N) |
| `11245501377626014972` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `4707493349660855478` | 2.00 | valid (cand S) | -10.00 | valid (cand N) |
| `89351670316142122` | -10.00 | valid (cand S) | -6.00 | valid (cand N) |
| `3625370522854471219` | -28.00 | valid (cand S) | 12.00 | valid (cand N) |

