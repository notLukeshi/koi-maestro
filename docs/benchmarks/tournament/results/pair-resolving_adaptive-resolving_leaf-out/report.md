# Koi-Koi paired benchmark — P6 field: 9-entrant definitive tournament: resolving_adaptive vs resolving_leaf

**verdict: `development`** — 20 of 256 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `d3ccc84185d703c8` |
| referee | `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| baseline | `resolving_adaptive` @ `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| candidate | `resolving_leaf` @ `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| stopping | `gsprt_accept_lower` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.5000`  CI99 `[0.3750, 0.6250]`
- **candidate margin**: `0.20`  CI99 `[-5.90, 6.45]`
- clusters `20`, legs `40` — wins `20`, draws `0`, losses `20`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 439 | 419.89 | 24.79 | 1680.43 | 2813.84 | 4608.33 | 0 |
| candidate | 434 | 1020.62 | 199.54 | 4729.19 | 10333.50 | 11073.72 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | 56.00 | valid (cand S) | -6.00 | valid (cand N) |
| `7619234134199958248` | -12.00 | valid (cand S) | 10.00 | valid (cand N) |
| `9318061777837526749` | 32.00 | valid (cand S) | 2.00 | valid (cand N) |
| `6001197866264162459` | 12.00 | valid (cand S) | -10.00 | valid (cand N) |
| `6044635964106635247` | -4.00 | valid (cand S) | -12.00 | valid (cand N) |
| `6903715913044025677` | -10.00 | valid (cand S) | -2.00 | valid (cand N) |
| `10814382527142015852` | -4.00 | valid (cand S) | 2.00 | valid (cand N) |
| `13535137607426657764` | -2.00 | valid (cand S) | 2.00 | valid (cand N) |
| `431149292466098373` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `14974509662690242321` | -12.00 | valid (cand S) | 10.00 | valid (cand N) |
| `15928830168549992736` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `4112377384228269587` | 10.00 | valid (cand S) | -2.00 | valid (cand N) |
| `18000172133288265584` | -48.00 | valid (cand S) | 32.00 | valid (cand N) |
| `17685270382932053218` | 10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `9991098352591498860` | 2.00 | valid (cand S) | -32.00 | valid (cand N) |
| `13872560487504887217` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `5109558618046986729` | -6.00 | valid (cand S) | 10.00 | valid (cand N) |
| `18422795515743521302` | 28.00 | valid (cand S) | -28.00 | valid (cand N) |
| `13075277902204111389` | -60.00 | valid (cand S) | 10.00 | valid (cand N) |
| `15724539146521815599` | -12.00 | valid (cand S) | 32.00 | valid (cand N) |

