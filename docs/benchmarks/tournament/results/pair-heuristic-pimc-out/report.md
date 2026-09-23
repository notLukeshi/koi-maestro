# Koi-Koi paired benchmark — P5 field: 7-entrant definitive tournament: heuristic vs pimc

**verdict: `development`** — 52 of 256 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `e332b4d7d1970bf9` |
| referee | `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| baseline | `heuristic` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| candidate | `pimc` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| stopping | `gsprt_accept_lower` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.5288`  CI99 `[0.4279, 0.6250]`
- **candidate margin**: `2.35`  CI99 `[-1.17, 6.13]`
- clusters `52`, legs `104` — wins `54`, draws `2`, losses `48`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 1159 | 0.46 | 0.14 | 0.21 | 50.09 | 5.10 | 0 |
| candidate | 1167 | 86.46 | 22.96 | 317.43 | 889.74 | 970.20 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | -8.00 | valid (cand S) | -20.00 | valid (cand N) |
| `7619234134199958248` | 44.00 | valid (cand S) | -10.00 | valid (cand N) |
| `9318061777837526749` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `6001197866264162459` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `6044635964106635247` | -6.00 | valid (cand S) | -4.00 | valid (cand N) |
| `6903715913044025677` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `10814382527142015852` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `13535137607426657764` | 10.00 | valid (cand S) | -28.00 | valid (cand N) |
| `431149292466098373` | 12.00 | valid (cand S) | 8.00 | valid (cand N) |
| `14974509662690242321` | -10.00 | valid (cand S) | 12.00 | valid (cand N) |
| `15928830168549992736` | 2.00 | valid (cand S) | 28.00 | valid (cand N) |
| `4112377384228269587` | 56.00 | valid (cand S) | 2.00 | valid (cand N) |
| `18000172133288265584` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `17685270382932053218` | 8.00 | valid (cand S) | 8.00 | valid (cand N) |
| `9991098352591498860` | 48.00 | valid (cand S) | -12.00 | valid (cand N) |
| `13872560487504887217` | 10.00 | valid (cand S) | -12.00 | valid (cand N) |
| `5109558618046986729` | 28.00 | valid (cand S) | -10.00 | valid (cand N) |
| `18422795515743521302` | -40.00 | valid (cand S) | -6.00 | valid (cand N) |
| `13075277902204111389` | 32.00 | valid (cand S) | -10.00 | valid (cand N) |
| `15724539146521815599` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `5644602462366436724` | -4.00 | valid (cand S) | 6.00 | valid (cand N) |
| `5285643395559536116` | 2.00 | valid (cand S) | 4.00 | valid (cand N) |
| `17028051217783541004` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `11245501377626014972` | -20.00 | valid (cand S) | -6.00 | valid (cand N) |
| `4707493349660855478` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `89351670316142122` | 28.00 | valid (cand S) | -12.00 | valid (cand N) |
| `3625370522854471219` | 10.00 | valid (cand S) | 4.00 | valid (cand N) |
| `5331815729948895851` | 10.00 | valid (cand S) | 4.00 | valid (cand N) |
| `7901749859059848878` | -12.00 | valid (cand S) | 10.00 | valid (cand N) |
| `2193579961990494438` | 10.00 | valid (cand S) | -6.00 | valid (cand N) |
| `7877213756195515994` | 10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `17231885404968630994` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `3724930574856835696` | 2.00 | valid (cand S) | 4.00 | valid (cand N) |
| `15994427755875030898` | -4.00 | valid (cand S) | 40.00 | valid (cand N) |
| `757423280274912588` | -10.00 | valid (cand S) | 12.00 | valid (cand N) |
| `1757862071514044645` | -4.00 | valid (cand S) | 0.00 | valid (cand N) |
| `3502288721485664850` | -12.00 | valid (cand S) | 12.00 | valid (cand N) |
| `1974005245704069748` | 12.00 | valid (cand S) | -12.00 | valid (cand N) |
| `11117729078908733636` | 0.00 | valid (cand S) | -10.00 | valid (cand N) |
| `1697213996681928136` | 2.00 | valid (cand S) | 28.00 | valid (cand N) |
| `5862763182494943542` | -10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `13207253463605622503` | 2.00 | valid (cand S) | -2.00 | valid (cand N) |
| `5604342051154158021` | 80.00 | valid (cand S) | -10.00 | valid (cand N) |
| `13508708212012915326` | 12.00 | valid (cand S) | -12.00 | valid (cand N) |
| `12230826148952381943` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `3394161117189566975` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `14452421214146983403` | 28.00 | valid (cand S) | -28.00 | valid (cand N) |
| `16659526999119862915` | -4.00 | valid (cand S) | 6.00 | valid (cand N) |
| `5476173071870707070` | 20.00 | valid (cand S) | -10.00 | valid (cand N) |
| `7449148081597852909` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `10633510334691990785` | 12.00 | valid (cand S) | -56.00 | valid (cand N) |
| `14882551271329702063` | -10.00 | valid (cand S) | -4.00 | valid (cand N) |

