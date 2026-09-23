# Koi-Koi paired benchmark — P6 field: 9-entrant definitive tournament: pimc vs leaf_policy

**verdict: `development`** — 23 of 256 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `caa102287f139adf` |
| referee | `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| baseline | `pimc` @ `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| candidate | `leaf_policy` @ `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| stopping | `gsprt_accept_lower` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.4565`  CI99 `[0.2826, 0.6304]`
- **candidate margin**: `-2.74`  CI99 `[-9.13, 5.39]`
- clusters `23`, legs `46` — wins `20`, draws `2`, losses `24`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 545 | 25.65 | 2.24 | 132.52 | 231.88 | 303.90 | 0 |
| candidate | 529 | 27.02 | 36.34 | 90.08 | 123.66 | 310.72 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | -60.00 | valid (cand S) | 28.00 | valid (cand N) |
| `7619234134199958248` | 10.00 | valid (cand S) | -12.00 | valid (cand N) |
| `9318061777837526749` | 4.00 | valid (cand S) | 10.00 | valid (cand N) |
| `6001197866264162459` | -12.00 | valid (cand S) | 10.00 | valid (cand N) |
| `6044635964106635247` | 12.00 | valid (cand S) | -12.00 | valid (cand N) |
| `6903715913044025677` | -40.00 | valid (cand S) | 10.00 | valid (cand N) |
| `10814382527142015852` | 28.00 | valid (cand S) | -2.00 | valid (cand N) |
| `13535137607426657764` | -8.00 | valid (cand S) | -28.00 | valid (cand N) |
| `431149292466098373` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `14974509662690242321` | 0.00 | valid (cand S) | -10.00 | valid (cand N) |
| `15928830168549992736` | 10.00 | valid (cand S) | 2.00 | valid (cand N) |
| `4112377384228269587` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `18000172133288265584` | 4.00 | valid (cand S) | 0.00 | valid (cand N) |
| `17685270382932053218` | 68.00 | valid (cand S) | 28.00 | valid (cand N) |
| `9991098352591498860` | -32.00 | valid (cand S) | -2.00 | valid (cand N) |
| `13872560487504887217` | -28.00 | valid (cand S) | 2.00 | valid (cand N) |
| `5109558618046986729` | 10.00 | valid (cand S) | -28.00 | valid (cand N) |
| `18422795515743521302` | 2.00 | valid (cand S) | -8.00 | valid (cand N) |
| `13075277902204111389` | -4.00 | valid (cand S) | -2.00 | valid (cand N) |
| `15724539146521815599` | 2.00 | valid (cand S) | 2.00 | valid (cand N) |
| `5644602462366436724` | -2.00 | valid (cand S) | -28.00 | valid (cand N) |
| `5285643395559536116` | -32.00 | valid (cand S) | -4.00 | valid (cand N) |
| `17028051217783541004` | -4.00 | valid (cand S) | -10.00 | valid (cand N) |

