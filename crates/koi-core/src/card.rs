//! Strongly-typed card, month, and bitboard newtypes for Hanafuda Koi-Koi.

/// A validated card index in `0..48`.
///
/// Cards are mapped deterministically by month and position:
/// `card = month * 4 + index_in_month`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Card(u8);

impl Card {
    pub const COUNT: u8 = 48;

    pub const fn new(index: u8) -> Option<Self> {
        if index < Self::COUNT {
            Some(Card(index))
        } else {
            None
        }
    }

    pub const fn new_unchecked(index: u8) -> Self {
        assert!(index < Self::COUNT, "card index out of range");
        Card(index)
    }

    pub const fn index(self) -> u8 {
        self.0
    }

    pub const fn month(self) -> Month {
        Month::new_unchecked(self.0 / 4)
    }

    pub const fn index_in_month(self) -> u8 {
        self.0 % 4
    }

    pub const fn from_month_and_index(month: Month, index: u8) -> Option<Self> {
        if index < 4 {
            Some(Card::new_unchecked(month.0 * 4 + index))
        } else {
            None
        }
    }
}

impl TryFrom<u8> for Card {
    type Error = &'static str;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Card::new(value).ok_or("card index out of range")
    }
}

/// A validated month index in `0..12`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Month(u8);

impl Month {
    pub const COUNT: u8 = 12;

    pub const fn new(index: u8) -> Option<Self> {
        if index < Self::COUNT {
            Some(Month(index))
        } else {
            None
        }
    }

    pub const fn new_unchecked(index: u8) -> Self {
        assert!(index < Self::COUNT, "month index out of range");
        Month(index)
    }

    pub const fn index(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for Month {
    type Error = &'static str;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Month::new(value).ok_or("month index out of range")
    }
}

/// A validated bitboard over `Card` values, using the low 48 bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CardSet(u64);

impl CardSet {
    pub const EMPTY: Self = CardSet(0);
    pub const ALL: Self = CardSet((1u64 << 48) - 1);

    pub const fn new(bits: u64) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(CardSet(bits))
        } else {
            None
        }
    }

    pub const fn new_unchecked(bits: u64) -> Self {
        debug_assert!(bits & !Self::ALL.0 == 0);
        CardSet(bits & Self::ALL.0)
    }

    pub const fn from_card(card: Card) -> Self {
        CardSet(1u64 << card.0)
    }

    pub const fn from_month(month: Month) -> Self {
        CardSet(0xF_u64 << (month.0 * 4))
    }

    pub const fn bits(self) -> u64 {
        self.0
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub const fn count(self) -> u32 {
        self.0.count_ones()
    }

    pub const fn contains(self, card: Card) -> bool {
        (self.0 >> card.0) & 1 != 0
    }

    pub const fn insert(self, card: Card) -> Self {
        CardSet(self.0 | (1u64 << card.0))
    }

    pub const fn remove(self, card: Card) -> Self {
        CardSet(self.0 & !(1u64 << card.0))
    }

    pub const fn toggle(self, card: Card) -> Self {
        CardSet(self.0 ^ (1u64 << card.0))
    }

    pub const fn union(self, other: Self) -> Self {
        CardSet(self.0 | other.0)
    }

    pub const fn intersection(self, other: Self) -> Self {
        CardSet(self.0 & other.0)
    }

    pub const fn difference(self, other: Self) -> Self {
        CardSet(self.0 & !other.0)
    }

    pub const fn month_cards(self, month: Month) -> Self {
        CardSet(self.0 & Self::from_month(month).0)
    }

    pub const fn is_subset(self, other: Self) -> bool {
        self.0 & !other.0 == 0
    }

    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

impl From<Card> for CardSet {
    fn from(card: Card) -> Self {
        Self::from_card(card)
    }
}

impl From<Month> for CardSet {
    fn from(month: Month) -> Self {
        Self::from_month(month)
    }
}

impl Default for CardSet {
    fn default() -> Self {
        Self::EMPTY
    }
}

/// Iterator over the cards contained in a `CardSet`.
pub struct CardSetIter {
    bits: u64,
}

impl Iterator for CardSetIter {
    type Item = Card;
    fn next(&mut self) -> Option<Self::Item> {
        if self.bits == 0 {
            None
        } else {
            let index = self.bits.trailing_zeros() as u8;
            self.bits &= self.bits - 1;
            Some(Card::new_unchecked(index))
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let count = self.bits.count_ones() as usize;
        (count, Some(count))
    }
}

impl FromIterator<Card> for CardSet {
    fn from_iter<I: IntoIterator<Item = Card>>(iter: I) -> Self {
        let mut set = CardSet::EMPTY;
        for card in iter {
            set = set.insert(card);
        }
        set
    }
}

impl IntoIterator for CardSet {
    type Item = Card;
    type IntoIter = CardSetIter;
    fn into_iter(self) -> Self::IntoIter {
        CardSetIter { bits: self.0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_month_and_index_round_trip() {
        for index in 0..Card::COUNT {
            let card = Card::new_unchecked(index);
            let month = card.month();
            let in_month = card.index_in_month();
            assert_eq!(
                Card::from_month_and_index(month, in_month),
                Some(card),
                "card {index} round-trip failed"
            );
        }
    }

    #[test]
    fn card_set_validates_and_iterates() {
        let set = CardSet::ALL;
        assert_eq!(set.count(), 48);
        assert_eq!(set.into_iter().count(), 48);

        let mut built = CardSet::EMPTY;
        for card in CardSet::ALL {
            built = built.insert(card);
        }
        assert_eq!(built, CardSet::ALL);
    }

    #[test]
    fn invalid_card_and_set_are_rejected() {
        assert!(Card::new(48).is_none());
        assert!(Month::new(12).is_none());
        assert!(CardSet::new(CardSet::ALL.0 | (1u64 << 63)).is_none());
    }
}
