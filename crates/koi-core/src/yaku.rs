//! Yaku (scoring combination) definitions and evaluation.

use crate::card::{Card, CardSet, Month};
use crate::rules::{Ruleset, YakuPoints};

/// Ribbon type for Tanzaku.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RibbonKind {
    /// Red poetry ribbon (Jan, Feb, Mar). Contributes to Akatan.
    RedPoetry,
    /// Plain red ribbon (Apr, May, Jul, Nov). Counts as Tanzaku but no named Yaku.
    Red,
    /// Blue ribbon (Jun, Sep, Oct). Contributes to Aotan.
    Blue,
    /// Plain / lesser ribbon (not used in the standard 48-card mapping).
    Plain,
}

/// The primary rank of a Hanafuda card for scoring.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CardRank {
    /// Hikari (light) card.
    Hikari,
    /// Tane (animal) card.
    Tane,
    /// Tanzaku (ribbon) card.
    Tanzaku(RibbonKind),
    /// Kasu (chaff/plain) card.
    Kasu,
    /// The Sake Cup from September.
    SakeCup,
}

/// Static attributes for every card in the deck.
#[derive(Debug, Clone, Copy)]
pub struct CardAttributes {
    pub card: Card,
    pub month: Month,
    pub rank: CardRank,
}

/// The fixed attributes for all 48 Hanafuda cards.
///
/// The mapping is the standard one used in Nintendo and Fuda-Wiki rules:
/// per month, index 0 is the Hikari or Tane, index 1 is the Tanzaku
/// (when present), and the remaining indices are Kasu.
#[rustfmt::skip]
pub const CARD_ATTRIBUTES: [CardAttributes; 48] = {
    use CardRank::*;
    use RibbonKind::*;
    // SAFETY: all indices are within 0..48 and months within 0..12.
    [
        // January — Pine
        CardAttributes { card: Card::new_unchecked(0),  month: Month::new_unchecked(0),  rank: Hikari },
        CardAttributes { card: Card::new_unchecked(1),  month: Month::new_unchecked(0),  rank: Tanzaku(RedPoetry) },
        CardAttributes { card: Card::new_unchecked(2),  month: Month::new_unchecked(0),  rank: Kasu },
        CardAttributes { card: Card::new_unchecked(3),  month: Month::new_unchecked(0),  rank: Kasu },
        // February — Plum
        CardAttributes { card: Card::new_unchecked(4),  month: Month::new_unchecked(1),  rank: Tane },
        CardAttributes { card: Card::new_unchecked(5),  month: Month::new_unchecked(1),  rank: Tanzaku(RedPoetry) },
        CardAttributes { card: Card::new_unchecked(6),  month: Month::new_unchecked(1),  rank: Kasu },
        CardAttributes { card: Card::new_unchecked(7),  month: Month::new_unchecked(1),  rank: Kasu },
        // March — Cherry Blossom
        CardAttributes { card: Card::new_unchecked(8),  month: Month::new_unchecked(2),  rank: Hikari },
        CardAttributes { card: Card::new_unchecked(9),  month: Month::new_unchecked(2),  rank: Tanzaku(RedPoetry) },
        CardAttributes { card: Card::new_unchecked(10), month: Month::new_unchecked(2),  rank: Kasu },
        CardAttributes { card: Card::new_unchecked(11), month: Month::new_unchecked(2),  rank: Kasu },
        // April — Wisteria
        CardAttributes { card: Card::new_unchecked(12), month: Month::new_unchecked(3),  rank: Tane },
        CardAttributes { card: Card::new_unchecked(13), month: Month::new_unchecked(3),  rank: Tanzaku(Red) },
        CardAttributes { card: Card::new_unchecked(14), month: Month::new_unchecked(3),  rank: Kasu },
        CardAttributes { card: Card::new_unchecked(15), month: Month::new_unchecked(3),  rank: Kasu },
        // May — Iris
        CardAttributes { card: Card::new_unchecked(16), month: Month::new_unchecked(4),  rank: Tane },
        CardAttributes { card: Card::new_unchecked(17), month: Month::new_unchecked(4),  rank: Tanzaku(Red) },
        CardAttributes { card: Card::new_unchecked(18), month: Month::new_unchecked(4),  rank: Kasu },
        CardAttributes { card: Card::new_unchecked(19), month: Month::new_unchecked(4),  rank: Kasu },
        // June — Peony
        CardAttributes { card: Card::new_unchecked(20), month: Month::new_unchecked(5),  rank: Tane },
        CardAttributes { card: Card::new_unchecked(21), month: Month::new_unchecked(5),  rank: Tanzaku(Blue) },
        CardAttributes { card: Card::new_unchecked(22), month: Month::new_unchecked(5),  rank: Kasu },
        CardAttributes { card: Card::new_unchecked(23), month: Month::new_unchecked(5),  rank: Kasu },
        // July — Bush Clover
        CardAttributes { card: Card::new_unchecked(24), month: Month::new_unchecked(6),  rank: Tane },
        CardAttributes { card: Card::new_unchecked(25), month: Month::new_unchecked(6),  rank: Tanzaku(Red) },
        CardAttributes { card: Card::new_unchecked(26), month: Month::new_unchecked(6),  rank: Kasu },
        CardAttributes { card: Card::new_unchecked(27), month: Month::new_unchecked(6),  rank: Kasu },
        // August — Pampas
        CardAttributes { card: Card::new_unchecked(28), month: Month::new_unchecked(7),  rank: Hikari },
        CardAttributes { card: Card::new_unchecked(29), month: Month::new_unchecked(7),  rank: Tane },
        CardAttributes { card: Card::new_unchecked(30), month: Month::new_unchecked(7),  rank: Kasu },
        CardAttributes { card: Card::new_unchecked(31), month: Month::new_unchecked(7),  rank: Kasu },
        // September — Chrysanthemum
        CardAttributes { card: Card::new_unchecked(32), month: Month::new_unchecked(8),  rank: SakeCup },
        CardAttributes { card: Card::new_unchecked(33), month: Month::new_unchecked(8),  rank: Tanzaku(Blue) },
        CardAttributes { card: Card::new_unchecked(34), month: Month::new_unchecked(8),  rank: Kasu },
        CardAttributes { card: Card::new_unchecked(35), month: Month::new_unchecked(8),  rank: Kasu },
        // October — Maple
        CardAttributes { card: Card::new_unchecked(36), month: Month::new_unchecked(9),  rank: Tane },
        CardAttributes { card: Card::new_unchecked(37), month: Month::new_unchecked(9),  rank: Tanzaku(Blue) },
        CardAttributes { card: Card::new_unchecked(38), month: Month::new_unchecked(9),  rank: Kasu },
        CardAttributes { card: Card::new_unchecked(39), month: Month::new_unchecked(9),  rank: Kasu },
        // November — Willow
        CardAttributes { card: Card::new_unchecked(40), month: Month::new_unchecked(10), rank: Hikari },
        CardAttributes { card: Card::new_unchecked(41), month: Month::new_unchecked(10), rank: Tane },
        CardAttributes { card: Card::new_unchecked(42), month: Month::new_unchecked(10), rank: Tanzaku(Red) },
        CardAttributes { card: Card::new_unchecked(43), month: Month::new_unchecked(10), rank: Kasu },
        // December — Paulownia
        CardAttributes { card: Card::new_unchecked(44), month: Month::new_unchecked(11), rank: Hikari },
        CardAttributes { card: Card::new_unchecked(45), month: Month::new_unchecked(11), rank: Kasu },
        CardAttributes { card: Card::new_unchecked(46), month: Month::new_unchecked(11), rank: Kasu },
        CardAttributes { card: Card::new_unchecked(47), month: Month::new_unchecked(11), rank: Kasu },
    ]
};

/// Returns the attributes for a card.
#[inline]
pub const fn attributes(card: Card) -> CardAttributes {
    CARD_ATTRIBUTES[card.index() as usize]
}

/// Returns the rank of a card.
#[inline]
pub const fn rank(card: Card) -> CardRank {
    attributes(card).rank
}

/// Returns `true` if the card counts as Tane (animal) for the given ruleset.
///
/// The Sake Cup is a Tane by default.
pub const fn is_tane(card: Card) -> bool {
    matches!(rank(card), CardRank::Tane | CardRank::SakeCup)
}

/// Returns `true` if the card counts as Hikari (light).
pub const fn is_hikari(card: Card) -> bool {
    matches!(rank(card), CardRank::Hikari)
}

/// Returns `true` if the card counts as Tanzaku (ribbon).
pub const fn is_tanzaku(card: Card) -> bool {
    matches!(rank(card), CardRank::Tanzaku(_))
}

/// Returns `true` if the card counts as Kasu (chaff).
///
/// The Sake Cup does **not** count as Kasu by default; the ruleset decides
/// whether to include it.
pub const fn is_kasu(card: Card) -> bool {
    matches!(rank(card), CardRank::Kasu)
}

/// Counts cards of a given predicate in a set.
pub fn count_in_set(set: CardSet, predicate: fn(Card) -> bool) -> u32 {
    set.into_iter().filter(|&c| predicate(c)).count() as u32
}

/// Counts Hikari cards in a captured set.
pub fn hikari_count(set: CardSet) -> u32 {
    count_in_set(set, is_hikari)
}

/// Counts Tane cards in a captured set.
pub fn tane_count(set: CardSet) -> u32 {
    count_in_set(set, is_tane)
}

/// Counts Tanzaku cards in a captured set.
pub fn tanzaku_count(set: CardSet) -> u32 {
    count_in_set(set, is_tanzaku)
}

/// Counts Kasu cards in a captured set according to the ruleset.
pub fn kasu_count(set: CardSet, points: &YakuPoints) -> u32 {
    let mut count = count_in_set(set, is_kasu);
    if set.contains(Card::new_unchecked(32)) {
        if points.sake_cup_double_kasu {
            count += 2;
        } else if points.sake_cup_counts_as_kasu {
            count += 1;
        }
    }
    count
}

/// Returns the set of red-poetry Tanzaku cards (Akatan).
pub const fn akatan_cards() -> CardSet {
    CardSet::from_card(Card::new_unchecked(1))
        .union(CardSet::from_card(Card::new_unchecked(5)))
        .union(CardSet::from_card(Card::new_unchecked(9)))
}

/// Returns the set of blue Tanzaku cards (Aotan).
pub const fn aotan_cards() -> CardSet {
    CardSet::from_card(Card::new_unchecked(21))
        .union(CardSet::from_card(Card::new_unchecked(33)))
        .union(CardSet::from_card(Card::new_unchecked(37)))
}

/// Returns the Ino-Shika-Chō (Boar, Deer, Butterflies) Tane set.
pub const fn inoshikacho_cards() -> CardSet {
    CardSet::from_card(Card::new_unchecked(20)) // Butterflies (Jun)
        .union(CardSet::from_card(Card::new_unchecked(24))) // Boar (Jul)
        .union(CardSet::from_card(Card::new_unchecked(36))) // Deer (Oct)
}

/// Returns the Hanami (Cherry Curtain + Sake Cup) card set.
pub const fn hanami_cards() -> CardSet {
    CardSet::from_card(Card::new_unchecked(8)) // Curtain (Mar)
        .union(CardSet::from_card(Card::new_unchecked(32))) // Sake Cup (Sep)
}

/// Returns the Tsukimi (Moon + Sake Cup) card set.
pub const fn tsukimi_cards() -> CardSet {
    CardSet::from_card(Card::new_unchecked(28)) // Moon (Aug)
        .union(CardSet::from_card(Card::new_unchecked(32))) // Sake Cup (Sep)
}

/// A named Yaku with a point value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Yaku {
    pub name: &'static str,
    pub points: u32,
}

/// Computes the base Yaku score for a captured set under a ruleset.
pub fn score_yaku(captured: CardSet, rules: &Ruleset) -> u32 {
    let points = rules.yaku_points();

    score_hikari(captured, &points)
        + score_tane(captured, &points)
        + score_tanzaku(captured, &points)
        + score_viewing(captured, &points)
        + score_kasu(captured, &points)
}

fn score_hikari(captured: CardSet, points: &YakuPoints) -> u32 {
    let count = hikari_count(captured);
    let has_rain = captured.contains(Card::new_unchecked(40));

    match (count, has_rain) {
        (5, _) => points.gokou,
        (4, false) => points.shikou,
        (4, true) => points.ame_shikou,
        (3, false) => points.sankou,
        _ => 0,
    }
}

fn score_tane(captured: CardSet, points: &YakuPoints) -> u32 {
    let count = tane_count(captured);
    if count < 3 {
        return 0;
    }

    let generic_tane = || {
        if count >= points.tane_threshold {
            points.tane_base + (count - points.tane_threshold) * points.tane_extra
        } else {
            0
        }
    };

    if inoshikacho_cards().is_subset(captured) {
        if points.best_yaku_per_category_only {
            // The single best yaku in this category is the Ino-Shika-Chō combo
            // plus one point for each additional animal beyond the three.
            return points.inoshikacho + (count - 3) * points.tane_extra;
        }
        // Stack the base combo with the generic "any five animals" yaku.
        return points.inoshikacho + generic_tane();
    }

    generic_tane()
}

fn score_tanzaku(captured: CardSet, points: &YakuPoints) -> u32 {
    let has_akatan = akatan_cards().is_subset(captured);
    let has_aotan = aotan_cards().is_subset(captured);
    let count = tanzaku_count(captured);

    let generic_tanzaku = || {
        if count >= points.tanzaku_threshold {
            points.tanzaku_base + (count - points.tanzaku_threshold) * points.tanzaku_extra
        } else {
            0
        }
    };

    if has_akatan && has_aotan {
        if points.best_yaku_per_category_only {
            return points.akatan_aotan + count.saturating_sub(6) * points.tanzaku_extra;
        }
        return points.akatan_aotan + generic_tanzaku();
    }

    if has_akatan || has_aotan {
        if points.best_yaku_per_category_only {
            let (base, combo_size) = if has_akatan {
                (points.akatan, 3)
            } else {
                (points.aotan, 3)
            };
            return base + (count - combo_size) * points.tanzaku_extra;
        }
        let named = if has_akatan { points.akatan } else { points.aotan };
        return named + generic_tanzaku();
    }

    generic_tanzaku()
}

fn score_viewing(captured: CardSet, points: &YakuPoints) -> u32 {
    let has_hanami = hanami_cards().is_subset(captured);
    let has_tsukimi = tsukimi_cards().is_subset(captured);
    let all_viewing_set = hanami_cards().union(tsukimi_cards());
    let has_all_viewing = points.all_viewing > 0 && all_viewing_set.is_subset(captured);

    if points.best_yaku_per_category_only {
        // Viewing yaku are mutually exclusive in best-only mode, so take the
        // single best applicable value.
        let hanami_tsukimi =
            (if has_hanami { points.hanami } else { 0 }) + (if has_tsukimi { points.tsukimi } else { 0 });
        return if has_all_viewing {
            hanami_tsukimi.max(points.all_viewing)
        } else {
            hanami_tsukimi
        };
    }

    // Stacking mode: add every viewing yaku that applies.
    let mut total = 0;
    if has_hanami {
        total += points.hanami;
    }
    if has_tsukimi {
        total += points.tsukimi;
    }
    if has_all_viewing {
        total += points.all_viewing;
    }
    total
}

fn score_kasu(captured: CardSet, points: &YakuPoints) -> u32 {
    let count = kasu_count(captured, points);
    if count >= points.kasu_threshold {
        return points.kasu_base + (count - points.kasu_threshold) * points.kasu_extra;
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nintendo_points() -> YakuPoints {
        Ruleset::default().yaku_points()
    }

    fn fuda_wiki_points() -> YakuPoints {
        Ruleset::fuda_wiki().yaku_points()
    }

    fn stacking_points() -> YakuPoints {
        let mut points = nintendo_points();
        points.best_yaku_per_category_only = false;
        points
    }

    #[test]
    fn inoshikacho_scoring() {
        let set = inoshikacho_cards().insert(Card::new_unchecked(4)); // extra Tane
        assert_eq!(score_tane(set, &nintendo_points()), 6);
    }

    #[test]
    fn tane_stacks_when_best_only_is_false() {
        // Ino-Shika-Chō with 7 animals. Nintendo best-only: 5 + 4 extra = 9.
        // Stacking: 5 + generic tane (1 + (7 - 5)) = 8.
        let set = inoshikacho_cards()
            .insert(Card::new_unchecked(4)) // Cuckoo (Feb Tane)
            .insert(Card::new_unchecked(12)) // Wisteria (Apr Tane)
            .insert(Card::new_unchecked(16)) // Iris Bridge (May Tane)
            .insert(Card::new_unchecked(29)); // Pampas + Geese (Aug Tane)

        assert_eq!(score_tane(set, &nintendo_points()), 9);
        assert_eq!(score_tane(set, &stacking_points()), 8);
    }

    #[test]
    fn akatan_aotan_and_overlap() {
        let akatan = akatan_cards();
        let aotan = aotan_cards();
        let both = akatan.union(aotan);

        assert_eq!(score_tanzaku(akatan, &nintendo_points()), 5);
        assert_eq!(score_tanzaku(aotan, &nintendo_points()), 5);
        assert_eq!(score_tanzaku(both, &nintendo_points()), 10);

        // Best-only with an extra scroll: 10 + 1.
        let both_extra = both.insert(Card::new_unchecked(13)); // Wisteria ribbon
        assert_eq!(score_tanzaku(both_extra, &nintendo_points()), 11);
    }

    #[test]
    fn tanzaku_stacks_when_best_only_is_false() {
        let akatan = akatan_cards();
        let with_extra = akatan.insert(Card::new_unchecked(13)).insert(Card::new_unchecked(17));
        // Best-only: 5 + 2 extra scrolls = 7.
        // Stacking: 5 + generic tanzaku (1) = 6.
        assert_eq!(score_tanzaku(with_extra, &nintendo_points()), 7);
        assert_eq!(score_tanzaku(with_extra, &stacking_points()), 6);
    }

    #[test]
    fn gokou_shikou_ame_shikou_sankou() {
        let all_hikari = CardSet::from_card(Card::new_unchecked(0)) // Crane
            .union(CardSet::from_card(Card::new_unchecked(8))) // Curtain
            .union(CardSet::from_card(Card::new_unchecked(28))) // Moon
            .union(CardSet::from_card(Card::new_unchecked(40))) // Rain
            .union(CardSet::from_card(Card::new_unchecked(44))); // Phoenix

        assert_eq!(score_hikari(all_hikari, &nintendo_points()), 10);
        assert_eq!(score_hikari(all_hikari, &fuda_wiki_points()), 15);

        let four_no_rain = all_hikari.remove(Card::new_unchecked(40));
        assert_eq!(score_hikari(four_no_rain, &nintendo_points()), 8);

        let four_with_rain = all_hikari.remove(Card::new_unchecked(0));
        assert_eq!(score_hikari(four_with_rain, &nintendo_points()), 7);

        let three_no_rain = all_hikari
            .remove(Card::new_unchecked(40))
            .remove(Card::new_unchecked(44));
        assert_eq!(score_hikari(three_no_rain, &nintendo_points()), 5);
        assert_eq!(score_hikari(three_no_rain, &fuda_wiki_points()), 6);
    }

    #[test]
    fn hanami_and_tsukimi() {
        let hanami = hanami_cards();
        let tsukimi = tsukimi_cards();

        assert_eq!(score_viewing(hanami, &nintendo_points()), 5);
        assert_eq!(score_viewing(tsukimi, &nintendo_points()), 5);
        assert_eq!(score_viewing(hanami.union(tsukimi), &nintendo_points()), 10);
    }

    #[test]
    fn kasu_scoring_counts_sake_cup_when_enabled() {
        let mut set = CardSet::EMPTY;
        for card in CardSet::ALL {
            if is_kasu(card) {
                set = set.insert(card);
            }
        }

        let mut disabled = nintendo_points();
        disabled.sake_cup_counts_as_kasu = false;

        assert_eq!(score_kasu(set, &disabled), 1 + (24 - 10));

        let with_cup = set.insert(Card::new_unchecked(32));
        assert_eq!(score_kasu(with_cup, &nintendo_points()), 1 + (24 - 10) + 1);
    }
}
