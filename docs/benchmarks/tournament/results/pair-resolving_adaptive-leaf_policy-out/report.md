# Koi-Koi paired benchmark — P6 field: 9-entrant definitive tournament: resolving_adaptive vs leaf_policy

**verdict: `development`** — 42 of 256 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `262aa880745e2b6a` |
| referee | `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| baseline | `resolving_adaptive` @ `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| candidate | `leaf_policy` @ `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| stopping | `gsprt_accept_lower` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.5238`  CI99 `[0.4107, 0.6369]`
- **candidate margin**: `-3.52`  CI99 `[-7.88, 0.98]`
- clusters `42`, legs `84` — wins `42`, draws `4`, losses `38`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 978 | 424.50 | 12.22 | 1773.46 | 2677.70 | 4942.39 | 0 |
| candidate | 972 | 30.25 | 38.86 | 100.17 | 150.87 | 350.03 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | 10.00 | valid (cand S) | -60.00 | valid (cand N) |
| `7619234134199958248` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `9318061777837526749` | 8.00 | valid (cand S) | 10.00 | valid (cand N) |
| `6001197866264162459` | 12.00 | valid (cand S) | -12.00 | valid (cand N) |
| `6044635964106635247` | 6.00 | valid (cand S) | -6.00 | valid (cand N) |
| `6903715913044025677` | 10.00 | valid (cand S) | -40.00 | valid (cand N) |
| `10814382527142015852` | 28.00 | valid (cand S) | -44.00 | valid (cand N) |
| `13535137607426657764` | 2.00 | valid (cand S) | -32.00 | valid (cand N) |
| `431149292466098373` | 4.00 | valid (cand S) | 0.00 | valid (cand N) |
| `14974509662690242321` | 10.00 | valid (cand S) | -4.00 | valid (cand N) |
| `15928830168549992736` | 2.00 | valid (cand S) | -4.00 | valid (cand N) |
| `4112377384228269587` | 12.00 | valid (cand S) | -12.00 | valid (cand N) |
| `18000172133288265584` | 0.00 | valid (cand S) | -4.00 | valid (cand N) |
| `17685270382932053218` | 56.00 | valid (cand S) | 2.00 | valid (cand N) |
| `9991098352591498860` | 0.00 | valid (cand S) | -12.00 | valid (cand N) |
| `13872560487504887217` | 2.00 | valid (cand S) | -2.00 | valid (cand N) |
| `5109558618046986729` | 2.00 | valid (cand S) | 28.00 | valid (cand N) |
| `18422795515743521302` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `13075277902204111389` | 10.00 | valid (cand S) | 2.00 | valid (cand N) |
| `15724539146521815599` | -10.00 | valid (cand S) | 2.00 | valid (cand N) |
| `5644602462366436724` | -12.00 | valid (cand S) | -28.00 | valid (cand N) |
| `5285643395559536116` | 2.00 | valid (cand S) | -2.00 | valid (cand N) |
| `17028051217783541004` | 12.00 | valid (cand S) | -10.00 | valid (cand N) |
| `11245501377626014972` | 2.00 | valid (cand S) | 10.00 | valid (cand N) |
| `4707493349660855478` | 2.00 | valid (cand S) | -4.00 | valid (cand N) |
| `89351670316142122` | -2.00 | valid (cand S) | -28.00 | valid (cand N) |
| `3625370522854471219` | 4.00 | valid (cand S) | -8.00 | valid (cand N) |
| `5331815729948895851` | 10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `7901749859059848878` | 12.00 | valid (cand S) | -32.00 | valid (cand N) |
| `2193579961990494438` | 10.00 | valid (cand S) | -32.00 | valid (cand N) |
| `7877213756195515994` | -10.00 | valid (cand S) | -12.00 | valid (cand N) |
| `17231885404968630994` | 28.00 | valid (cand S) | -36.00 | valid (cand N) |
| `3724930574856835696` | 10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `15994427755875030898` | 2.00 | valid (cand S) | -32.00 | valid (cand N) |
| `757423280274912588` | 10.00 | valid (cand S) | -12.00 | valid (cand N) |
| `1757862071514044645` | -4.00 | valid (cand S) | 0.00 | valid (cand N) |
| `3502288721485664850` | 10.00 | valid (cand S) | -72.00 | valid (cand N) |
| `1974005245704069748` | 2.00 | valid (cand S) | 10.00 | valid (cand N) |
| `11117729078908733636` | -12.00 | valid (cand S) | 10.00 | valid (cand N) |
| `1697213996681928136` | 4.00 | valid (cand S) | -32.00 | valid (cand N) |
| `5862763182494943542` | -10.00 | valid (cand S) | -32.00 | valid (cand N) |
| `13207253463605622503` | -8.00 | valid (cand S) | -12.00 | valid (cand N) |

