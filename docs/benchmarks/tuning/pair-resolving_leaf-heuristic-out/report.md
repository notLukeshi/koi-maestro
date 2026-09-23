# Koi-Koi paired benchmark — A5 tuning: resolving_leaf vs heuristic

**verdict: `development`** — 16 of 16 declared clusters played
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `1308db13514046b4` |
| referee | `988fb19db2be124a8cdb0e801bc2fdfe803d6b7c` (dirty) |
| baseline | `resolving_leaf` @ `988fb19db2be124a8cdb0e801bc2fdfe803d6b7c` (dirty) |
| candidate | `heuristic` @ `988fb19db2be124a8cdb0e801bc2fdfe803d6b7c` (dirty) |
| stopping | `gsprt_cap_reached` |

## Result

- **candidate win score**: `0.5938`  CI99 `[0.4062, 0.7656]`
- **candidate margin**: `-1.06`  CI99 `[-7.38, 4.25]`
- clusters `16`, legs `32` — wins `18`, draws `2`, losses `12`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 312 | 1490.45 | 50.81 | 5605.37 | 8659.42 | 14531.84 | 0 |
| candidate | 327 | 0.16 | 0.14 | 0.20 | 2.58 | 1.66 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `180000` | -10.00 | valid (cand S) | -4.00 | valid (cand N) |
| `180001` | -48.00 | valid (cand S) | 48.00 | valid (cand N) |
| `180002` | 12.00 | valid (cand S) | -12.00 | valid (cand N) |
| `180003` | -76.00 | valid (cand S) | 28.00 | valid (cand N) |
| `180004` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `180005` | 28.00 | valid (cand S) | 2.00 | valid (cand N) |
| `180006` | 0.00 | valid (cand S) | 10.00 | valid (cand N) |
| `180007` | 10.00 | valid (cand S) | -32.00 | valid (cand N) |
| `180008` | 4.00 | valid (cand S) | 4.00 | valid (cand N) |
| `180009` | -8.00 | valid (cand S) | 10.00 | valid (cand N) |
| `180010` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `180011` | -28.00 | valid (cand S) | 0.00 | valid (cand N) |
| `180012` | -6.00 | valid (cand S) | 4.00 | valid (cand N) |
| `180013` | 2.00 | valid (cand S) | 12.00 | valid (cand N) |
| `180014` | 10.00 | valid (cand S) | 4.00 | valid (cand N) |
| `180015` | 12.00 | valid (cand S) | -10.00 | valid (cand N) |

