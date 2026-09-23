# Koi-Koi paired benchmark — P6 field: 9-entrant definitive tournament: ismcts vs resolving_leaf

**verdict: `development`** — 14 of 256 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `578933bedc2b2dde` |
| referee | `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| baseline | `ismcts` @ `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| candidate | `resolving_leaf` @ `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| stopping | `gsprt_accept_upper` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.6607`  CI99 `[0.5357, 0.8214]`
- **candidate margin**: `1.71`  CI99 `[-4.43, 8.64]`
- clusters `14`, legs `28` — wins `18`, draws `1`, losses `9`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 349 | 17.70 | 11.24 | 72.94 | 147.39 | 220.68 | 0 |
| candidate | 369 | 1235.11 | 215.59 | 5945.79 | 10256.08 | 16276.98 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | 2.00 | valid (cand S) | -4.00 | valid (cand N) |
| `7619234134199958248` | 28.00 | valid (cand S) | -28.00 | valid (cand N) |
| `9318061777837526749` | -36.00 | valid (cand S) | 36.00 | valid (cand N) |
| `6001197866264162459` | -28.00 | valid (cand S) | 2.00 | valid (cand N) |
| `6044635964106635247` | 2.00 | valid (cand S) | 10.00 | valid (cand N) |
| `6903715913044025677` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `10814382527142015852` | 32.00 | valid (cand S) | 12.00 | valid (cand N) |
| `13535137607426657764` | 0.00 | valid (cand S) | 36.00 | valid (cand N) |
| `431149292466098373` | 4.00 | valid (cand S) | -2.00 | valid (cand N) |
| `14974509662690242321` | 12.00 | valid (cand S) | 2.00 | valid (cand N) |
| `15928830168549992736` | -12.00 | valid (cand S) | 2.00 | valid (cand N) |
| `4112377384228269587` | -2.00 | valid (cand S) | 4.00 | valid (cand N) |
| `18000172133288265584` | -32.00 | valid (cand S) | 4.00 | valid (cand N) |
| `17685270382932053218` | 2.00 | valid (cand S) | 2.00 | valid (cand N) |

