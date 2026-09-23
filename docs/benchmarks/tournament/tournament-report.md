# Tournament: P6 field: 9-entrant definitive tournament

Entrants: **9**; pairings: **36**; clusters per pairing (declared panel): **256**  
Referee: `e3d43226b78dbc404ac2c33175c800a94c127a34` (dirty)  
Total invalid legs: **0**; total time-forfeit legs: **0**  
Hard worker deadline: `60` seconds per decision

## Provenance

| # | Entrant | Commit | Tree digest | Binary hash | Dirty |
|---|---|---|---|---|---|
| 0 | `random` | `e3d43226b78dbc404ac2c33175c800a94c127a34` | `e6511c283ea680f1fe331fa1eb43e5be54f33939` | `ec4d3dfc4db0aae0` | `true` |
| 1 | `heuristic` | `e3d43226b78dbc404ac2c33175c800a94c127a34` | `e6511c283ea680f1fe331fa1eb43e5be54f33939` | `ec4d3dfc4db0aae0` | `true` |
| 2 | `ismcts` | `e3d43226b78dbc404ac2c33175c800a94c127a34` | `e6511c283ea680f1fe331fa1eb43e5be54f33939` | `ec4d3dfc4db0aae0` | `true` |
| 3 | `pimc` | `e3d43226b78dbc404ac2c33175c800a94c127a34` | `e6511c283ea680f1fe331fa1eb43e5be54f33939` | `ec4d3dfc4db0aae0` | `true` |
| 4 | `endgame` | `e3d43226b78dbc404ac2c33175c800a94c127a34` | `e6511c283ea680f1fe331fa1eb43e5be54f33939` | `ec4d3dfc4db0aae0` | `true` |
| 5 | `resolving_fixed` | `e3d43226b78dbc404ac2c33175c800a94c127a34` | `e6511c283ea680f1fe331fa1eb43e5be54f33939` | `ec4d3dfc4db0aae0` | `true` |
| 6 | `resolving_adaptive` | `e3d43226b78dbc404ac2c33175c800a94c127a34` | `e6511c283ea680f1fe331fa1eb43e5be54f33939` | `ec4d3dfc4db0aae0` | `true` |
| 7 | `resolving_leaf` | `8e778ce98d918195f3c94ca7df440da08ce5a173` | `e6ef7fdddeed7341dd4162216ec04d8265c58929` | `b4ca8178e56ec323` | `true` |
| 8 | `leaf_policy` | `8e778ce98d918195f3c94ca7df440da08ce5a173` | `e6ef7fdddeed7341dd4162216ec04d8265c58929` | `b4ca8178e56ec323` | `true` |

Additional builds attested across pairings (same label + config hash; legs pooled across binaries — per-pairing attestation lives in each pair's report):

| Entrant | Commit | Tree digest | Binary hash | Dirty |
|---|---|---|---|---|
| `random` | `8e778ce98d918195f3c94ca7df440da08ce5a173` | `e6ef7fdddeed7341dd4162216ec04d8265c58929` | `b4ca8178e56ec323` | `true` |
| `heuristic` | `8e778ce98d918195f3c94ca7df440da08ce5a173` | `e6ef7fdddeed7341dd4162216ec04d8265c58929` | `b4ca8178e56ec323` | `true` |
| `ismcts` | `8e778ce98d918195f3c94ca7df440da08ce5a173` | `e6ef7fdddeed7341dd4162216ec04d8265c58929` | `b4ca8178e56ec323` | `true` |
| `pimc` | `8e778ce98d918195f3c94ca7df440da08ce5a173` | `e6ef7fdddeed7341dd4162216ec04d8265c58929` | `b4ca8178e56ec323` | `true` |
| `endgame` | `8e778ce98d918195f3c94ca7df440da08ce5a173` | `e6ef7fdddeed7341dd4162216ec04d8265c58929` | `b4ca8178e56ec323` | `true` |
| `resolving_fixed` | `8e778ce98d918195f3c94ca7df440da08ce5a173` | `e6ef7fdddeed7341dd4162216ec04d8265c58929` | `b4ca8178e56ec323` | `true` |
| `resolving_adaptive` | `8e778ce98d918195f3c94ca7df440da08ce5a173` | `e6ef7fdddeed7341dd4162216ec04d8265c58929` | `b4ca8178e56ec323` | `true` |

## Win score (row vs column)

| | `random` | `heuristic` | `ismcts` | `pimc` | `endgame` | `resolving_fixed` | `resolving_adaptive` | `resolving_leaf` | `leaf_policy` |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `random` | — | 0.167 | 0.355 | 0.117 | 0.208 | 0.292 | 0.250 | 0.208 | 0.265 |
| `heuristic` | 0.833 | — | 0.500 | 0.471 | 0.458 | 0.533 | 0.534 | 0.470 | 0.491 |
| `ismcts` | 0.645 | 0.500 | — | 0.292 | 0.403 | 0.409 | 0.556 | 0.339 | 0.411 |
| `pimc` | 0.883 | 0.529 | 0.708 | — | 0.474 | 0.667 | 0.500 | 0.353 | 0.543 |
| `endgame` | 0.792 | 0.542 | 0.597 | 0.526 | — | 0.510 | 0.583 | 0.500 | 0.611 |
| `resolving_fixed` | 0.708 | 0.467 | 0.591 | 0.333 | 0.490 | — | 0.556 | 0.500 | 0.492 |
| `resolving_adaptive` | 0.750 | 0.466 | 0.444 | 0.500 | 0.417 | 0.444 | — | 0.500 | 0.476 |
| `resolving_leaf` | 0.792 | 0.530 | 0.661 | 0.647 | 0.500 | 0.500 | 0.500 | — | 0.722 |
| `leaf_policy` | 0.735 | 0.509 | 0.589 | 0.457 | 0.389 | 0.508 | 0.524 | 0.278 | — |

## Mean margin (row vs column)

| | `random` | `heuristic` | `ismcts` | `pimc` | `endgame` | `resolving_fixed` | `resolving_adaptive` | `resolving_leaf` | `leaf_policy` |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `random` | — | -9.92 | -8.13 | -12.13 | -5.33 | -10.06 | -14.83 | -4.50 | -5.24 |
| `heuristic` | 9.92 | — | -3.17 | -2.35 | 0.81 | -5.27 | 0.27 | 0.76 | 1.22 |
| `ismcts` | 8.13 | 3.17 | — | -2.00 | -1.37 | -3.74 | 0.72 | -1.71 | 1.48 |
| `pimc` | 12.13 | 2.35 | 2.00 | — | 4.34 | 3.56 | 4.22 | -4.65 | 2.74 |
| `endgame` | 5.33 | -0.81 | 1.37 | -4.34 | — | -2.60 | -0.22 | -3.61 | 4.44 |
| `resolving_fixed` | 10.06 | 5.27 | 3.74 | -3.56 | 2.60 | — | 4.56 | 0.10 | 6.13 |
| `resolving_adaptive` | 14.83 | -0.27 | -0.72 | -4.22 | 0.22 | -4.56 | — | -0.20 | 3.52 |
| `resolving_leaf` | 4.50 | -0.76 | 1.71 | 4.65 | 3.61 | -0.10 | 0.20 | — | 7.11 |
| `leaf_policy` | 5.24 | -1.22 | -1.48 | -2.74 | -4.44 | -6.13 | -3.52 | -7.11 | — |

## Ranking (Davidson-ties Bradley–Terry)

Tie parameter nu: **0.0327** [0.0098, 0.0596]; log-likelihood -467.497; converged: `true` (bootstrap refits converged: 83.5%); stopping: `adaptive_stopping_schedule(22xgsprt_accept_lower,14xgsprt_accept_upper)_stopped_pairs_selection_biased`; headline fit on the 9-cluster common prefix  

| Entrant | W/D/L | Win score | Margin | Elo | x-Multiplier | Latency mean/p95 |
|---|---|---:|---:|---:|---:|---|
| `resolving_leaf` | 87/1/56 | 0.6076 [0.5139, 0.7083] | +3.514 [-0.069, +6.875] | +72.5 [+10.5, +145.7] | 4.50x [2.85, 9.06] | 1115.1 [1053.1, 1192.2] / 5438.9 [5169.8, 5889.2] ms |
| `endgame` | 83/2/59 | 0.5833 [0.5556, 0.6181] | +1.167 [-1.042, +3.500] | +56.3 [+36.7, +83.2] | 4.10x [2.63, 6.80] | 0.4 [0.2, 0.6] / 0.2 [0.2, 0.2] ms |
| `pimc` | 79/4/61 | 0.5625 [0.4722, 0.6389] | +2.708 [+0.597, +4.486] | +42.5 [-17.8, +94.5] | 3.78x [1.96, 6.42] | 83.2 [74.9, 91.9] / 329.5 [302.4, 359.5] ms |
| `heuristic` | 75/2/67 | 0.5278 [0.4583, 0.5972] | +0.000 [-2.833, +2.583] | +19.8 [-25.9, +67.2] | 3.32x [2.15, 5.79] | 0.4 [0.3, 0.6] / 0.2 [0.2, 0.2] ms |
| `leaf_policy` | 74/2/68 | 0.5208 [0.4514, 0.5868] | -3.431 [-6.292, -0.500] | +15.2 [-29.1, +59.8] | 3.23x [2.11, 5.61] | 30.2 [28.8, 31.7] / 100.5 [94.9, 107.0] ms |
| `resolving_fixed` | 71/3/70 | 0.5035 [0.3958, 0.5903] | +3.278 [-1.375, +7.917] | +3.9 [-68.4, +60.9] | 3.03x [1.71, 5.23] | 833.0 [802.6, 887.4] / 3659.1 [3520.1, 3936.7] ms |
| `resolving_adaptive` | 70/3/71 | 0.4965 [0.4167, 0.5660] | +0.569 [-3.472, +3.667] | -0.6 [-53.9, +45.7] | 2.95x [1.68, 4.71] | 802.2 [764.7, 832.2] / 3591.7 [3397.5, 3839.8] ms |
| `ismcts` | 66/2/76 | 0.4653 [0.4028, 0.5139] | +0.986 [-2.278, +3.903] | -21.0 [-63.2, +12.4] | 2.63x [1.65, 4.42] | 51.8 [47.0, 58.7] / 155.4 [134.9, 187.3] ms |
| `random` | 33/1/110 | 0.2326 [0.1632, 0.3160] | -8.792 [-10.208, -7.375] | -188.7 [-260.0, -121.9] | 1.00x [1.00, 1.00] | 0.3 [0.2, 0.4] / 0.2 [0.2, 0.2] ms |

*Legend: win score is the {0, 0.5, 1} leg mean; margin is the mean yaku-point difference; Elo is the Davidson-ties Bradley-Terry strength (gamma normalized to geometric mean 1, reported as 400*log10 gamma centered to mean 0); the x-multiplier is the strength ratio gamma_i/gamma_min -- a win-score ratio degenerates when the floor is ~0; latency is the referee-measured per-decision milliseconds. Every number carries a 99% cluster-bootstrap interval over 10000 resamples (one deal-index draw over the common prefix applied jointly to every pairing -- the mirrored-panel covariance across pairings is preserved). The headline fit uses only the common prefix shared by every pairing, so sequentially stopped pairs contribute outcome-independent evidence; stopping provenance: `adaptive_stopping_schedule(22xgsprt_accept_lower,14xgsprt_accept_upper)_stopped_pairs_selection_biased`. Rankings are the best among the evaluated configurations under the frozen protocol and budget -- measured tournament evidence, not a certified optimality claim.*

### Descriptive full-data fit (selection-biased -- not the headline)

*The fit below pools each pairing's untruncated evidence, including clusters the stopping rule selected. Sequential stopping makes this basis selection-biased; it is reported for context only and carries no intervals.*

| Entrant | W/D/L (full data) | Elo (full-data fit) |
|---|---:|---:|
| `resolving_leaf` | 179/2/133 | +60.4 |
| `endgame` | 227/6/175 | +50.2 |
| `pimc` | 180/8/144 | +48.2 |
| `heuristic` | 227/7/226 | +25.9 |
| `resolving_fixed` | 216/9/181 | +20.0 |
| `leaf_policy` | 239/14/207 | +15.2 |
| `resolving_adaptive` | 136/6/140 | +0.4 |
| `ismcts` | 243/16/313 | -39.1 |
| `random` | 64/2/192 | -181.1 |

*Full-data fit: log-likelihood -1310.228, nu 0.0421, converged: `false`; clusters per pairing: [12, 31, 15, 12, 18, 12, 12, 17, 12, 52, 48, 15, 22, 42, 27, 12, 49, 77, 18, 14, 73, 29, 9, 9, 17, 23, 25, 9, 23, 9, 9, 20, 30, 20, 42, 9].*

**Intransitivity detected: 6 of 84 triples (7.1%) are strict win-score cycles.** The scalar BT rating can mislead under cycles — the Nash-averaging mixture below is the reliable summary. Witnessed cycles (i beats j beats k beats i):

- `ismcts` → `resolving_adaptive` → `resolving_leaf` → `ismcts`
- `pimc` → `resolving_fixed` → `endgame` → `pimc`
- `endgame` → `leaf_policy` → `resolving_fixed` → `endgame`
- `resolving_fixed` → `resolving_adaptive` → `resolving_leaf` → `resolving_fixed`
- `resolving_adaptive` → `resolving_leaf` → `leaf_policy` → `resolving_adaptive`
- `heuristic` → `pimc` → `ismcts` → `heuristic`

| Entrant | Nash weight | Expected win score under the mixture |
|---|---:|---:|
| `random` | 0.0000 | 0.1898 |
| `heuristic` | 0.0000 | 0.4447 |
| `ismcts` | 0.0000 | 0.3729 |
| `pimc` | 0.0000 | 0.4011 |
| `endgame` | 0.4010 | 0.5000 |
| `resolving_fixed` | 0.0030 | 0.4894 |
| `resolving_adaptive` | 0.0023 | 0.4994 |
| `resolving_leaf` | 0.5936 | 0.5000 |
| `leaf_policy` | 0.0000 | 0.3240 |

## Pairwise comparisons (Holm step-down, family alpha = 0.010)

Each row tests one played pairing's win-score advantage (`second` − even) against level play; raw p is the two-sided recentered cluster-bootstrap exceedance, the CI is its descriptive percentile interval. The Holm column controls the family-wise error rate across all 36 tests at alpha = 0.010 — rows marked **separated** are the multiplicity-controlled claims; everything else is descriptive only.

| First | Second | Second advantage | CI | p (raw) | p (Holm) | Verdict |
|---|---|---:|---:|---:|---:|---|
| `random` | `pimc` | +0.4167 | [+0.2500, +0.5000] | 0.0001 | 0.0036 | **separated** |
| `random` | `resolving_leaf` | +0.3333 | [+0.1111, +0.5000] | 0.0001 | 0.0036 | **separated** |
| `random` | `heuristic` | +0.3333 | [+0.1111, +0.5000] | 0.0002 | 0.0068 | **separated** |
| `random` | `leaf_policy` | +0.2778 | [+0.0556, +0.4444] | 0.0004 | 0.0132 | level |
| `random` | `endgame` | +0.2778 | [+0.0556, +0.5000] | 0.0008 | 0.0256 | level |
| `resolving_leaf` | `leaf_policy` | -0.2222 | [-0.4444, -0.0556] | 0.0124 | 0.3844 | level |
| `random` | `ismcts` | +0.1667 | [+0.0000, +0.3889] | 0.0652 | 1.0000 | level |
| `random` | `resolving_fixed` | +0.1667 | [-0.1111, +0.4444] | 0.2037 | 1.0000 | level |
| `random` | `resolving_adaptive` | +0.1667 | [+0.0000, +0.3889] | 0.0679 | 1.0000 | level |
| `heuristic` | `ismcts` | +0.0278 | [+0.0000, +0.1111] | 0.6158 | 1.0000 | level |
| `heuristic` | `pimc` | -0.0556 | [-0.2778, +0.1667] | 0.7698 | 1.0000 | level |
| `heuristic` | `endgame` | +0.0556 | [+0.0000, +0.2222] | 0.6029 | 1.0000 | level |
| `heuristic` | `resolving_fixed` | +0.0000 | [-0.2222, +0.2222] | 1.0000 | 1.0000 | level |
| `heuristic` | `resolving_adaptive` | +0.0278 | [-0.1111, +0.2222] | 0.8471 | 1.0000 | level |
| `heuristic` | `resolving_leaf` | +0.0556 | [-0.1667, +0.2778] | 0.7694 | 1.0000 | level |
| `heuristic` | `leaf_policy` | +0.0000 | [-0.2222, +0.2222] | 1.0000 | 1.0000 | level |
| `ismcts` | `pimc` | +0.1667 | [+0.0000, +0.3889] | 0.0727 | 1.0000 | level |
| `ismcts` | `endgame` | +0.1111 | [+0.0000, +0.3333] | 0.2264 | 1.0000 | level |
| `ismcts` | `resolving_fixed` | +0.0556 | [-0.1667, +0.2778] | 0.7662 | 1.0000 | level |
| `ismcts` | `resolving_adaptive` | -0.0556 | [-0.2778, +0.1667] | 0.7670 | 1.0000 | level |
| `ismcts` | `resolving_leaf` | +0.1389 | [+0.0000, +0.3333] | 0.0712 | 1.0000 | level |
| `ismcts` | `leaf_policy` | +0.0556 | [+0.0000, +0.2222] | 0.6158 | 1.0000 | level |
| `pimc` | `endgame` | +0.0833 | [+0.0000, +0.2500] | 0.2216 | 1.0000 | level |
| `pimc` | `resolving_fixed` | -0.1667 | [-0.3889, +0.0000] | 0.0677 | 1.0000 | level |
| `pimc` | `resolving_adaptive` | +0.0000 | [+0.0000, +0.0000] | 1.0000 | 1.0000 | level |
| `pimc` | `resolving_leaf` | +0.1111 | [+0.0000, +0.3333] | 0.2264 | 1.0000 | level |
| `pimc` | `leaf_policy` | +0.0000 | [-0.2222, +0.2222] | 1.0000 | 1.0000 | level |
| `endgame` | `resolving_fixed` | +0.0556 | [-0.1667, +0.3333] | 0.7650 | 1.0000 | level |
| `endgame` | `resolving_adaptive` | -0.0833 | [-0.3056, +0.0833] | 0.3994 | 1.0000 | level |
| `endgame` | `resolving_leaf` | +0.0000 | [-0.2778, +0.2778] | 1.0000 | 1.0000 | level |
| `endgame` | `leaf_policy` | -0.1111 | [-0.2778, +0.0000] | 0.2273 | 1.0000 | level |
| `resolving_fixed` | `resolving_adaptive` | -0.0556 | [-0.2222, +0.0000] | 0.6131 | 1.0000 | level |
| `resolving_fixed` | `resolving_leaf` | +0.0556 | [-0.1667, +0.2778] | 0.7642 | 1.0000 | level |
| `resolving_fixed` | `leaf_policy` | +0.0833 | [-0.1667, +0.3333] | 0.4803 | 1.0000 | level |
| `resolving_adaptive` | `resolving_leaf` | -0.0556 | [-0.2778, +0.1667] | 0.7636 | 1.0000 | level |
| `resolving_adaptive` | `leaf_policy` | +0.0833 | [+0.0000, +0.2500] | 0.2225 | 1.0000 | level |
## Pairings

| First | Second | Second win score | Second margin | Stopping | Invalid | Time forfeit |
|---|---|---:|---:|---|---:|---:|
| `random` | `heuristic` | 0.833333 | 9.917 | `gsprt_accept_upper` | 0 | 0 |
| `random` | `ismcts` | 0.645161 | 8.129 | `gsprt_accept_upper` | 0 | 0 |
| `random` | `pimc` | 0.883333 | 12.133 | `gsprt_accept_upper` | 0 | 0 |
| `random` | `endgame` | 0.791667 | 5.333 | `gsprt_accept_upper` | 0 | 0 |
| `random` | `resolving_fixed` | 0.708333 | 10.056 | `gsprt_accept_upper` | 0 | 0 |
| `random` | `resolving_adaptive` | 0.750000 | 14.833 | `gsprt_accept_upper` | 0 | 0 |
| `random` | `resolving_leaf` | 0.791667 | 4.500 | `gsprt_accept_upper` | 0 | 0 |
| `random` | `leaf_policy` | 0.735294 | 5.235 | `gsprt_accept_upper` | 0 | 0 |
| `heuristic` | `ismcts` | 0.500000 | 3.167 | `gsprt_accept_lower` | 0 | 0 |
| `heuristic` | `pimc` | 0.528846 | 2.346 | `gsprt_accept_lower` | 0 | 0 |
| `heuristic` | `endgame` | 0.541667 | -0.812 | `gsprt_accept_lower` | 0 | 0 |
| `heuristic` | `resolving_fixed` | 0.466667 | 5.267 | `gsprt_accept_lower` | 0 | 0 |
| `heuristic` | `resolving_adaptive` | 0.465909 | -0.273 | `gsprt_accept_lower` | 0 | 0 |
| `heuristic` | `resolving_leaf` | 0.529762 | -0.762 | `gsprt_accept_lower` | 0 | 0 |
| `heuristic` | `leaf_policy` | 0.509259 | -1.222 | `gsprt_accept_lower` | 0 | 0 |
| `ismcts` | `pimc` | 0.708333 | 2.000 | `gsprt_accept_upper` | 0 | 0 |
| `ismcts` | `endgame` | 0.596939 | 1.367 | `gsprt_accept_upper` | 0 | 0 |
| `ismcts` | `resolving_fixed` | 0.590909 | 3.740 | `gsprt_accept_upper` | 0 | 0 |
| `ismcts` | `resolving_adaptive` | 0.444444 | -0.722 | `gsprt_accept_lower` | 0 | 0 |
| `ismcts` | `resolving_leaf` | 0.660714 | 1.714 | `gsprt_accept_upper` | 0 | 0 |
| `ismcts` | `leaf_policy` | 0.589041 | -1.479 | `gsprt_accept_upper` | 0 | 0 |
| `pimc` | `endgame` | 0.525862 | -4.345 | `gsprt_accept_lower` | 0 | 0 |
| `pimc` | `resolving_fixed` | 0.333333 | -3.556 | `gsprt_accept_lower` | 0 | 0 |
| `pimc` | `resolving_adaptive` | 0.500000 | -4.222 | `gsprt_accept_lower` | 0 | 0 |
| `pimc` | `resolving_leaf` | 0.647059 | 4.647 | `gsprt_accept_upper` | 0 | 0 |
| `pimc` | `leaf_policy` | 0.456522 | -2.739 | `gsprt_accept_lower` | 0 | 0 |
| `endgame` | `resolving_fixed` | 0.490000 | 2.600 | `gsprt_accept_lower` | 0 | 0 |
| `endgame` | `resolving_adaptive` | 0.416667 | 0.222 | `gsprt_accept_lower` | 0 | 0 |
| `endgame` | `resolving_leaf` | 0.500000 | 3.609 | `gsprt_accept_lower` | 0 | 0 |
| `endgame` | `leaf_policy` | 0.388889 | -4.444 | `gsprt_accept_lower` | 0 | 0 |
| `resolving_fixed` | `resolving_adaptive` | 0.444444 | -4.556 | `gsprt_accept_lower` | 0 | 0 |
| `resolving_fixed` | `resolving_leaf` | 0.500000 | -0.100 | `gsprt_accept_lower` | 0 | 0 |
| `resolving_fixed` | `leaf_policy` | 0.508333 | -6.133 | `gsprt_accept_lower` | 0 | 0 |
| `resolving_adaptive` | `resolving_leaf` | 0.500000 | 0.200 | `gsprt_accept_lower` | 0 | 0 |
| `resolving_adaptive` | `leaf_policy` | 0.523810 | -3.524 | `gsprt_accept_lower` | 0 | 0 |
| `resolving_leaf` | `leaf_policy` | 0.277778 | -7.111 | `gsprt_accept_lower` | 0 | 0 |

