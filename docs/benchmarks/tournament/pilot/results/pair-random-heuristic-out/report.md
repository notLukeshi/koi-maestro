# Koi-Koi paired benchmark — P5 pilot: sizing + lane measurement: random vs heuristic

**verdict: `development`** — 16 of 16 declared clusters played
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `9508cdc23cd5be20` |
| referee | `3e0067c95474606c29745b8e2ecc9e1ee6ce8462` (dirty) |
| baseline | `random` @ `3e0067c95474606c29745b8e2ecc9e1ee6ce8462` (dirty) |
| candidate | `heuristic` @ `3e0067c95474606c29745b8e2ecc9e1ee6ce8462` (dirty) |
| stopping | `gsprt_cap_reached` |

## Result

- **candidate win score**: `0.6875`  CI99 `[0.4844, 0.8438]`
- **candidate margin**: `7.25`  CI99 `[-0.88, 14.94]`
- clusters `16`, legs `32` — wins `20`, draws `4`, losses `8`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 386 | 0.12 | 0.11 | 0.18 | 0.27 | 1.45 | 0 |
| candidate | 420 | 0.18 | 0.11 | 0.19 | 22.17 | 2.32 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `16034480500904598573` | -10.00 | valid (cand S) | -28.00 | valid (cand N) |
| `4677710776332959296` | 24.00 | valid (cand S) | 10.00 | valid (cand N) |
| `13271961482927859836` | 10.00 | valid (cand S) | 0.00 | valid (cand N) |
| `10529852546119080842` | -4.00 | valid (cand S) | 10.00 | valid (cand N) |
| `14671832180405412331` | -6.00 | valid (cand S) | 4.00 | valid (cand N) |
| `17017001798808896126` | 10.00 | valid (cand S) | 0.00 | valid (cand N) |
| `18145539812079460165` | 4.00 | valid (cand S) | 28.00 | valid (cand N) |
| `49272188150520080` | 8.00 | valid (cand S) | -8.00 | valid (cand N) |
| `4902487414315408253` | 6.00 | valid (cand S) | 28.00 | valid (cand N) |
| `14075327324793344651` | 28.00 | valid (cand S) | 32.00 | valid (cand N) |
| `18351434641185641655` | 6.00 | valid (cand S) | -8.00 | valid (cand N) |
| `7575956051109723050` | 0.00 | valid (cand S) | 32.00 | valid (cand N) |
| `1745845176427152899` | 4.00 | valid (cand S) | -2.00 | valid (cand N) |
| `10356463945147717012` | 4.00 | valid (cand S) | 0.00 | valid (cand N) |
| `10703843830651834799` | 48.00 | valid (cand S) | 10.00 | valid (cand N) |
| `15561891488122254539` | 12.00 | valid (cand S) | -20.00 | valid (cand N) |

