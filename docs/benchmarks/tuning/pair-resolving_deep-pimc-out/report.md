# Koi-Koi paired benchmark — P6-A3 exploratory: deep resolving (24w/600k/300cfr/d6) vs pimc — relaxed 120s time contract

**verdict: `development`** — 9 of 32 declared clusters played
- sequential stop: point estimates are selection-biased; the boundary decision is the evidence
- at least one attested build is dirty — development evidence, not canonical

| field | value |
|---|---|
| ruleset | `nintendo` |
| manifest | `23a5c977339e36f7` |
| referee | `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| baseline | `pimc` @ `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| candidate | `resolving_deep` @ `8e778ce98d918195f3c94ca7df440da08ce5a173` (dirty) |
| stopping | `gsprt_accept_lower` — sequential stop: point estimates are selection-biased; the boundary decision is the evidence |

## Result

- **candidate win score**: `0.4722`  CI99 `[0.3889, 0.5000]`
- **candidate margin**: `-5.56`  CI99 `[-11.78, 0.33]`
- clusters `9`, legs `18` — wins `8`, draws `1`, losses `9`, invalid `0`, time-forfeits `0`

## Latency (referee-measured decision ms)

| artifact | decisions | mean | median | p95 | max | leg mean | legs over cap |
|---|---|---|---|---|---|---|---|
| baseline | 232 | 28.89 | 2.23 | 129.55 | 275.82 | 372.34 | 0 |
| candidate | 227 | 1052.32 | 19.26 | 5749.35 | 10487.73 | 13270.91 | 0 |

## Clusters

| seed | candidate-deals margin | status | baseline-deals margin | status |
|---|---|---|---|---|
| `180100` | 2.00 | valid (cand S) | -2.00 | valid (cand N) |
| `180101` | 0.00 | valid (cand S) | -32.00 | valid (cand N) |
| `180102` | 20.00 | valid (cand S) | -10.00 | valid (cand N) |
| `180103` | 2.00 | valid (cand S) | -10.00 | valid (cand N) |
| `180104` | -48.00 | valid (cand S) | 32.00 | valid (cand N) |
| `180105` | 10.00 | valid (cand S) | -40.00 | valid (cand N) |
| `180106` | -10.00 | valid (cand S) | 10.00 | valid (cand N) |
| `180107` | -2.00 | valid (cand S) | 4.00 | valid (cand N) |
| `180108` | 2.00 | valid (cand S) | -28.00 | valid (cand N) |

