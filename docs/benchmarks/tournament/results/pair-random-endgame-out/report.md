# Koi-Koi paired benchmark — P5 field: 7-entrant definitive tournament: random vs endgame

**verdict: `development`** — 12 of 256 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `7c195850621095d1` |
| referee | `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| baseline | `random` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| candidate | `endgame` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| stopping | `gsprt_accept_upper` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.7917`  CI99 `[0.6250, 0.9583]`
- **candidate margin**: `5.33`  CI99 `[-1.08, 11.67]`
- clusters `12`, legs `24` — wins `19`, draws `0`, losses `5`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 256 | 0.12 | 0.11 | 0.18 | 1.13 | 1.30 | 0 |
| candidate | 281 | 0.13 | 0.11 | 0.18 | 1.74 | 1.47 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | 10.00 | valid (cand S) | 28.00 | valid (cand N) |
| `7619234134199958248` | 32.00 | valid (cand S) | 2.00 | valid (cand N) |
| `9318061777837526749` | 2.00 | valid (cand S) | -12.00 | valid (cand N) |
| `6001197866264162459` | 10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `6044635964106635247` | 12.00 | valid (cand S) | -12.00 | valid (cand N) |
| `6903715913044025677` | 2.00 | valid (cand S) | 2.00 | valid (cand N) |
| `10814382527142015852` | 44.00 | valid (cand S) | -28.00 | valid (cand N) |
| `13535137607426657764` | 12.00 | valid (cand S) | 2.00 | valid (cand N) |
| `431149292466098373` | -6.00 | valid (cand S) | 2.00 | valid (cand N) |
| `14974509662690242321` | -32.00 | valid (cand S) | 12.00 | valid (cand N) |
| `15928830168549992736` | 28.00 | valid (cand S) | 4.00 | valid (cand N) |
| `4112377384228269587` | 2.00 | valid (cand S) | 2.00 | valid (cand N) |

