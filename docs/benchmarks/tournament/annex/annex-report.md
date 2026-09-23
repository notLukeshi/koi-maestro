# Annex aggregate report — champion/runner-up vs the archetype suite

**Status**: measured evidence (annex legs played under the §0.5 protocol on
the shared 256-seed panel; small samples — pairs stopped at the GSPRT
`accept_lower` boundary, so per-cell counts are 9–20 clusters, not the
field's powered evidence). Release binary, dirty-tree build, binary-hash
bound (`freeze.json` sha256 `890228ce…`, FNV `ec4d3dfc…` in each report).

Champion `pimc` and runner-up `endgame` each played all 7 scripted
archetypes (`archetype` solver entrant, `seed=1`) — 14 pairs, 298 valid
legs, 0 invalid, 0 forfeits. Every leg contributes **both** mirrored
seats: `candidate_deals` has the archetype on South, `baseline_deals`
has the entrant on South; the entrant's win score is `1 −
candidate_win_score` in both legs. South carries a measurable first-move
advantage in this game — seat-split columns are reported so the aggregate
is never confounded with seat.

## Win-score matrix (entrant vs archetype, both seats)

| Archetype | `pimc` overall | `pimc` as South | `pimc` as North | `endgame` overall | `endgame` as South | `endgame` as North |
|---|---|---|---|---|---|---|
| banker | **0.577** | 0.615 | 0.538 | 0.500 | 0.444 | 0.556 |
| gambler | **0.722** | 0.667 | 0.778 | **0.750** | 0.722 | 0.778 |
| heuristic | **0.577** | 0.654 | 0.500 | **0.556** | 0.667 | 0.444 |
| materialist | **0.537** | 0.450 | 0.625 | **0.556** | 0.222 | 0.889 |
| random | **0.577** | 0.923 | 0.231 | **0.833** | 1.000 | 0.667 |
| timid | **0.639** | 0.778 | 0.500 | 0.500 | 0.556 | 0.444 |
| yaku_chaser | **0.833** | 0.833 | 0.833 | **0.917** | 0.944 | 0.889 |

- `pimc` beats **all 7** archetypes (win score > 0.5 in every cell).
- `endgame` beats **5 of 7**, tying `banker` and `timid` at exactly 0.500.
- Every pair stopped `gsprt_accept_lower` — the sequential test certified
  archetype inferiority before the cap; cells are floor-to-mid panel
  prefixes (9–20 clusters), so seat splits are descriptive, not powered.

## Per-pair provenance

| Pair | Clusters | Valid legs | Stopping | Entrant ws |
|---|---|---|---|---|
| pimc vs banker | 13 | 26 | gsprt_accept_lower | 0.577 |
| pimc vs gambler | 9 | 18 | gsprt_accept_lower | 0.722 |
| pimc vs heuristic | 13 | 26 | gsprt_accept_lower | 0.577 |
| pimc vs materialist | 20 | 40 | gsprt_accept_lower | 0.537 |
| pimc vs random | 13 | 26 | gsprt_accept_lower | 0.577 |
| pimc vs timid | 9 | 18 | gsprt_accept_lower | 0.639 |
| pimc vs yaku_chaser | 9 | 18 | gsprt_accept_lower | 0.833 |
| endgame vs banker | 9 | 18 | gsprt_accept_lower | 0.500 |
| endgame vs gambler | 9 | 18 | gsprt_accept_lower | 0.750 |
| endgame vs heuristic | 9 | 18 | gsprt_accept_lower | 0.556 |
| endgame vs materialist | 9 | 18 | gsprt_accept_lower | 0.556 |
| endgame vs random | 9 | 18 | gsprt_accept_lower | 0.833 |
| endgame vs timid | 9 | 18 | gsprt_accept_lower | 0.500 |
| endgame vs yaku_chaser | 9 | 18 | gsprt_accept_lower | 0.917 |

## Reading notes

- The annex answers "how badly do the top finishers beat the scripted
  portfolio" — it is an exploitation readout, not a ranking input.
  Archetypes are instruments (D-02), never champion candidates.
- Seat asymmetry is visible in single cells (e.g. `pimc` vs `random`:
  0.923 as South / 0.231 as North). Both-seats aggregation is the only
  honest headline; the earlier `candidate_deals`-only analysis inverted
  several cells and was corrected here.
- `endgame`'s two exact ties (banker, timid) are small-sample results:
  9 clusters each, margin evidence thin in both directions.
