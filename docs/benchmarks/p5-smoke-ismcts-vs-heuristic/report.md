# Koi-Koi paired benchmark — P5 smoke: ismcts vs heuristic (post-stable_ln)

**verdict: `development`** — 4 of 4 declared clusters played
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `df7f44748077828e` |
| referee | `3e0067c95474606c29745b8e2ecc9e1ee6ce8462` (dirty) |
| baseline | `heuristic` @ `3e0067c95474606c29745b8e2ecc9e1ee6ce8462` (dirty) |
| candidate | `ismcts` @ `3e0067c95474606c29745b8e2ecc9e1ee6ce8462` (dirty) |
| stopping | `gsprt_cap_reached` |

## Result

- **candidate win score**: `0.0000`  CI99 `[0.0000, 0.0000]`
- **candidate margin**: `-11.25`  CI99 `[-15.00, -7.75]`
- clusters `4`, legs `8` — wins `0`, draws `0`, losses `8`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 90 | 0.14 | 0.13 | 0.22 | 0.26 | 1.53 | 0 |
| candidate | 74 | 32.23 | 14.71 | 129.99 | 162.26 | 298.08 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `180001` | -10.00 | valid (cand S) | -4.00 | valid (cand N) |
| `180002` | -4.00 | valid (cand S) | -28.00 | valid (cand N) |
| `180003` | -10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `180004` | -20.00 | valid (cand S) | -4.00 | valid (cand N) |

