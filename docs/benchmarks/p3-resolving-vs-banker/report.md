# Koi-Koi paired benchmark — P3: resolving_adaptive vs banker

**verdict: `canonical`** — 19 of 64 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `4b6f9032efcb304f` |
| referee | `504251ef8de5694217ed354c9875b09bba880a71` |
| baseline | `banker` @ `504251ef8de5694217ed354c9875b09bba880a71` |
| candidate | `resolving_adaptive` @ `504251ef8de5694217ed354c9875b09bba880a71` |
| stopping | `gsprt_accept_lower` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.4868`  CI99 `[0.3421, 0.6316]`
- **candidate margin**: `-0.42`  CI99 `[-6.63, 4.68]`
- clusters `19`, legs `38` — wins `18`, draws `1`, losses `19`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 362 | 0.20 | 0.13 | 0.19 | 11.94 | 1.91 | 0 |
| candidate | 360 | 423.00 | 6.62 | 1831.34 | 2594.84 | 4007.32 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `1` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `2` | -10.00 | valid (cand S) | 2.00 | valid (cand N) |
| `3` | -12.00 | valid (cand S) | 2.00 | valid (cand N) |
| `4` | 10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `5` | -2.00 | valid (cand S) | 28.00 | valid (cand N) |
| `6` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `7` | -10.00 | valid (cand S) | 28.00 | valid (cand N) |
| `8` | 2.00 | valid (cand S) | -10.00 | valid (cand N) |
| `9` | -28.00 | valid (cand S) | -2.00 | valid (cand N) |
| `10` | -56.00 | valid (cand S) | -2.00 | valid (cand N) |
| `11` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `12` | -2.00 | valid (cand S) | 2.00 | valid (cand N) |
| `13` | 2.00 | valid (cand S) | -2.00 | valid (cand N) |
| `14` | 2.00 | valid (cand S) | 28.00 | valid (cand N) |
| `15` | 2.00 | valid (cand S) | -2.00 | valid (cand N) |
| `16` | -2.00 | valid (cand S) | 10.00 | valid (cand N) |
| `17` | 12.00 | valid (cand S) | -12.00 | valid (cand N) |
| `18` | 0.00 | valid (cand S) | -4.00 | valid (cand N) |
| `19` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |

