# Koi-Koi paired benchmark — P5 field: 7-entrant definitive tournament: endgame vs resolving_adaptive

**verdict: `development`** — 9 of 256 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `9c55a6834fd66102` |
| referee | `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| baseline | `endgame` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| candidate | `resolving_adaptive` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| stopping | `gsprt_accept_lower` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.4167`  CI99 `[0.1944, 0.5833]`
- **candidate margin**: `0.22`  CI99 `[-1.89, 3.33]`
- clusters `9`, legs `18` — wins `7`, draws `1`, losses `10`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 210 | 0.42 | 0.14 | 0.20 | 43.48 | 4.86 | 0 |
| candidate | 205 | 1091.08 | 149.46 | 4278.84 | 6088.94 | 12426.17 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | 0.00 | valid (cand S) | 2.00 | valid (cand N) |
| `7619234134199958248` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `9318061777837526749` | -4.00 | valid (cand S) | -2.00 | valid (cand N) |
| `6001197866264162459` | -12.00 | valid (cand S) | 28.00 | valid (cand N) |
| `6044635964106635247` | -2.00 | valid (cand S) | -4.00 | valid (cand N) |
| `6903715913044025677` | 2.00 | valid (cand S) | -4.00 | valid (cand N) |
| `10814382527142015852` | 12.00 | valid (cand S) | -10.00 | valid (cand N) |
| `13535137607426657764` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `431149292466098373` | 2.00 | valid (cand S) | -4.00 | valid (cand N) |

