# Koi-Koi paired benchmark — P5 field: 7-entrant definitive tournament: random vs heuristic

**verdict: `development`** — 12 of 256 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `9f9ddcc6b0ae1b7d` |
| referee | `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| baseline | `random` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| candidate | `heuristic` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| stopping | `gsprt_accept_upper` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.8333`  CI99 `[0.6667, 1.0000]`
- **candidate margin**: `9.92`  CI99 `[5.00, 16.00]`
- clusters `12`, legs `24` — wins `20`, draws `0`, losses `4`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 176 | 0.86 | 0.11 | 0.17 | 37.88 | 6.32 | 0 |
| candidate | 210 | 0.83 | 0.11 | 0.17 | 47.03 | 7.29 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | 10.00 | valid (cand S) | 4.00 | valid (cand N) |
| `7619234134199958248` | -2.00 | valid (cand S) | 6.00 | valid (cand N) |
| `9318061777837526749` | 4.00 | valid (cand S) | 10.00 | valid (cand N) |
| `6001197866264162459` | 12.00 | valid (cand S) | 10.00 | valid (cand N) |
| `6044635964106635247` | 10.00 | valid (cand S) | 4.00 | valid (cand N) |
| `6903715913044025677` | 40.00 | valid (cand S) | -10.00 | valid (cand N) |
| `10814382527142015852` | 10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `13535137607426657764` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `431149292466098373` | 28.00 | valid (cand S) | 28.00 | valid (cand N) |
| `14974509662690242321` | -4.00 | valid (cand S) | 10.00 | valid (cand N) |
| `15928830168549992736` | 10.00 | valid (cand S) | 28.00 | valid (cand N) |
| `4112377384228269587` | 10.00 | valid (cand S) | 10.00 | valid (cand N) |

