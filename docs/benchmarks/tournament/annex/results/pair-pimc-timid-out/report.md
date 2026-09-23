# Koi-Koi paired benchmark — annex: pimc vs timid

**verdict: `development`** — 9 of 256 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `9195b0879552e606` |
| referee | `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| baseline | `pimc` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| candidate | `timid` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| stopping | `gsprt_accept_lower` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.3611`  CI99 `[0.1111, 0.5833]`
- **candidate margin**: `-3.56`  CI99 `[-6.56, -0.67]`
- clusters `9`, legs `18` — wins `6`, draws `1`, losses `11`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 226 | 81.49 | 7.86 | 338.17 | 669.05 | 1023.20 | 0 |
| candidate | 210 | 0.28 | 0.14 | 0.23 | 27.61 | 3.27 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | 10.00 | valid (cand S) | -12.00 | valid (cand N) |
| `7619234134199958248` | 10.00 | valid (cand S) | -28.00 | valid (cand N) |
| `9318061777837526749` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `6001197866264162459` | 0.00 | valid (cand S) | 2.00 | valid (cand N) |
| `6044635964106635247` | -4.00 | valid (cand S) | -8.00 | valid (cand N) |
| `6903715913044025677` | -2.00 | valid (cand S) | -12.00 | valid (cand N) |
| `10814382527142015852` | -10.00 | valid (cand S) | -4.00 | valid (cand N) |
| `13535137607426657764` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `431149292466098373` | -10.00 | valid (cand S) | 4.00 | valid (cand N) |

