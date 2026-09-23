# Koi-Koi paired benchmark — annex: endgame vs random

**verdict: `development`** — 9 of 256 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `7c5ee54513c07ed8` |
| referee | `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| baseline | `endgame` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| candidate | `random` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| stopping | `gsprt_accept_lower` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.1667`  CI99 `[0.0000, 0.3889]`
- **candidate margin**: `-10.89`  CI99 `[-22.89, -2.89]`
- clusters `9`, legs `18` — wins `3`, draws `0`, losses `15`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 195 | 0.33 | 0.11 | 0.20 | 33.68 | 3.53 | 0 |
| candidate | 172 | 0.12 | 0.11 | 0.17 | 0.30 | 1.12 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | -28.00 | valid (cand S) | -4.00 | valid (cand N) |
| `7619234134199958248` | 2.00 | valid (cand S) | -4.00 | valid (cand N) |
| `9318061777837526749` | 2.00 | valid (cand S) | -2.00 | valid (cand N) |
| `6001197866264162459` | -10.00 | valid (cand S) | -28.00 | valid (cand N) |
| `6044635964106635247` | -10.00 | valid (cand S) | -68.00 | valid (cand N) |
| `6903715913044025677` | -10.00 | valid (cand S) | -2.00 | valid (cand N) |
| `10814382527142015852` | -12.00 | valid (cand S) | -2.00 | valid (cand N) |
| `13535137607426657764` | 2.00 | valid (cand S) | -4.00 | valid (cand N) |
| `431149292466098373` | -8.00 | valid (cand S) | -10.00 | valid (cand N) |

