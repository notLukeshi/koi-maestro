# Koi-Koi paired benchmark — P6 field: 9-entrant definitive tournament: ismcts vs resolving_leaf

**verdict: `vacuous`** — 9 of 256 declared clusters played
- 18 of 18 legs are forfeits carrying synthetic margins (±256), not played evidence
- zero valid legs: no played evidence was produced; point estimates and stopping outcomes are not interpretable
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `cebd0d22e923cc5a` |
| referee | `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| baseline | `ismcts` @ `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| candidate | `resolving_leaf` @ `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| stopping | `gsprt_accept_lower` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.0000`  CI99 `[0.0000, 0.0000]`
- **candidate margin**: `-256.00`  CI99 `[-256.00, -256.00]`
- clusters `9`, legs `18` — wins `0`, draws `0`, losses `18`, invalid `0`, time-forfeits `18`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 92 | 22.47 | 19.74 | 86.70 | 102.98 | 114.86 | 0 |
| candidate | 99 | 6722.33 | 509.48 | 21797.00 | 37010.33 | 36972.79 | 18 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | -256.00 | time-forfeit: Candidate | -256.00 | time-forfeit: Candidate |
| `7619234134199958248` | -256.00 | time-forfeit: Candidate | -256.00 | time-forfeit: Candidate |
| `9318061777837526749` | -256.00 | time-forfeit: Candidate | -256.00 | time-forfeit: Candidate |
| `6001197866264162459` | -256.00 | time-forfeit: Candidate | -256.00 | time-forfeit: Candidate |
| `6044635964106635247` | -256.00 | time-forfeit: Candidate | -256.00 | time-forfeit: Candidate |
| `6903715913044025677` | -256.00 | time-forfeit: Candidate | -256.00 | time-forfeit: Candidate |
| `10814382527142015852` | -256.00 | time-forfeit: Candidate | -256.00 | time-forfeit: Candidate |
| `13535137607426657764` | -256.00 | time-forfeit: Candidate | -256.00 | time-forfeit: Candidate |
| `431149292466098373` | -256.00 | time-forfeit: Candidate | -256.00 | time-forfeit: Candidate |

