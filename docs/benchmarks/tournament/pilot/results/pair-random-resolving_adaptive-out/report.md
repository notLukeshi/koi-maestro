# Koi-Koi paired benchmark — P5 pilot: sizing + lane measurement: random vs resolving_adaptive

**verdict: `development`** — 16 of 16 declared clusters played
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `6e45df1be28d94a5` |
| referee | `3e0067c95474606c29745b8e2ecc9e1ee6ce8462` (dirty) |
| baseline | `random` @ `3e0067c95474606c29745b8e2ecc9e1ee6ce8462` (dirty) |
| candidate | `resolving_adaptive` @ `3e0067c95474606c29745b8e2ecc9e1ee6ce8462` (dirty) |
| stopping | `gsprt_cap_reached` |

## Result

- **candidate win score**: `0.6875`  CI99 `[0.5000, 0.8750]`
- **candidate margin**: `11.31`  CI99 `[4.12, 18.25]`
- clusters `16`, legs `32` — wins `22`, draws `0`, losses `10`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 403 | 0.26 | 0.14 | 0.21 | 26.12 | 3.34 | 0 |
| candidate | 440 | 697.76 | 16.39 | 3163.58 | 4241.30 | 9594.18 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `16034480500904598573` | 28.00 | valid (cand S) | 28.00 | valid (cand N) |
| `4677710776332959296` | 10.00 | valid (cand S) | -12.00 | valid (cand N) |
| `13271961482927859836` | 4.00 | valid (cand S) | 4.00 | valid (cand N) |
| `10529852546119080842` | 10.00 | valid (cand S) | 4.00 | valid (cand N) |
| `14671832180405412331` | 52.00 | valid (cand S) | -10.00 | valid (cand N) |
| `17017001798808896126` | -4.00 | valid (cand S) | 8.00 | valid (cand N) |
| `18145539812079460165` | -12.00 | valid (cand S) | 28.00 | valid (cand N) |
| `49272188150520080` | 52.00 | valid (cand S) | 10.00 | valid (cand N) |
| `4902487414315408253` | -4.00 | valid (cand S) | 36.00 | valid (cand N) |
| `14075327324793344651` | 32.00 | valid (cand S) | -2.00 | valid (cand N) |
| `18351434641185641655` | -16.00 | valid (cand S) | -10.00 | valid (cand N) |
| `7575956051109723050` | -2.00 | valid (cand S) | 4.00 | valid (cand N) |
| `1745845176427152899` | 28.00 | valid (cand S) | -10.00 | valid (cand N) |
| `10356463945147717012` | 2.00 | valid (cand S) | 40.00 | valid (cand N) |
| `10703843830651834799` | 12.00 | valid (cand S) | 8.00 | valid (cand N) |
| `15561891488122254539` | 32.00 | valid (cand S) | 12.00 | valid (cand N) |

