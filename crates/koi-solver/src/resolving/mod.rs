//! Public-belief-state continual resolving for the P3 engine.
//!
//! The resolver replaces determinization averaging with information-set-
//! aware subgame solving: at every decision it builds the belief over
//! compatible opponent worlds ([`belief`]), optionally reweights it by the
//! opponent model's ledger likelihood ([`model`]), wraps each world in the
//! opponent's opt-out safety gadget ([`gadget`]), and solves the compiled
//! subgame with CFR+ ([`resolver`]). [`reconstruct`] replays the public
//! ledger into the decision frames and infoset anchors everything shares;
//! [`artifact`] persists reduced-variant blueprint profiles for the
//! certified gadget path.

pub mod artifact;
pub mod belief;
pub mod gadget;
pub mod model;
pub mod reconstruct;
pub mod resolver;

pub use artifact::{load_gadget, save_blueprint_artifact, ArtifactError, BlueprintArtifact, BlueprintProvenance};
pub use belief::{build_belief, BeliefError, World};
pub use gadget::{learned_margin_south, GadgetOracle, LeafMarginOracle, MarginAnchors, OracleRef, SafetyGadget};
pub use model::{Archetype, OpponentModel};
pub use reconstruct::{pin_pending_draw, replay_ledger, DecisionFrame, LedgerReplay, ReconstructError};
pub use resolver::{
    ox_action, resolve_decision, resolve_decision_with_worlds, ResolveContext, ResolveError, ResolveSpec,
    ResolvedDecision,
};
