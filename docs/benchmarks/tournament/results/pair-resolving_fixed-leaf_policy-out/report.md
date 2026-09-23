# Koi-Koi paired benchmark — P6 field: 9-entrant definitive tournament: resolving_fixed vs leaf_policy

**verdict: `development`** — 30 of 256 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `bfdce211c839c2e5` |
| referee | `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| baseline | `resolving_fixed` @ `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| candidate | `leaf_policy` @ `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| stopping | `gsprt_accept_lower` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.5083`  CI99 `[0.3833, 0.6333]`
- **candidate margin**: `-6.13`  CI99 `[-11.30, -1.57]`
- clusters `30`, legs `60` — wins `30`, draws `1`, losses `29`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 674 | 445.67 | 12.08 | 1799.58 | 2709.37 | 5006.32 | 0 |
| candidate | 666 | 31.10 | 39.07 | 103.26 | 155.61 | 345.21 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `7619234134199958248` | 10.00 | valid (cand S) | 2.00 | valid (cand N) |
| `9318061777837526749` | 2.00 | valid (cand S) | -64.00 | valid (cand N) |
| `6001197866264162459` | -10.00 | valid (cand S) | -28.00 | valid (cand N) |
| `6044635964106635247` | 10.00 | valid (cand S) | -32.00 | valid (cand N) |
| `6903715913044025677` | 2.00 | valid (cand S) | 2.00 | valid (cand N) |
| `10814382527142015852` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `13535137607426657764` | 10.00 | valid (cand S) | 0.00 | valid (cand N) |
| `431149292466098373` | -28.00 | valid (cand S) | 2.00 | valid (cand N) |
| `14974509662690242321` | 2.00 | valid (cand S) | -8.00 | valid (cand N) |
| `15928830168549992736` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `4112377384228269587` | 2.00 | valid (cand S) | 2.00 | valid (cand N) |
| `18000172133288265584` | -2.00 | valid (cand S) | -10.00 | valid (cand N) |
| `17685270382932053218` | -6.00 | valid (cand S) | 2.00 | valid (cand N) |
| `9991098352591498860` | -2.00 | valid (cand S) | 2.00 | valid (cand N) |
| `13872560487504887217` | 2.00 | valid (cand S) | -4.00 | valid (cand N) |
| `5109558618046986729` | -40.00 | valid (cand S) | 10.00 | valid (cand N) |
| `18422795515743521302` | -6.00 | valid (cand S) | 10.00 | valid (cand N) |
| `13075277902204111389` | 10.00 | valid (cand S) | -48.00 | valid (cand N) |
| `15724539146521815599` | 10.00 | valid (cand S) | 2.00 | valid (cand N) |
| `5644602462366436724` | -4.00 | valid (cand S) | 12.00 | valid (cand N) |
| `5285643395559536116` | 2.00 | valid (cand S) | -2.00 | valid (cand N) |
| `17028051217783541004` | 2.00 | valid (cand S) | -2.00 | valid (cand N) |
| `11245501377626014972` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `4707493349660855478` | 6.00 | valid (cand S) | -36.00 | valid (cand N) |
| `89351670316142122` | -32.00 | valid (cand S) | -10.00 | valid (cand N) |
| `3625370522854471219` | -28.00 | valid (cand S) | 2.00 | valid (cand N) |
| `5331815729948895851` | 10.00 | valid (cand S) | -28.00 | valid (cand N) |
| `7901749859059848878` | 2.00 | valid (cand S) | -2.00 | valid (cand N) |
| `2193579961990494438` | -10.00 | valid (cand S) | -56.00 | valid (cand N) |

