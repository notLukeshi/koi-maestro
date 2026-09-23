# Koi-Koi Rules Reference (Engine Semantics)

Canonical, implementation-aligned rules summary. The executable source of truth
is `crates/koi-core/src/rules.rs` (rulesets) and `crates/koi-core/src/yaku.rs`
(scoring); this document is the human-readable contract they implement.

## Deck and deal

- 48 Hanafuda cards: 12 months × 4 cards (`card = month * 4 + index`).
- Deal: 8 cards per player, 8 face-up on the field, 24-card stock.
- Dealer (Oya) plays first; the winner of a round becomes dealer of the next.

## Turn structure (per turn)

1. **Hand phase**: play one hand card to the field — matching month captures;
   no match leaves the card on the field.
2. **Stock phase**: flip the top stock card and resolve captures identically.
   A card matching two field cards captures one; matching three captures all.
3. **Decision phase**: if the player formed/improved a Yaku, choose
   `KoiKoi` (continue) or `Shobu` (stop and score).

Koi-Koi requires: an active-player hand card, an opponent hand card, and at
least 2 remaining stock cards (both players must be able to draw next turn).

## Yaku and base points `V(Y)`

Nintendo is the baseline table; the only FudaWiki deltas are Gokō and Sankō
(`YakuPoints::nintendo()` / `YakuPoints::fuda_wiki()` in `rules.rs`).

| Category | Yaku | Nintendo | FudaWiki |
|---|---|---|---|
| Hikari | Gokō (5 lights) | 10 | **15** |
| Hikari | Shikō (4 lights, no Rain) | 8 | 8 |
| Hikari | Ame-Shikō (4 lights incl. Rain) | 7 | 7 |
| Hikari | Sankō (3 lights, no Rain) | 5 | **6** |
| Tane | Ino-Shika-Chō (Boar+Deer+Butterfly) | 5 +1/extra | 5 +1/extra |
| Tane | Tane (5 animals) | 1 +1/extra | 1 +1/extra |
| Tanzaku | Akatan (3 red ribbons) | 5 | 5 |
| Tanzaku | Aotan (3 blue ribbons) | 5 | 5 |
| Tanzaku | Akatan-Aotan (all 6) | 10 | 10 |
| Tanzaku | Ribbons (5) | 1 +1/extra | 1 +1/extra |
| Viewing | Hanami-de-ippai (Cherry + Sake Cup) | 5 | 5 |
| Viewing | Tsukimi-de-ippai (Moon + Sake Cup) | 5 | 5 |
| Kasu | Kasu (10 chaff) | 1 +1/extra | 1 +1/extra |

Sake Cup may count as Kasu under some rulesets
(`YakuPoints::sake_cup_counts_as_kasu`).

## Scoring multipliers

```
P = V(Y) × 2  if V(Y) ≥ 7 (Nintendo threshold; configurable)
         × 2  if the opponent called Koi-Koi this round
```

`best_yaku_per_category_only` selects the best single result per category
instead of stacking base yaku.

## Round end and match

- Round ends on `Shobu` (winner scores `P`), or when hands/stock exhaust.
- Koi-Koi and ≥7-point thresholds double as above; winner deals next.
- Null round (8 turns, no Yaku) resolution is ruleset-configurable:
  `DrawOpponentDeals` (Nintendo draw), `DealerKeepsDeal`,
  `DealerWinsOnePoint` (dealer keeps deal + 1 point), or `Redeal`
  (no round counter increment). See `Ruleset::null_round`.

## Deal anomalies

Detected at construction (`KoiGameState::check_deal`), returned as
`Option<DealAnomaly>` from `new_deal`/`new_from_parts`:

- `Teshi`: 4 same-month cards in an initial hand (instant win under most
  rulesets).
- `FourPairs`: 4 same-month pairs in a player's initial hand — eight cards
  covering four months exactly twice (instant win under most rulesets).
- `FieldVoid`: all 4 same-month cards on the field (redeal under most
  rulesets).

## Rulesets

`Ruleset::{Nintendo, FudaWiki, House}` — point tables, take-back Koi-Koi,
call limits, multipliers, null-round resolution, and anomaly handling are all
ruleset-parameterized. A solver/blueprint is always bound to one fixed
`Ruleset` for its lifetime.

Behavioral differences between the two built-in presets
(`Ruleset::house_rules()` in `rules.rs`):

| Flag | Nintendo | FudaWiki |
|---|---|---|
| `koi_koi_calls_per_round` | 1 | 255 (unlimited) |
| `take_back_koi_koi` | no | yes |
| `null_round` | `DrawOpponentDeals` | `DealerKeepsDeal` |
| `opponent_koi_koi_doubles` | yes | yes |
| `seven_plus_doubles` | yes | yes |
