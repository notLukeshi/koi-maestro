# Koi-Koi paired benchmark — P3: resolving_adaptive vs heuristic

**verdict: `canonical`** — 23 of 64 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `465ab7f4d9cb1cc4` |
| referee | `504251ef8de5694217ed354c9875b09bba880a71` |
| baseline | `heuristic` @ `504251ef8de5694217ed354c9875b09bba880a71` |
| candidate | `resolving_adaptive` @ `504251ef8de5694217ed354c9875b09bba880a71` |
| stopping | `gsprt_accept_lower` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.4891`  CI99 `[0.3370, 0.6413]`
- **candidate margin**: `0.78`  CI99 `[-4.61, 6.91]`
- clusters `23`, legs `46` — wins `21`, draws `3`, losses `22`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 534 | 0.16 | 0.13 | 0.19 | 11.11 | 1.87 | 0 |
| candidate | 522 | 380.95 | 5.82 | 1603.25 | 2563.01 | 4322.96 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `1` | 2.00 | valid (cand S) | -2.00 | valid (cand N) |
| `2` | 10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `3` | -8.00 | valid (cand S) | -4.00 | valid (cand N) |
| `4` | -10.00 | valid (cand S) | 0.00 | valid (cand N) |
| `5` | -10.00 | valid (cand S) | 80.00 | valid (cand N) |
| `6` | 20.00 | valid (cand S) | 2.00 | valid (cand N) |
| `7` | -12.00 | valid (cand S) | 28.00 | valid (cand N) |
| `8` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `9` | 10.00 | valid (cand S) | -40.00 | valid (cand N) |
| `10` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `11` | 36.00 | valid (cand S) | -28.00 | valid (cand N) |
| `12` | 0.00 | valid (cand S) | 2.00 | valid (cand N) |
| `13` | -28.00 | valid (cand S) | 10.00 | valid (cand N) |
| `14` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `15` | 2.00 | valid (cand S) | 12.00 | valid (cand N) |
| `16` | -32.00 | valid (cand S) | -10.00 | valid (cand N) |
| `17` | 4.00 | valid (cand S) | -4.00 | valid (cand N) |
| `18` | -2.00 | valid (cand S) | -2.00 | valid (cand N) |
| `19` | -10.00 | valid (cand S) | 24.00 | valid (cand N) |
| `20` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `21` | 28.00 | valid (cand S) | -32.00 | valid (cand N) |
| `22` | 2.00 | valid (cand S) | 0.00 | valid (cand N) |
| `23` | -8.00 | valid (cand S) | -4.00 | valid (cand N) |

