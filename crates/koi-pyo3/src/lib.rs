//! # Koi-PyO3
//!
//! Python C-extension binding layer for Koi-Maestro: the `_engine`
//! extension module (maturin `module-name = "koi_maestro._engine"`) and
//! its `pyo3-stub-gen` gatherer for the committed `_engine.pyi`.
//!
//! The surface mirrors the solver engine's own concepts — `Player`,
//! `Card`, `CaptureChoice`, `Action`, `GameState`, `PublicView`,
//! `LedgerEntry`, `PublicObservation`, `Solver` — with `py.detach` on
//! every heavy path so the GIL never serializes a resolve.

use std::sync::Arc;

use koi_core::{
    deal_from_seed, derive_named_seed, Action, AnomalyResolution, Card, KoiGameState, LedgerEntry, Player,
    PublicObservation, PublicView, Ruleset, TurnPhase,
};
use koi_solver::config::{Config, SolverMethod, ValidatedConfig};
use koi_solver::resolving::{load_gadget, GadgetOracle};
use koi_solver::Solver;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyclass_enum, gen_stub_pymethods};

/// The redeal stream tag the referee uses — kept in lockstep with
/// `runner.rs::REDEAL_STREAM_TAG` so a Python `GameState.deal(seed)` walks
/// the same deck sequence a mirrored leg would.
const REDEAL_STREAM_TAG: u64 = 0x4b4f_4952_444c_0001;
const MAX_REDEALS: u32 = 16;

fn to_value_error(error: impl std::fmt::Display) -> PyErr {
    PyValueError::new_err(error.to_string())
}

fn to_runtime_error(error: impl std::fmt::Display) -> PyErr {
    PyRuntimeError::new_err(error.to_string())
}

fn parse_ruleset(name: &str) -> PyResult<Ruleset> {
    match name {
        "nintendo" => Ok(Ruleset::nintendo()),
        "fuda_wiki" => Ok(Ruleset::fuda_wiki()),
        other => Err(PyValueError::new_err(format!(
            "unknown ruleset '{other}'; expected 'nintendo' or 'fuda_wiki'"
        ))),
    }
}

/// Resolves the deal loop the referee runs: anomalies either settle the
/// round instantly or redeal from the tagged stream.
fn deal_state(seed: u64, rules: Ruleset, prev: Option<&KoiGameState>) -> PyResult<(KoiGameState, u32)> {
    let deal_seed = seed;
    let mut redeals = 0u32;
    loop {
        let deck = if redeals == 0 {
            deal_from_seed(deal_seed)
        } else {
            deal_from_seed(derive_named_seed(deal_seed, REDEAL_STREAM_TAG + u64::from(redeals)))
        };
        let (mut state, anomaly) = match prev {
            None => KoiGameState::new_deal(deck, rules),
            Some(prev) => KoiGameState::next_round(deck, prev),
        };
        match anomaly {
            None => return Ok((state, redeals)),
            Some(anomaly) => match state.resolve_deal_anomaly(anomaly) {
                AnomalyResolution::InstantWin { .. } => return Ok((state, redeals)),
                AnomalyResolution::Redeal => {
                    redeals += 1;
                    if redeals > MAX_REDEALS {
                        return Err(PyRuntimeError::new_err(format!(
                            "deal anomaly persisted across {MAX_REDEALS} redeals"
                        )));
                    }
                }
            },
        }
    }
}

// ============================================================================
// Player
// ============================================================================

/// The two seats. `South` is the conventional "observer" seat; `North` its
/// opponent. Int comparisons work (`Player.South == 0`).
#[gen_stub_pyclass_enum]
#[pyclass(name = "Player", eq, eq_int, from_py_object)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PyPlayer {
    South,
    North,
}

impl From<PyPlayer> for Player {
    fn from(player: PyPlayer) -> Self {
        match player {
            PyPlayer::South => Player::South,
            PyPlayer::North => Player::North,
        }
    }
}

impl From<Player> for PyPlayer {
    fn from(player: Player) -> Self {
        match player {
            Player::South => PyPlayer::South,
            Player::North => PyPlayer::North,
        }
    }
}

// ============================================================================
// Card
// ============================================================================

/// One of the 48 Hanafuda cards, addressed by deck index `0..=47`
/// (`month * 4 + index_in_month`).
#[gen_stub_pyclass]
#[pyclass(name = "Card", from_py_object)]
#[derive(Debug, Clone, Copy)]
pub struct PyCard {
    pub(crate) inner: Card,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyCard {
    /// Args:
    ///     index (int): Deck index in `0..=47`.
    #[new]
    pub fn new(index: u8) -> PyResult<Self> {
        Card::new(index)
            .map(|inner| Self { inner })
            .ok_or_else(|| PyValueError::new_err(format!("card index {index} out of range 0..=47")))
    }

    /// The deck index `0..=47`.
    #[getter]
    pub fn index(&self) -> u8 {
        self.inner.index()
    }

    /// The month index `0..=11` (January = 0).
    #[getter]
    pub fn month(&self) -> u8 {
        self.inner.month().index()
    }

    /// The card's position inside its month `0..=3`.
    #[getter]
    pub fn index_in_month(&self) -> u8 {
        self.inner.index_in_month()
    }

    fn __repr__(&self) -> String {
        format!("Card({})", self.inner.index())
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        self.inner.index() as u64
    }
}

// ============================================================================
// CaptureChoice / Action
// ============================================================================

/// Which field cards a play or draw captures.
#[gen_stub_pyclass]
#[pyclass(name = "CaptureChoice", from_py_object)]
#[derive(Debug, Clone, Copy)]
pub struct PyCaptureChoice {
    pub(crate) inner: koi_core::CaptureChoice,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyCaptureChoice {
    /// The card joins the field — no capture.
    #[staticmethod]
    pub fn no_match() -> Self {
        Self {
            inner: koi_core::CaptureChoice::NoMatch,
        }
    }

    /// Exactly one matching card on the field; capture it.
    #[staticmethod]
    pub fn single(card: PyCard) -> Self {
        Self {
            inner: koi_core::CaptureChoice::Single(card.inner),
        }
    }

    /// Two matching cards on the field; capture the chosen one.
    #[staticmethod]
    pub fn pair(card: PyCard) -> Self {
        Self {
            inner: koi_core::CaptureChoice::Pair(card.inner),
        }
    }

    /// Three matching cards on the field; capture all of them.
    #[staticmethod]
    pub fn triple() -> Self {
        Self {
            inner: koi_core::CaptureChoice::Triple,
        }
    }

    /// "no_match" | "single" | "pair" | "triple".
    #[getter]
    pub fn kind(&self) -> &'static str {
        match self.inner {
            koi_core::CaptureChoice::NoMatch => "no_match",
            koi_core::CaptureChoice::Single(_) => "single",
            koi_core::CaptureChoice::Pair(_) => "pair",
            koi_core::CaptureChoice::Triple => "triple",
        }
    }

    /// The chosen field card for `single`/`pair`, else `None`.
    #[getter]
    pub fn card(&self) -> Option<PyCard> {
        match self.inner {
            koi_core::CaptureChoice::Single(card) | koi_core::CaptureChoice::Pair(card) => Some(PyCard { inner: card }),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        match self.inner {
            koi_core::CaptureChoice::NoMatch => "CaptureChoice.no_match()".to_owned(),
            koi_core::CaptureChoice::Single(card) => format!("CaptureChoice.single(Card({}))", card.index()),
            koi_core::CaptureChoice::Pair(card) => format!("CaptureChoice.pair(Card({}))", card.index()),
            koi_core::CaptureChoice::Triple => "CaptureChoice.triple()".to_owned(),
        }
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

/// A player decision: play a hand card, resolve the drawn stock card, call
/// Koi-Koi, or call Shōbu.
#[gen_stub_pyclass]
#[pyclass(name = "Action", from_py_object)]
#[derive(Debug, Clone, Copy)]
pub struct PyAction {
    pub(crate) inner: Action,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyAction {
    /// Play `card` from the hand, taking `capture` on the field.
    #[staticmethod]
    pub fn play_from_hand(card: PyCard, capture: PyCaptureChoice) -> Self {
        Self {
            inner: Action::PlayFromHand {
                card: card.inner,
                capture: capture.inner,
            },
        }
    }

    /// Resolve the publicly drawn stock card against the field.
    #[staticmethod]
    pub fn resolve_stock(capture: PyCaptureChoice) -> Self {
        Self {
            inner: Action::ResolveStock { capture: capture.inner },
        }
    }

    /// Call Koi-Koi — continue the round for a bigger score.
    #[staticmethod]
    pub fn koi_koi() -> Self {
        Self { inner: Action::KoiKoi }
    }

    /// Call Shōbu — stop the round and bank the score.
    #[staticmethod]
    pub fn shobu() -> Self {
        Self { inner: Action::Shobu }
    }

    /// "play_from_hand" | "resolve_stock" | "koi_koi" | "shobu".
    #[getter]
    pub fn kind(&self) -> &'static str {
        match self.inner {
            Action::PlayFromHand { .. } => "play_from_hand",
            Action::ResolveStock { .. } => "resolve_stock",
            Action::KoiKoi => "koi_koi",
            Action::Shobu => "shobu",
        }
    }

    /// The played card for `play_from_hand`, else `None`.
    #[getter]
    pub fn card(&self) -> Option<PyCard> {
        match self.inner {
            Action::PlayFromHand { card, .. } => Some(PyCard { inner: card }),
            _ => None,
        }
    }

    /// The capture choice for `play_from_hand`/`resolve_stock`, else `None`.
    #[getter]
    pub fn capture(&self) -> Option<PyCaptureChoice> {
        match self.inner {
            Action::PlayFromHand { capture, .. } | Action::ResolveStock { capture } => {
                Some(PyCaptureChoice { inner: capture })
            }
            _ => None,
        }
    }

    /// The canonical total-ordering key — stable across platforms and
    /// builds; sorts every evaluator/digest the same way.
    #[getter]
    pub fn action_key(&self) -> u64 {
        self.inner.action_key()
    }

    fn __repr__(&self) -> String {
        match self.inner {
            Action::PlayFromHand { card, capture } => {
                format!(
                    "Action.play_from_hand(Card({}), {:?})",
                    card.index(),
                    PyCaptureChoice { inner: capture }
                )
            }
            Action::ResolveStock { capture } => {
                format!("Action.resolve_stock({:?})", PyCaptureChoice { inner: capture })
            }
            Action::KoiKoi => "Action.koi_koi()".to_owned(),
            Action::Shobu => "Action.shobu()".to_owned(),
        }
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        self.inner.action_key()
    }
}

// ============================================================================
// PublicView / LedgerEntry / PublicObservation
// ============================================================================

/// The observer-relative public snapshot — every public zone, no hidden
/// cards. Obtained from `GameState.public_view(observer)`.
#[gen_stub_pyclass]
#[pyclass(name = "PublicView", from_py_object)]
#[derive(Debug, Clone)]
pub struct PyPublicView {
    pub(crate) inner: PublicView,
}

macro_rules! card_vec {
    ($set:expr) => {
        $set.into_iter().map(|card| PyCard { inner: card }).collect::<Vec<_>>()
    };
}

#[gen_stub_pymethods]
#[pymethods]
impl PyPublicView {
    /// The seat this snapshot is visible to.
    #[getter]
    pub fn observer(&self) -> PyPlayer {
        self.inner.observer.into()
    }

    /// The dealer seat.
    #[getter]
    pub fn dealer(&self) -> PyPlayer {
        self.inner.dealer.into()
    }

    /// The seat to move.
    #[getter]
    pub fn active(&self) -> PyPlayer {
        self.inner.active.into()
    }

    /// Completed player-turns this round.
    #[getter]
    pub fn turn(&self) -> u8 {
        self.inner.turn
    }

    /// The match round index.
    #[getter]
    pub fn round(&self) -> u8 {
        self.inner.round
    }

    /// The match score `(south, north)`.
    #[getter]
    pub fn score(&self) -> (i32, i32) {
        (self.inner.score[0], self.inner.score[1])
    }

    /// "hand" | "stock" | "stop" | "ended".
    #[getter]
    pub fn phase(&self) -> &'static str {
        match self.inner.phase {
            TurnPhase::AwaitingHandAction => "hand",
            TurnPhase::AwaitingStockResolution { .. } => "stock",
            TurnPhase::AwaitingStopDecision { .. } => "stop",
            TurnPhase::Ended => "ended",
        }
    }

    /// The publicly drawn card at a stock-resolution decision, else `None`.
    #[getter]
    pub fn drawn(&self) -> Option<PyCard> {
        match self.inner.phase {
            TurnPhase::AwaitingStockResolution { drawn } => Some(PyCard { inner: drawn }),
            _ => None,
        }
    }

    /// The offered base score at a stop decision, else `None`.
    #[getter]
    pub fn base_score(&self) -> Option<u32> {
        match self.inner.phase {
            TurnPhase::AwaitingStopDecision { base_score } => Some(base_score),
            _ => None,
        }
    }

    /// The observer's own hand.
    #[getter]
    pub fn own_hand(&self) -> Vec<PyCard> {
        card_vec!(self.inner.own_hand)
    }

    /// The opponent's hand size — a count, never the cards.
    #[getter]
    pub fn opponent_hand_count(&self) -> u8 {
        self.inner.opponent_hand_count
    }

    /// The field cards.
    #[getter]
    pub fn field(&self) -> Vec<PyCard> {
        card_vec!(self.inner.field)
    }

    /// The captured piles `(south, north)`.
    #[getter]
    pub fn captured(&self) -> (Vec<PyCard>, Vec<PyCard>) {
        (card_vec!(self.inner.captured[0]), card_vec!(self.inner.captured[1]))
    }

    /// Stock size including any pinned drawn card.
    #[getter]
    pub fn stock_count(&self) -> u8 {
        self.inner.stock_count
    }

    /// The Koi-Koi caller seat, if any.
    #[getter]
    pub fn koi_koi_caller(&self) -> Option<PyPlayer> {
        self.inner.koi_koi_caller.map(Into::into)
    }

    /// Per-seat Koi-Koi call counts `(south, north)`.
    #[getter]
    pub fn koi_koi_calls(&self) -> (u8, u8) {
        (self.inner.koi_koi_calls[0], self.inner.koi_koi_calls[1])
    }

    /// Last recorded yaku score per seat `(south, north)`.
    #[getter]
    pub fn last_yaku_score(&self) -> (u32, u32) {
        (self.inner.last_yaku_score[0], self.inner.last_yaku_score[1])
    }
}

/// One recorded public action in a round's ledger.
#[gen_stub_pyclass]
#[pyclass(name = "LedgerEntry", from_py_object)]
#[derive(Debug, Clone)]
pub struct PyLedgerEntry {
    /// The acting seat.
    #[pyo3(get)]
    pub player: PyPlayer,
    /// The action applied.
    #[pyo3(get)]
    pub action: PyAction,
    /// The publicly drawn card a `resolve_stock` entry resolved — required
    /// on that kind, `None` on every other.
    #[pyo3(get)]
    pub drawn: Option<PyCard>,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyLedgerEntry {
    /// Args:
    ///     player (Player): The acting seat.
    ///     action (Action): The applied action.
    ///     drawn (Card | None): The drawn card for `resolve_stock` entries.
    #[new]
    #[pyo3(signature = (player, action, drawn = None))]
    pub fn new(player: PyPlayer, action: PyAction, drawn: Option<PyCard>) -> Self {
        Self { player, action, drawn }
    }
}

impl From<&PyLedgerEntry> for LedgerEntry {
    fn from(entry: &PyLedgerEntry) -> Self {
        LedgerEntry {
            player: entry.player.into(),
            action: entry.action.inner,
            drawn: entry.drawn.map(|card| card.inner),
        }
    }
}

/// The public observation a ledger-driven solver consumes: the deal-time
/// view plus the ordered action ledger. Hidden cards never appear — the
/// observation is exactly what the attested seat could see.
#[gen_stub_pyclass]
#[pyclass(name = "PublicObservation", from_py_object)]
#[derive(Debug, Clone)]
pub struct PyPublicObservation {
    inner: PublicObservation,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyPublicObservation {
    /// Args:
    ///     initial (PublicView): The deal-time view for the observing seat —
    ///         `state.public_view(seat)` immediately after the deal.
    #[new]
    pub fn new(initial: PyPublicView) -> Self {
        Self {
            inner: PublicObservation {
                initial: initial.inner,
                entries: Vec::new(),
            },
        }
    }

    /// The ledger entries, in play order.
    #[getter]
    pub fn entries(&self) -> Vec<PyLedgerEntry> {
        self.inner
            .entries
            .iter()
            .map(|entry| PyLedgerEntry {
                player: entry.player.into(),
                action: PyAction { inner: entry.action },
                drawn: entry.drawn.map(|card| PyCard { inner: card }),
            })
            .collect()
    }

    /// Appends one applied action to the ledger.
    ///
    /// Args:
    ///     entry (LedgerEntry): The recorded public action.
    pub fn push(&mut self, entry: PyLedgerEntry) {
        self.inner.entries.push(LedgerEntry::from(&entry));
    }

    /// Number of recorded actions — `len(observation)`.
    pub fn __len__(&self) -> usize {
        self.inner.entries.len()
    }
}

// ============================================================================
// GameState
// ============================================================================

/// A live Koi-Koi round state — the transition kernel's own object, so
/// legality, phases, and scoring are exactly the bench's.
#[gen_stub_pyclass]
#[pyclass(name = "GameState", from_py_object)]
#[derive(Debug, Clone)]
pub struct PyGameState {
    pub(crate) inner: KoiGameState,
    /// Redeals this state consumed — provenance for the deal stream.
    #[pyo3(get)]
    pub redeals: u32,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyGameState {
    /// Deals a fresh round. Anomalies resolve like the referee's: instant
    /// wins settle immediately, voids redeal from the tagged stream.
    ///
    /// Args:
    ///     seed (int): Deal seed.
    ///     ruleset (str): "nintendo" (default) or "fuda_wiki".
    ///
    /// Returns:
    ///     GameState: The dealt state.
    #[staticmethod]
    #[pyo3(signature = (seed, ruleset = "nintendo"))]
    pub fn deal(seed: u64, ruleset: &str) -> PyResult<Self> {
        let rules = parse_ruleset(ruleset)?;
        let (inner, redeals) = deal_state(seed, rules, None)?;
        Ok(Self { inner, redeals })
    }

    /// Deals the next round of the match, carrying dealer rotation and the
    /// match score forward. The current state must have ended.
    ///
    /// Args:
    ///     seed (int): Deal seed for the next round's deck.
    #[pyo3(signature = (seed))]
    pub fn next_round(&self, seed: u64) -> PyResult<Self> {
        if !self.inner.is_ended() {
            return Err(PyValueError::new_err("next_round requires a finished round"));
        }
        let rules = self.inner.rules;
        let (inner, redeals) = deal_state(seed, rules, Some(&self.inner))?;
        Ok(Self { inner, redeals })
    }

    /// The kernel-legal actions for the seat to move.
    pub fn legal_actions(&self) -> Vec<PyAction> {
        self.inner
            .legal_actions()
            .into_iter()
            .map(|inner| PyAction { inner })
            .collect()
    }

    /// Applies a legal action, mutating the state in place.
    ///
    /// Raises:
    ///     ValueError: The kernel rejected the transition.
    pub fn apply_action(&mut self, action: PyAction) -> PyResult<()> {
        self.inner = self.inner.apply_action(action.inner).map_err(to_value_error)?;
        Ok(())
    }

    /// `True` once the round has ended.
    #[getter]
    pub fn is_ended(&self) -> bool {
        self.inner.is_ended()
    }

    /// "hand" | "stock" | "stop" | "ended".
    #[getter]
    pub fn phase(&self) -> &'static str {
        match self.inner.phase {
            TurnPhase::AwaitingHandAction => "hand",
            TurnPhase::AwaitingStockResolution { .. } => "stock",
            TurnPhase::AwaitingStopDecision { .. } => "stop",
            TurnPhase::Ended => "ended",
        }
    }

    /// The publicly drawn card at a stock-resolution decision, else `None`.
    #[getter]
    pub fn drawn(&self) -> Option<PyCard> {
        match self.inner.phase {
            TurnPhase::AwaitingStockResolution { drawn } => Some(PyCard { inner: drawn }),
            _ => None,
        }
    }

    /// The dealer seat.
    #[getter]
    pub fn dealer(&self) -> PyPlayer {
        self.inner.dealer.into()
    }

    /// The seat to move.
    #[getter]
    pub fn active(&self) -> PyPlayer {
        self.inner.active.into()
    }

    /// Completed player-turns this round.
    #[getter]
    pub fn turn(&self) -> u8 {
        self.inner.turn
    }

    /// The match round index.
    #[getter]
    pub fn round(&self) -> u8 {
        self.inner.round
    }

    /// The match score `(south, north)` — leg margin is `score[s] - score[n]`.
    #[getter]
    pub fn score(&self) -> (i32, i32) {
        (self.inner.score[0], self.inner.score[1])
    }

    /// A seat's hand — full information; use `public_view` for the
    /// observer-relative snapshot a solver is allowed to see.
    pub fn hand(&self, player: PyPlayer) -> Vec<PyCard> {
        card_vec!(self.inner.hands[Player::from(player).index()])
    }

    /// The field cards.
    #[getter]
    pub fn field(&self) -> Vec<PyCard> {
        card_vec!(self.inner.field)
    }

    /// Remaining stock size including any pinned drawn card.
    #[getter]
    pub fn stock_count(&self) -> usize {
        self.inner.stock.len()
    }

    /// Per-seat Koi-Koi call counts `(south, north)`.
    #[getter]
    pub fn koi_koi_calls(&self) -> (u8, u8) {
        (self.inner.koi_koi_calls[0], self.inner.koi_koi_calls[1])
    }

    /// The Koi-Koi caller seat, if any.
    #[getter]
    pub fn koi_koi_caller(&self) -> Option<PyPlayer> {
        self.inner.koi_koi_caller.map(Into::into)
    }

    /// Last recorded yaku score per seat `(south, north)`.
    #[getter]
    pub fn last_yaku_score(&self) -> (u32, u32) {
        (self.inner.last_yaku_score[0], self.inner.last_yaku_score[1])
    }

    /// The leg margin from `player`'s seat (`score[p] - score[opp]`).
    pub fn leg_margin(&self, player: PyPlayer) -> i32 {
        self.inner.leg_margin(player.into())
    }

    /// The observer-relative public snapshot of the current state.
    ///
    /// Args:
    ///     observer (Player): The seat the snapshot is visible to.
    pub fn public_view(&self, observer: PyPlayer) -> PyPublicView {
        PyPublicView {
            inner: self.inner.public_view(observer.into()),
        }
    }
}

// ============================================================================
// Solver settings
// ============================================================================

/// ISMCTS settings — `solver.ismcts.*`.
#[gen_stub_pyclass]
#[pyclass(name = "IsmctsSettings", from_py_object)]
#[derive(Debug, Clone)]
pub struct PyIsmctsSettings {
    /// Iterations per decision. Default: 10000.
    #[pyo3(get, set)]
    pub iterations: usize,
    /// Reward normalization bound. Default: 20.0.
    #[pyo3(get, set)]
    pub score_norm: f32,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyIsmctsSettings {
    #[new]
    pub fn new() -> Self {
        let defaults = koi_solver::config::IsmctsConfig::default();
        Self {
            iterations: defaults.iterations,
            score_norm: defaults.score_norm,
        }
    }
}

impl Default for PyIsmctsSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// PIMC settings — `solver.pimc.*`.
#[gen_stub_pyclass]
#[pyclass(name = "PimcSettings", from_py_object)]
#[derive(Debug, Clone)]
pub struct PyPimcSettings {
    /// Determinizations sampled per decision. Default: 64.
    #[pyo3(get, set)]
    pub max_simulations: usize,
    /// Rayon chunk size over the determinization list. Default: 32.
    #[pyo3(get, set)]
    pub batch_size: usize,
    /// "mean" or "win_rate". Default: "mean".
    #[pyo3(get, set)]
    pub scoring_method: String,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyPimcSettings {
    #[new]
    pub fn new() -> Self {
        let defaults = koi_solver::config::PimcConfig::default();
        Self {
            max_simulations: defaults.max_simulations,
            batch_size: defaults.batch_size,
            scoring_method: defaults.scoring_method,
        }
    }
}

impl Default for PyPimcSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Resolving settings — `solver.resolving.*`.
#[gen_stub_pyclass]
#[pyclass(name = "ResolvingSettings", from_py_object)]
#[derive(Debug, Clone)]
pub struct PyResolvingSettings {
    /// Belief cap: opponent-hand worlds per resolve. Default: 12.
    #[pyo3(get, set)]
    pub max_worlds: usize,
    /// Compiled gadget-tree node budget. Default: 150000.
    #[pyo3(get, set)]
    pub max_nodes: usize,
    /// CFR+ iterations over the gadget tree. Default: 60.
    #[pyo3(get, set)]
    pub cfr_iterations: usize,
    /// Decision-ply cap below each world root. Default: 3.
    #[pyo3(get, set)]
    pub max_decision_depth: usize,
    /// Opponent-model reweight + archetype posterior. Default: True.
    #[pyo3(get, set)]
    pub adaptive: bool,
    /// OX exploitation arm — requires `adaptive`. Default: False.
    #[pyo3(get, set)]
    pub ox: bool,
    /// Blueprint artifact path for the certified gadget oracle; `None`
    /// selects the learned rollout oracle (empirical guard).
    #[pyo3(get, set)]
    pub blueprint_artifact: Option<String>,
    /// Learned leaf evaluator for oracle leaves: `"builtin:handcrafted"`
    /// selects the deterministic fallback evaluator; any other value is an
    /// ONNX model path (requires a `leaf-ort` build). `None` prices leaves
    /// through the margin oracle.
    #[pyo3(get, set)]
    pub leaf_model: Option<String>,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyResolvingSettings {
    #[new]
    pub fn new() -> Self {
        let defaults = koi_solver::config::ResolvingConfig::default();
        Self {
            max_worlds: defaults.max_worlds,
            max_nodes: defaults.max_nodes,
            cfr_iterations: defaults.cfr_iterations,
            max_decision_depth: defaults.max_decision_depth,
            adaptive: defaults.adaptive,
            ox: defaults.ox,
            blueprint_artifact: defaults.blueprint_artifact,
            leaf_model: defaults.leaf_model,
        }
    }
}

impl Default for PyResolvingSettings {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Solver
// ============================================================================

/// The solver facade: configuration, build, and decision calls for every
/// backend the engine ships.
#[gen_stub_pyclass]
#[pyclass(name = "Solver")]
pub struct PySolver {
    inner: Option<Solver>,

    /// The backend: "random", "heuristic", "ismcts", "pimc", "endgame",
    /// "resolving" (alias "resolving_adaptive"), "leaf_policy", or
    /// "archetype".
    #[pyo3(get, set)]
    pub method: String,

    /// The scripted archetype name when `method == "archetype"`:
    /// "heuristic" | "materialist" | "yaku_chaser" | "banker" | "gambler" |
    /// "timid" | "random".
    #[pyo3(get, set)]
    pub archetype: Option<String>,

    /// ISMCTS settings — `solver.ismcts.*`.
    #[pyo3(get)]
    pub ismcts: Py<PyIsmctsSettings>,

    /// PIMC settings — `solver.pimc.*`.
    #[pyo3(get)]
    pub pimc: Py<PyPimcSettings>,

    /// Resolving settings — `solver.resolving.*`.
    #[pyo3(get)]
    pub resolving: Py<PyResolvingSettings>,

    /// ONNX leaf-model path for `method == "leaf_policy"`; `None` selects
    /// the deterministic handcrafted evaluator.
    #[pyo3(get, set)]
    pub leaf_model: Option<String>,

    /// Root-parallel search where the backend supports it.
    #[pyo3(get, set)]
    pub use_parallel: bool,

    /// "error" | "warn" | "info" | "debug" | "trace".
    #[pyo3(get, set)]
    pub log_level: String,

    /// The master seed.
    #[pyo3(get)]
    pub seed: u64,
}

impl PySolver {
    fn raw_config(&self, py: Python<'_>) -> Config {
        let ismcts = self.ismcts.borrow(py);
        let pimc = self.pimc.borrow(py);
        let resolving = self.resolving.borrow(py);
        Config {
            solver: self.method.clone(),
            archetype: self.archetype.clone(),
            ismcts: koi_solver::config::IsmctsConfig {
                iterations: ismcts.iterations,
                score_norm: ismcts.score_norm,
            },
            pimc: koi_solver::config::PimcConfig {
                max_simulations: pimc.max_simulations,
                batch_size: pimc.batch_size,
                scoring_method: pimc.scoring_method.clone(),
            },
            resolving: koi_solver::config::ResolvingConfig {
                max_worlds: resolving.max_worlds,
                max_nodes: resolving.max_nodes,
                cfr_iterations: resolving.cfr_iterations,
                max_decision_depth: resolving.max_decision_depth,
                adaptive: resolving.adaptive,
                ox: resolving.ox,
                blueprint_artifact: resolving.blueprint_artifact.clone(),
                leaf_model: resolving.leaf_model.clone(),
            },
            leaf_policy: koi_solver::config::LeafPolicyConfig {
                leaf_model: self.leaf_model.clone(),
            },
            use_parallel: self.use_parallel,
            log_level: self.log_level.clone(),
            seed: Some(self.seed),
        }
    }

    fn validated_config(&self, py: Python<'_>) -> PyResult<ValidatedConfig> {
        self.raw_config(py).validate().map_err(to_value_error)
    }

    fn ensure_current_build<'a>(&'a self, config: &ValidatedConfig) -> PyResult<&'a Solver> {
        self.inner
            .as_ref()
            .filter(|solver| solver.config().effective_solver() == config.effective_solver())
            .ok_or_else(|| PyRuntimeError::new_err("solver settings changed after build; call build() again"))
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PySolver {
    /// Creates a solver with default settings.
    ///
    /// Args:
    ///     method (str): "random", "heuristic", "ismcts", "pimc", "endgame",
    ///         "resolving" (alias "resolving_adaptive"), or "archetype".
    ///     archetype (str | None): Required when `method == "archetype"`.
    ///     seed (int | None): Master seed; a fresh one is drawn when omitted.
    #[new]
    #[pyo3(signature = (method, archetype = None, seed = None))]
    pub fn new(py: Python<'_>, method: String, archetype: Option<String>, seed: Option<u64>) -> PyResult<Self> {
        method
            .parse::<SolverMethod>()
            .map_err(|detail| PyValueError::new_err(format!("unknown solver method '{method}': {detail}")))?;
        let seed = seed
            .filter(|seed| *seed != 0)
            .unwrap_or_else(koi_core::seed::random_nonzero_seed);
        Ok(PySolver {
            inner: None,
            method,
            archetype,
            ismcts: Py::new(py, PyIsmctsSettings::new())?,
            pimc: Py::new(py, PyPimcSettings::new())?,
            resolving: Py::new(py, PyResolvingSettings::new())?,
            leaf_model: None,
            use_parallel: true,
            log_level: "warn".to_owned(),
            seed,
        })
    }

    /// Builds the engine under the current settings.
    ///
    /// Returns:
    ///     Solver: A built solver. Mutating settings afterwards requires
    ///         another `build()` before deciding.
    pub fn build(&mut self, py: Python<'_>) -> PyResult<()> {
        let config = self.validated_config(py)?;
        let solver = Solver::from_validated_config(config.with_effective_seed(self.seed));
        self.inner = Some(solver);
        Ok(())
    }

    /// Picks an action for the current state.
    ///
    /// Args:
    ///     state (GameState): The decision state.
    ///     observation (PublicObservation | None): The attested public
    ///         observation — required by `resolving` (the ledger IS its
    ///         input; omitting it fails rather than falling back), ignored
    ///         by the other backends.
    ///     solver_seed (int | None): Per-decision seed; defaults to the
    ///         solver's master seed.
    ///
    /// Returns:
    ///     Action | None: The chosen action; `None` on terminal states.
    #[pyo3(signature = (state, observation = None, solver_seed = None))]
    pub fn find_best_action(
        &self,
        py: Python<'_>,
        state: &PyGameState,
        observation: Option<&PyPublicObservation>,
        solver_seed: Option<u64>,
    ) -> PyResult<Option<PyAction>> {
        let config = self.validated_config(py)?;
        let solver = self.ensure_current_build(&config)?;
        let solver_seed = solver_seed.unwrap_or(self.seed);
        let state = state.inner;
        match (config.solver == SolverMethod::Resolving, observation) {
            (true, None) => {
                // The resolving contract is fail-closed, not a silent
                // fallback — mirror the engine's own MissingObservation.
                Err(PyValueError::new_err(
                    "resolving requires the public ledger observation",
                ))
            }
            (true, Some(observation)) => {
                let gadget = match &config.resolving.blueprint_artifact {
                    None => GadgetOracle::Learned,
                    Some(path) => {
                        let gadget = load_gadget(std::path::Path::new(path)).map_err(to_runtime_error)?;
                        GadgetOracle::Blueprint(Arc::new(gadget))
                    }
                };
                let observation = observation.inner.clone();
                py.detach(|| solver.find_best_action_observed(&state, &observation, &gadget, solver_seed))
                    .map(|action| action.map(|inner| PyAction { inner }))
                    .map_err(to_runtime_error)
            }
            _ => py
                .detach(|| solver.find_best_action(&state, solver_seed))
                .map(|action| action.map(|inner| PyAction { inner }))
                .map_err(to_runtime_error),
        }
    }
}

// ============================================================================
// Module
// ============================================================================

#[pymodule]
#[pyo3(name = "_engine")]
fn _engine(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add_class::<PyPlayer>()?;
    m.add_class::<PyCard>()?;
    m.add_class::<PyCaptureChoice>()?;
    m.add_class::<PyAction>()?;
    m.add_class::<PyPublicView>()?;
    m.add_class::<PyLedgerEntry>()?;
    m.add_class::<PyPublicObservation>()?;
    m.add_class::<PyGameState>()?;
    m.add_class::<PyIsmctsSettings>()?;
    m.add_class::<PyPimcSettings>()?;
    m.add_class::<PyResolvingSettings>()?;
    m.add_class::<PySolver>()?;
    Ok(())
}

/// Gathers stub info for `stub_gen`. `pyproject.toml` lives at the
/// repository root (two levels above the crate manifest), so the default
/// `define_stub_info_gatherer!` path does not apply.
pub fn stub_info() -> pyo3_stub_gen::Result<pyo3_stub_gen::StubInfo> {
    // Walk ancestors — the crate's parent in the legacy layout, its
    // grandparent under `crates/<member>` in the workspace.
    let manifest_dir: &std::path::Path = env!("CARGO_MANIFEST_DIR").as_ref();
    let pyproject = manifest_dir
        .ancestors()
        .map(|dir| dir.join("pyproject.toml"))
        .find(|path| path.exists())
        .unwrap_or_else(|| manifest_dir.join("pyproject.toml"));
    pyo3_stub_gen::StubInfo::from_pyproject_toml(pyproject)
}

/// Renders the committed `src/koi_maestro/_engine.pyi` — shared by
/// `stub_gen` (writes it) and the freshness test (diffs it) so the two
/// can never drift.
///
/// `m.add("__version__", ...)` is invisible to the gatherer — the
/// declaration is appended so the stub matches the runtime surface
/// exactly.
pub fn render_stub_pyi() -> pyo3_stub_gen::Result<String> {
    let stub = stub_info()?;
    let module = stub
        .modules
        .iter()
        .find(|(name, _)| name.ends_with("_engine"))
        .map(|(_, module)| module)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "no `_engine` module gathered; modules: {:?}",
                stub.modules.keys().collect::<Vec<_>>()
            )
        })?;
    Ok(format!(
        "{}\n__version__: builtins.str\n",
        module.format_with_config(stub.config.use_type_statement)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// The committed `_engine.pyi` must be regenerated, not edited — a
    /// drifted stub misstates the runtime surface for every typed caller.
    #[test]
    fn committed_stub_matches_generated() {
        let rendered = render_stub_pyi().expect("stub info gathers");
        let manifest_dir: &Path = env!("CARGO_MANIFEST_DIR").as_ref();
        let committed = manifest_dir
            .ancestors()
            .map(|dir| dir.join("src").join("koi_maestro").join("_engine.pyi"))
            .find(|path| path.exists())
            .expect("src/koi_maestro/_engine.pyi exists");
        let committed = std::fs::read_to_string(committed).expect("stub is readable");
        assert_eq!(
            committed, rendered,
            "src/koi_maestro/_engine.pyi is stale — regenerate with `cargo run -p koi-pyo3 --bin stub_gen`"
        );
    }
}
