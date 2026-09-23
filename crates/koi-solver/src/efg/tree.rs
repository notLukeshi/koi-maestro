//! Materialized extensive-form game trees compiled from the koi-core state machine.
//!
//! Structure adapted from a sibling research codebase's `solver/efg/tree.rs` (v2-era
//! snapshot): the node/infoset/indexing machinery
//! is game-agnostic; the card universe, action type, and information-set key
//! are koi-specific (48-card `CardSet` zones, `Action`/`action_key`, per-turn
//! public stock draws in the history).

use rustc_hash::FxHashMap;

use koi_core::{Action, Player};

use super::compiler::KoiVariant;

pub type NodeId = usize;
pub type InfosetId = usize;

/// Tag for the stock-draw event in `InfoSetKey::public_history`. Action kinds
/// occupy tags 0..=3 in `Action::action_key`; draws use a disjoint tag so the
/// flat event stream cannot collide.
pub const DRAW_EVENT_TAG: u64 = 4;

/// The public-history key a stock draw contributes once revealed.
pub fn draw_event_key(card: koi_core::Card) -> u64 {
    (DRAW_EVENT_TAG << 32) | card.index() as u64
}

/// Tag for the resolving gadget's virtual opt-out decision in
/// `InfoSetKey::public_history`. Disjoint from `action_key` kinds 0..=3 and
/// the draw tag so a real event stream can never collide with it.
pub const GADGET_EVENT_TAG: u64 = 5;

/// The history key marking a gadget opt-out/enter decision.
pub const fn gadget_event_key() -> u64 {
    GADGET_EVENT_TAG << 32
}

/// One chance outcome: an event with its normalized probability and child
/// subtree. `dealt` records the card indices the event resolves: the root
/// deal's zones (South hand, then North hand, then field), or a single drawn
/// stock card for mid-round draw nodes.
#[derive(Debug, Clone, PartialEq)]
pub struct ChanceOutcome {
    pub probability: f64,
    pub dealt: Vec<u8>,
    pub child: NodeId,
}

/// One tree node: a chance fork, a player's decision, or a round end whose
/// utility is South's leg margin (`u_north = -u_south`).
#[derive(Debug, Clone, PartialEq)]
pub enum NodeType {
    Chance {
        outcomes: Vec<ChanceOutcome>,
    },
    Decision {
        player: Player,
        infoset: InfosetId,
        actions: Vec<Action>,
        children: Vec<NodeId>,
    },
    Terminal {
        utility_south: f64,
    },
}

/// A decision node's information set: the acting player and the action list
/// every member shares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InfoSet {
    pub player: Player,
    pub actions: Vec<Action>,
    pub members: Vec<NodeId>,
}

/// Perfect-recall information-set key: the acting player, their private
/// hand, the publicly dealt initial field, and the complete public event
/// history (hand plays, revealed draws, stock resolutions, stop decisions —
/// all as `action_key`/draw-event keys). Current field, captured piles,
/// scores, koi-koi state, and zone counts are all derivable from the initial
/// field plus the observed event stream; the opponent's hand and the stock
/// never enter the key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InfoSetKey {
    pub player: Player,
    /// `CardSet::bits()` of the acting player's current hand.
    pub own_hand: u64,
    /// `CardSet::bits()` of the field as dealt (or the subgame's base field).
    pub initial_field: u64,
    /// Flat public event stream; see `draw_event_key`.
    pub public_history: Vec<u64>,
}

/// A materialized EFG compiled from a [`KoiVariant`] (or a belief-weighted
/// subgame — same node/infoset structure with a different root).
#[derive(Debug, Clone)]
pub struct EfgTree {
    variant: KoiVariant,
    nodes: Vec<NodeType>,
    infosets: Vec<InfoSet>,
    // FxHash over std's SipHash: the compiler probes this map once per
    // decision node, and the key is an already-mixed u64-heavy struct —
    // the deterministic hasher also removes SipHash seed entropy from
    // any ordering-sensitive behavior.
    infoset_index: FxHashMap<InfoSetKey, InfosetId>,
    root: NodeId,
}

impl EfgTree {
    pub(crate) fn new(variant: KoiVariant) -> Self {
        Self {
            variant,
            nodes: Vec::new(),
            infosets: Vec::new(),
            infoset_index: FxHashMap::default(),
            root: 0,
        }
    }

    pub fn variant(&self) -> &KoiVariant {
        &self.variant
    }

    pub fn root(&self) -> NodeId {
        self.root
    }

    pub fn node(&self, id: NodeId) -> &NodeType {
        &self.nodes[id]
    }

    pub fn nodes(&self) -> &[NodeType] {
        &self.nodes
    }

    pub fn infosets(&self) -> &[InfoSet] {
        &self.infosets
    }

    /// The information set a perfect-recall key belongs to, if compiled.
    pub fn infoset_for(&self, key: &InfoSetKey) -> Option<InfosetId> {
        self.infoset_index.get(key).copied()
    }

    /// Every infoset key in `infosets()` order — the structural identity
    /// row `i` of a strategy profile binds.
    pub fn infoset_keys_in_order(&self) -> Vec<&InfoSetKey> {
        let mut by_id: Vec<(&InfosetId, &InfoSetKey)> = self.infoset_index.iter().map(|(key, id)| (id, key)).collect();
        by_id.sort_by_key(|(id, _)| **id);
        by_id.into_iter().map(|(_, key)| key).collect()
    }

    pub fn terminal_count(&self) -> usize {
        self.nodes
            .iter()
            .filter(|node| matches!(node, NodeType::Terminal { .. }))
            .count()
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub(crate) fn push_chance(&mut self, outcomes: Vec<ChanceOutcome>) -> NodeId {
        self.nodes.push(NodeType::Chance { outcomes });
        self.nodes.len() - 1
    }

    pub(crate) fn push_decision(&mut self, player: Player, infoset: InfosetId, actions: Vec<Action>) -> NodeId {
        self.nodes.push(NodeType::Decision {
            player,
            infoset,
            actions,
            children: Vec::new(),
        });
        self.nodes.len() - 1
    }

    pub(crate) fn push_terminal(&mut self, utility_south: f64) -> NodeId {
        self.nodes.push(NodeType::Terminal { utility_south });
        self.nodes.len() - 1
    }

    /// Overwrites a terminal's South utility — used by the batched leaf
    /// evaluator which pushes placeholders during expansion and fills the
    /// values after the batch inference completes.
    pub(crate) fn set_terminal_value(&mut self, id: NodeId, utility_south: f64) {
        if let NodeType::Terminal { utility_south: slot } = &mut self.nodes[id] {
            *slot = utility_south;
        }
    }

    pub(crate) fn set_children(&mut self, id: NodeId, children: Vec<NodeId>) {
        if let NodeType::Decision { children: slot, .. } = &mut self.nodes[id] {
            *slot = children;
        }
    }

    pub(crate) fn push_infoset(&mut self, key: InfoSetKey, infoset: InfoSet) -> InfosetId {
        self.infosets.push(infoset);
        let id = self.infosets.len() - 1;
        self.infoset_index.insert(key, id);
        id
    }

    pub(crate) fn infoset_mut(&mut self, id: InfosetId) -> &mut InfoSet {
        &mut self.infosets[id]
    }

    pub(crate) fn set_root(&mut self, root: NodeId) {
        self.root = root;
    }
}
