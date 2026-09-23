# Koi-Koi paired benchmark — P3: resolving_adaptive vs materialist

**verdict: `canonical`** — 31 of 64 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `32381536e8e66533` |
| referee | `504251ef8de5694217ed354c9875b09bba880a71` |
| baseline | `materialist` @ `504251ef8de5694217ed354c9875b09bba880a71` |
| candidate | `resolving_adaptive` @ `504251ef8de5694217ed354c9875b09bba880a71` |
| stopping | `gsprt_accept_lower` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.5000`  CI99 `[0.3710, 0.6290]`
- **candidate margin**: `1.58`  CI99 `[-1.65, 5.65]`
- clusters `31`, legs `62` — wins `31`, draws `0`, losses `31`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 639 | 0.28 | 0.13 | 0.20 | 17.88 | 2.92 | 0 |
| candidate | 640 | 404.14 | 4.53 | 1743.23 | 2437.84 | 4171.81 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `1` | 4.00 | valid (cand S) | 32.00 | valid (cand N) |
| `2` | -10.00 | valid (cand S) | 40.00 | valid (cand N) |
| `3` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `4` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `5` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `6` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `7` | -6.00 | valid (cand S) | 6.00 | valid (cand N) |
| `8` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `9` | 4.00 | valid (cand S) | -10.00 | valid (cand N) |
| `10` | 4.00 | valid (cand S) | 12.00 | valid (cand N) |
| `11` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `12` | 4.00 | valid (cand S) | -10.00 | valid (cand N) |
| `13` | 10.00 | valid (cand S) | -8.00 | valid (cand N) |
| `14` | 2.00 | valid (cand S) | 10.00 | valid (cand N) |
| `15` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `16` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `17` | -4.00 | valid (cand S) | 2.00 | valid (cand N) |
| `18` | 4.00 | valid (cand S) | 12.00 | valid (cand N) |
| `19` | 4.00 | valid (cand S) | -6.00 | valid (cand N) |
| `20` | -4.00 | valid (cand S) | -10.00 | valid (cand N) |
| `21` | -2.00 | valid (cand S) | -10.00 | valid (cand N) |
| `22` | -28.00 | valid (cand S) | 32.00 | valid (cand N) |
| `23` | 10.00 | valid (cand S) | 52.00 | valid (cand N) |
| `24` | 12.00 | valid (cand S) | -6.00 | valid (cand N) |
| `25` | -4.00 | valid (cand S) | -10.00 | valid (cand N) |
| `26` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `27` | -4.00 | valid (cand S) | 6.00 | valid (cand N) |
| `28` | -28.00 | valid (cand S) | 32.00 | valid (cand N) |
| `29` | -10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `30` | -12.00 | valid (cand S) | 10.00 | valid (cand N) |
| `31` | -4.00 | valid (cand S) | -10.00 | valid (cand N) |

