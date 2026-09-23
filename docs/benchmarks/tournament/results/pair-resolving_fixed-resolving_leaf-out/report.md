# Koi-Koi paired benchmark — P6 field: 9-entrant definitive tournament: resolving_fixed vs resolving_leaf

**verdict: `development`** — 20 of 256 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `b66694e80dba8f09` |
| referee | `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| baseline | `resolving_fixed` @ `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| candidate | `resolving_leaf` @ `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| stopping | `gsprt_accept_lower` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.5000`  CI99 `[0.3750, 0.6250]`
- **candidate margin**: `-0.10`  CI99 `[-4.85, 4.05]`
- clusters `20`, legs `40` — wins `20`, draws `0`, losses `20`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 433 | 378.41 | 18.76 | 1603.46 | 2124.40 | 4096.26 | 0 |
| candidate | 432 | 1090.00 | 202.14 | 5203.68 | 10090.34 | 11771.95 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `17501323343323347481` | 2.00 | valid (cand S) | 10.00 | valid (cand N) |
| `7619234134199958248` | -4.00 | valid (cand S) | -10.00 | valid (cand N) |
| `9318061777837526749` | 12.00 | valid (cand S) | 10.00 | valid (cand N) |
| `6001197866264162459` | 40.00 | valid (cand S) | -40.00 | valid (cand N) |
| `6044635964106635247` | -10.00 | valid (cand S) | 40.00 | valid (cand N) |
| `6903715913044025677` | -12.00 | valid (cand S) | 10.00 | valid (cand N) |
| `10814382527142015852` | 4.00 | valid (cand S) | -4.00 | valid (cand N) |
| `13535137607426657764` | -12.00 | valid (cand S) | 10.00 | valid (cand N) |
| `431149292466098373` | -28.00 | valid (cand S) | 12.00 | valid (cand N) |
| `14974509662690242321` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `15928830168549992736` | 10.00 | valid (cand S) | -12.00 | valid (cand N) |
| `4112377384228269587` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `18000172133288265584` | -10.00 | valid (cand S) | 20.00 | valid (cand N) |
| `17685270382932053218` | 4.00 | valid (cand S) | -6.00 | valid (cand N) |
| `9991098352591498860` | -4.00 | valid (cand S) | 2.00 | valid (cand N) |
| `13872560487504887217` | -40.00 | valid (cand S) | -8.00 | valid (cand N) |
| `5109558618046986729` | -10.00 | valid (cand S) | 2.00 | valid (cand N) |
| `18422795515743521302` | 12.00 | valid (cand S) | -10.00 | valid (cand N) |
| `13075277902204111389` | -12.00 | valid (cand S) | 28.00 | valid (cand N) |
| `15724539146521815599` | 2.00 | valid (cand S) | -2.00 | valid (cand N) |

