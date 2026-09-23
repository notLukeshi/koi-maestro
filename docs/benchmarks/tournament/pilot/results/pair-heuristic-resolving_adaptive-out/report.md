# Koi-Koi paired benchmark — P5 pilot: sizing + lane measurement: heuristic vs resolving_adaptive

**verdict: `development`** — 13 of 16 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `afe4d73d32dee9bf` |
| referee | `3e0067c95474606c29745b8e2ecc9e1ee6ce8462` (dirty) |
| baseline | `heuristic` @ `3e0067c95474606c29745b8e2ecc9e1ee6ce8462` (dirty) |
| candidate | `resolving_adaptive` @ `3e0067c95474606c29745b8e2ecc9e1ee6ce8462` (dirty) |
| stopping | `gsprt_accept_lower` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.4038`  CI99 `[0.2115, 0.6154]`
- **candidate margin**: `2.38`  CI99 `[-3.62, 8.69]`
- clusters `13`, legs `26` — wins `10`, draws `1`, losses `15`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 293 | 0.23 | 0.14 | 0.21 | 25.22 | 2.55 | 0 |
| candidate | 287 | 424.06 | 13.40 | 1961.45 | 3392.85 | 4680.96 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `16034480500904598573` | 48.00 | valid (cand S) | -28.00 | valid (cand N) |
| `4677710776332959296` | 4.00 | valid (cand S) | -4.00 | valid (cand N) |
| `13271961482927859836` | 4.00 | valid (cand S) | -10.00 | valid (cand N) |
| `10529852546119080842` | -4.00 | valid (cand S) | -6.00 | valid (cand N) |
| `14671832180405412331` | -10.00 | valid (cand S) | 36.00 | valid (cand N) |
| `17017001798808896126` | 40.00 | valid (cand S) | -10.00 | valid (cand N) |
| `18145539812079460165` | -12.00 | valid (cand S) | 0.00 | valid (cand N) |
| `49272188150520080` | -12.00 | valid (cand S) | -10.00 | valid (cand N) |
| `4902487414315408253` | 4.00 | valid (cand S) | 28.00 | valid (cand N) |
| `14075327324793344651` | -10.00 | valid (cand S) | -4.00 | valid (cand N) |
| `18351434641185641655` | 28.00 | valid (cand S) | -10.00 | valid (cand N) |
| `7575956051109723050` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `1745845176427152899` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |

