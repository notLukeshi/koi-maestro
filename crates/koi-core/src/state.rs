//! Complete payoff-relevant Markov state for Koi-Koi.

use smallvec::SmallVec;

use crate::action::{Action, CaptureChoice};
use crate::card::{Card, CardSet, Month};
use crate::rules::Ruleset;
use crate::yaku;

/// A player in the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Player {
    South = 0,
    North = 1,
}

impl Player {
    pub const fn opponent(self) -> Self {
        match self {
            Player::South => Player::North,
            Player::North => Player::South,
        }
    }

    pub const fn index(self) -> usize {
        self as usize
    }
}

/// Errors that can occur when applying an action or constructing a state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StateError {
    InvalidCard,
    NotActivePlayer,
    IllegalAction,
    NotInPhase,
    MissingCard,
    CaptureChoiceMismatch,
    StockEmpty,
    /// A public view or hidden-zone assignment is not a partition of the
    /// 48 cards — overlap, wrong cardinality, or a drawn card that is not
    /// where the phase says it must be.
    InconsistentState,
}

impl std::fmt::Display for StateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for StateError {}

/// A special deal condition that must be resolved before normal play.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DealAnomaly {
    /// A player was dealt all four cards of a month; this is an automatic win
    /// for that player in most rule sets.
    Teshi { player: Player, month: Month },
    /// A player was dealt four matching pairs (eight cards with four months
    /// appearing exactly twice). Many rules award an automatic win.
    FourPairs { player: Player },
    /// Four cards of the same month were dealt to the field; the hand is void
    /// and must be re-dealt under standard rules.
    FieldVoid { month: Month },
}

/// Current phase of a turn. The state machine is strictly sequential within a
/// round: hand action, stock resolution, stop decision (if a new or improved
/// Yaku was formed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TurnPhase {
    /// Active player must play a card from hand.
    AwaitingHandAction,
    /// A stock card has been drawn and must be resolved against the field.
    AwaitingStockResolution { drawn: Card },
    /// Active player formed or improved a Yaku and must stop or continue.
    AwaitingStopDecision { base_score: u32 },
    /// The round has ended.
    Ended,
}

/// How a detected deal anomaly resolves in-engine.
///
/// Benchmark panels reject anomalous initial deals at validation (decision
/// E2), so this path only fires inside a running leg: a match's later rounds
/// and null-round redeals deal derived decks that can hit the ~1% anomaly
/// rate and cannot be rejected mid-flight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnomalyResolution {
    /// A lucky hand (Teshi / FourPairs) pays a fixed amount and the round
    /// ends immediately; the holder deals the next round.
    InstantWin { player: Player, points: u32 },
    /// The deal is void: the round ends uncounted (`round` is not advanced)
    /// and the caller redeals from the next derived seed.
    Redeal,
}

/// The acting player's public view of a state.
///
/// Every payoff-relevant zone except the opponent's hand and the stock
/// order, which are carried as counts only. `from_public_view` materializes
/// those hidden zones as canonical placeholders (lowest-index unknowns to
/// the opponent's hand, the remainder to the stock in index order) so
/// workers and determinizers rebuild a state without ever seeing hidden
/// cards.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicView {
    /// The player this view belongs to.
    pub observer: Player,
    /// The ruleset the leg is played under.
    pub rules: Ruleset,
    /// The dealer for the current round.
    pub dealer: Player,
    /// The player to move.
    pub active: Player,
    /// Completed player-turns this round.
    pub turn: u8,
    /// Round index within the match.
    pub round: u8,
    /// Accumulated match score.
    pub score: [i32; 2],
    /// Current turn phase; the drawn card is public during stock resolution.
    pub phase: TurnPhase,
    /// The observer's real hand.
    pub own_hand: CardSet,
    /// Number of cards in the opponent's hand (identity hidden).
    pub opponent_hand_count: u8,
    /// Cards on the field.
    pub field: CardSet,
    /// Both captured piles (public).
    pub captured: [CardSet; 2],
    /// Number of cards left in the stock (order hidden, drawn card on top).
    pub stock_count: u8,
    /// Who called Koi-Koi, if anyone.
    pub koi_koi_caller: Option<Player>,
    /// Per-player Koi-Koi call counts this round.
    pub koi_koi_calls: [u8; 2],
    /// Last recorded Yaku score for each player.
    pub last_yaku_score: [u32; 2],
}

/// One ordered entry of the public action ledger: who acted, what they
/// chose, and — on `ResolveStock` entries — which stock card was publicly
/// drawn for that resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LedgerEntry {
    /// The acting player.
    pub player: Player,
    /// The action taken.
    pub action: crate::Action,
    /// The publicly drawn card a `ResolveStock` entry resolved; `None` on
    /// every other entry kind.
    pub drawn: Option<Card>,
}

/// The observer-relative public observation of a round: the deal-time view
/// the ledger replays from plus the ordered public actions since.
///
/// This is the resolving substrate the benchmark wire already transports —
/// an observer cannot derive the ordered ledger from a bare
/// [`KoiGameState`] (play order is not a zone function), so stateful
/// consumers receive it explicitly.
#[derive(Debug, Clone)]
pub struct PublicObservation {
    /// The public view at the deal (empty piles, turn 0).
    pub initial: PublicView,
    /// Ordered public actions since the deal.
    pub entries: Vec<LedgerEntry>,
}

/// The hidden stock as a fixed-capacity array with a top cursor.
///
/// `cards[top..]` is the live draw order, top of stock first. Drawing is a
/// cursor bump — not a `Vec::remove(0)` shift — and the whole state stays
/// `Copy`, so `apply_action` transitions are memcpy rather than heap
/// allocation (debt item D1: the CFR/rollout hot path must not allocate
/// per node).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stock {
    cards: [Card; Stock::CAPACITY],
    top: u8,
}

impl Stock {
    /// Maximum stock depth — the full 48-card deck.
    pub const CAPACITY: usize = 48;

    /// Builds a stock from a draw-ordered slice (`[0]` is the top). The
    /// live region is always the array tail: `top = CAPACITY - len`, so
    /// `len()` is the real count and drawing just bumps the cursor.
    pub fn from_slice(cards: &[Card]) -> Result<Self, StateError> {
        if cards.len() > Self::CAPACITY {
            return Err(StateError::InconsistentState);
        }
        let top = Self::CAPACITY - cards.len();
        let mut stock = Self {
            cards: [Card::new_unchecked(0); Self::CAPACITY],
            top: top as u8,
        };
        stock.cards[top..].copy_from_slice(cards);
        Ok(stock)
    }

    /// The live draw-ordered region (`[0]` is the top card).
    pub fn as_slice(&self) -> &[Card] {
        &self.cards[usize::from(self.top)..]
    }

    /// The live region, mutable — used by hidden-zone reassignment to pin
    /// a drawn card to the head.
    pub fn as_mut_slice(&mut self) -> &mut [Card] {
        &mut self.cards[usize::from(self.top)..]
    }

    /// Cards remaining in the stock.
    pub fn len(&self) -> usize {
        Self::CAPACITY - usize::from(self.top)
    }

    /// Whether the stock is exhausted.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The top card without removing it.
    pub fn first(&self) -> Option<&Card> {
        self.as_slice().first()
    }

    /// Draws the top card — a cursor bump, O(1).
    pub fn pop_front(&mut self) -> Option<Card> {
        let card = *self.cards.get(usize::from(self.top))?;
        self.top += 1;
        Some(card)
    }

    /// Iterates the live region top-to-bottom.
    pub fn iter(&self) -> std::slice::Iter<'_, Card> {
        self.as_slice().iter()
    }
}

impl std::ops::Index<usize> for Stock {
    type Output = Card;
    fn index(&self, index: usize) -> &Card {
        &self.as_slice()[index]
    }
}

impl<'a> IntoIterator for &'a Stock {
    type Item = &'a Card;
    type IntoIter = std::slice::Iter<'a, Card>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// A complete payoff-relevant Markov state for Koi-Koi.
///
/// This contains everything needed to determine future payoffs: the hidden
/// stock ordering, both hands, the field, captured cards, Yaku history,
/// Koi-Koi state, dealer/turn/round, and match score.
#[derive(Debug, Clone, Copy)]
pub struct KoiGameState {
    /// Hidden stock, ordered from top to bottom.
    pub stock: Stock,
    /// Each player's hand.
    pub hands: [CardSet; 2],
    /// Cards currently on the field.
    pub field: CardSet,
    /// Cards each player has captured.
    pub captured: [CardSet; 2],
    /// The dealer for the current round.
    pub dealer: Player,
    /// The player whose turn it is.
    pub active: Player,
    /// Completed player-turn count within the current round (0..16).
    /// A player-turn is one hand play plus one stock resolution.
    pub turn: u8,
    /// Round count within the match.
    pub round: u8,
    /// Accumulated match score. Positive for South.
    pub score: [i32; 2],
    /// Ruleset in effect.
    pub rules: Ruleset,
    /// Current turn phase.
    pub phase: TurnPhase,
    /// Who called Koi-Koi, if anyone.
    pub koi_koi_caller: Option<Player>,
    /// Number of Koi-Koi calls made by each player this round.
    pub koi_koi_calls: [u8; 2],
    /// Last recorded Yaku score for each player.
    pub last_yaku_score: [u32; 2],
}

impl KoiGameState {
    /// Creates a state from explicit parts. Useful for tests and replays.
    ///
    /// Returns the constructed state along with any deal-time anomaly. Callers
    /// must handle a returned anomaly before starting normal play.
    ///
    /// # Panics
    /// Panics if the provided card sets overlap.
    pub fn new_from_parts(
        stock: &[Card],
        hands: [CardSet; 2],
        field: CardSet,
        dealer: Player,
        active: Player,
        rules: Ruleset,
    ) -> (Self, Option<DealAnomaly>) {
        assert!(hands[0].intersection(hands[1]).is_empty(), "hands must be disjoint");
        assert!(
            field.intersection(hands[0].union(hands[1])).is_empty(),
            "field must be disjoint from hands"
        );

        let on_table = hands[0].union(hands[1]).union(field);
        for card in stock {
            assert!(!on_table.contains(*card), "stock must be disjoint from hands and field");
        }

        let state = Self {
            stock: Stock::from_slice(stock).expect("a deal stock never exceeds the deck"),
            hands,
            field,
            captured: [CardSet::EMPTY; 2],
            dealer,
            active,
            turn: 0,
            round: 0,
            score: [0; 2],
            rules,
            phase: TurnPhase::AwaitingHandAction,
            koi_koi_caller: None,
            koi_koi_calls: [0; 2],
            last_yaku_score: [0; 2],
        };

        let anomaly = state.check_deal();
        (state, anomaly)
    }

    /// Deals a shuffled 48-card deck into a standard initial state.
    ///
    /// Distribution: 8 cards to South, 8 to the field, 8 to North, the rest
    /// form the stock. South is the initial dealer. The anomaly half of the
    /// returned pair is `Some` when the dealt hand contains a Teyaku or a
    /// field-void that must be resolved before play.
    pub fn new_deal(deck: Vec<Card>, rules: Ruleset) -> (Self, Option<DealAnomaly>) {
        assert_eq!(deck.len(), 48, "deck must contain 48 cards");

        let mut south = CardSet::EMPTY;
        let mut north = CardSet::EMPTY;
        let mut field = CardSet::EMPTY;
        let mut stock = Vec::with_capacity(24);

        for (i, &card) in deck.iter().enumerate() {
            match i {
                0..=7 => south = south.insert(card),
                8..=15 => field = field.insert(card),
                16..=23 => north = north.insert(card),
                _ => stock.push(card),
            }
        }

        Self::new_from_parts(&stock, [south, north], field, Player::South, Player::South, rules)
    }

    /// Checks for deal-time special conditions (Teyaku / field-void).
    ///
    /// Returns the first detected anomaly in the order: Teshi, Field-Void,
    /// Four-Pairs. If `None` is returned, the deal is normal and play can begin.
    pub fn check_deal(&self) -> Option<DealAnomaly> {
        for month in 0..Month::COUNT {
            let month = Month::new_unchecked(month);
            let month_mask = CardSet::from_month(month);

            // Teshi: all four of a month in a single hand.
            for player in [Player::South, Player::North] {
                if self.hands[player.index()].intersection(month_mask) == month_mask {
                    return Some(DealAnomaly::Teshi { player, month });
                }
            }

            // Field-void: all four of a month on the field.
            if self.field.intersection(month_mask) == month_mask {
                return Some(DealAnomaly::FieldVoid { month });
            }
        }

        // Four-pairs: a hand whose 8 cards form exactly four matching pairs.
        for player in [Player::South, Player::North] {
            let hand = self.hands[player.index()];
            let mut pair_count = 0u32;
            for month in 0..Month::COUNT {
                let month = Month::new_unchecked(month);
                let in_month = hand.intersection(CardSet::from_month(month)).count();
                if in_month == 2 {
                    pair_count += 1;
                } else if in_month != 0 && in_month != 2 {
                    pair_count = u32::MAX;
                    break;
                }
            }
            if pair_count == 4 {
                return Some(DealAnomaly::FourPairs { player });
            }
        }

        None
    }

    /// Deals the next round of a match from a fresh deck, carrying over the
    /// dealer, match score, and round counter from `prev` (which must have
    /// finished a round — `phase == Ended`).
    ///
    /// The deck layout follows `new_deal`; the returned anomaly is handled
    /// by `resolve_deal_anomaly` exactly like the initial deal.
    pub fn next_round(deck: Vec<Card>, prev: &KoiGameState) -> (Self, Option<DealAnomaly>) {
        debug_assert!(
            matches!(prev.phase, TurnPhase::Ended),
            "next_round requires a finished round"
        );
        let (mut state, anomaly) = Self::new_deal(deck, prev.rules);
        state.dealer = prev.dealer;
        state.active = prev.dealer;
        state.round = prev.round;
        state.score = prev.score;
        (state, anomaly)
    }

    /// Resolves a deal-time anomaly in place.
    ///
    /// A lucky hand pays `rules.anomaly_win_points()` through the round-win
    /// channel (winner deals next, `round` advances). A field void — or a
    /// lucky hand under a ruleset whose `anomaly_win_points` is zero —
    /// leaves the state `Ended` with `round` unadvanced so the caller
    /// redeals from the next derived seed.
    pub fn resolve_deal_anomaly(&mut self, anomaly: DealAnomaly) -> AnomalyResolution {
        let holder = match anomaly {
            DealAnomaly::Teshi { player, .. } | DealAnomaly::FourPairs { player } => Some(player),
            DealAnomaly::FieldVoid { .. } => None,
        };
        let points = holder.map(|_| self.rules.anomaly_win_points()).unwrap_or(0);
        match holder.zip((points > 0).then_some(points)) {
            Some((player, points)) => {
                let index = player.index();
                self.score[index] += points as i32;
                self.score[player.opponent().index()] -= points as i32;
                self.round = self.round.saturating_add(1);
                self.dealer = player;
                self.active = player;
                self.turn = 0;
                self.phase = TurnPhase::Ended;
                AnomalyResolution::InstantWin { player, points }
            }
            None => {
                self.phase = TurnPhase::Ended;
                AnomalyResolution::Redeal
            }
        }
    }

    /// The public view for `observer` — every public zone plus the counts of
    /// the two hidden zones (opponent hand, stock tail).
    pub fn public_view(&self, observer: Player) -> PublicView {
        PublicView {
            observer,
            rules: self.rules,
            dealer: self.dealer,
            active: self.active,
            turn: self.turn,
            round: self.round,
            score: self.score,
            phase: self.phase,
            own_hand: self.hands[observer.index()],
            opponent_hand_count: self.hands[observer.opponent().index()].count() as u8,
            field: self.field,
            captured: self.captured,
            stock_count: self.stock.len() as u8,
            koi_koi_caller: self.koi_koi_caller,
            koi_koi_calls: self.koi_koi_calls,
            last_yaku_score: self.last_yaku_score,
        }
    }

    /// Rebuilds a state from a public view, materializing the hidden zones
    /// as canonical placeholders: the opponent's hand takes the lowest-index
    /// unknown cards and the stock tail takes the rest in index order. A
    /// drawn stock card stays pinned at `stock[0]`.
    ///
    /// Fails closed on any inconsistent partition — overlap between public
    /// zones, a hidden-zone count that does not match the unknown-card set,
    /// or a drawn card that is not actually hidden.
    pub fn from_public_view(view: &PublicView) -> Result<Self, StateError> {
        let observer = view.observer;
        let opponent = observer.opponent();

        let public_zones = [view.own_hand, view.field, view.captured[0], view.captured[1]];
        for (index, zone) in public_zones.iter().enumerate() {
            for other in &public_zones[index + 1..] {
                if zone.intersects(*other) {
                    return Err(StateError::InconsistentState);
                }
            }
        }
        let known = public_zones.iter().fold(CardSet::EMPTY, |acc, zone| acc.union(*zone));
        let mut unknown = CardSet::ALL.difference(known);

        // The drawn card is public during stock resolution and pinned to the
        // top of the stock; it leaves the hidden pool.
        let drawn = match view.phase {
            TurnPhase::AwaitingStockResolution { drawn } => {
                if !unknown.contains(drawn) {
                    return Err(StateError::InconsistentState);
                }
                unknown = unknown.remove(drawn);
                Some(drawn)
            }
            _ => None,
        };
        let stock_tail = view
            .stock_count
            .checked_sub(u8::from(drawn.is_some()))
            .ok_or(StateError::InconsistentState)?;
        if unknown.count() != u32::from(view.opponent_hand_count) + u32::from(stock_tail) {
            return Err(StateError::InconsistentState);
        }
        if view.turn > 16 {
            return Err(StateError::InconsistentState);
        }

        let mut hands = [CardSet::EMPTY; 2];
        hands[observer.index()] = view.own_hand;
        let mut opponent_hand = CardSet::EMPTY;
        let mut remaining = unknown;
        for _ in 0..view.opponent_hand_count {
            let card = remaining.into_iter().next().ok_or(StateError::InconsistentState)?;
            opponent_hand = opponent_hand.insert(card);
            remaining = remaining.remove(card);
        }
        hands[opponent.index()] = opponent_hand;

        let mut stock = [Card::new_unchecked(0); Stock::CAPACITY];
        let mut stock_len = 0usize;
        if let Some(drawn) = drawn {
            stock[stock_len] = drawn;
            stock_len += 1;
        }
        // `CardSet` iterates in ascending index order — the canonical
        // placeholder order for the hidden stock tail.
        for card in remaining {
            stock[stock_len] = card;
            stock_len += 1;
        }

        Ok(Self {
            stock: Stock::from_slice(&stock[..stock_len])?,
            hands,
            field: view.field,
            captured: view.captured,
            dealer: view.dealer,
            active: view.active,
            turn: view.turn,
            round: view.round,
            score: view.score,
            rules: view.rules,
            phase: view.phase,
            koi_koi_caller: view.koi_koi_caller,
            koi_koi_calls: view.koi_koi_calls,
            last_yaku_score: view.last_yaku_score,
        })
    }

    /// Returns a copy of this state with `opponent`'s hand and the stock
    /// order replaced by an explicit assignment — the determinization
    /// primitive for imperfect-information search.
    ///
    /// `opponent` is the seat whose hand is hidden from the observer (the
    /// fixed root perspective). The assignment must partition the currently
    /// unknown cards exactly: `opponent_hand` must be a subset of the
    /// unknowns with the same cardinality as the opponent's current hand,
    /// and `stock` must be the remaining unknowns in draw order, with a
    /// drawn card still on top during `AwaitingStockResolution`.
    pub fn rebuild_hidden(&self, opponent: Player, opponent_hand: CardSet, stock: &[Card]) -> Result<Self, StateError> {
        self.rebuild_hidden_universe(opponent, opponent_hand, stock, CardSet::ALL)
    }

    /// `rebuild_hidden` against a declared universe — reduced-variant
    /// domains partition a subset of the deck, so the complement of the
    /// public zones is taken relative to `universe`, not the full deck.
    /// Fails closed when the public zones are not a subset of `universe`.
    pub fn rebuild_hidden_universe(
        &self,
        opponent: Player,
        opponent_hand: CardSet,
        stock: &[Card],
        universe: CardSet,
    ) -> Result<Self, StateError> {
        let observer = opponent.opponent();
        let known = self.hands[observer.index()]
            .union(self.field)
            .union(self.captured[0])
            .union(self.captured[1]);
        if !known.is_subset(universe) {
            return Err(StateError::InconsistentState);
        }
        let unknown = universe.difference(known);

        if opponent_hand.count() != self.hands[opponent.index()].count() || !opponent_hand.is_subset(unknown) {
            return Err(StateError::InconsistentState);
        }
        let expected_stock = unknown.difference(opponent_hand);
        let given_stock: CardSet = stock.iter().copied().collect();
        if given_stock != expected_stock || stock.len() != expected_stock.count() as usize {
            return Err(StateError::InconsistentState);
        }
        if let TurnPhase::AwaitingStockResolution { drawn } = self.phase {
            if stock.first() != Some(&drawn) {
                return Err(StateError::InconsistentState);
            }
        }

        let mut next = *self;
        next.hands[opponent.index()] = opponent_hand;
        next.stock = Stock::from_slice(stock)?;
        Ok(next)
    }

    /// The margin of the current leg's accumulated score for `player`
    /// (positive when `player` leads). The leg margin is the score delta
    /// since the leg started — legs always start from a fresh `[0, 0]`
    /// match score, so this is just the current differential.
    pub fn leg_margin(&self, player: Player) -> i32 {
        self.score[player.index()] - self.score[player.opponent().index()]
    }

    /// Returns `true` if the round has ended.
    pub const fn is_ended(&self) -> bool {
        matches!(self.phase, TurnPhase::Ended)
    }

    /// Returns the set of legal actions for the active player.
    pub fn legal_actions(&self) -> SmallVec<[Action; 24]> {
        match self.phase {
            TurnPhase::AwaitingHandAction => self.legal_hand_actions(),
            TurnPhase::AwaitingStockResolution { drawn } => self.legal_stock_actions(drawn),
            TurnPhase::AwaitingStopDecision { .. } => {
                let mut actions = SmallVec::new();
                actions.push(Action::Shobu);
                if self.can_call_koi_koi() {
                    actions.push(Action::KoiKoi);
                }
                actions
            }
            TurnPhase::Ended => SmallVec::new(),
        }
    }

    fn legal_hand_actions(&self) -> SmallVec<[Action; 24]> {
        let hand = self.hands[self.active.index()];
        let mut actions = SmallVec::new();

        for card in hand {
            let month_cards = self.field.month_cards(card.month());
            let count = month_cards.count();

            match count {
                0 => {
                    actions.push(Action::PlayFromHand {
                        card,
                        capture: CaptureChoice::NoMatch,
                    });
                }
                1 => {
                    let target = month_cards.into_iter().next().unwrap();
                    actions.push(Action::PlayFromHand {
                        card,
                        capture: CaptureChoice::Single(target),
                    });
                }
                2 => {
                    for target in month_cards {
                        actions.push(Action::PlayFromHand {
                            card,
                            capture: CaptureChoice::Pair(target),
                        });
                    }
                }
                3 => {
                    actions.push(Action::PlayFromHand {
                        card,
                        capture: CaptureChoice::Triple,
                    });
                }
                4 => {
                    // All four cards of this month are on the field; the
                    // matching hand card cannot legally be played. Skip it.
                    continue;
                }
                _ => unreachable!(),
            }
        }

        actions
    }

    fn legal_stock_actions(&self, drawn: Card) -> SmallVec<[Action; 24]> {
        let month_cards = self.field.month_cards(drawn.month());
        let count = month_cards.count();
        let mut actions = SmallVec::new();

        match count {
            0 => actions.push(Action::ResolveStock {
                capture: CaptureChoice::NoMatch,
            }),
            1 => {
                let target = month_cards.into_iter().next().unwrap();
                actions.push(Action::ResolveStock {
                    capture: CaptureChoice::Single(target),
                });
            }
            2 => {
                for target in month_cards {
                    actions.push(Action::ResolveStock {
                        capture: CaptureChoice::Pair(target),
                    });
                }
            }
            3 => actions.push(Action::ResolveStock {
                capture: CaptureChoice::Triple,
            }),
            // A four-of-a-month field is a misdeal / field-void state; there is
            // no legal stock action for that month.
            4 => {}
            _ => unreachable!(),
        }

        actions
    }

    /// Applies a legal action and returns the resulting state.
    ///
    /// This is a pure function: the original state is not mutated.
    pub fn apply_action(&self, action: Action) -> Result<Self, StateError> {
        match self.phase {
            TurnPhase::AwaitingHandAction => self.apply_hand_action(action),
            TurnPhase::AwaitingStockResolution { drawn } => self.apply_stock_action(action, drawn),
            TurnPhase::AwaitingStopDecision { base_score } => self.apply_stop_decision(action, base_score),
            TurnPhase::Ended => Err(StateError::NotInPhase),
        }
    }

    fn apply_hand_action(&self, action: Action) -> Result<Self, StateError> {
        let Action::PlayFromHand { card, capture } = action else {
            return Err(StateError::IllegalAction);
        };

        let mut next = *self;

        if !next.hands[next.active.index()].contains(card) {
            return Err(StateError::MissingCard);
        }

        let month = card.month();
        let month_cards = next.field.month_cards(month);

        let (captured, new_field) = next.resolve_capture(card, month_cards, capture)?;
        next.hands[next.active.index()] = next.hands[next.active.index()].remove(card);
        next.field = new_field;
        next.captured[next.active.index()] = next.captured[next.active.index()].union(captured);

        next.phase = TurnPhase::AwaitingStockResolution {
            drawn: *next.stock.first().ok_or(StateError::StockEmpty)?,
        };

        Ok(next)
    }

    fn apply_stock_action(&self, action: Action, drawn: Card) -> Result<Self, StateError> {
        let Action::ResolveStock { capture } = action else {
            return Err(StateError::IllegalAction);
        };

        let mut next = *self;

        if next.stock.is_empty() || next.stock[0] != drawn {
            return Err(StateError::IllegalAction);
        }

        next.stock.pop_front();

        let month = drawn.month();
        let month_cards = next.field.month_cards(month);

        let (captured, new_field) = next.resolve_capture(drawn, month_cards, capture)?;
        next.field = new_field;
        next.captured[next.active.index()] = next.captured[next.active.index()].union(captured);

        // Evaluate Yaku.
        let active_idx = next.active.index();
        let base = yaku::score_yaku(next.captured[active_idx], &next.rules);

        if base > next.last_yaku_score[active_idx] {
            next.last_yaku_score[active_idx] = base;
            next.phase = TurnPhase::AwaitingStopDecision { base_score: base };
        } else {
            next.turn = next.turn.saturating_add(1);
            next.active = next.active.opponent();
            next.phase = TurnPhase::AwaitingHandAction;

            let hands_empty = next.hands[0].is_empty() && next.hands[1].is_empty();
            let new_active_empty = next.hands[next.active.index()].is_empty();
            if next.turn >= 16 || hands_empty || next.stock.is_empty() || new_active_empty {
                next.end_round_null()?;
            }
        }

        Ok(next)
    }

    fn resolve_capture(
        &self,
        played: Card,
        month_cards: CardSet,
        capture: CaptureChoice,
    ) -> Result<(CardSet, CardSet), StateError> {
        let count = month_cards.count();

        let captured = match (count, capture) {
            (0, CaptureChoice::NoMatch) => CardSet::EMPTY,
            (1, CaptureChoice::Single(target)) => {
                if !month_cards.contains(target) {
                    return Err(StateError::CaptureChoiceMismatch);
                }
                CardSet::from_card(played).insert(target)
            }
            (2, CaptureChoice::Pair(target)) => {
                if !month_cards.contains(target) {
                    return Err(StateError::CaptureChoiceMismatch);
                }
                CardSet::from_card(played).insert(target)
            }
            (3, CaptureChoice::Triple) => CardSet::from_card(played).union(month_cards),
            _ => return Err(StateError::CaptureChoiceMismatch),
        };

        let new_field = if captured.is_empty() {
            self.field.insert(played)
        } else {
            let captured_field = month_cards.intersection(captured);
            self.field.difference(captured_field)
        };

        Ok((captured, new_field))
    }

    fn apply_stop_decision(&self, action: Action, base_score: u32) -> Result<Self, StateError> {
        match action {
            Action::Shobu => self.finalize_round(self.active, base_score),
            Action::KoiKoi => {
                if !self.can_call_koi_koi() {
                    return Err(StateError::IllegalAction);
                }

                let mut next = *self;
                let active = next.active;
                next.koi_koi_caller = Some(active);
                next.koi_koi_calls[active.index()] = next.koi_koi_calls[active.index()].saturating_add(1);

                next.turn = next.turn.saturating_add(1);
                next.active = next.active.opponent();
                next.phase = TurnPhase::AwaitingHandAction;

                // Defensive: if the new active player cannot actually continue
                // (constructed state or unexpected exhaustion), end the round
                // rather than deadlocking.
                if next.stock.is_empty() || next.hands[next.active.index()].is_empty() {
                    next.end_round_null()?;
                }

                Ok(next)
            }
            _ => Err(StateError::IllegalAction),
        }
    }

    fn can_call_koi_koi(&self) -> bool {
        if self.koi_koi_calls[self.active.index()] >= self.rules.koi_koi_calls_per_round() {
            return false;
        }
        if let Some(caller) = self.koi_koi_caller {
            if caller != self.active && !self.rules.take_back_koi_koi() {
                return false;
            }
        }

        // A Koi-Koi call must be playable: both players need at least one hand
        // card left and the stock must contain enough cards for the opponent's
        // next full turn plus the caller's following turn.
        let opponent = self.active.opponent();
        !self.hands[self.active.index()].is_empty() && !self.hands[opponent.index()].is_empty() && self.stock.len() >= 2
    }

    fn finalize_round(&self, winner: Player, base_score: u32) -> Result<Self, StateError> {
        let mut next = *self;
        let active_idx = winner.index();
        let opponent_idx = winner.opponent().index();

        let opponent_called = self.koi_koi_caller.is_some_and(|caller| caller != winner);
        let score = self.rules.apply_multipliers(base_score, opponent_called);

        next.score[active_idx] += score as i32;
        next.score[opponent_idx] -= score as i32;

        next.round += 1;
        next.dealer = winner;
        next.active = next.dealer;
        next.turn = 0;
        next.phase = TurnPhase::Ended;

        Ok(next)
    }

    fn end_round_null(&mut self) -> Result<(), StateError> {
        match self.rules.null_round_resolution() {
            crate::rules::NullRoundResolution::DealerKeepsDeal => {
                self.score[self.dealer.index()] += 0;
            }
            crate::rules::NullRoundResolution::DealerWinsOnePoint => {
                self.score[self.dealer.index()] += 1;
                self.score[self.dealer.opponent().index()] -= 1;
            }
            crate::rules::NullRoundResolution::DrawOpponentDeals => {
                self.dealer = self.dealer.opponent();
            }
            crate::rules::NullRoundResolution::Redeal => {
                // A re-deal is not a completed round, so `round` is not
                // incremented. The caller is responsible for dealing again.
                self.phase = TurnPhase::Ended;
                return Ok(());
            }
        }

        self.round += 1;
        self.active = self.dealer;
        self.turn = 0;
        self.phase = TurnPhase::Ended;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardSet;

    fn fresh_deck() -> Vec<Card> {
        // A round-robin order that guarantees no four-of-a-month in any of the
        // 8-card slots created by `new_deal`, producing a normal initial state.
        let mut deck = Vec::with_capacity(48);
        for index_in_month in 0..4u8 {
            for month in 0..12u8 {
                deck.push(Card::from_month_and_index(Month::new_unchecked(month), index_in_month).unwrap());
            }
        }
        deck
    }

    #[test]
    fn deal_conserves_cards() {
        let state = KoiGameState::new_deal(fresh_deck(), Ruleset::default()).0;
        let on_table = state.hands[0]
            .union(state.hands[1])
            .union(state.field)
            .union(state.captured[0].union(state.captured[1]));
        let in_stock: CardSet = state.stock.iter().copied().collect();
        let all = on_table.union(in_stock);
        assert_eq!(all, CardSet::ALL);
        assert_eq!(state.hands[0].count(), 8);
        assert_eq!(state.hands[1].count(), 8);
        assert_eq!(state.field.count(), 8);
        assert_eq!(state.stock.len(), 24);
    }

    #[test]
    fn hand_action_legal_for_all_cards() {
        let state = KoiGameState::new_deal(fresh_deck(), Ruleset::default()).0;
        let actions = state.legal_actions();
        assert!(!actions.is_empty());
        for action in actions {
            assert!(state.apply_action(action).is_ok());
        }
    }

    #[test]
    fn played_card_leaves_hand_and_enters_stock_phase() {
        let state = KoiGameState::new_deal(fresh_deck(), Ruleset::default()).0;
        let actions = state.legal_actions();
        let action = actions.into_iter().next().unwrap();

        let next = state.apply_action(action).unwrap();
        assert!(matches!(next.phase, TurnPhase::AwaitingStockResolution { .. }));
    }

    #[test]
    fn triple_capture_removes_all_three_from_field() {
        let _deck = fresh_deck();
        // Force month 0 (January) to have three cards on the field and one in hand.
        // We take a simple deterministic deal by constructing from parts.
        let jan = CardSet::from_month(crate::card::Month::new_unchecked(0));
        let hand0 = CardSet::from_card(Card::new_unchecked(0)); // Crane
        let field = jan.remove(Card::new_unchecked(0));
        let hand1 = CardSet::EMPTY;
        let stock: Vec<Card> = (8..48).map(Card::new_unchecked).collect();

        let mut state = KoiGameState::new_from_parts(
            &stock,
            [hand0, hand1],
            field,
            Player::South,
            Player::South,
            Ruleset::default(),
        )
        .0;
        state.hands[0] = hand0;

        let action = Action::PlayFromHand {
            card: Card::new_unchecked(0),
            capture: CaptureChoice::Triple,
        };
        let next = state.apply_action(action).unwrap();

        assert_eq!(next.captured[0].count(), 4);
        assert!(!next.field.intersects(jan));
        assert!(next.hands[0].is_empty());
    }

    fn stop_state(
        hands: [CardSet; 2],
        stock: &[Card],
        active: Player,
        rules: Ruleset,
        base_score: u32,
        caller: Option<Player>,
        calls: [u8; 2],
    ) -> KoiGameState {
        let (mut state, _) = KoiGameState::new_from_parts(stock, hands, CardSet::EMPTY, Player::South, active, rules);
        state.phase = TurnPhase::AwaitingStopDecision { base_score };
        state.koi_koi_caller = caller;
        state.koi_koi_calls = calls;
        state
    }

    #[test]
    fn koi_koi_requires_cards_and_stock_to_continue() {
        // Both players have cards and stock remains: Koi-Koi is legal.
        // Stock needs at least 2 cards: one for the opponent's next turn and
        // one for the caller's following turn.
        let hand0 = CardSet::from_card(Card::new_unchecked(0));
        let hand1 = CardSet::from_card(Card::new_unchecked(4));
        let stock = vec![Card::new_unchecked(8), Card::new_unchecked(12)];
        let state = stop_state(
            [hand0, hand1],
            &stock,
            Player::South,
            Ruleset::default(),
            5,
            None,
            [0; 2],
        );
        let actions = state.legal_actions();
        assert!(actions.contains(&Action::KoiKoi));
        assert!(actions.contains(&Action::Shobu));

        // Opponent hand is empty: cannot call Koi-Koi because the round cannot
        // continue to the next player.
        let state = stop_state(
            [hand0, CardSet::EMPTY],
            &[Card::new_unchecked(8)],
            Player::South,
            Ruleset::default(),
            5,
            None,
            [0; 2],
        );
        let actions = state.legal_actions();
        assert!(!actions.contains(&Action::KoiKoi));
        assert!(actions.contains(&Action::Shobu));

        // Active hand is empty (just played their last card): cannot improve.
        let state = stop_state(
            [CardSet::EMPTY, hand1],
            &[Card::new_unchecked(8)],
            Player::South,
            Ruleset::default(),
            5,
            None,
            [0; 2],
        );
        let actions = state.legal_actions();
        assert!(!actions.contains(&Action::KoiKoi));

        // No stock left: cannot draw a replacement card.
        let state = stop_state([hand0, hand1], &[], Player::South, Ruleset::default(), 5, None, [0; 2]);
        let actions = state.legal_actions();
        assert!(!actions.contains(&Action::KoiKoi));
    }

    #[test]
    fn nintendo_blocks_take_back() {
        let hand0 = CardSet::from_card(Card::new_unchecked(0));
        let hand1 = CardSet::from_card(Card::new_unchecked(4));
        let stock = vec![Card::new_unchecked(8)];

        // South already called Koi-Koi; North may not call it under Nintendo.
        let state = stop_state(
            [hand0, hand1],
            &stock,
            Player::North,
            Ruleset::default(),
            5,
            Some(Player::South),
            [1, 0],
        );
        assert!(!state.legal_actions().contains(&Action::KoiKoi));

        // South called Koi-Koi earlier; South may not call it a second time.
        let state = stop_state(
            [hand0, hand1],
            &stock,
            Player::South,
            Ruleset::default(),
            7,
            Some(Player::South),
            [1, 0],
        );
        assert!(!state.legal_actions().contains(&Action::KoiKoi));
    }

    #[test]
    fn fuda_wiki_allows_take_back() {
        let hand0 = CardSet::from_card(Card::new_unchecked(0));
        let hand1 = CardSet::from_card(Card::new_unchecked(4));
        let stock = vec![Card::new_unchecked(8), Card::new_unchecked(12)];

        let state = stop_state(
            [hand0, hand1],
            &stock,
            Player::North,
            Ruleset::fuda_wiki(),
            5,
            Some(Player::South),
            [1, 0],
        );
        let actions = state.legal_actions();
        assert!(actions.contains(&Action::KoiKoi));
    }

    #[test]
    fn winner_becomes_dealer_and_score_is_doubled() {
        // South wins with a 5-point yaku while North has called Koi-Koi.
        // Dealer is currently South.
        let (mut state, _) = KoiGameState::new_from_parts(
            &[Card::new_unchecked(8)],
            [CardSet::EMPTY, CardSet::EMPTY],
            CardSet::EMPTY,
            Player::South,
            Player::South,
            Ruleset::default(),
        );
        state.koi_koi_caller = Some(Player::North);

        let next = state.finalize_round(Player::South, 5).unwrap();
        assert_eq!(next.score[0], 10); // doubled because opponent called
        assert_eq!(next.score[1], -10);
        assert_eq!(next.dealer, Player::South); // winner becomes dealer
        assert_eq!(next.active, Player::South);
        assert!(next.is_ended());

        // South wins with 7+ points: doubled for 7+ and doubled for opponent call.
        let next = state.finalize_round(Player::South, 7).unwrap();
        assert_eq!(next.score[0], 28);
        assert_eq!(next.score[1], -28);

        // North wins with no Koi-Koi call: no extra double.
        state.koi_koi_caller = None;
        let next = state.finalize_round(Player::North, 5).unwrap();
        assert_eq!(next.score[1], 5);
        assert_eq!(next.score[0], -5);
        assert_eq!(next.dealer, Player::North);
    }

    #[test]
    fn detect_teshi_and_field_void_at_deal() {
        let jan = CardSet::from_month(Month::new_unchecked(0));

        // Teshi: all four January cards in South's hand.
        let south = jan
            .insert(Card::new_unchecked(4))
            .insert(Card::new_unchecked(5))
            .insert(Card::new_unchecked(8))
            .insert(Card::new_unchecked(9));
        let field = CardSet::from_card(Card::new_unchecked(16))
            .insert(Card::new_unchecked(20))
            .insert(Card::new_unchecked(24))
            .insert(Card::new_unchecked(28))
            .insert(Card::new_unchecked(32))
            .insert(Card::new_unchecked(36))
            .insert(Card::new_unchecked(40))
            .insert(Card::new_unchecked(44));
        let north = CardSet::from_card(Card::new_unchecked(17))
            .insert(Card::new_unchecked(21))
            .insert(Card::new_unchecked(25))
            .insert(Card::new_unchecked(29))
            .insert(Card::new_unchecked(33))
            .insert(Card::new_unchecked(37))
            .insert(Card::new_unchecked(41))
            .insert(Card::new_unchecked(45));
        let used = south.union(field).union(north);
        let stock: Vec<Card> = CardSet::ALL.into_iter().filter(|c| !used.contains(*c)).collect();

        let state = KoiGameState::new_from_parts(
            &stock,
            [south, north],
            field,
            Player::South,
            Player::South,
            Ruleset::default(),
        )
        .0;
        assert!(matches!(
            state.check_deal(),
            Some(DealAnomaly::Teshi {
                player: Player::South,
                month,
            }) if month.index() == 0
        ));

        // Field-void: all four January cards on the field.
        let south = CardSet::from_card(Card::new_unchecked(4))
            .insert(Card::new_unchecked(5))
            .insert(Card::new_unchecked(8))
            .insert(Card::new_unchecked(9))
            .insert(Card::new_unchecked(12))
            .insert(Card::new_unchecked(13))
            .insert(Card::new_unchecked(16))
            .insert(Card::new_unchecked(17));
        let field = jan;
        let north = CardSet::from_card(Card::new_unchecked(20))
            .insert(Card::new_unchecked(21))
            .insert(Card::new_unchecked(24))
            .insert(Card::new_unchecked(25))
            .insert(Card::new_unchecked(28))
            .insert(Card::new_unchecked(29))
            .insert(Card::new_unchecked(32))
            .insert(Card::new_unchecked(33));
        let used = south.union(field).union(north);
        let stock: Vec<Card> = CardSet::ALL.into_iter().filter(|c| !used.contains(*c)).collect();

        let state = KoiGameState::new_from_parts(
            &stock,
            [south, north],
            field,
            Player::South,
            Player::South,
            Ruleset::default(),
        )
        .0;
        assert!(matches!(
            state.check_deal(),
            Some(DealAnomaly::FieldVoid { month }) if month.index() == 0
        ));
    }

    #[test]
    fn detect_four_pairs_at_deal() {
        // South's hand with exactly four pairs: Jan, Feb, Mar, Apr.
        let south = CardSet::from_card(Card::new_unchecked(0))
            .insert(Card::new_unchecked(1))
            .insert(Card::new_unchecked(4))
            .insert(Card::new_unchecked(5))
            .insert(Card::new_unchecked(8))
            .insert(Card::new_unchecked(9))
            .insert(Card::new_unchecked(12))
            .insert(Card::new_unchecked(13));
        // One card from each of months 0..7, avoiding the cards already in South's hand.
        let field = CardSet::from_card(Card::new_unchecked(2))
            .insert(Card::new_unchecked(6))
            .insert(Card::new_unchecked(10))
            .insert(Card::new_unchecked(14))
            .insert(Card::new_unchecked(16))
            .insert(Card::new_unchecked(20))
            .insert(Card::new_unchecked(24))
            .insert(Card::new_unchecked(28));
        let north = CardSet::from_card(Card::new_unchecked(3))
            .insert(Card::new_unchecked(7))
            .insert(Card::new_unchecked(11))
            .insert(Card::new_unchecked(15))
            .insert(Card::new_unchecked(17))
            .insert(Card::new_unchecked(21))
            .insert(Card::new_unchecked(25))
            .insert(Card::new_unchecked(29));
        let used = south.union(field).union(north);
        let stock: Vec<Card> = CardSet::ALL.into_iter().filter(|c| !used.contains(*c)).collect();

        let state = KoiGameState::new_from_parts(
            &stock,
            [south, north],
            field,
            Player::South,
            Player::South,
            Ruleset::default(),
        )
        .0;
        assert!(matches!(
            state.check_deal(),
            Some(DealAnomaly::FourPairs { player: Player::South })
        ));
    }

    #[test]
    fn null_round_dealer_rotation() {
        // Null round with DrawOpponentDeals moves the deal to North.
        let (state, _) = KoiGameState::new_from_parts(
            &[Card::new_unchecked(8)],
            [CardSet::EMPTY, CardSet::EMPTY],
            CardSet::EMPTY,
            Player::South,
            Player::South,
            Ruleset::default(),
        );
        let mut next = state;
        next.end_round_null().unwrap();
        assert_eq!(next.dealer, Player::North);
        assert!(next.is_ended());

        // DealerKeepsDeal leaves the deal with South.
        let mut rules = Ruleset::default().house_rules();
        rules.null_round = crate::rules::NullRoundResolution::DealerKeepsDeal;
        let (state, _) = KoiGameState::new_from_parts(
            &[Card::new_unchecked(8)],
            [CardSet::EMPTY, CardSet::EMPTY],
            CardSet::EMPTY,
            Player::South,
            Player::South,
            Ruleset::House(rules),
        );
        let mut next = state;
        next.end_round_null().unwrap();
        assert_eq!(next.dealer, Player::South);
    }

    #[test]
    fn seeded_deals_are_deterministic_and_well_formed() {
        let first = crate::seed::deal_from_seed(0xC0FFEE);
        let second = crate::seed::deal_from_seed(0xC0FFEE);
        assert_eq!(first, second, "the canonical deal must be deterministic");
        assert_eq!(first.len(), 48);
        let mask: CardSet = first.iter().copied().collect();
        assert_eq!(mask, CardSet::ALL, "the deal must be a full permutation");
    }

    #[test]
    fn public_view_round_trips_through_reconstruction() {
        // Play a few plies, then rebuild from the active player's view and
        // confirm every public zone survives while hidden zones are
        // placeholders that still partition the deck.
        let mut state = KoiGameState::new_deal(fresh_deck(), Ruleset::default()).0;
        for _ in 0..4 {
            let action = state.legal_actions().into_iter().next().unwrap();
            state = state.apply_action(action).unwrap();
            if state.is_ended() {
                break;
            }
        }
        if state.is_ended() {
            return;
        }
        let observer = state.active;
        let view = state.public_view(observer);
        let rebuilt = KoiGameState::from_public_view(&view).unwrap();

        assert_eq!(rebuilt.hands[observer.index()], state.hands[observer.index()]);
        assert_eq!(rebuilt.field, state.field);
        assert_eq!(rebuilt.captured, state.captured);
        assert_eq!(rebuilt.stock.len(), state.stock.len());
        assert_eq!(
            rebuilt.hands[observer.opponent().index()].count(),
            state.hands[observer.opponent().index()].count()
        );
        // Hidden zones still partition the deck.
        let all = rebuilt.hands[0]
            .union(rebuilt.hands[1])
            .union(rebuilt.field)
            .union(rebuilt.captured[0])
            .union(rebuilt.captured[1])
            .union(rebuilt.stock.iter().copied().collect());
        assert_eq!(all, CardSet::ALL);
        assert_eq!(rebuilt.phase, state.phase);
        assert_eq!(rebuilt.active, state.active);
        assert_eq!(rebuilt.score, state.score);
        // The rebuilt state must offer the same legal move count.
        assert_eq!(rebuilt.legal_actions().len(), state.legal_actions().len());
    }

    #[test]
    fn public_view_rejects_inconsistent_partitions() {
        let state = KoiGameState::new_deal(fresh_deck(), Ruleset::default()).0;
        let mut view = state.public_view(Player::South);
        // Overlap between the observer's hand and the field is impossible.
        view.field = view.field.union(state.hands[0].into_iter().next().unwrap().into());
        assert!(KoiGameState::from_public_view(&view).is_err());

        let mut view = state.public_view(Player::South);
        // Claiming more hidden cards than exist fails closed.
        view.opponent_hand_count = 40;
        assert!(KoiGameState::from_public_view(&view).is_err());
    }

    #[test]
    fn rebuild_hidden_reassigns_only_hidden_zones() {
        let state = KoiGameState::new_deal(fresh_deck(), Ruleset::default()).0;
        let observer = state.active;
        let opponent = observer.opponent();

        // The observer's real view, with a concrete hidden assignment.
        let view = state.public_view(observer);
        let rebuilt = KoiGameState::from_public_view(&view).unwrap();
        let unknown: CardSet = CardSet::ALL.difference(
            rebuilt.hands[observer.index()]
                .union(rebuilt.field)
                .union(rebuilt.captured[0])
                .union(rebuilt.captured[1]),
        );
        let opp_count = rebuilt.hands[opponent.index()].count();
        let mut new_opp = CardSet::EMPTY;
        let mut rest = unknown;
        for _ in 0..opp_count {
            let card = rest.into_iter().next().unwrap();
            new_opp = new_opp.insert(card);
            rest = rest.remove(card);
        }
        let new_stock: Vec<Card> = rest.into_iter().collect();
        let rebuilt2 = rebuilt.rebuild_hidden(opponent, new_opp, &new_stock).unwrap();
        assert_eq!(rebuilt2.hands[opponent.index()], new_opp);
        assert_eq!(rebuilt2.hands[observer.index()], rebuilt.hands[observer.index()]);

        // Wrong cardinality fails closed.
        let bad = rebuilt.rebuild_hidden(opponent, CardSet::EMPTY, &[]);
        assert!(bad.is_err());
        // An opponent hand containing a known card fails closed.
        let known_card = rebuilt.field.into_iter().next().unwrap();
        let bad = rebuilt.rebuild_hidden(opponent, CardSet::from_card(known_card), &[]);
        assert!(bad.is_err());
    }

    #[test]
    fn anomaly_resolution_scores_lucky_hands_and_voids_redeal() {
        let jan = CardSet::from_month(Month::new_unchecked(0));
        let south = jan
            .insert(Card::new_unchecked(4))
            .insert(Card::new_unchecked(5))
            .insert(Card::new_unchecked(8))
            .insert(Card::new_unchecked(9));
        let field = CardSet::from_card(Card::new_unchecked(16))
            .insert(Card::new_unchecked(20))
            .insert(Card::new_unchecked(24))
            .insert(Card::new_unchecked(28))
            .insert(Card::new_unchecked(32))
            .insert(Card::new_unchecked(36))
            .insert(Card::new_unchecked(40))
            .insert(Card::new_unchecked(44));
        let north = CardSet::from_card(Card::new_unchecked(17))
            .insert(Card::new_unchecked(21))
            .insert(Card::new_unchecked(25))
            .insert(Card::new_unchecked(29))
            .insert(Card::new_unchecked(33))
            .insert(Card::new_unchecked(37))
            .insert(Card::new_unchecked(41))
            .insert(Card::new_unchecked(45));
        let used = south.union(field).union(north);
        let stock: Vec<Card> = CardSet::ALL.into_iter().filter(|c| !used.contains(*c)).collect();

        let (mut state, anomaly) = KoiGameState::new_from_parts(
            &stock,
            [south, north],
            field,
            Player::South,
            Player::South,
            Ruleset::default(),
        );
        let anomaly = anomaly.unwrap();
        assert_eq!(
            state.resolve_deal_anomaly(anomaly),
            AnomalyResolution::InstantWin {
                player: Player::South,
                points: 6
            }
        );
        assert_eq!(state.score, [6, -6]);
        assert!(state.is_ended());
        assert_eq!(state.dealer, Player::South);

        // A field void redeals: the round does not advance. January moves
        // wholesale onto the field; the vacated hand slots are filled by
        // stock cards so the partition stays a permutation.
        let jan = CardSet::from_month(Month::new_unchecked(0));
        let south = CardSet::from_card(Card::new_unchecked(4))
            .insert(Card::new_unchecked(5))
            .insert(Card::new_unchecked(8))
            .insert(Card::new_unchecked(9))
            .insert(Card::new_unchecked(12))
            .insert(Card::new_unchecked(13))
            .insert(Card::new_unchecked(16))
            .insert(Card::new_unchecked(17));
        let north = CardSet::from_card(Card::new_unchecked(20))
            .insert(Card::new_unchecked(21))
            .insert(Card::new_unchecked(24))
            .insert(Card::new_unchecked(25))
            .insert(Card::new_unchecked(28))
            .insert(Card::new_unchecked(29))
            .insert(Card::new_unchecked(32))
            .insert(Card::new_unchecked(33));
        let used = south.union(north).union(jan);
        let stock: Vec<Card> = CardSet::ALL.into_iter().filter(|c| !used.contains(*c)).collect();
        let (mut state, anomaly) = KoiGameState::new_from_parts(
            &stock,
            [south, north],
            jan,
            Player::South,
            Player::South,
            Ruleset::default(),
        );
        let anomaly = anomaly.unwrap();
        assert_eq!(state.resolve_deal_anomaly(anomaly), AnomalyResolution::Redeal);
        assert_eq!(state.round, 0);
        assert!(state.is_ended());
    }

    #[test]
    fn next_round_carries_match_state() {
        let (mut state, _) = KoiGameState::new_deal(fresh_deck(), Ruleset::default());
        state.score = [7, -7];
        state.round = 3;
        state.dealer = Player::North;
        state.phase = TurnPhase::Ended;
        let (next, anomaly) = KoiGameState::next_round(fresh_deck(), &state);
        assert!(anomaly.is_none());
        assert_eq!(next.score, [7, -7]);
        assert_eq!(next.round, 3);
        assert_eq!(next.dealer, Player::North);
        assert_eq!(next.active, Player::North);
        assert!(matches!(next.phase, TurnPhase::AwaitingHandAction));
        assert_eq!(next.hands[0].count(), 8);
        assert_eq!(next.stock.len(), 24);
    }
}
