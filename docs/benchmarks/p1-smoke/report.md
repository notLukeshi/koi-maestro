# Koi-Koi paired benchmark — example: heuristic vs random

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `7a7729b0ccc2b8cb` |
| referee | `71db9850ee2c19b504183e9b19f0ed9323734a4b` (dirty) |
| baseline | `random` @ `71db9850ee2c19b504183e9b19f0ed9323734a4b` (dirty) |
| candidate | `heuristic` @ `71db9850ee2c19b504183e9b19f0ed9323734a4b` (dirty) |
| stopping | `gsprt_accept_upper` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.8750`  CI99 `[0.7083, 1.0000]`
- **candidate margin**: `9.67`  CI99 `[2.50, 16.83]`
- clusters `12`, legs `24` — wins `21`, draws `0`, losses `3`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 246 | 0.06 | 0.06 | 0.09 | 0.13 | 0.63 | 0 |
| candidate | 285 | 0.07 | 0.06 | 0.10 | 0.16 | 0.78 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `1` | 28.00 | valid (cand S) | 10.00 | valid (cand N) |
| `2` | 10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `3` | 28.00 | valid (cand S) | 28.00 | valid (cand N) |
| `4` | 4.00 | valid (cand S) | 12.00 | valid (cand N) |
| `5` | 10.00 | valid (cand S) | 28.00 | valid (cand N) |
| `6` | 4.00 | valid (cand S) | 28.00 | valid (cand N) |
| `7` | 4.00 | valid (cand S) | 12.00 | valid (cand N) |
| `8` | 4.00 | valid (cand S) | -4.00 | valid (cand N) |
| `9` | 10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `10` | 12.00 | valid (cand S) | -10.00 | valid (cand N) |
| `11` | 12.00 | valid (cand S) | -32.00 | valid (cand N) |
| `12` | 6.00 | valid (cand S) | 8.00 | valid (cand N) |

