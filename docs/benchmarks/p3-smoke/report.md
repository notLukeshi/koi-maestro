# Koi-Koi paired benchmark — P3 smoke: resolving_adaptive vs banker

**verdict: `canonical`** — 4 of 4 declared clusters played

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `7ac0ea6a51fdb692` |
| referee | `504251ef8de5694217ed354c9875b09bba880a71` |
| baseline | `banker` @ `504251ef8de5694217ed354c9875b09bba880a71` |
| candidate | `resolving_adaptive` @ `504251ef8de5694217ed354c9875b09bba880a71` |
| stopping | `gsprt_cap_reached` |

## Result

- **candidate win score**: `0.3750`  CI99 `[0.1250, 0.5000]`
- **candidate margin**: `0.75`  CI99 `[-1.50, 3.25]`
- clusters `4`, legs `8` — wins `3`, draws `0`, losses `5`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 93 | 0.13 | 0.13 | 0.18 | 0.20 | 1.53 | 0 |
| candidate | 89 | 424.46 | 4.89 | 2017.18 | 2245.00 | 4722.07 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `1` | 28.00 | valid (cand S) | -28.00 | valid (cand N) |
| `2` | 4.00 | valid (cand S) | -2.00 | valid (cand N) |
| `3` | -2.00 | valid (cand S) | -2.00 | valid (cand N) |
| `4` | 10.00 | valid (cand S) | -2.00 | valid (cand N) |

