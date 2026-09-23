//! Worker wire protocol (version 1).
//!
//! A decision frame carries the acting seat's public view plus the round's
//! public ledger — every action and every publicly drawn card since the
//! deal. The worker rebuilds a canonical state via
//! `KoiGameState::from_public_view`, replays the ledger through the real
//! state machine with lazy placeholder assignment for hidden cards, and
//! fails closed if the ledger and the snapshot disagree. Hidden card
//! identities never cross the wire.
//!
//! Unlike the reference harness, koi starts at version 1 *with* the ledger
//! — there is no ledger-less legacy frame to stay compatible with.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use koi_core::{
    Action, CaptureChoice, Card, CardSet, KoiGameState, LedgerEntry, Player, PublicObservation, PublicView, Ruleset,
    TurnPhase,
};
use koi_solver::resolving::{pin_pending_draw, replay_ledger};

/// The newest worker protocol this build implements.
pub const WORKER_PROTOCOL_VERSION: u32 = 1;
/// The oldest worker protocol still served.
pub const MIN_WORKER_PROTOCOL_VERSION: u32 = 1;
/// Every protocol version a worker built from this tree can serve.
pub const SUPPORTED_WORKER_PROTOCOLS: &[u32] = &[1];
pub const RESPONSE_PREFIX: &str = "@@KOI_BENCH@@";

/// The `kind` tag a pre-flight negotiation frame carries on the wire.
pub const NEGOTIATION_REQUEST_KIND: &str = "negotiate";
pub const NEGOTIATION_RESPONSE_KIND: &str = "negotiation";

/// Hard bound on ledger entries: one round is at most 16 player-turns × 3
/// phase actions; a longer ledger is malformed.
pub const MAX_LEDGER_ENTRIES: usize = 64;

/// Pre-flight capability probe the referee sends before the first leg.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NegotiationRequest {
    pub kind: String,
    pub request_id: u64,
}

impl NegotiationRequest {
    pub fn new(request_id: u64) -> Self {
        Self {
            kind: NEGOTIATION_REQUEST_KIND.to_owned(),
            request_id,
        }
    }
}

/// The typed answer a current worker gives to [`NegotiationRequest`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NegotiationResponse {
    pub kind: String,
    pub request_id: u64,
    pub supported_protocols: Vec<u32>,
    pub worker_build_commit: String,
    pub worker_build_tree: String,
    pub worker_build_dirty: bool,
}

impl NegotiationResponse {
    pub fn new(request_id: u64) -> Self {
        Self {
            kind: NEGOTIATION_RESPONSE_KIND.to_owned(),
            request_id,
            supported_protocols: SUPPORTED_WORKER_PROTOCOLS.to_vec(),
            worker_build_commit: crate::artifact::BUILD_COMMIT.to_string(),
            worker_build_tree: crate::artifact::BUILD_TREE.to_string(),
            worker_build_dirty: crate::artifact::build_dirty(),
        }
    }
}

/// A capture choice on the wire: a kind tag plus the matched field card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireCapture {
    /// "no_match" | "single" | "pair" | "triple".
    pub kind: String,
    /// The matched field card for single/pair; absent otherwise.
    pub card: Option<u8>,
}

impl WireCapture {
    pub fn from_capture(capture: CaptureChoice) -> Self {
        match capture {
            CaptureChoice::NoMatch => Self {
                kind: "no_match".to_owned(),
                card: None,
            },
            CaptureChoice::Single(card) => Self {
                kind: "single".to_owned(),
                card: Some(card.index()),
            },
            CaptureChoice::Pair(card) => Self {
                kind: "pair".to_owned(),
                card: Some(card.index()),
            },
            CaptureChoice::Triple => Self {
                kind: "triple".to_owned(),
                card: None,
            },
        }
    }

    pub fn to_capture(&self) -> Result<CaptureChoice> {
        let card = self
            .card
            .map(|index| Card::new(index).context("capture card index out of range"))
            .transpose()?;
        match self.kind.as_str() {
            "no_match" if self.card.is_none() => Ok(CaptureChoice::NoMatch),
            "single" => card.map(CaptureChoice::Single).context("single capture needs a card"),
            "pair" => card.map(CaptureChoice::Pair).context("pair capture needs a card"),
            "triple" if self.card.is_none() => Ok(CaptureChoice::Triple),
            other => bail!("invalid wire capture kind '{other}'"),
        }
    }
}

/// An action on the wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireAction {
    /// "play_from_hand" | "resolve_stock" | "koi_koi" | "shobu".
    pub kind: String,
    /// The played card for `play_from_hand`; absent otherwise.
    pub card: Option<u8>,
    /// The capture choice for play/resolve kinds; absent for calls.
    pub capture: Option<WireCapture>,
}

impl WireAction {
    pub fn from_action(action: Action) -> Self {
        match action {
            Action::PlayFromHand { card, capture } => Self {
                kind: "play_from_hand".to_owned(),
                card: Some(card.index()),
                capture: Some(WireCapture::from_capture(capture)),
            },
            Action::ResolveStock { capture } => Self {
                kind: "resolve_stock".to_owned(),
                card: None,
                capture: Some(WireCapture::from_capture(capture)),
            },
            Action::KoiKoi => Self {
                kind: "koi_koi".to_owned(),
                card: None,
                capture: None,
            },
            Action::Shobu => Self {
                kind: "shobu".to_owned(),
                card: None,
                capture: None,
            },
        }
    }

    pub fn to_action(&self) -> Result<Action> {
        match self.kind.as_str() {
            "play_from_hand" => {
                let card = self
                    .card
                    .and_then(Card::new)
                    .context("play_from_hand needs a valid card index")?;
                let capture = self
                    .capture
                    .as_ref()
                    .context("play_from_hand needs a capture choice")?
                    .to_capture()?;
                Ok(Action::PlayFromHand { card, capture })
            }
            "resolve_stock" => {
                if self.card.is_some() {
                    bail!("resolve_stock carries no played card");
                }
                let capture = self
                    .capture
                    .as_ref()
                    .context("resolve_stock needs a capture choice")?
                    .to_capture()?;
                Ok(Action::ResolveStock { capture })
            }
            "koi_koi" if self.card.is_none() && self.capture.is_none() => Ok(Action::KoiKoi),
            "shobu" if self.card.is_none() && self.capture.is_none() => Ok(Action::Shobu),
            other => bail!("invalid wire action kind '{other}'"),
        }
    }
}

/// One public ledger entry: who acted, what they did, and — for a stock
/// resolution — which card was drawn face-up. The drawn card is public at
/// draw time, so recording it leaks nothing yet lets the worker replay the
/// exact public trajectory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireLedgerEntry {
    /// 0 = South, 1 = North.
    pub player: u8,
    pub action: WireAction,
    /// The publicly drawn card for `resolve_stock` entries; absent else.
    pub drawn: Option<u8>,
}

/// The public view at the round's deal — the ledger's replay origin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireDealView {
    /// The observer's dealt hand (own cards only).
    pub own_hand: u64,
    pub opponent_hand_count: u8,
    pub field: u64,
    pub stock_count: u8,
    pub dealer: u8,
    /// Match score carried into this round — the ledger replays a single
    /// round, so the replay origin needs the score the deal started with.
    pub deal_score: [i32; 2],
}

/// The public-state snapshot a decision request carries. Everything is
/// observer-relative: `own_*` is the deciding seat's view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireState {
    /// The deciding seat this snapshot belongs to (0 = South, 1 = North).
    pub observer: u8,
    /// The ruleset label: "nintendo" or "fuda_wiki".
    pub ruleset: String,
    pub dealer: u8,
    /// The player to move — always `observer` for a decision frame.
    pub active: u8,
    pub turn: u8,
    pub round: u8,
    pub score: [i32; 2],
    /// "hand" | "stock" | "stop" | "ended".
    pub phase: String,
    /// The publicly drawn card when `phase == "stock"`.
    pub drawn: Option<u8>,
    /// The offered base score when `phase == "stop"`.
    pub base_score: Option<u32>,
    pub own_hand: u64,
    pub opponent_hand_count: u8,
    pub field: u64,
    pub captured: [u64; 2],
    /// Total stock size including any pinned drawn card.
    pub stock_count: u8,
    /// Koi-Koi caller seat, if any.
    pub koi_koi_caller: Option<u8>,
    pub koi_koi_calls: [u8; 2],
    pub last_yaku_score: [u32; 2],
    /// The deal this round started from (observer's view).
    pub initial: WireDealView,
    /// The public action ledger since the deal.
    pub ledger: Vec<WireLedgerEntry>,
}

impl WireState {
    /// Snapshots `state` from `observer`'s seat. The caller fills
    /// `initial`/`ledger` separately — see [`Self::from_parts`].
    pub fn view_fields(state: &KoiGameState, observer: Player) -> Self {
        let view = state.public_view(observer);
        let (phase, drawn, base_score) = match view.phase {
            TurnPhase::AwaitingHandAction => ("hand".to_owned(), None, None),
            TurnPhase::AwaitingStockResolution { drawn } => ("stock".to_owned(), Some(drawn.index()), None),
            TurnPhase::AwaitingStopDecision { base_score } => ("stop".to_owned(), None, Some(base_score)),
            TurnPhase::Ended => ("ended".to_owned(), None, None),
        };
        Self {
            observer: player_to_wire(observer),
            ruleset: ruleset_to_wire(&view.rules),
            dealer: player_to_wire(view.dealer),
            active: player_to_wire(view.active),
            turn: view.turn,
            round: view.round,
            score: view.score,
            phase,
            drawn,
            base_score,
            own_hand: view.own_hand.bits(),
            opponent_hand_count: view.opponent_hand_count,
            field: view.field.bits(),
            captured: [view.captured[0].bits(), view.captured[1].bits()],
            stock_count: view.stock_count,
            koi_koi_caller: view.koi_koi_caller.map(player_to_wire),
            koi_koi_calls: view.koi_koi_calls,
            last_yaku_score: view.last_yaku_score,
            initial: WireDealView {
                own_hand: 0,
                opponent_hand_count: 0,
                field: 0,
                stock_count: 0,
                dealer: player_to_wire(view.dealer),
                deal_score: view.score,
            },
            ledger: Vec::new(),
        }
    }

    /// The observer-relative public observation the frame attests: the
    /// deal-time view plus the ordered ledger, in the kernel types the
    /// solver's ledger replay consumes. A stateful solver (the P3
    /// resolver) needs this explicitly — play order is not a zone
    /// function, so it cannot be derived from the bare snapshot.
    pub fn to_observation(&self) -> Result<PublicObservation> {
        if self.ledger.len() > MAX_LEDGER_ENTRIES {
            bail!("ledger exceeds the {MAX_LEDGER_ENTRIES}-entry round bound");
        }
        let observer = wire_to_player(self.observer)?;
        let dealer = wire_to_player(self.initial.dealer)?;
        let initial = PublicView {
            observer,
            rules: wire_to_ruleset(&self.ruleset)?,
            dealer,
            active: dealer,
            turn: 0,
            round: self.round,
            score: self.initial.deal_score,
            phase: TurnPhase::AwaitingHandAction,
            own_hand: CardSet::new(self.initial.own_hand).context("initial own_hand has bits above card 47")?,
            opponent_hand_count: self.initial.opponent_hand_count,
            field: CardSet::new(self.initial.field).context("initial field has bits above card 47")?,
            captured: [CardSet::EMPTY, CardSet::EMPTY],
            stock_count: self.initial.stock_count,
            koi_koi_caller: None,
            koi_koi_calls: [0; 2],
            last_yaku_score: [0; 2],
        };
        let mut entries = Vec::with_capacity(self.ledger.len());
        for entry in &self.ledger {
            entries.push(LedgerEntry {
                player: wire_to_player(entry.player)?,
                action: entry.action.to_action()?,
                drawn: entry
                    .drawn
                    .map(|index| Card::new(index).context("drawn card index out of range"))
                    .transpose()?,
            });
        }
        Ok(PublicObservation { initial, entries })
    }

    /// Rebuilds the canonical `KoiGameState` from the snapshot, then replays
    /// the ledger to prove the snapshot is reachable — fail-closed.
    pub fn into_state(&self) -> Result<KoiGameState> {
        let observer = wire_to_player(self.observer)?;
        let dealer = wire_to_player(self.dealer)?;
        let active = wire_to_player(self.active)?;
        if active != observer {
            bail!("a decision frame must be addressed to the acting player");
        }
        let rules = wire_to_ruleset(&self.ruleset)?;
        let phase = match self.phase.as_str() {
            "hand" if self.drawn.is_none() && self.base_score.is_none() => TurnPhase::AwaitingHandAction,
            "stock" => {
                let drawn = self
                    .drawn
                    .and_then(Card::new)
                    .context("stock phase needs a valid drawn card")?;
                if self.base_score.is_some() {
                    bail!("stock phase carries no base score");
                }
                TurnPhase::AwaitingStockResolution { drawn }
            }
            "stop" => {
                let base_score = self.base_score.context("stop phase needs a base score")?;
                if self.drawn.is_some() {
                    bail!("stop phase carries no drawn card");
                }
                TurnPhase::AwaitingStopDecision { base_score }
            }
            "ended" if self.drawn.is_none() && self.base_score.is_none() => TurnPhase::Ended,
            other => bail!("invalid wire phase '{other}'"),
        };
        // Scores enter the i32 accumulator during replay — bound the wire
        // values to a range no real match approaches so a crafted frame
        // cannot wrap (release) or panic (debug) the accumulator.
        const MAX_WIRE_SCORE: i32 = 1_000_000;
        if self
            .score
            .iter()
            .chain(self.initial.deal_score.iter())
            .any(|score| score.abs() > MAX_WIRE_SCORE)
        {
            bail!("wire score outside the sane-match range");
        }
        // The remaining u32/u8 bookkeeping feeds `apply_multipliers`' `*=`
        // and `finalize_round`'s `round += 1` — same wrap/panic channel.
        if self.base_score.is_some_and(|base| base > MAX_WIRE_SCORE as u32)
            || self.round >= 200
            || self.last_yaku_score.iter().any(|score| *score > MAX_WIRE_SCORE as u32)
            || self.koi_koi_calls.iter().any(|calls| *calls > 32)
        {
            bail!("wire bookkeeping outside the sane-match range");
        }
        let own_hand = CardSet::new(self.own_hand).context("own_hand has bits above card 47")?;
        let field = CardSet::new(self.field).context("field has bits above card 47")?;
        let captured = [
            CardSet::new(self.captured[0]).context("captured[0] has bits above card 47")?,
            CardSet::new(self.captured[1]).context("captured[1] has bits above card 47")?,
        ];
        let view = PublicView {
            observer,
            rules,
            dealer,
            active,
            turn: self.turn,
            round: self.round,
            score: self.score,
            phase,
            own_hand,
            opponent_hand_count: self.opponent_hand_count,
            field,
            captured,
            stock_count: self.stock_count,
            koi_koi_caller: self.koi_koi_caller.map(wire_to_player).transpose()?,
            koi_koi_calls: self.koi_koi_calls,
            last_yaku_score: self.last_yaku_score,
        };
        let state = KoiGameState::from_public_view(&view)
            .map_err(|error| anyhow::anyhow!("snapshot violates state invariants: {error}"))?;
        validate_wire_ledger(self, observer, &state)
            .context("attested public ledger is inconsistent with the snapshot")?;
        Ok(state)
    }
}

/// Replays the ledger from the declared deal to the current snapshot.
///
/// Hidden cards are assigned lazily: when the ledger says the opponent
/// played a card its placeholder hand does not contain, an unidentified
/// placeholder is swapped out to the stock; when a drawn card is named, it
/// is pinned to the stock head. Every action then passes through the real
/// `apply_action` legality — capture validity, phase machine, Koi-Koi
/// bookkeeping — so a legal-looking but unreachable ledger fails closed.
fn validate_wire_ledger(frame: &WireState, observer: Player, snapshot: &KoiGameState) -> Result<()> {
    // The replay runs through the solver's canonical ledger replayer — one
    // implementation of hidden-card materialization, shared with the
    // resolver so the wire gate and the world model can never drift.
    let observation = frame.to_observation().context("attested public ledger is malformed")?;
    let replay = replay_ledger(&observation).map_err(|error| anyhow::anyhow!("{error}"))?;
    let opponent = observer.opponent();
    let mut state = replay.state;

    // The frame's pending stock resolution names a real drawn card that no
    // ledger entry carries — it is the *current* decision's public fact.
    // Pin it before the snapshot comparison or the placeholder draw never
    // matches.
    if let TurnPhase::AwaitingStockResolution { drawn } = snapshot.phase {
        let replayed = match state.phase {
            TurnPhase::AwaitingStockResolution { drawn } => drawn,
            _ => bail!("ledger replay ends in a different phase than the snapshot"),
        };
        if replayed != drawn {
            state = pin_pending_draw(state, opponent, drawn)
                .map_err(|error| anyhow::anyhow!("pending drawn-card pinning failed: {error}"))?;
        }
    }

    // The replayed trajectory must land on the snapshot's public zones
    // exactly — a desyncing worker cannot launder a phantom capture.
    for (label, replayed, declared) in [
        ("field", state.field, snapshot.field),
        ("captured[0]", state.captured[0], snapshot.captured[0]),
        ("captured[1]", state.captured[1], snapshot.captured[1]),
        (
            "own_hand",
            state.hands[observer.index()],
            snapshot.hands[observer.index()],
        ),
    ] {
        if replayed != declared {
            bail!("ledger replay ends on a different {label} than the snapshot");
        }
    }
    if state.hands[opponent.index()].count() != snapshot.hands[opponent.index()].count() {
        bail!("ledger replay ends on a different opponent hand count than the snapshot");
    }
    if state.stock.len() != snapshot.stock.len() {
        bail!("ledger replay ends on a different stock size than the snapshot");
    }
    if state.phase != snapshot.phase
        || state.turn != snapshot.turn
        || state.active != snapshot.active
        || state.dealer != snapshot.dealer
        || state.koi_koi_caller != snapshot.koi_koi_caller
        || state.koi_koi_calls != snapshot.koi_koi_calls
        || state.last_yaku_score != snapshot.last_yaku_score
        || state.score != snapshot.score
    {
        bail!("ledger replay ends on different bookkeeping than the snapshot");
    }
    Ok(())
}

fn player_to_wire(player: Player) -> u8 {
    match player {
        Player::South => 0,
        Player::North => 1,
    }
}

fn wire_to_player(value: u8) -> Result<Player> {
    match value {
        0 => Ok(Player::South),
        1 => Ok(Player::North),
        _ => bail!("invalid wire player value {value}"),
    }
}

fn ruleset_to_wire(rules: &Ruleset) -> String {
    match rules {
        Ruleset::Nintendo => "nintendo".to_owned(),
        Ruleset::FudaWiki => "fuda_wiki".to_owned(),
        Ruleset::House(_) => "house".to_owned(),
    }
}

fn wire_to_ruleset(value: &str) -> Result<Ruleset> {
    match value {
        "nintendo" => Ok(Ruleset::nintendo()),
        "fuda_wiki" => Ok(Ruleset::fuda_wiki()),
        other => bail!("unsupported wire ruleset '{other}'"),
    }
}

/// The decision frame the referee writes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionRequest {
    pub protocol_version: u32,
    pub request_id: u64,
    pub solver_seed: u64,
    pub expected_worker_commit: String,
    pub expected_worker_tree: String,
    pub expected_worker_clean: bool,
    /// The worker's solver config — the same schema `koi_solver::Config`
    /// validates, so `deny_unknown_fields` propagates to the wire.
    pub config: koi_solver::Config,
    pub state: WireState,
    /// Advisory: milliseconds left in the seat's hard-cap bank for this leg.
    #[serde(default)]
    pub remaining_budget_ms: Option<u64>,
    /// Advisory: this transaction's liveness bound (`worker_timeout`).
    #[serde(default)]
    pub deadline_ms: Option<u64>,
}

/// The worker's decision response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionResponse {
    /// Echoes the request's version.
    pub protocol_version: u32,
    pub request_id: u64,
    pub worker_build_commit: String,
    pub worker_build_tree: String,
    pub worker_build_dirty: bool,
    /// Every version this binary serves — always `[1]` today.
    #[serde(default)]
    pub supported_protocols: Vec<u32>,
    pub selected_action: Option<WireAction>,
    pub error: Option<String>,
    /// Worker-measured milliseconds inside leaf-model evaluation — `0`
    /// until a leaf evaluator lands.
    #[serde(default)]
    pub leaf_eval_latency_ms: f64,
}

impl DecisionResponse {
    pub fn success(request_id: u64, protocol_version: u32, selected_action: Option<Action>) -> Self {
        Self {
            protocol_version,
            request_id,
            worker_build_commit: crate::artifact::BUILD_COMMIT.to_string(),
            worker_build_tree: crate::artifact::BUILD_TREE.to_string(),
            worker_build_dirty: crate::artifact::build_dirty(),
            supported_protocols: SUPPORTED_WORKER_PROTOCOLS.to_vec(),
            selected_action: selected_action.map(WireAction::from_action),
            error: None,
            leaf_eval_latency_ms: 0.0,
        }
    }

    pub fn failure(request_id: u64, protocol_version: u32, error: impl Into<String>) -> Self {
        Self {
            protocol_version,
            request_id,
            worker_build_commit: crate::artifact::BUILD_COMMIT.to_string(),
            worker_build_tree: crate::artifact::BUILD_TREE.to_string(),
            worker_build_dirty: crate::artifact::build_dirty(),
            supported_protocols: SUPPORTED_WORKER_PROTOCOLS.to_vec(),
            selected_action: None,
            error: Some(error.into()),
            leaf_eval_latency_ms: 0.0,
        }
    }
}

/// One response frame read off the worker's stdout.
#[derive(Debug, Clone)]
pub enum WorkerReply {
    Negotiation(NegotiationResponse),
    Decision(DecisionResponse),
}

impl WorkerReply {
    pub fn parse(payload: &str) -> std::result::Result<Self, String> {
        let peek: serde_json::Value =
            serde_json::from_str(payload).map_err(|error| format!("invalid response JSON: {error}"))?;
        if peek.get("kind").and_then(|kind| kind.as_str()) == Some(NEGOTIATION_RESPONSE_KIND) {
            return serde_json::from_value::<NegotiationResponse>(peek)
                .map(Self::Negotiation)
                .map_err(|error| format!("invalid negotiation frame: {error}"));
        }
        serde_json::from_value::<DecisionResponse>(peek)
            .map(Self::Decision)
            .map_err(|error| format!("invalid decision frame: {error}"))
    }
}

/// Fail-closed response validation: version echo, request id, build
/// identity, and self-consistent capability attestation.
pub fn validate_response(
    response: &DecisionResponse,
    expected_request_id: u64,
    expected_version: u32,
    expected_commit: &str,
    expected_tree: &str,
    expected_clean: bool,
) -> Result<()> {
    if response.protocol_version != expected_version {
        bail!(
            "worker protocol mismatch: received {}, the request was version {}",
            response.protocol_version,
            expected_version
        );
    }
    if !response.supported_protocols.is_empty() && !response.supported_protocols.contains(&response.protocol_version) {
        bail!("worker attestation omits the protocol version it served");
    }
    if response.request_id != expected_request_id {
        bail!(
            "worker response id mismatch: received {}, expected {}",
            response.request_id,
            expected_request_id
        );
    }
    if response.worker_build_commit != expected_commit {
        bail!(
            "worker build mismatch: received {}, expected {}",
            bounded_echo(&response.worker_build_commit),
            expected_commit
        );
    }
    if response.worker_build_tree != expected_tree || (expected_clean && response.worker_build_dirty) {
        bail!(
            "worker build tree/cleanliness does not match the requested artifact (received {})",
            bounded_echo(&response.worker_build_tree)
        );
    }
    // Worker-authored fields that land in evidence must be safe to
    // persist: a non-finite f64 serializes as `null` and would poison the
    // checkpoint's own JSONL on replay.
    if !response.leaf_eval_latency_ms.is_finite() || response.leaf_eval_latency_ms < 0.0 {
        bail!("worker leaf_eval_latency_ms is not a finite nonnegative value");
    }
    if let Some(error) = &response.error {
        const MAX_ERROR_CHARS: usize = 2048;
        if error.chars().count() > MAX_ERROR_CHARS {
            bail!("worker error string exceeds {MAX_ERROR_CHARS} characters");
        }
    }
    Ok(())
}

/// Worker-supplied strings are untrusted input; echoing them into hard
/// errors must not scale with the worker's frame budget.
fn bounded_echo(value: &str) -> std::borrow::Cow<'_, str> {
    const MAX_ECHO_CHARS: usize = 64;
    if value.chars().count() <= MAX_ECHO_CHARS {
        std::borrow::Cow::Borrowed(value)
    } else {
        std::borrow::Cow::Owned(format!(
            "{}… [truncated]",
            value.chars().take(MAX_ECHO_CHARS).collect::<String>()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use koi_core::deal_from_seed;

    /// Builds the deal view + ledger of a live round from the observer seat,
    /// applying `actions` so the snapshot is mid-round.
    fn live_frame(seed: u64, plays: usize) -> (WireState, KoiGameState) {
        let (mut state, anomaly) = KoiGameState::new_deal(deal_from_seed(seed), Ruleset::nintendo());
        assert!(anomaly.is_none(), "fixture seed must deal clean");
        // The frame is addressed to whoever acts at frame time, but the
        // initial deal view must be that same observer's private deal — so
        // capture both deal views up front.
        let deal_view = |state: &KoiGameState, observer: Player| WireDealView {
            own_hand: state.hands[observer.index()].bits(),
            opponent_hand_count: state.hands[observer.opponent().index()].count() as u8,
            field: state.field.bits(),
            stock_count: state.stock.len() as u8,
            dealer: player_to_wire(state.dealer),
            deal_score: state.score,
        };
        let initials = [deal_view(&state, Player::South), deal_view(&state, Player::North)];
        let mut ledger = Vec::new();
        for _ in 0..plays {
            if state.is_ended() {
                break;
            }
            let drawn = match state.phase {
                TurnPhase::AwaitingStockResolution { drawn } => Some(drawn.index()),
                _ => None,
            };
            let action = state.legal_actions()[0];
            ledger.push(WireLedgerEntry {
                player: player_to_wire(state.active),
                action: WireAction::from_action(action),
                drawn,
            });
            state = state.apply_action(action).unwrap();
        }
        let mut wire = WireState::view_fields(&state, state.active);
        wire.initial = initials[state.active.index()].clone();
        wire.ledger = ledger;
        (wire, state)
    }

    /// Every decision frame in a live leg must replay to the attested
    /// public view — the resolver's consistency contract. A pending draw
    /// is pinned from the snapshot since no ledger entry names it yet.
    #[test]
    fn replayed_observation_matches_the_attested_public_view() {
        for seed in 1u64..=16 {
            let (mut state, anomaly) = KoiGameState::new_deal(deal_from_seed(seed), Ruleset::nintendo());
            if anomaly.is_some() {
                continue;
            }
            let deal_view = |state: &KoiGameState, observer: Player| WireDealView {
                own_hand: state.hands[observer.index()].bits(),
                opponent_hand_count: state.hands[observer.opponent().index()].count() as u8,
                field: state.field.bits(),
                stock_count: state.stock.len() as u8,
                dealer: player_to_wire(state.dealer),
                deal_score: state.score,
            };
            let initials = [deal_view(&state, Player::South), deal_view(&state, Player::North)];
            let mut ledger = Vec::new();
            while !state.is_ended() {
                let observer = state.active;
                let mut wire = WireState::view_fields(&state, observer);
                wire.initial = initials[observer.index()].clone();
                wire.ledger = ledger.clone();
                let direct = state.public_view(observer);
                let observation = wire.to_observation().expect("observation");
                let replay = koi_solver::resolving::replay_ledger(&observation).expect("replay");
                // Mirror the engine's consistency check: the pending draw
                // is a public fact no ledger entry carries — pin it from
                // the attested state before comparing the views.
                let replay_state = match state.phase {
                    TurnPhase::AwaitingStockResolution { drawn } => {
                        koi_solver::resolving::pin_pending_draw(replay.state, observer.opponent(), drawn).expect("pin")
                    }
                    _ => replay.state,
                };
                let replayed = replay_state.public_view(observer);
                assert_eq!(replayed, direct, "seed {seed} diverged at turn {}", direct.turn);
                let drawn = match state.phase {
                    TurnPhase::AwaitingStockResolution { drawn } => Some(drawn.index()),
                    _ => None,
                };
                let action = state.legal_actions()[0];
                ledger.push(WireLedgerEntry {
                    player: player_to_wire(observer),
                    action: WireAction::from_action(action),
                    drawn,
                });
                state = state.apply_action(action).unwrap();
            }
        }
    }

    #[test]
    fn wire_action_round_trips() {
        for action in [
            Action::PlayFromHand {
                card: Card::new_unchecked(5),
                capture: CaptureChoice::Pair(Card::new_unchecked(6)),
            },
            Action::ResolveStock {
                capture: CaptureChoice::Triple,
            },
            Action::KoiKoi,
            Action::Shobu,
        ] {
            assert_eq!(WireAction::from_action(action).to_action().unwrap(), action);
        }
        assert!(WireAction {
            kind: "play_from_hand".to_owned(),
            card: Some(48),
            capture: None,
        }
        .to_action()
        .is_err());
    }

    #[test]
    fn empty_ledger_reconstructs_the_deal() {
        let (wire, state) = live_frame(3, 0);
        let rebuilt = wire.into_state().unwrap();
        assert_eq!(rebuilt.field, state.field);
        assert_eq!(rebuilt.hands[0].count(), 8);
        assert_eq!(rebuilt.stock.len(), 24);
    }

    #[test]
    fn ledger_replay_validates_a_live_round() {
        for seed in [3, 4, 5, 6] {
            let (wire, _state) = live_frame(seed, 6);
            wire.into_state()
                .unwrap_or_else(|error| panic!("seed {seed}: {error:?}"));
        }
    }

    #[test]
    fn tampered_ledger_fails_closed() {
        let (mut wire, _state) = live_frame(3, 4);
        // Flip a ledger action to one that was not taken.
        wire.ledger[1].action = WireAction::from_action(Action::KoiKoi);
        assert!(wire.into_state().is_err());
    }

    #[test]
    fn response_validation_fails_closed() {
        let response = DecisionResponse::success(7, 1, None);
        assert!(validate_response(
            &response,
            8,
            1,
            crate::artifact::BUILD_COMMIT,
            crate::artifact::BUILD_TREE,
            false
        )
        .is_err());
        assert!(validate_response(&response, 7, 1, "wrong", crate::artifact::BUILD_TREE, false).is_err());
        let mut malformed = DecisionResponse::success(7, 1, None);
        malformed.supported_protocols = vec![9];
        assert!(validate_response(
            &malformed,
            7,
            1,
            crate::artifact::BUILD_COMMIT,
            crate::artifact::BUILD_TREE,
            false
        )
        .is_err());
    }
}
