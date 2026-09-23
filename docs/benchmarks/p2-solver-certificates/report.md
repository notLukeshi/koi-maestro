# Koi-Maestro P2 solver certificates — p2-solver-certificates

**verdict: `canonical`**

- exact results cover the declared reduced domains only — not the full 48-card game
- sampled algorithms (mccfr, ccs_mccfr) carry their declared seeds

| field | value |
|---|---|
| manifest | `5360ab4d8fe22ff0` |
| referee | `545bcf23e8c3b79915f741433d43306db459d332` |
| dirty | `false` |
| profile | `release` |

## Exact LP certificate

- domain `nano_6`: 2519 nodes, 1062 infosets, 315 south / 845 north sequences (266175 payoff cells)
- value (South) `1.777778`, value (North) `-1.777778`
- joint-profile exploitability `0.000e0` vs tolerance `1.0e-6` — **CERTIFIED epsilon-Nash**
- wall `1.548` s

Scope: exact equilibrium of the declared reduced domain only — not a claim about the full 48-card game.

## CFR-family convergence

Domain `micro_8`: 41161 nodes, 18672 infosets.

| algorithm | iterations | exploitability | value (South) | wall s |
|---|---|---|---|---|
| cfr_plus | 10 | 0.002832 | 0.4229 | 0.03 |
| cfr_plus | 30 | 0.000317 | 0.4279 | 0.09 |
| cfr_plus | 100 | 0.000029 | 0.4285 | 0.32 |
| cfr_plus | 300 | 0.000003 | 0.4286 | 0.79 |
| cfr_plus | 1000 | 0.000000 | 0.4286 | 1.38 |
| cfr_plus | 3000 | 0.000000 | 0.4286 | 4.17 |
| dcfr | 10 | 0.001293 | 0.4260 | 0.04 |
| dcfr | 30 | 0.000058 | 0.4285 | 0.12 |
| dcfr | 100 | 0.000002 | 0.4286 | 0.39 |
| dcfr | 300 | 0.000000 | 0.4286 | 1.18 |
| dcfr | 1000 | 0.000000 | 0.4286 | 2.87 |
| dcfr | 3000 | 0.000000 | 0.4286 | 4.95 |
| pcfr_plus | 10 | 0.001293 | 0.4260 | 0.03 |
| pcfr_plus | 30 | 0.000058 | 0.4285 | 0.09 |
| pcfr_plus | 100 | 0.000002 | 0.4286 | 0.27 |
| pcfr_plus | 300 | 0.000000 | 0.4286 | 0.97 |
| pcfr_plus | 1000 | 0.000000 | 0.4286 | 3.10 |
| pcfr_plus | 3000 | 0.000000 | 0.4286 | 5.65 |
| mccfr | 500 | 0.066632 | 0.2953 | 0.01 |
| mccfr | 2000 | 0.052210 | 0.3242 | 0.01 |
| mccfr | 10000 | 0.022463 | 0.3836 | 0.02 |
| mccfr | 40000 | 0.006946 | 0.4147 | 0.07 |
| ccs_mccfr | 500 | 0.061360 | 0.3059 | 0.00 |
| ccs_mccfr | 2000 | 0.047399 | 0.3338 | 0.01 |
| ccs_mccfr | 10000 | 0.020275 | 0.3880 | 0.02 |
| ccs_mccfr | 40000 | 0.005898 | 0.4168 | 0.07 |

Scope: exploitability is the exact best-response measure on the declared reduced domain; sampled curves carry their declared seed.
