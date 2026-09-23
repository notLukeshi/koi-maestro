# Koi-Koi paired benchmark — P5 field: 7-entrant definitive tournament: resolving_fixed vs resolving_adaptive

**verdict: `development`** — 9 of 256 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `98d0719c49294824` |
| referee | `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| baseline | `resolving_fixed` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| candidate | `resolving_adaptive` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| stopping | `gsprt_accept_lower` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.4444`  CI99 `[0.2778, 0.5000]`
- **candidate margin**: `-4.56`  CI99 `[-18.67, 0.44]`
- clusters `9`, legs `18` — wins `8`, draws `0`, losses `10`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 229 | 908.80 | 30.34 | 3992.48 | 5776.77 | 11561.89 | 0 |
| candidate | 225 | 932.41 | 29.05 | 4195.77 | 5066.94 | 11655.15 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | -36.00 | valid (cand S) | 36.00 | valid (cand N) |
| `7619234134199958248` | 68.00 | valid (cand S) | -68.00 | valid (cand N) |
| `9318061777837526749` | 4.00 | valid (cand S) | -4.00 | valid (cand N) |
| `6001197866264162459` | -2.00 | valid (cand S) | 4.00 | valid (cand N) |
| `6044635964106635247` | 8.00 | valid (cand S) | -8.00 | valid (cand N) |
| `6903715913044025677` | -28.00 | valid (cand S) | 28.00 | valid (cand N) |
| `10814382527142015852` | 2.00 | valid (cand S) | -2.00 | valid (cand N) |
| `13535137607426657764` | 28.00 | valid (cand S) | -28.00 | valid (cand N) |
| `431149292466098373` | -80.00 | valid (cand S) | -4.00 | valid (cand N) |

