# Koi-Koi paired benchmark — P6 field: 9-entrant definitive tournament: random vs resolving_leaf

**verdict: `development`** — 12 of 256 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `24eef40a055d8b6b` |
| referee | `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| baseline | `random` @ `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| candidate | `resolving_leaf` @ `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| stopping | `gsprt_accept_upper` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.7917`  CI99 `[0.6250, 0.9583]`
- **candidate margin**: `4.50`  CI99 `[-3.50, 12.58]`
- clusters `12`, legs `24` — wins `19`, draws `0`, losses `5`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 285 | 0.20 | 0.14 | 0.21 | 17.21 | 2.40 | 0 |
| candidate | 314 | 1040.62 | 195.39 | 5370.00 | 8466.85 | 13614.84 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | 2.00 | valid (cand S) | 4.00 | valid (cand N) |
| `7619234134199958248` | 2.00 | valid (cand S) | 4.00 | valid (cand N) |
| `9318061777837526749` | 2.00 | valid (cand S) | 10.00 | valid (cand N) |
| `6001197866264162459` | 10.00 | valid (cand S) | 2.00 | valid (cand N) |
| `6044635964106635247` | 2.00 | valid (cand S) | 2.00 | valid (cand N) |
| `6903715913044025677` | 52.00 | valid (cand S) | 2.00 | valid (cand N) |
| `10814382527142015852` | 36.00 | valid (cand S) | -2.00 | valid (cand N) |
| `13535137607426657764` | 4.00 | valid (cand S) | -28.00 | valid (cand N) |
| `431149292466098373` | 2.00 | valid (cand S) | -4.00 | valid (cand N) |
| `14974509662690242321` | 12.00 | valid (cand S) | 2.00 | valid (cand N) |
| `15928830168549992736` | 2.00 | valid (cand S) | -32.00 | valid (cand N) |
| `4112377384228269587` | 32.00 | valid (cand S) | -10.00 | valid (cand N) |

