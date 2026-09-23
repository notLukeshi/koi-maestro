# Koi-Koi paired benchmark — P6 field: 9-entrant definitive tournament: heuristic vs resolving_leaf

**verdict: `development`** — 42 of 256 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `af27c86237e95069` |
| referee | `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| baseline | `heuristic` @ `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| candidate | `resolving_leaf` @ `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| stopping | `gsprt_accept_lower` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.5298`  CI99 `[0.4286, 0.6310]`
- **candidate margin**: `-0.76`  CI99 `[-3.26, 1.67]`
- clusters `42`, legs `84` — wins `44`, draws `1`, losses `39`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 996 | 0.17 | 0.14 | 0.22 | 13.22 | 2.05 | 0 |
| candidate | 988 | 1044.13 | 194.44 | 5174.32 | 10673.29 | 12280.93 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | -4.00 | valid (cand S) | -4.00 | valid (cand N) |
| `7619234134199958248` | -4.00 | valid (cand S) | 2.00 | valid (cand N) |
| `9318061777837526749` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `6001197866264162459` | 12.00 | valid (cand S) | -10.00 | valid (cand N) |
| `6044635964106635247` | 20.00 | valid (cand S) | 2.00 | valid (cand N) |
| `6903715913044025677` | -10.00 | valid (cand S) | 12.00 | valid (cand N) |
| `10814382527142015852` | 2.00 | valid (cand S) | 2.00 | valid (cand N) |
| `13535137607426657764` | 10.00 | valid (cand S) | -28.00 | valid (cand N) |
| `431149292466098373` | 2.00 | valid (cand S) | -4.00 | valid (cand N) |
| `14974509662690242321` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `15928830168549992736` | -4.00 | valid (cand S) | 2.00 | valid (cand N) |
| `4112377384228269587` | -8.00 | valid (cand S) | 6.00 | valid (cand N) |
| `18000172133288265584` | 4.00 | valid (cand S) | 10.00 | valid (cand N) |
| `17685270382932053218` | 2.00 | valid (cand S) | -2.00 | valid (cand N) |
| `9991098352591498860` | 40.00 | valid (cand S) | -10.00 | valid (cand N) |
| `13872560487504887217` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `5109558618046986729` | 2.00 | valid (cand S) | -10.00 | valid (cand N) |
| `18422795515743521302` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `13075277902204111389` | 10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `15724539146521815599` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `5644602462366436724` | -6.00 | valid (cand S) | 4.00 | valid (cand N) |
| `5285643395559536116` | 10.00 | valid (cand S) | 4.00 | valid (cand N) |
| `17028051217783541004` | -8.00 | valid (cand S) | 2.00 | valid (cand N) |
| `11245501377626014972` | 4.00 | valid (cand S) | -4.00 | valid (cand N) |
| `4707493349660855478` | -10.00 | valid (cand S) | 8.00 | valid (cand N) |
| `89351670316142122` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `3625370522854471219` | -8.00 | valid (cand S) | 4.00 | valid (cand N) |
| `5331815729948895851` | 2.00 | valid (cand S) | 2.00 | valid (cand N) |
| `7901749859059848878` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `2193579961990494438` | -12.00 | valid (cand S) | 2.00 | valid (cand N) |
| `7877213756195515994` | 20.00 | valid (cand S) | 2.00 | valid (cand N) |
| `17231885404968630994` | -4.00 | valid (cand S) | -28.00 | valid (cand N) |
| `3724930574856835696` | 0.00 | valid (cand S) | -2.00 | valid (cand N) |
| `15994427755875030898` | -12.00 | valid (cand S) | -10.00 | valid (cand N) |
| `757423280274912588` | 40.00 | valid (cand S) | -40.00 | valid (cand N) |
| `1757862071514044645` | 40.00 | valid (cand S) | -32.00 | valid (cand N) |
| `3502288721485664850` | -4.00 | valid (cand S) | 2.00 | valid (cand N) |
| `1974005245704069748` | -28.00 | valid (cand S) | 4.00 | valid (cand N) |
| `11117729078908733636` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `1697213996681928136` | 4.00 | valid (cand S) | -28.00 | valid (cand N) |
| `5862763182494943542` | 4.00 | valid (cand S) | -8.00 | valid (cand N) |
| `13207253463605622503` | -2.00 | valid (cand S) | -28.00 | valid (cand N) |

