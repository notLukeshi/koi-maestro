# Koi-Koi paired benchmark — P6 field: 9-entrant definitive tournament: heuristic vs resolving_leaf

**verdict: `development`** — 9 of 256 declared clusters played
- 16 of 18 legs are forfeits carrying synthetic margins (±256), not played evidence
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `e2a172f5bdf25563` |
| referee | `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| baseline | `heuristic` @ `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| candidate | `resolving_leaf` @ `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| stopping | `gsprt_accept_lower` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.0000`  CI99 `[0.0000, 0.0000]`
- **candidate margin**: `-228.67`  CI99 `[-256.00, -187.67]`
- clusters `9`, legs `18` — wins `0`, draws `0`, losses `18`, invalid `0`, time-forfeits `16`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 102 | 0.31 | 0.14 | 0.19 | 17.21 | 1.73 | 0 |
| candidate | 100 | 6044.48 | 515.64 | 19726.31 | 34482.82 | 33580.43 | 16 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | -256.00 | time-forfeit: Candidate | -256.00 | time-forfeit: Candidate |
| `7619234134199958248` | -10.00 | valid (cand S) | -256.00 | time-forfeit: Candidate |
| `9318061777837526749` | -256.00 | time-forfeit: Candidate | -256.00 | time-forfeit: Candidate |
| `6001197866264162459` | -256.00 | time-forfeit: Candidate | -256.00 | time-forfeit: Candidate |
| `6044635964106635247` | -256.00 | time-forfeit: Candidate | -256.00 | time-forfeit: Candidate |
| `6903715913044025677` | -256.00 | time-forfeit: Candidate | -10.00 | valid (cand N) |
| `10814382527142015852` | -256.00 | time-forfeit: Candidate | -256.00 | time-forfeit: Candidate |
| `13535137607426657764` | -256.00 | time-forfeit: Candidate | -256.00 | time-forfeit: Candidate |
| `431149292466098373` | -256.00 | time-forfeit: Candidate | -256.00 | time-forfeit: Candidate |

