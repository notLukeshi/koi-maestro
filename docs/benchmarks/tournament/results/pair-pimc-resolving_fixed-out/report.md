# Koi-Koi paired benchmark — P5 field: 7-entrant definitive tournament: pimc vs resolving_fixed

**verdict: `development`** — 9 of 256 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `df32da745ef8a04f` |
| referee | `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| baseline | `pimc` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| candidate | `resolving_fixed` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| stopping | `gsprt_accept_lower` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.3333`  CI99 `[0.1111, 0.5000]`
- **candidate margin**: `-3.56`  CI99 `[-9.89, 0.44]`
- clusters `9`, legs `18` — wins `5`, draws `2`, losses `11`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 238 | 104.25 | 27.09 | 424.16 | 1013.13 | 1378.47 | 0 |
| candidate | 226 | 797.36 | 0.84 | 3326.08 | 4354.30 | 10011.25 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | -4.00 | valid (cand S) | -32.00 | valid (cand N) |
| `7619234134199958248` | -4.00 | valid (cand S) | -20.00 | valid (cand N) |
| `9318061777837526749` | 0.00 | valid (cand S) | 0.00 | valid (cand N) |
| `6001197866264162459` | -10.00 | valid (cand S) | 12.00 | valid (cand N) |
| `6044635964106635247` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `6903715913044025677` | 52.00 | valid (cand S) | -52.00 | valid (cand N) |
| `10814382527142015852` | -6.00 | valid (cand S) | -2.00 | valid (cand N) |
| `13535137607426657764` | 4.00 | valid (cand S) | -2.00 | valid (cand N) |
| `431149292466098373` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |

