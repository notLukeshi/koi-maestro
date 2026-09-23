# Koi-Koi paired benchmark — P6 probe: resolving_leaf 8w/40000n/60cfr/d4 vs heuristic

**verdict: `development`** — 6 of 6 declared clusters played
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `53e3369aaf566a23` |
| referee | `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| baseline | `heuristic` @ `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| candidate | `resolving_leaf` @ `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| stopping | `gsprt_cap_reached` |

## Result

- **candidate win score**: `0.5417`  CI99 `[0.2083, 0.8333]`
- **candidate margin**: `-1.17`  CI99 `[-16.33, 10.00]`
- clusters `6`, legs `12` — wins `6`, draws `1`, losses `5`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 144 | 0.21 | 0.14 | 0.20 | 10.28 | 2.52 | 0 |
| candidate | 147 | 723.49 | 153.37 | 3563.33 | 6868.11 | 8862.79 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `180200` | 12.00 | valid (cand S) | 2.00 | valid (cand N) |
| `180201` | -10.00 | valid (cand S) | 40.00 | valid (cand N) |
| `180202` | 2.00 | valid (cand S) | -4.00 | valid (cand N) |
| `180203` | -12.00 | valid (cand S) | -40.00 | valid (cand N) |
| `180204` | 2.00 | valid (cand S) | -8.00 | valid (cand N) |
| `180205` | 2.00 | valid (cand S) | 0.00 | valid (cand N) |

