# Koi-Koi paired benchmark — P6 field: 9-entrant definitive tournament: random vs leaf_policy

**verdict: `development`** — 17 of 256 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `c1d953b0942a22cd` |
| referee | `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| baseline | `random` @ `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| candidate | `leaf_policy` @ `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| stopping | `gsprt_accept_upper` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.7353`  CI99 `[0.5294, 0.9118]`
- **candidate margin**: `5.24`  CI99 `[-2.41, 11.94]`
- clusters `17`, legs `34` — wins `25`, draws `0`, losses `9`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 416 | 0.22 | 0.15 | 0.23 | 16.28 | 2.74 | 0 |
| candidate | 447 | 26.76 | 35.66 | 94.36 | 131.23 | 351.84 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | 6.00 | valid (cand S) | 2.00 | valid (cand N) |
| `7619234134199958248` | -4.00 | valid (cand S) | 2.00 | valid (cand N) |
| `9318061777837526749` | 2.00 | valid (cand S) | 4.00 | valid (cand N) |
| `6001197866264162459` | 12.00 | valid (cand S) | -2.00 | valid (cand N) |
| `6044635964106635247` | 44.00 | valid (cand S) | -10.00 | valid (cand N) |
| `6903715913044025677` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `10814382527142015852` | 4.00 | valid (cand S) | 2.00 | valid (cand N) |
| `13535137607426657764` | 20.00 | valid (cand S) | 10.00 | valid (cand N) |
| `431149292466098373` | 2.00 | valid (cand S) | 10.00 | valid (cand N) |
| `14974509662690242321` | 10.00 | valid (cand S) | 8.00 | valid (cand N) |
| `15928830168549992736` | -4.00 | valid (cand S) | -48.00 | valid (cand N) |
| `4112377384228269587` | 60.00 | valid (cand S) | -12.00 | valid (cand N) |
| `18000172133288265584` | 28.00 | valid (cand S) | 2.00 | valid (cand N) |
| `17685270382932053218` | 6.00 | valid (cand S) | 10.00 | valid (cand N) |
| `9991098352591498860` | -4.00 | valid (cand S) | 36.00 | valid (cand N) |
| `13872560487504887217` | -32.00 | valid (cand S) | 2.00 | valid (cand N) |
| `5109558618046986729` | 2.00 | valid (cand S) | 10.00 | valid (cand N) |

