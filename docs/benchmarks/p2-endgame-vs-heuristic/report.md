# Koi-Koi paired benchmark — P2: endgame vs heuristic

**verdict: `canonical`** — 24 of 64 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `e7d8f243f8db4491` |
| referee | `8974354461db7d3ef521b17ebb3c0390c34ca346` |
| baseline | `heuristic` @ `8974354461db7d3ef521b17ebb3c0390c34ca346` |
| candidate | `endgame` @ `8974354461db7d3ef521b17ebb3c0390c34ca346` |
| stopping | `gsprt_accept_upper` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.6042`  CI99 `[0.5208, 0.7083]`
- **candidate margin**: `0.50`  CI99 `[-2.17, 3.04]`
- clusters `24`, legs `48` — wins `29`, draws `0`, losses `19`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 513 | 0.14 | 0.13 | 0.21 | 0.28 | 1.49 | 0 |
| candidate | 523 | 0.14 | 0.13 | 0.21 | 0.35 | 1.52 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `1` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `2` | -6.00 | valid (cand S) | 2.00 | valid (cand N) |
| `3` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `4` | 28.00 | valid (cand S) | -28.00 | valid (cand N) |
| `5` | 6.00 | valid (cand S) | 20.00 | valid (cand N) |
| `6` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `7` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `8` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `9` | 4.00 | valid (cand S) | 2.00 | valid (cand N) |
| `10` | 4.00 | valid (cand S) | 2.00 | valid (cand N) |
| `11` | 2.00 | valid (cand S) | -4.00 | valid (cand N) |
| `12` | -12.00 | valid (cand S) | 12.00 | valid (cand N) |
| `13` | -6.00 | valid (cand S) | 2.00 | valid (cand N) |
| `14` | -32.00 | valid (cand S) | 2.00 | valid (cand N) |
| `15` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `16` | 12.00 | valid (cand S) | -12.00 | valid (cand N) |
| `17` | -4.00 | valid (cand S) | 2.00 | valid (cand N) |
| `18` | 2.00 | valid (cand S) | 4.00 | valid (cand N) |
| `19` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `20` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `21` | 12.00 | valid (cand S) | -12.00 | valid (cand N) |
| `22` | 40.00 | valid (cand S) | -40.00 | valid (cand N) |
| `23` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `24` | 20.00 | valid (cand S) | 2.00 | valid (cand N) |

