# Koi-Koi paired benchmark — A5 tuning: resolving_leaf vs resolving_adaptive

**verdict: `development`** — 16 of 16 declared clusters played
- 4 of 32 legs are forfeits carrying synthetic margins (±256), not played evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `671d1329b4828a44` |
| referee | `988fb19db2be124a8cdb0e801bc2fdfe803d6b7c` (dirty) |
| baseline | `resolving_leaf` @ `988fb19db2be124a8cdb0e801bc2fdfe803d6b7c` (dirty) |
| candidate | `resolving_adaptive` @ `988fb19db2be124a8cdb0e801bc2fdfe803d6b7c` (dirty) |
| stopping | `gsprt_cap_reached` |

## Result

- **candidate win score**: `0.5000`  CI99 `[0.3125, 0.6875]`
- **candidate margin**: `31.69`  CI99 `[-2.94, 74.12]`
- clusters `16`, legs `32` — wins `16`, draws `0`, losses `16`, invalid `0`, time-forfeits `4`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 375 | 2094.68 | 249.07 | 8058.30 | 14438.53 | 24547.06 | 4 |
| candidate | 364 | 376.89 | 0.53 | 1549.40 | 2019.18 | 4287.13 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `180000` | 48.00 | valid (cand S) | -10.00 | valid (cand N) |
| `180001` | -2.00 | valid (cand S) | 4.00 | valid (cand N) |
| `180002` | -6.00 | valid (cand S) | 12.00 | valid (cand N) |
| `180003` | 44.00 | valid (cand S) | 256.00 | time-forfeit: Baseline |
| `180004` | -2.00 | valid (cand S) | 256.00 | time-forfeit: Baseline |
| `180005` | 10.00 | valid (cand S) | -32.00 | valid (cand N) |
| `180006` | -2.00 | valid (cand S) | -40.00 | valid (cand N) |
| `180007` | -2.00 | valid (cand S) | -10.00 | valid (cand N) |
| `180008` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `180009` | 256.00 | time-forfeit: Baseline | 10.00 | valid (cand N) |
| `180010` | -2.00 | valid (cand S) | 2.00 | valid (cand N) |
| `180011` | -10.00 | valid (cand S) | 12.00 | valid (cand N) |
| `180012` | -2.00 | valid (cand S) | 4.00 | valid (cand N) |
| `180013` | 6.00 | valid (cand S) | -10.00 | valid (cand N) |
| `180014` | -4.00 | valid (cand S) | -32.00 | valid (cand N) |
| `180015` | 4.00 | valid (cand S) | 256.00 | time-forfeit: Baseline |

