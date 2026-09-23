# Koi-Koi paired benchmark — annex: pimc vs banker

**verdict: `development`** — 13 of 256 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `01e886af6f6be308` |
| referee | `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| baseline | `pimc` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| candidate | `banker` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| stopping | `gsprt_accept_lower` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.4231`  CI99 `[0.2308, 0.6154]`
- **candidate margin**: `-6.08`  CI99 `[-15.77, 2.46]`
- clusters `13`, legs `26` — wins `11`, draws `0`, losses `15`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 289 | 116.18 | 22.51 | 483.91 | 1101.51 | 1291.34 | 0 |
| candidate | 275 | 0.20 | 0.14 | 0.20 | 13.62 | 2.09 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | -10.00 | valid (cand S) | 28.00 | valid (cand N) |
| `7619234134199958248` | 28.00 | valid (cand S) | -28.00 | valid (cand N) |
| `9318061777837526749` | 28.00 | valid (cand S) | 2.00 | valid (cand N) |
| `6001197866264162459` | 10.00 | valid (cand S) | -12.00 | valid (cand N) |
| `6044635964106635247` | -28.00 | valid (cand S) | 10.00 | valid (cand N) |
| `6903715913044025677` | 10.00 | valid (cand S) | -60.00 | valid (cand N) |
| `10814382527142015852` | -12.00 | valid (cand S) | -2.00 | valid (cand N) |
| `13535137607426657764` | -40.00 | valid (cand S) | -28.00 | valid (cand N) |
| `431149292466098373` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `14974509662690242321` | -6.00 | valid (cand S) | -10.00 | valid (cand N) |
| `15928830168549992736` | 2.00 | valid (cand S) | -6.00 | valid (cand N) |
| `4112377384228269587` | -44.00 | valid (cand S) | 10.00 | valid (cand N) |
| `18000172133288265584` | 28.00 | valid (cand S) | -28.00 | valid (cand N) |

