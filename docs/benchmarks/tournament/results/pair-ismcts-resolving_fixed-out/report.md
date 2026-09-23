# Koi-Koi paired benchmark — P5 field: 7-entrant definitive tournament: ismcts vs resolving_fixed

**verdict: `development`** — 77 of 256 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `25f696f28fac06a6` |
| referee | `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| baseline | `ismcts` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| candidate | `resolving_fixed` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| stopping | `gsprt_accept_upper` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.5909`  CI99 `[0.5130, 0.6688]`
- **candidate margin**: `3.74`  CI99 `[0.44, 7.04]`
- clusters `77`, legs `154` — wins `89`, draws `4`, losses `61`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 1842 | 78.60 | 33.20 | 485.05 | 1210.94 | 940.19 | 0 |
| candidate | 1918 | 857.28 | 21.32 | 3614.00 | 6329.03 | 10677.02 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | -2.00 | valid (cand S) | -28.00 | valid (cand N) |
| `7619234134199958248` | 28.00 | valid (cand S) | -2.00 | valid (cand N) |
| `9318061777837526749` | -40.00 | valid (cand S) | 40.00 | valid (cand N) |
| `6001197866264162459` | -40.00 | valid (cand S) | 56.00 | valid (cand N) |
| `6044635964106635247` | -2.00 | valid (cand S) | 2.00 | valid (cand N) |
| `6903715913044025677` | -4.00 | valid (cand S) | 4.00 | valid (cand N) |
| `10814382527142015852` | 44.00 | valid (cand S) | 4.00 | valid (cand N) |
| `13535137607426657764` | 6.00 | valid (cand S) | 4.00 | valid (cand N) |
| `431149292466098373` | 2.00 | valid (cand S) | -2.00 | valid (cand N) |
| `14974509662690242321` | 28.00 | valid (cand S) | 6.00 | valid (cand N) |
| `15928830168549992736` | 12.00 | valid (cand S) | 8.00 | valid (cand N) |
| `4112377384228269587` | -24.00 | valid (cand S) | 12.00 | valid (cand N) |
| `18000172133288265584` | -10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `17685270382932053218` | 12.00 | valid (cand S) | 68.00 | valid (cand N) |
| `9991098352591498860` | -10.00 | valid (cand S) | 12.00 | valid (cand N) |
| `13872560487504887217` | 10.00 | valid (cand S) | 28.00 | valid (cand N) |
| `5109558618046986729` | 32.00 | valid (cand S) | -12.00 | valid (cand N) |
| `18422795515743521302` | 12.00 | valid (cand S) | -28.00 | valid (cand N) |
| `13075277902204111389` | 36.00 | valid (cand S) | -28.00 | valid (cand N) |
| `15724539146521815599` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `5644602462366436724` | -8.00 | valid (cand S) | 4.00 | valid (cand N) |
| `5285643395559536116` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `17028051217783541004` | -10.00 | valid (cand S) | 2.00 | valid (cand N) |
| `11245501377626014972` | 2.00 | valid (cand S) | 2.00 | valid (cand N) |
| `4707493349660855478` | 2.00 | valid (cand S) | 2.00 | valid (cand N) |
| `89351670316142122` | 4.00 | valid (cand S) | -6.00 | valid (cand N) |
| `3625370522854471219` | 0.00 | valid (cand S) | -2.00 | valid (cand N) |
| `5331815729948895851` | 2.00 | valid (cand S) | -2.00 | valid (cand N) |
| `7901749859059848878` | -2.00 | valid (cand S) | 44.00 | valid (cand N) |
| `2193579961990494438` | 32.00 | valid (cand S) | -36.00 | valid (cand N) |
| `7877213756195515994` | -32.00 | valid (cand S) | 32.00 | valid (cand N) |
| `17231885404968630994` | 12.00 | valid (cand S) | -12.00 | valid (cand N) |
| `3724930574856835696` | 28.00 | valid (cand S) | -32.00 | valid (cand N) |
| `15994427755875030898` | 2.00 | valid (cand S) | 4.00 | valid (cand N) |
| `757423280274912588` | 40.00 | valid (cand S) | -40.00 | valid (cand N) |
| `1757862071514044645` | -10.00 | valid (cand S) | 40.00 | valid (cand N) |
| `3502288721485664850` | 44.00 | valid (cand S) | -28.00 | valid (cand N) |
| `1974005245704069748` | 80.00 | valid (cand S) | -60.00 | valid (cand N) |
| `11117729078908733636` | -40.00 | valid (cand S) | 28.00 | valid (cand N) |
| `1697213996681928136` | -10.00 | valid (cand S) | 52.00 | valid (cand N) |
| `5862763182494943542` | 6.00 | valid (cand S) | -2.00 | valid (cand N) |
| `13207253463605622503` | 28.00 | valid (cand S) | -2.00 | valid (cand N) |
| `5604342051154158021` | -32.00 | valid (cand S) | -2.00 | valid (cand N) |
| `13508708212012915326` | -10.00 | valid (cand S) | 12.00 | valid (cand N) |
| `12230826148952381943` | 12.00 | valid (cand S) | 40.00 | valid (cand N) |
| `3394161117189566975` | 12.00 | valid (cand S) | -48.00 | valid (cand N) |
| `14452421214146983403` | 28.00 | valid (cand S) | -10.00 | valid (cand N) |
| `16659526999119862915` | 6.00 | valid (cand S) | 4.00 | valid (cand N) |
| `5476173071870707070` | 2.00 | valid (cand S) | 28.00 | valid (cand N) |
| `7449148081597852909` | 10.00 | valid (cand S) | 12.00 | valid (cand N) |
| `10633510334691990785` | -28.00 | valid (cand S) | -4.00 | valid (cand N) |
| `14882551271329702063` | 28.00 | valid (cand S) | -2.00 | valid (cand N) |
| `5435282970375642844` | -48.00 | valid (cand S) | 36.00 | valid (cand N) |
| `4909978446978078675` | -12.00 | valid (cand S) | 4.00 | valid (cand N) |
| `13543147368718070832` | 4.00 | valid (cand S) | 12.00 | valid (cand N) |
| `12003341261269753726` | 2.00 | valid (cand S) | 20.00 | valid (cand N) |
| `16435730818243151335` | 4.00 | valid (cand S) | -2.00 | valid (cand N) |
| `7427841346095480495` | -2.00 | valid (cand S) | -10.00 | valid (cand N) |
| `8479304945038862688` | -52.00 | valid (cand S) | 28.00 | valid (cand N) |
| `6391033478745877943` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `6222777361425726683` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `4792741220661876131` | 32.00 | valid (cand S) | 0.00 | valid (cand N) |
| `3581263945264044395` | -2.00 | valid (cand S) | 2.00 | valid (cand N) |
| `16662341577870142413` | 84.00 | valid (cand S) | -40.00 | valid (cand N) |
| `4586865259456751513` | 6.00 | valid (cand S) | 52.00 | valid (cand N) |
| `8562230381632726911` | 4.00 | valid (cand S) | -10.00 | valid (cand N) |
| `10463810097117964814` | 10.00 | valid (cand S) | -52.00 | valid (cand N) |
| `16332039622979753095` | -2.00 | valid (cand S) | 6.00 | valid (cand N) |
| `9502358135458596456` | 10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `5417497814226228437` | -10.00 | valid (cand S) | 28.00 | valid (cand N) |
| `14788341470457634628` | -2.00 | valid (cand S) | 6.00 | valid (cand N) |
| `1229059045408138764` | 12.00 | valid (cand S) | -12.00 | valid (cand N) |
| `4408363188443756323` | 0.00 | valid (cand S) | 12.00 | valid (cand N) |
| `9285861254477473652` | -40.00 | valid (cand S) | 10.00 | valid (cand N) |
| `16462925917828128099` | 6.00 | valid (cand S) | -20.00 | valid (cand N) |
| `5620467037882493181` | 44.00 | valid (cand S) | 4.00 | valid (cand N) |
| `18156393412990643621` | 4.00 | valid (cand S) | 0.00 | valid (cand N) |

