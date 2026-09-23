# Koi-Koi paired benchmark — annex: pimc vs yaku_chaser

**verdict: `development`** — 9 of 256 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `35fc00aa26bddae6` |
| referee | `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| baseline | `pimc` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| candidate | `yaku_chaser` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| stopping | `gsprt_accept_lower` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.1667`  CI99 `[0.0000, 0.3611]`
- **candidate margin**: `-17.11`  CI99 `[-21.33, -9.89]`
- clusters `9`, legs `18` — wins `2`, draws `2`, losses `14`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 260 | 78.21 | 11.60 | 316.72 | 830.78 | 1129.67 | 0 |
| candidate | 235 | 0.27 | 0.14 | 0.20 | 17.61 | 3.57 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | 0.00 | valid (cand S) | -48.00 | valid (cand N) |
| `7619234134199958248` | -10.00 | valid (cand S) | -28.00 | valid (cand N) |
| `9318061777837526749` | -2.00 | valid (cand S) | 4.00 | valid (cand N) |
| `6001197866264162459` | -28.00 | valid (cand S) | -4.00 | valid (cand N) |
| `6044635964106635247` | 4.00 | valid (cand S) | -44.00 | valid (cand N) |
| `6903715913044025677` | -32.00 | valid (cand S) | -10.00 | valid (cand N) |
| `10814382527142015852` | -2.00 | valid (cand S) | -32.00 | valid (cand N) |
| `13535137607426657764` | -32.00 | valid (cand S) | 0.00 | valid (cand N) |
| `431149292466098373` | -4.00 | valid (cand S) | -40.00 | valid (cand N) |

