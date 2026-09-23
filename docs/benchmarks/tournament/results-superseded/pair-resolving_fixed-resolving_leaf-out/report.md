# Koi-Koi paired benchmark — P6 field: 9-entrant definitive tournament: resolving_fixed vs resolving_leaf

**verdict: `development`** — 9 of 256 declared clusters played
- 15 of 18 legs are forfeits carrying synthetic margins (±256), not played evidence
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `0c6a4834f6857283` |
| referee | `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| baseline | `resolving_fixed` @ `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| candidate | `resolving_leaf` @ `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| stopping | `gsprt_accept_lower` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.0556`  CI99 `[0.0000, 0.2222]`
- **candidate margin**: `-213.89`  CI99 `[-256.00, -128.56]`
- clusters `9`, legs `18` — wins `1`, draws `0`, losses `17`, invalid `0`, time-forfeits `15`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 97 | 538.16 | 264.50 | 1886.68 | 2149.11 | 2900.11 | 0 |
| candidate | 101 | 5959.08 | 477.77 | 19854.48 | 32648.51 | 33437.06 | 15 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | -256.00 | time-forfeit: Candidate | -256.00 | time-forfeit: Candidate |
| `7619234134199958248` | -10.00 | valid (cand S) | -256.00 | time-forfeit: Candidate |
| `9318061777837526749` | -256.00 | time-forfeit: Candidate | -256.00 | time-forfeit: Candidate |
| `6001197866264162459` | -12.00 | valid (cand S) | 12.00 | valid (cand N) |
| `6044635964106635247` | -256.00 | time-forfeit: Candidate | -256.00 | time-forfeit: Candidate |
| `6903715913044025677` | -256.00 | time-forfeit: Candidate | -256.00 | time-forfeit: Candidate |
| `10814382527142015852` | -256.00 | time-forfeit: Candidate | -256.00 | time-forfeit: Candidate |
| `13535137607426657764` | -256.00 | time-forfeit: Candidate | -256.00 | time-forfeit: Candidate |
| `431149292466098373` | -256.00 | time-forfeit: Candidate | -256.00 | time-forfeit: Candidate |

