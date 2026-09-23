# Koi-Koi paired benchmark — P5 field: 7-entrant definitive tournament: random vs pimc

**verdict: `development`** — 15 of 256 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `fb5e3c127626cab1` |
| referee | `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| baseline | `random` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| candidate | `pimc` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| stopping | `gsprt_accept_upper` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.8833`  CI99 `[0.6667, 1.0000]`
- **candidate margin**: `12.13`  CI99 `[4.93, 18.60]`
- clusters `15`, legs `30` — wins `26`, draws `1`, losses `3`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 351 | 0.14 | 0.14 | 0.19 | 0.27 | 1.63 | 0 |
| candidate | 401 | 87.36 | 6.43 | 355.75 | 1048.48 | 1167.68 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | 32.00 | valid (cand S) | 10.00 | valid (cand N) |
| `7619234134199958248` | 6.00 | valid (cand S) | 6.00 | valid (cand N) |
| `9318061777837526749` | 2.00 | valid (cand S) | 8.00 | valid (cand N) |
| `6001197866264162459` | 24.00 | valid (cand S) | 12.00 | valid (cand N) |
| `6044635964106635247` | -10.00 | valid (cand S) | 12.00 | valid (cand N) |
| `6903715913044025677` | 10.00 | valid (cand S) | 40.00 | valid (cand N) |
| `10814382527142015852` | 0.00 | valid (cand S) | 12.00 | valid (cand N) |
| `13535137607426657764` | 40.00 | valid (cand S) | 10.00 | valid (cand N) |
| `431149292466098373` | 2.00 | valid (cand S) | 10.00 | valid (cand N) |
| `14974509662690242321` | -24.00 | valid (cand S) | -2.00 | valid (cand N) |
| `15928830168549992736` | 28.00 | valid (cand S) | 2.00 | valid (cand N) |
| `4112377384228269587` | 4.00 | valid (cand S) | 10.00 | valid (cand N) |
| `18000172133288265584` | 10.00 | valid (cand S) | 28.00 | valid (cand N) |
| `17685270382932053218` | 12.00 | valid (cand S) | 40.00 | valid (cand N) |
| `9991098352591498860` | 2.00 | valid (cand S) | 28.00 | valid (cand N) |

