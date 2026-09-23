# Koi-Koi paired benchmark — annex: endgame vs yaku_chaser

**verdict: `development`** — 9 of 256 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `d75c4f4f07518784` |
| referee | `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| baseline | `endgame` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| candidate | `yaku_chaser` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| stopping | `gsprt_accept_lower` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.0833`  CI99 `[0.0000, 0.1944]`
- **candidate margin**: `-7.67`  CI99 `[-16.89, -2.56]`
- clusters `9`, legs `18` — wins `0`, draws `3`, losses `15`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 221 | 0.12 | 0.11 | 0.17 | 0.22 | 1.45 | 0 |
| candidate | 193 | 0.13 | 0.11 | 0.19 | 0.79 | 1.34 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | -60.00 | valid (cand S) | -4.00 | valid (cand N) |
| `7619234134199958248` | -2.00 | valid (cand S) | -2.00 | valid (cand N) |
| `9318061777837526749` | -2.00 | valid (cand S) | 0.00 | valid (cand N) |
| `6001197866264162459` | -4.00 | valid (cand S) | -12.00 | valid (cand N) |
| `6044635964106635247` | 0.00 | valid (cand S) | -10.00 | valid (cand N) |
| `6903715913044025677` | -10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `10814382527142015852` | -4.00 | valid (cand S) | -4.00 | valid (cand N) |
| `13535137607426657764` | -2.00 | valid (cand S) | -10.00 | valid (cand N) |
| `431149292466098373` | 0.00 | valid (cand S) | -2.00 | valid (cand N) |

