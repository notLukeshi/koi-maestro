# Koi-Koi paired benchmark — P6 probe: resolving_leaf 12w/60000n/80cfr/d4 vs heuristic

**verdict: `development`** — 6 of 6 declared clusters played
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `47b89781bf5a9e87` |
| referee | `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| baseline | `heuristic` @ `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| candidate | `resolving_leaf` @ `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| stopping | `gsprt_cap_reached` |

## Result

- **candidate win score**: `0.5000`  CI99 `[0.5000, 0.5000]`
- **candidate margin**: `-0.33`  CI99 `[-10.33, 9.67]`
- clusters `6`, legs `12` — wins `6`, draws `0`, losses `6`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 100 | 0.14 | 0.14 | 0.19 | 0.21 | 1.16 | 0 |
| candidate | 99 | 1054.50 | 219.07 | 4775.54 | 10160.04 | 8699.66 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `180200` | -10.00 | valid (cand S) | 12.00 | valid (cand N) |
| `180201` | -4.00 | valid (cand S) | 2.00 | valid (cand N) |
| `180202` | 40.00 | valid (cand S) | -10.00 | valid (cand N) |
| `180203` | 12.00 | valid (cand S) | -12.00 | valid (cand N) |
| `180204` | 2.00 | valid (cand S) | -4.00 | valid (cand N) |
| `180205` | 4.00 | valid (cand S) | -36.00 | valid (cand N) |

