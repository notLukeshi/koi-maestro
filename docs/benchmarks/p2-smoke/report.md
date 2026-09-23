# Koi-Maestro P2 solver certificates — p2-solver-certificates

**verdict: `development`**

| field | value |
|---|---|
| manifest | `5360ab4d8fe22ff0` |
| referee | `0ed00701b942782ac877d71c9b1f54430c380d6c` |
| dirty | `true` |

## Exact LP certificate

- domain `nano_6`: 2519 nodes, 1062 infosets, 315 south / 845 north sequences (266175 payoff cells)
- value (South) `1.777778`, value (North) `-1.777778`
- joint-profile exploitability `0.000e0` vs tolerance `1.0e-6` — **CERTIFIED epsilon-Nash**
- wall `1.285` s

Scope: exact equilibrium of the declared reduced domain only — not a claim about the full 48-card game.

## CFR-family convergence

Domain `micro_8`: 41161 nodes, 18672 infosets.

| algorithm | iterations | exploitability | value (South) | wall s |
|---|---|---|---|---|
| cfr_plus | 10 | 0.000637 | 0.4273 | 0.05 |
| cfr_plus | 30 | 0.000071 | 0.4284 | 0.10 |
| cfr_plus | 100 | 0.000006 | 0.4286 | 0.36 |
| cfr_plus | 300 | 0.000001 | 0.4286 | 1.12 |
| cfr_plus | 1000 | 0.000000 | 0.4286 | 3.95 |
| cfr_plus | 3000 | 0.000000 | 0.4286 | 8.96 |
| dcfr | 10 | 0.000291 | 0.4280 | 0.04 |
| dcfr | 30 | 0.000013 | 0.4285 | 0.10 |
| dcfr | 100 | 0.000000 | 0.4286 | 0.29 |
| dcfr | 300 | 0.000000 | 0.4286 | 0.82 |
| dcfr | 1000 | 0.000000 | 0.4286 | 2.95 |
| dcfr | 3000 | 0.000000 | 0.4286 | 9.44 |
| pcfr_plus | 10 | 0.000291 | 0.4280 | 0.03 |
| pcfr_plus | 30 | 0.000013 | 0.4285 | 0.10 |
| pcfr_plus | 100 | 0.000000 | 0.4286 | 0.34 |
| pcfr_plus | 300 | 0.000000 | 0.4286 | 1.04 |
| pcfr_plus | 1000 | 0.000000 | 0.4286 | 3.40 |
| pcfr_plus | 3000 | 0.000000 | 0.4286 | 13.38 |
| mccfr | 500 | 0.066632 | 0.2953 | 0.01 |
| mccfr | 2000 | 0.052210 | 0.3242 | 0.01 |
| mccfr | 10000 | 0.022463 | 0.3836 | 0.04 |
| mccfr | 40000 | 0.006946 | 0.4147 | 0.11 |
| ccs_mccfr | 500 | 0.061360 | 0.3059 | 0.00 |
| ccs_mccfr | 2000 | 0.047399 | 0.3338 | 0.01 |
| ccs_mccfr | 10000 | 0.020275 | 0.3880 | 0.04 |
| ccs_mccfr | 40000 | 0.005898 | 0.4168 | 0.15 |

Scope: exploitability is the exact best-response measure on the declared reduced domain; sampled curves carry their declared seed.
