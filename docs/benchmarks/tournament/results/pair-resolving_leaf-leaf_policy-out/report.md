# Koi-Koi paired benchmark — P6 field: 9-entrant definitive tournament: resolving_leaf vs leaf_policy

**verdict: `development`** — 9 of 256 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `3a1b23290eb113a4` |
| referee | `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| baseline | `resolving_leaf` @ `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| candidate | `leaf_policy` @ `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| stopping | `gsprt_accept_lower` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.2778`  CI99 `[0.0556, 0.4444]`
- **candidate margin**: `-7.11`  CI99 `[-15.00, -0.78]`
- clusters `9`, legs `18` — wins `5`, draws `0`, losses `13`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 187 | 1065.07 | 204.01 | 5169.78 | 9576.54 | 11064.94 | 0 |
| candidate | 171 | 30.14 | 35.75 | 114.16 | 168.33 | 286.29 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | -10.00 | valid (cand S) | -2.00 | valid (cand N) |
| `7619234134199958248` | 2.00 | valid (cand S) | -2.00 | valid (cand N) |
| `9318061777837526749` | -10.00 | valid (cand S) | -2.00 | valid (cand N) |
| `6001197866264162459` | -20.00 | valid (cand S) | -28.00 | valid (cand N) |
| `6044635964106635247` | 6.00 | valid (cand S) | -2.00 | valid (cand N) |
| `6903715913044025677` | -36.00 | valid (cand S) | -2.00 | valid (cand N) |
| `10814382527142015852` | 2.00 | valid (cand S) | -4.00 | valid (cand N) |
| `13535137607426657764` | -48.00 | valid (cand S) | 28.00 | valid (cand N) |
| `431149292466098373` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |

