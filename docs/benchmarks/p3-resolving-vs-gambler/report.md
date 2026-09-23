# Koi-Koi paired benchmark — P3: resolving_adaptive vs gambler

**verdict: `canonical`** — 39 of 64 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `114ee2255aa6b593` |
| referee | `504251ef8de5694217ed354c9875b09bba880a71` |
| baseline | `gambler` @ `504251ef8de5694217ed354c9875b09bba880a71` |
| candidate | `resolving_adaptive` @ `504251ef8de5694217ed354c9875b09bba880a71` |
| stopping | `gsprt_accept_upper` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.6218`  CI99 `[0.5000, 0.7436]`
- **candidate margin**: `2.18`  CI99 `[-2.92, 7.41]`
- clusters `39`, legs `78` — wins `48`, draws `1`, losses `29`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 924 | 0.19 | 0.13 | 0.20 | 12.38 | 2.23 | 0 |
| candidate | 937 | 353.83 | 4.77 | 1674.61 | 2469.20 | 4250.55 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `1` | 28.00 | valid (cand S) | 2.00 | valid (cand N) |
| `2` | -4.00 | valid (cand S) | 4.00 | valid (cand N) |
| `3` | 28.00 | valid (cand S) | 12.00 | valid (cand N) |
| `4` | 56.00 | valid (cand S) | 10.00 | valid (cand N) |
| `5` | 60.00 | valid (cand S) | 4.00 | valid (cand N) |
| `6` | -2.00 | valid (cand S) | -4.00 | valid (cand N) |
| `7` | 2.00 | valid (cand S) | -4.00 | valid (cand N) |
| `8` | 4.00 | valid (cand S) | 2.00 | valid (cand N) |
| `9` | 28.00 | valid (cand S) | -56.00 | valid (cand N) |
| `10` | -12.00 | valid (cand S) | 10.00 | valid (cand N) |
| `11` | 10.00 | valid (cand S) | 8.00 | valid (cand N) |
| `12` | 2.00 | valid (cand S) | -2.00 | valid (cand N) |
| `13` | 2.00 | valid (cand S) | 6.00 | valid (cand N) |
| `14` | -6.00 | valid (cand S) | 40.00 | valid (cand N) |
| `15` | 2.00 | valid (cand S) | -2.00 | valid (cand N) |
| `16` | -6.00 | valid (cand S) | -4.00 | valid (cand N) |
| `17` | 8.00 | valid (cand S) | -4.00 | valid (cand N) |
| `18` | 2.00 | valid (cand S) | 8.00 | valid (cand N) |
| `19` | -40.00 | valid (cand S) | 10.00 | valid (cand N) |
| `20` | -28.00 | valid (cand S) | -28.00 | valid (cand N) |
| `21` | 2.00 | valid (cand S) | -4.00 | valid (cand N) |
| `22` | -40.00 | valid (cand S) | 2.00 | valid (cand N) |
| `23` | -28.00 | valid (cand S) | 28.00 | valid (cand N) |
| `24` | -12.00 | valid (cand S) | 10.00 | valid (cand N) |
| `25` | 2.00 | valid (cand S) | 0.00 | valid (cand N) |
| `26` | 4.00 | valid (cand S) | -4.00 | valid (cand N) |
| `27` | -6.00 | valid (cand S) | 12.00 | valid (cand N) |
| `28` | 12.00 | valid (cand S) | -12.00 | valid (cand N) |
| `29` | 40.00 | valid (cand S) | -12.00 | valid (cand N) |
| `30` | 10.00 | valid (cand S) | -28.00 | valid (cand N) |
| `31` | 28.00 | valid (cand S) | 2.00 | valid (cand N) |
| `32` | 10.00 | valid (cand S) | 4.00 | valid (cand N) |
| `33` | -36.00 | valid (cand S) | 40.00 | valid (cand N) |
| `34` | 10.00 | valid (cand S) | -48.00 | valid (cand N) |
| `35` | -44.00 | valid (cand S) | 40.00 | valid (cand N) |
| `36` | 2.00 | valid (cand S) | -4.00 | valid (cand N) |
| `37` | 28.00 | valid (cand S) | -28.00 | valid (cand N) |
| `38` | 20.00 | valid (cand S) | 10.00 | valid (cand N) |
| `39` | 10.00 | valid (cand S) | 4.00 | valid (cand N) |

