# Koi-Koi paired benchmark — P6 field: 9-entrant definitive tournament: endgame vs leaf_policy

**verdict: `development`** — 9 of 256 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `f4d7d265b43b691d` |
| referee | `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| baseline | `endgame` @ `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| candidate | `leaf_policy` @ `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| stopping | `gsprt_accept_lower` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.3889`  CI99 `[0.2222, 0.5000]`
- **candidate margin**: `-4.44`  CI99 `[-11.33, 0.00]`
- clusters `9`, legs `18` — wins `7`, draws `0`, losses `11`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 171 | 0.14 | 0.13 | 0.20 | 0.36 | 1.31 | 0 |
| candidate | 163 | 32.97 | 35.09 | 105.89 | 141.47 | 298.60 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `7619234134199958248` | 12.00 | valid (cand S) | -12.00 | valid (cand N) |
| `9318061777837526749` | -2.00 | valid (cand S) | 2.00 | valid (cand N) |
| `6001197866264162459` | 12.00 | valid (cand S) | -12.00 | valid (cand N) |
| `6044635964106635247` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `6903715913044025677` | 10.00 | valid (cand S) | -10.00 | valid (cand N) |
| `10814382527142015852` | -40.00 | valid (cand S) | 2.00 | valid (cand N) |
| `13535137607426657764` | -2.00 | valid (cand S) | -10.00 | valid (cand N) |
| `431149292466098373` | -2.00 | valid (cand S) | -28.00 | valid (cand N) |

