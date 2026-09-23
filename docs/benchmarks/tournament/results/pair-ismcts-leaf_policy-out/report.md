# Koi-Koi paired benchmark — P6 field: 9-entrant definitive tournament: ismcts vs leaf_policy

**verdict: `development`** — 73 of 256 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `cb6401d0556aa3b8` |
| referee | `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| baseline | `ismcts` @ `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| candidate | `leaf_policy` @ `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| stopping | `gsprt_accept_upper` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.5890`  CI99 `[0.5205, 0.6610]`
- **candidate margin**: `-1.48`  CI99 `[-4.16, 1.11]`
- clusters `73`, legs `146` — wins `83`, draws `6`, losses `57`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 1682 | 18.97 | 12.31 | 76.62 | 209.87 | 218.51 | 0 |
| candidate | 1722 | 30.42 | 40.59 | 102.99 | 336.93 | 358.77 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | -4.00 | valid (cand S) | 2.00 | valid (cand N) |
| `7619234134199958248` | 10.00 | valid (cand S) | -28.00 | valid (cand N) |
| `9318061777837526749` | -2.00 | valid (cand S) | 2.00 | valid (cand N) |
| `6001197866264162459` | 4.00 | valid (cand S) | -4.00 | valid (cand N) |
| `6044635964106635247` | 10.00 | valid (cand S) | 28.00 | valid (cand N) |
| `6903715913044025677` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `10814382527142015852` | 2.00 | valid (cand S) | -4.00 | valid (cand N) |
| `13535137607426657764` | 4.00 | valid (cand S) | -4.00 | valid (cand N) |
| `431149292466098373` | -60.00 | valid (cand S) | 10.00 | valid (cand N) |
| `14974509662690242321` | 4.00 | valid (cand S) | 0.00 | valid (cand N) |
| `15928830168549992736` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `4112377384228269587` | 28.00 | valid (cand S) | -44.00 | valid (cand N) |
| `18000172133288265584` | 10.00 | valid (cand S) | -32.00 | valid (cand N) |
| `17685270382932053218` | 6.00 | valid (cand S) | -28.00 | valid (cand N) |
| `9991098352591498860` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `13872560487504887217` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `5109558618046986729` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `18422795515743521302` | 4.00 | valid (cand S) | 10.00 | valid (cand N) |
| `13075277902204111389` | 0.00 | valid (cand S) | 2.00 | valid (cand N) |
| `15724539146521815599` | -2.00 | valid (cand S) | 10.00 | valid (cand N) |
| `5644602462366436724` | 2.00 | valid (cand S) | -6.00 | valid (cand N) |
| `5285643395559536116` | 28.00 | valid (cand S) | -2.00 | valid (cand N) |
| `17028051217783541004` | 2.00 | valid (cand S) | -36.00 | valid (cand N) |
| `11245501377626014972` | -2.00 | valid (cand S) | 2.00 | valid (cand N) |
| `4707493349660855478` | 4.00 | valid (cand S) | 10.00 | valid (cand N) |
| `89351670316142122` | -40.00 | valid (cand S) | 10.00 | valid (cand N) |
| `3625370522854471219` | 2.00 | valid (cand S) | -2.00 | valid (cand N) |
| `5331815729948895851` | 0.00 | valid (cand S) | -10.00 | valid (cand N) |
| `7901749859059848878` | 28.00 | valid (cand S) | -48.00 | valid (cand N) |
| `2193579961990494438` | -32.00 | valid (cand S) | 2.00 | valid (cand N) |
| `7877213756195515994` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `17231885404968630994` | 4.00 | valid (cand S) | -4.00 | valid (cand N) |
| `3724930574856835696` | 2.00 | valid (cand S) | 2.00 | valid (cand N) |
| `15994427755875030898` | 2.00 | valid (cand S) | -28.00 | valid (cand N) |
| `757423280274912588` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `1757862071514044645` | 2.00 | valid (cand S) | 0.00 | valid (cand N) |
| `3502288721485664850` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `1974005245704069748` | -28.00 | valid (cand S) | 10.00 | valid (cand N) |
| `11117729078908733636` | -36.00 | valid (cand S) | -2.00 | valid (cand N) |
| `1697213996681928136` | 28.00 | valid (cand S) | 2.00 | valid (cand N) |
| `5862763182494943542` | 2.00 | valid (cand S) | 28.00 | valid (cand N) |
| `13207253463605622503` | -44.00 | valid (cand S) | 40.00 | valid (cand N) |
| `5604342051154158021` | 6.00 | valid (cand S) | -28.00 | valid (cand N) |
| `13508708212012915326` | -4.00 | valid (cand S) | -2.00 | valid (cand N) |
| `12230826148952381943` | 2.00 | valid (cand S) | -10.00 | valid (cand N) |
| `3394161117189566975` | 2.00 | valid (cand S) | 10.00 | valid (cand N) |
| `14452421214146983403` | -2.00 | valid (cand S) | 2.00 | valid (cand N) |
| `16659526999119862915` | -4.00 | valid (cand S) | 2.00 | valid (cand N) |
| `5476173071870707070` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `7449148081597852909` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `10633510334691990785` | 12.00 | valid (cand S) | -6.00 | valid (cand N) |
| `14882551271329702063` | 12.00 | valid (cand S) | -36.00 | valid (cand N) |
| `5435282970375642844` | 2.00 | valid (cand S) | -2.00 | valid (cand N) |
| `4909978446978078675` | 2.00 | valid (cand S) | 10.00 | valid (cand N) |
| `13543147368718070832` | 2.00 | valid (cand S) | -52.00 | valid (cand N) |
| `12003341261269753726` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `16435730818243151335` | 10.00 | valid (cand S) | 2.00 | valid (cand N) |
| `7427841346095480495` | 4.00 | valid (cand S) | -12.00 | valid (cand N) |
| `8479304945038862688` | 32.00 | valid (cand S) | -2.00 | valid (cand N) |
| `6391033478745877943` | 4.00 | valid (cand S) | 2.00 | valid (cand N) |
| `6222777361425726683` | 10.00 | valid (cand S) | -2.00 | valid (cand N) |
| `4792741220661876131` | 2.00 | valid (cand S) | 10.00 | valid (cand N) |
| `3581263945264044395` | 0.00 | valid (cand S) | 28.00 | valid (cand N) |
| `16662341577870142413` | 36.00 | valid (cand S) | -40.00 | valid (cand N) |
| `4586865259456751513` | -10.00 | valid (cand S) | -28.00 | valid (cand N) |
| `8562230381632726911` | 2.00 | valid (cand S) | 2.00 | valid (cand N) |
| `10463810097117964814` | 12.00 | valid (cand S) | -36.00 | valid (cand N) |
| `16332039622979753095` | 2.00 | valid (cand S) | -12.00 | valid (cand N) |
| `9502358135458596456` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `5417497814226228437` | 4.00 | valid (cand S) | 4.00 | valid (cand N) |
| `14788341470457634628` | 10.00 | valid (cand S) | 0.00 | valid (cand N) |
| `1229059045408138764` | 4.00 | valid (cand S) | -2.00 | valid (cand N) |
| `4408363188443756323` | 2.00 | valid (cand S) | 2.00 | valid (cand N) |

