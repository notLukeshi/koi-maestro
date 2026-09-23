# Koi-Koi paired benchmark — P5 field: 7-entrant definitive tournament: ismcts vs endgame

**verdict: `development`** — 49 of 256 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `9000cc924712d2c6` |
| referee | `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| baseline | `ismcts` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| candidate | `endgame` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| stopping | `gsprt_accept_upper` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.5969`  CI99 `[0.5153, 0.6837]`
- **candidate margin**: `1.37`  CI99 `[-1.00, 3.90]`
- clusters `49`, legs `98` — wins `57`, draws `3`, losses `38`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 943 | 56.00 | 26.00 | 184.51 | 989.34 | 538.84 | 0 |
| candidate | 974 | 0.53 | 0.14 | 0.21 | 76.32 | 5.26 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `7619234134199958248` | -4.00 | valid (cand S) | 40.00 | valid (cand N) |
| `9318061777837526749` | 2.00 | valid (cand S) | 10.00 | valid (cand N) |
| `6001197866264162459` | -2.00 | valid (cand S) | 2.00 | valid (cand N) |
| `6044635964106635247` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `6903715913044025677` | 2.00 | valid (cand S) | 2.00 | valid (cand N) |
| `10814382527142015852` | 10.00 | valid (cand S) | -2.00 | valid (cand N) |
| `13535137607426657764` | 12.00 | valid (cand S) | -10.00 | valid (cand N) |
| `431149292466098373` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `14974509662690242321` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `15928830168549992736` | 2.00 | valid (cand S) | 10.00 | valid (cand N) |
| `4112377384228269587` | -6.00 | valid (cand S) | -10.00 | valid (cand N) |
| `18000172133288265584` | -12.00 | valid (cand S) | 10.00 | valid (cand N) |
| `17685270382932053218` | 2.00 | valid (cand S) | -2.00 | valid (cand N) |
| `9991098352591498860` | -12.00 | valid (cand S) | 12.00 | valid (cand N) |
| `13872560487504887217` | 2.00 | valid (cand S) | 10.00 | valid (cand N) |
| `5109558618046986729` | 10.00 | valid (cand S) | -40.00 | valid (cand N) |
| `18422795515743521302` | -28.00 | valid (cand S) | 12.00 | valid (cand N) |
| `13075277902204111389` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `15724539146521815599` | 2.00 | valid (cand S) | 10.00 | valid (cand N) |
| `5644602462366436724` | -2.00 | valid (cand S) | 2.00 | valid (cand N) |
| `5285643395559536116` | 28.00 | valid (cand S) | -2.00 | valid (cand N) |
| `17028051217783541004` | 12.00 | valid (cand S) | 2.00 | valid (cand N) |
| `11245501377626014972` | -10.00 | valid (cand S) | 0.00 | valid (cand N) |
| `4707493349660855478` | 2.00 | valid (cand S) | -10.00 | valid (cand N) |
| `89351670316142122` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `3625370522854471219` | 6.00 | valid (cand S) | -6.00 | valid (cand N) |
| `5331815729948895851` | 6.00 | valid (cand S) | 2.00 | valid (cand N) |
| `7901749859059848878` | -12.00 | valid (cand S) | 12.00 | valid (cand N) |
| `2193579961990494438` | -40.00 | valid (cand S) | 10.00 | valid (cand N) |
| `7877213756195515994` | -12.00 | valid (cand S) | 10.00 | valid (cand N) |
| `17231885404968630994` | 0.00 | valid (cand S) | -2.00 | valid (cand N) |
| `3724930574856835696` | -12.00 | valid (cand S) | 10.00 | valid (cand N) |
| `15994427755875030898` | 10.00 | valid (cand S) | -2.00 | valid (cand N) |
| `757423280274912588` | 10.00 | valid (cand S) | -12.00 | valid (cand N) |
| `1757862071514044645` | -12.00 | valid (cand S) | 12.00 | valid (cand N) |
| `3502288721485664850` | 4.00 | valid (cand S) | -2.00 | valid (cand N) |
| `1974005245704069748` | -2.00 | valid (cand S) | 2.00 | valid (cand N) |
| `11117729078908733636` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `1697213996681928136` | -4.00 | valid (cand S) | 4.00 | valid (cand N) |
| `5862763182494943542` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `13207253463605622503` | 12.00 | valid (cand S) | 12.00 | valid (cand N) |
| `5604342051154158021` | 10.00 | valid (cand S) | -12.00 | valid (cand N) |
| `13508708212012915326` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `12230826148952381943` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `3394161117189566975` | 0.00 | valid (cand S) | 2.00 | valid (cand N) |
| `14452421214146983403` | 12.00 | valid (cand S) | 36.00 | valid (cand N) |
| `16659526999119862915` | 2.00 | valid (cand S) | 10.00 | valid (cand N) |
| `5476173071870707070` | 4.00 | valid (cand S) | 10.00 | valid (cand N) |

