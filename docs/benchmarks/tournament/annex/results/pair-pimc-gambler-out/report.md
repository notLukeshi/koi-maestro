# Koi-Koi paired benchmark — annex: pimc vs gambler

**verdict: `development`** — 9 of 256 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `80ed7d45729c79ff` |
| referee | `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| baseline | `pimc` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| candidate | `gambler` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| stopping | `gsprt_accept_lower` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.2778`  CI99 `[0.0556, 0.4444]`
- **candidate margin**: `-6.56`  CI99 `[-11.78, -1.67]`
- clusters `9`, legs `18` — wins `5`, draws `0`, losses `13`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 274 | 84.13 | 8.15 | 324.21 | 761.22 | 1280.68 | 0 |
| candidate | 256 | 0.17 | 0.14 | 0.21 | 6.36 | 2.38 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | -48.00 | valid (cand S) | 12.00 | valid (cand N) |
| `7619234134199958248` | -20.00 | valid (cand S) | -2.00 | valid (cand N) |
| `9318061777837526749` | -12.00 | valid (cand S) | -4.00 | valid (cand N) |
| `6001197866264162459` | -8.00 | valid (cand S) | -4.00 | valid (cand N) |
| `6044635964106635247` | 4.00 | valid (cand S) | -28.00 | valid (cand N) |
| `6903715913044025677` | -6.00 | valid (cand S) | -4.00 | valid (cand N) |
| `10814382527142015852` | -8.00 | valid (cand S) | 6.00 | valid (cand N) |
| `13535137607426657764` | 36.00 | valid (cand S) | -32.00 | valid (cand N) |
| `431149292466098373` | -12.00 | valid (cand S) | 12.00 | valid (cand N) |

