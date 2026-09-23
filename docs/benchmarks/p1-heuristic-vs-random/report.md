# Koi-Koi paired benchmark — example: heuristic vs random

**verdict: `canonical`** — 15 of 64 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `7a7729b0ccc2b8cb` |
| referee | `edf15e4b13767ab613f7bb10223b559b93970498` |
| baseline | `random` @ `edf15e4b13767ab613f7bb10223b559b93970498` |
| candidate | `heuristic` @ `edf15e4b13767ab613f7bb10223b559b93970498` |
| stopping | `gsprt_accept_upper` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.6667`  CI99 `[0.5000, 0.8333]`
- **candidate margin**: `4.87`  CI99 `[-2.80, 13.00]`
- clusters `15`, legs `30` — wins `19`, draws `2`, losses `9`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 386 | 0.07 | 0.07 | 0.11 | 0.16 | 0.95 | 0 |
| candidate | 409 | 0.08 | 0.08 | 0.11 | 0.20 | 1.04 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `1` | 8.00 | valid (cand S) | 10.00 | valid (cand N) |
| `2` | -12.00 | valid (cand S) | 4.00 | valid (cand N) |
| `3` | 32.00 | valid (cand S) | -2.00 | valid (cand N) |
| `4` | -20.00 | valid (cand S) | 10.00 | valid (cand N) |
| `5` | 10.00 | valid (cand S) | 4.00 | valid (cand N) |
| `6` | -48.00 | valid (cand S) | 10.00 | valid (cand N) |
| `7` | 0.00 | valid (cand S) | 8.00 | valid (cand N) |
| `8` | 2.00 | valid (cand S) | -4.00 | valid (cand N) |
| `9` | -2.00 | valid (cand S) | 10.00 | valid (cand N) |
| `10` | 0.00 | valid (cand S) | -4.00 | valid (cand N) |
| `11` | 36.00 | valid (cand S) | 20.00 | valid (cand N) |
| `12` | 12.00 | valid (cand S) | 44.00 | valid (cand N) |
| `13` | 10.00 | valid (cand S) | -8.00 | valid (cand N) |
| `14` | -6.00 | valid (cand S) | 4.00 | valid (cand N) |
| `15` | 8.00 | valid (cand S) | 10.00 | valid (cand N) |

