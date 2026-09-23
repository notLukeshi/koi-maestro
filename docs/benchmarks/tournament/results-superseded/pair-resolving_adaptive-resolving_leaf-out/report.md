# Koi-Koi paired benchmark — P6 field: 9-entrant definitive tournament: resolving_adaptive vs resolving_leaf

**verdict: `development`** — 9 of 256 declared clusters played
- 16 of 18 legs are forfeits carrying synthetic margins (±256), not played evidence
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `f946465390ee9920` |
| referee | `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| baseline | `resolving_adaptive` @ `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| candidate | `resolving_leaf` @ `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| stopping | `gsprt_accept_lower` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.0000`  CI99 `[0.0000, 0.0000]`
- **candidate margin**: `-228.67`  CI99 `[-256.00, -187.67]`
- clusters `9`, legs `18` — wins `0`, draws `0`, losses `18`, invalid `0`, time-forfeits `16`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 105 | 519.95 | 257.69 | 1966.61 | 2217.20 | 3033.02 | 0 |
| candidate | 108 | 5854.29 | 507.44 | 18601.12 | 29443.67 | 35125.72 | 16 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | -256.00 | time-forfeit: Candidate | -256.00 | time-forfeit: Candidate |
| `7619234134199958248` | -256.00 | time-forfeit: Candidate | -256.00 | time-forfeit: Candidate |
| `9318061777837526749` | -256.00 | time-forfeit: Candidate | -256.00 | time-forfeit: Candidate |
| `6001197866264162459` | -256.00 | time-forfeit: Candidate | -256.00 | time-forfeit: Candidate |
| `6044635964106635247` | -10.00 | valid (cand S) | -256.00 | time-forfeit: Candidate |
| `6903715913044025677` | -256.00 | time-forfeit: Candidate | -10.00 | valid (cand N) |
| `10814382527142015852` | -256.00 | time-forfeit: Candidate | -256.00 | time-forfeit: Candidate |
| `13535137607426657764` | -256.00 | time-forfeit: Candidate | -256.00 | time-forfeit: Candidate |
| `431149292466098373` | -256.00 | time-forfeit: Candidate | -256.00 | time-forfeit: Candidate |

