# Koi-Koi paired benchmark — annex: endgame vs gambler

**verdict: `development`** — 9 of 256 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `e7834d95b471dd07` |
| referee | `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| baseline | `endgame` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| candidate | `gambler` @ `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty) |
| stopping | `gsprt_accept_lower` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.2500`  CI99 `[0.0556, 0.4444]`
- **candidate margin**: `-0.67`  CI99 `[-10.56, 9.44]`
- clusters `9`, legs `18` — wins `4`, draws `1`, losses `13`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 229 | 0.13 | 0.11 | 0.17 | 2.48 | 1.62 | 0 |
| candidate | 221 | 0.12 | 0.11 | 0.16 | 0.44 | 1.43 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | 40.00 | valid (cand S) | -10.00 | valid (cand N) |
| `7619234134199958248` | -10.00 | valid (cand S) | 12.00 | valid (cand N) |
| `9318061777837526749` | -4.00 | valid (cand S) | 0.00 | valid (cand N) |
| `6001197866264162459` | -2.00 | valid (cand S) | -24.00 | valid (cand N) |
| `6044635964106635247` | -20.00 | valid (cand S) | -10.00 | valid (cand N) |
| `6903715913044025677` | 40.00 | valid (cand S) | -4.00 | valid (cand N) |
| `10814382527142015852` | -28.00 | valid (cand S) | -4.00 | valid (cand N) |
| `13535137607426657764` | -2.00 | valid (cand S) | -4.00 | valid (cand N) |
| `431149292466098373` | -10.00 | valid (cand S) | 28.00 | valid (cand N) |

