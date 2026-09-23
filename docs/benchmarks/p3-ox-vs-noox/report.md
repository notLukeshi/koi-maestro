# Koi-Koi paired benchmark — P3 ablation: resolving_ox vs resolving (identical CRN panel)

**verdict: `canonical`** — 9 of 64 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `0f9a44dc44aec75b` |
| referee | `504251ef8de5694217ed354c9875b09bba880a71` |
| baseline | `resolving_no_ox` @ `504251ef8de5694217ed354c9875b09bba880a71` |
| candidate | `resolving_ox` @ `504251ef8de5694217ed354c9875b09bba880a71` |
| stopping | `gsprt_accept_lower` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.5000`  CI99 `[0.5000, 0.5000]`
- **candidate margin**: `0.11`  CI99 `[0.00, 0.44]`
- clusters `9`, legs `18` — wins `9`, draws `0`, losses `9`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 143 | 435.27 | 13.52 | 1475.47 | 2238.19 | 3457.97 | 0 |
| candidate | 144 | 438.18 | 8.12 | 1702.56 | 2574.83 | 3505.43 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `1` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `2` | 40.00 | valid (cand S) | -40.00 | valid (cand N) |
| `3` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `4` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `5` | -4.00 | valid (cand S) | 6.00 | valid (cand N) |
| `6` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `7` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `8` | 40.00 | valid (cand S) | -40.00 | valid (cand N) |
| `9` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |

