# Koi-Koi paired benchmark — P6-A1 smoke: leaf_policy vs heuristic

**verdict: `development`** — 8 of 8 declared clusters played
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `1f1b2dfbf02d61b4` |
| referee | `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| baseline | `leaf_policy` @ `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| candidate | `heuristic` @ `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| stopping | `gsprt_cap_reached` |

## Result

- **candidate win score**: `0.5312`  CI99 `[0.5000, 0.6250]`
- **candidate margin**: `2.38`  CI99 `[-5.88, 13.62]`
- clusters `8`, legs `16` — wins `8`, draws `1`, losses `7`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 206 | 17.52 | 0.28 | 68.25 | 107.82 | 225.52 | 0 |
| candidate | 209 | 0.35 | 0.14 | 0.21 | 24.72 | 4.54 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `180016` | 4.00 | valid (cand S) | -2.00 | valid (cand N) |
| `180017` | -40.00 | valid (cand S) | 12.00 | valid (cand N) |
| `180018` | 0.00 | valid (cand S) | 2.00 | valid (cand N) |
| `180019` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `180020` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `180021` | 40.00 | valid (cand S) | -32.00 | valid (cand N) |
| `180022` | 56.00 | valid (cand S) | -2.00 | valid (cand N) |
| `180023` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |

