//! Actions available to a player in a Koi-Koi game.

use crate::card::Card;

/// A player decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    /// Play a card from the hand. If multiple captures are possible, `capture`
    /// selects which field card to match.
    PlayFromHand { card: Card, capture: CaptureChoice },
    /// Resolve a drawn stock card. If multiple captures are possible, `capture`
    /// selects which field card to match.
    ResolveStock { capture: CaptureChoice },
    /// Call Koi-Koi to continue the round.
    KoiKoi,
    /// Call Shōbu to stop and collect the score.
    Shobu,
}

/// Choice of which field cards to capture when playing or drawing a card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CaptureChoice {
    /// No matching month on the field; the card is discarded to the field.
    NoMatch,
    /// Exactly one matching card is on the field; capture it.
    Single(Card),
    /// Two matching cards are on the field; choose this one.
    Pair(Card),
    /// Three matching cards are on the field; capture all of them.
    Triple,
}

impl CaptureChoice {
    /// Compact key: tag in the high byte, the chosen card index in the low
    /// byte (`0xFF` when no card is selected).
    pub const fn key(self) -> u64 {
        match self {
            CaptureChoice::NoMatch => 0x00_FF,
            CaptureChoice::Single(card) => 0x01_00 | card.index() as u64,
            CaptureChoice::Pair(card) => 0x02_00 | card.index() as u64,
            CaptureChoice::Triple => 0x03_FF,
        }
    }
}

impl Action {
    /// Canonical total ordering key for an action.
    ///
    /// Layout: `kind << 32 | card << 16 | capture_key`. The order is stable
    /// across platforms and builds — leaf evaluators, ledger digests, and
    /// solver statistics all sort by it instead of relying on generation
    /// order.
    pub const fn action_key(&self) -> u64 {
        let (kind, card, capture) = match *self {
            Action::PlayFromHand { card, capture } => (0u64, card.index() as u64, capture.key()),
            Action::ResolveStock { capture } => (1, 0xFF, capture.key()),
            Action::KoiKoi => (2, 0xFF, 0xFFFF),
            Action::Shobu => (3, 0xFF, 0xFFFF),
        };
        (kind << 32) | (card << 16) | capture
    }
}
