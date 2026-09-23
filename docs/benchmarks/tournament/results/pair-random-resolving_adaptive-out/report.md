# Koi-Koi paired benchmark — P5 field: 7-entrant definitive tournament: random vs resolving_adaptive

**verdict: `development`** — 12 of 256 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `e0afbf3a367e4034` |
| referee | `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| baseline | `random` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| candidate | `resolving_adaptive` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| stopping | `gsprt_accept_upper` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.7500`  CI99 `[0.5833, 0.9167]`
- **candidate margin**: `14.83`  CI99 `[3.50, 28.42]`
- clusters `12`, legs `24` — wins `18`, draws `0`, losses `6`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 296 | 0.17 | 0.13 | 0.20 | 9.62 | 2.10 | 0 |
| candidate | 328 | 753.07 | 14.22 | 3304.53 | 4955.74 | 10291.97 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | 10.00 | valid (cand S) | -28.00 | valid (cand N) |
| `7619234134199958248` | -2.00 | valid (cand S) | 28.00 | valid (cand N) |
| `9318061777837526749` | 40.00 | valid (cand S) | 68.00 | valid (cand N) |
| `6001197866264162459` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `6044635964106635247` | -4.00 | valid (cand S) | 48.00 | valid (cand N) |
| `6903715913044025677` | 4.00 | valid (cand S) | -8.00 | valid (cand N) |
| `10814382527142015852` | 10.00 | valid (cand S) | 2.00 | valid (cand N) |
| `13535137607426657764` | 6.00 | valid (cand S) | 20.00 | valid (cand N) |
| `431149292466098373` | -4.00 | valid (cand S) | 12.00 | valid (cand N) |
| `14974509662690242321` | 4.00 | valid (cand S) | 52.00 | valid (cand N) |
| `15928830168549992736` | 12.00 | valid (cand S) | 6.00 | valid (cand N) |
| `4112377384228269587` | 52.00 | valid (cand S) | 28.00 | valid (cand N) |

