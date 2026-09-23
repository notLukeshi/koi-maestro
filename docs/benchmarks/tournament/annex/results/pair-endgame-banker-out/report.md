# Koi-Koi paired benchmark — annex: endgame vs banker

**verdict: `development`** — 9 of 256 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `241372ca5839bdfe` |
| referee | `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| baseline | `endgame` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| candidate | `banker` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| stopping | `gsprt_accept_lower` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.5000`  CI99 `[0.5000, 0.5000]`
- **candidate margin**: `0.00`  CI99 `[0.00, 0.00]`
- clusters `9`, legs `18` — wins `9`, draws `0`, losses `9`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 182 | 0.24 | 0.11 | 0.17 | 22.71 | 2.46 | 0 |
| candidate | 182 | 0.26 | 0.11 | 0.17 | 26.63 | 2.65 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `7619234134199958248` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `9318061777837526749` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `6001197866264162459` | 12.00 | valid (cand S) | -12.00 | valid (cand N) |
| `6044635964106635247` | -4.00 | valid (cand S) | 4.00 | valid (cand N) |
| `6903715913044025677` | 12.00 | valid (cand S) | -12.00 | valid (cand N) |
| `10814382527142015852` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `13535137607426657764` | 4.00 | valid (cand S) | -4.00 | valid (cand N) |
| `431149292466098373` | -2.00 | valid (cand S) | 2.00 | valid (cand N) |

