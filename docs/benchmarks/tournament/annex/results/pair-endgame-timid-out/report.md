# Koi-Koi paired benchmark — annex: endgame vs timid

**verdict: `development`** — 9 of 256 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `72cbb257cae226b8` |
| referee | `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| baseline | `endgame` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| candidate | `timid` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| stopping | `gsprt_accept_lower` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.5000`  CI99 `[0.5000, 0.5000]`
- **candidate margin**: `0.00`  CI99 `[0.00, 0.00]`
- clusters `9`, legs `18` — wins `9`, draws `0`, losses `9`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 207 | 0.22 | 0.11 | 0.16 | 22.12 | 2.58 | 0 |
| candidate | 207 | 0.27 | 0.11 | 0.16 | 32.52 | 3.13 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | -2.00 | valid (cand S) | 2.00 | valid (cand N) |
| `7619234134199958248` | -4.00 | valid (cand S) | 4.00 | valid (cand N) |
| `9318061777837526749` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `6001197866264162459` | 2.00 | valid (cand S) | -2.00 | valid (cand N) |
| `6044635964106635247` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `6903715913044025677` | 2.00 | valid (cand S) | -2.00 | valid (cand N) |
| `10814382527142015852` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `13535137607426657764` | -6.00 | valid (cand S) | 6.00 | valid (cand N) |
| `431149292466098373` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |

