//! Turn-7 subgame glue: belief-weighted residual EFGs solved exactly by the
//! sequence-form LP (when sequence counts fit) or approximately by CFR+
//! (P2-D6). This is imperfect-information subgame resolving: the root chance
//! node mixes every world consistent with the public ledger, weighted by the
//! caller's belief, and the tree below inherits the full infoset structure —
//! so the observer's strategy cannot condition on hidden state.
//!
//! Contrast `turn8_exact`: that module is exact only for the last observer
//! decision; here the residual tree keeps every remaining decision honest.

use koi_core::{CardSet, KoiGameState, Player};

use crate::cfr::{profile_value, train_cfr_plus};
use crate::efg::sequence::SequenceForm;
use crate::efg::{compile_subgame, EfgCompileError, EfgTree, KoiVariant, NodeType};

use super::solve::{solve_form, SolveError};
use super::turn8_exact::EndgameError;

/// Worlds enumerated for one subgame solve.
pub const MAX_SUBGAME_WORLDS: usize = 16_384;
/// Tree node budget for the residual EFG.
pub const MAX_SUBGAME_NODES: usize = 4_000_000;
/// CFR+ iterations when the LP path does not fit.
pub const SUBGAME_CFR_ITERATIONS: usize = 2_000;

/// Why a subgame solve failed.
#[derive(Debug, Clone)]
pub enum SubgameError {
    /// World enumeration / traversal budget (reused from turn8).
    Endgame(EndgameError),
    /// The residual tree could not be compiled.
    Compile(EfgCompileError),
    /// The sequence-form LP failed (not merely oversized — oversized falls
    /// back to CFR+; this is a numerical/structural failure).
    Solve(SolveError),
    /// A determinized world violated kernel invariants.
    State(koi_core::StateError),
}

impl std::fmt::Display for SubgameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Endgame(e) => write!(f, "{e}"),
            Self::Compile(e) => write!(f, "subgame compilation failed: {e}"),
            Self::Solve(e) => write!(f, "subgame LP failed: {e}"),
            Self::State(e) => write!(f, "inconsistent determinized world: {e}"),
        }
    }
}

impl std::error::Error for SubgameError {}

impl From<EndgameError> for SubgameError {
    fn from(e: EndgameError) -> Self {
        Self::Endgame(e)
    }
}
impl From<EfgCompileError> for SubgameError {
    fn from(e: EfgCompileError) -> Self {
        Self::Compile(e)
    }
}
impl From<SolveError> for SubgameError {
    fn from(e: SolveError) -> Self {
        Self::Solve(e)
    }
}
impl From<koi_core::StateError> for SubgameError {
    fn from(e: koi_core::StateError) -> Self {
        Self::State(e)
    }
}

/// Which solver produced the subgame answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubgameMethod {
    /// The exact sequence-form LP (`solve_tree`).
    SequenceFormLp,
    /// CFR+ approximation over the same compiled residual tree.
    CfrPlus { iterations: usize },
}

/// The solved residual game.
#[derive(Debug, Clone)]
pub struct SubgameSolution {
    /// The subgame value for `observer` (observer-margin units).
    pub value: f64,
    /// The full solved behavioral profile in the tree's infoset order —
    /// both players' strategies (uniform where a side was not the solve's
    /// subject; for the LP path both sides are solved independently).
    pub profile: Vec<Vec<f64>>,
    /// The global infoset id of the first decision under the root chance
    /// node — whichever player acts first in the residual.
    pub root_infoset: usize,
    /// Which solver produced the answer (exactness qualifier for evidence).
    pub method: SubgameMethod,
    /// Tree size diagnostics.
    pub tree_nodes: usize,
    pub worlds: usize,
}

impl SubgameSolution {
    /// The strategy at the first decision under the root.
    pub fn root_strategy(&self) -> &[f64] {
        if self.root_infoset == usize::MAX {
            &[]
        } else {
            &self.profile[self.root_infoset]
        }
    }
}

/// Everything a residual solve needs beyond the position itself: the public
/// anchors carried into every world's infoset key, the world prior, and the
/// budgets.
#[derive(Debug, Clone)]
pub struct SubgameContext<'a> {
    /// The *domain* the position belongs to (usually the full game spec or a
    /// reduced cert domain); it is metadata for the compiled tree, not a
    /// constraint — worlds carry their own states.
    pub variant: &'a KoiVariant,
    /// The dealt public field — identical across consistent worlds by
    /// definition; anchors every infoset key.
    pub initial_field: CardSet,
    /// The observed ledger prefix in `action_key`/`draw_event_key` encoding.
    pub history_prefix: Vec<u64>,
    /// One weight per enumerated world in enumeration order; `None` uses
    /// the uniform prior — correct under the uniform deal conditional on
    /// the public view.
    pub belief: Option<&'a [f64]>,
    /// Traversal budget for world enumeration.
    pub max_worlds: usize,
    /// Node budget for the residual EFG.
    pub max_nodes: usize,
    /// CFR+ iterations when the sequence form does not fit.
    pub cfr_iterations: usize,
}

impl Default for SubgameContext<'_> {
    fn default() -> Self {
        Self {
            variant: &KoiVariant::FULL,
            initial_field: CardSet::EMPTY,
            history_prefix: Vec::new(),
            belief: None,
            max_worlds: MAX_SUBGAME_WORLDS,
            max_nodes: MAX_SUBGAME_NODES,
            cfr_iterations: SUBGAME_CFR_ITERATIONS,
        }
    }
}

/// Solves the residual game at `state` for `observer`.
///
/// `ctx.initial_field` and `ctx.history_prefix` are the public anchors
/// carried into every world's infoset key (the dealt field and the observed
/// ledger); they are identical across consistent worlds by definition.
/// `ctx.belief`, when given, is one weight per enumerated world in
/// enumeration order; `None` uses the uniform prior — correct under the
/// uniform deal conditional on the public view.
pub fn solve_subgame(
    state: &KoiGameState,
    observer: Player,
    ctx: &SubgameContext<'_>,
) -> Result<SubgameSolution, SubgameError> {
    if state.is_ended() {
        return Err(SubgameError::Endgame(EndgameError::Terminal));
    }
    let worlds = crate::determinization::generate_exhaustive(state, observer, ctx.max_worlds).ok_or(
        EndgameError::TooManyWorlds {
            needed_over: ctx.max_worlds,
        },
    )??;

    let weights: Vec<f64> = match ctx.belief {
        Some(b) => {
            if b.len() != worlds.len() {
                return Err(SubgameError::State(koi_core::StateError::InconsistentState));
            }
            b.to_vec()
        }
        None => vec![1.0; worlds.len()],
    };
    let weighted: Vec<(KoiGameState, f64)> = worlds.into_iter().zip(weights).collect();

    let tree = compile_subgame(
        ctx.variant,
        &weighted,
        ctx.initial_field,
        ctx.history_prefix.clone(),
        ctx.max_nodes,
    )?;

    // The first decision under the root chance node — whichever player acts
    // first in the residual. Every world's child shares the acting player's
    // infoset (identical observations) — take any world's.
    let root_infoset = match tree.node(tree.root()) {
        NodeType::Chance { outcomes } => {
            let child = outcomes[0].child;
            match tree.node(child) {
                NodeType::Decision { infoset, .. } => *infoset,
                NodeType::Terminal { .. } => {
                    // Every world already ended: value is the belief-weighted
                    // terminal margin, no strategy to return.
                    let value: f64 = outcomes
                        .iter()
                        .map(|o| {
                            o.probability
                                * match tree.node(o.child) {
                                    NodeType::Terminal { utility_south } => match observer {
                                        Player::South => *utility_south,
                                        Player::North => -*utility_south,
                                    },
                                    _ => unreachable!(),
                                }
                        })
                        .sum();
                    return Ok(SubgameSolution {
                        value,
                        profile: Vec::new(),
                        root_infoset: usize::MAX,
                        method: SubgameMethod::SequenceFormLp,
                        tree_nodes: tree.node_count(),
                        worlds: outcomes.len(),
                    });
                }
                NodeType::Chance { .. } => {
                    // The observer moves later; descend to the first decision.
                    first_decision_infoset(&tree, child)?
                }
            }
        }
        _ => {
            return Err(SubgameError::Compile(EfgCompileError::InvalidVariant(
                "a subgame root must be a chance node",
            )))
        }
    };

    // Exact path when the sequence form fits; CFR+ otherwise. One
    // extraction serves both players' LPs (mirror = row-player swap).
    let base = match SequenceForm::compile(&tree) {
        Ok(form) => form,
        Err(crate::efg::SequenceFormError::TooLarge) => {
            return run_cfr_fallback(&tree, observer, root_infoset, ctx.cfr_iterations);
        }
        Err(e) => return Err(SubgameError::Solve(SolveError::SequenceForm(e))),
    };
    let (observer_form, opponent_form) = match observer {
        Player::South => {
            let mirrored = base.mirrored();
            (base, mirrored)
        }
        Player::North => {
            let mirrored = base.mirrored();
            (mirrored, base)
        }
    };
    let solve = |form: &SequenceForm| -> Result<_, Option<SolveError>> {
        match solve_form(form) {
            Err(SolveError::TableauTooLarge) => Err(None),
            other => other.map_err(Some),
        }
    };
    let (solved, opponent) = match (solve(&observer_form), solve(&opponent_form)) {
        (Ok(solved), Ok(opponent)) => (solved, opponent),
        (Err(None), _) | (_, Err(None)) => {
            return run_cfr_fallback(&tree, observer, root_infoset, ctx.cfr_iterations);
        }
        (Err(Some(e)), _) | (_, Err(Some(e))) => return Err(SubgameError::Solve(e)),
    };
    let mut profile = vec![Vec::new(); tree.infosets().len()];
    for (strategy, &global) in solved.strategy.iter().zip(&solved.global_infosets) {
        profile[global] = strategy.clone();
    }
    for (strategy, &global) in opponent.strategy.iter().zip(&opponent.global_infosets) {
        profile[global] = strategy.clone();
    }
    Ok(SubgameSolution {
        value: solved.value,
        profile,
        root_infoset,
        method: SubgameMethod::SequenceFormLp,
        tree_nodes: tree.node_count(),
        worlds: weighted_len(&tree),
    })
}

fn run_cfr_fallback(
    tree: &EfgTree,
    observer: Player,
    root_infoset: usize,
    iterations: usize,
) -> Result<SubgameSolution, SubgameError> {
    let blueprint = train_cfr_plus(tree, iterations);
    let profile = blueprint.averaged_profile();
    let value = profile_value(tree, &profile, observer);
    Ok(SubgameSolution {
        value,
        profile,
        root_infoset,
        method: SubgameMethod::CfrPlus { iterations },
        tree_nodes: tree.node_count(),
        worlds: weighted_len(tree),
    })
}

fn weighted_len(tree: &EfgTree) -> usize {
    match tree.node(tree.root()) {
        NodeType::Chance { outcomes } => outcomes.len(),
        _ => 0,
    }
}

/// Descends chance branches to the first decision node's infoset (the
/// observer may not move first inside the residual tree).
fn first_decision_infoset(tree: &EfgTree, node: usize) -> Result<usize, SubgameError> {
    match tree.node(node) {
        NodeType::Decision { infoset, .. } => Ok(*infoset),
        NodeType::Chance { outcomes } => first_decision_infoset(tree, outcomes[0].child),
        NodeType::Terminal { .. } => Err(SubgameError::Compile(EfgCompileError::InvalidVariant(
            "the subgame contains no decision",
        ))),
    }
}

#[cfg(test)]
mod tests {
    use koi_core::{Card, CardSet, PublicView, Ruleset, TurnPhase};

    use super::*;

    /// Two-hidden-card position (one opponent slot + one stock card), all
    /// other zones public — exactly 2 worlds for South's determinization.
    /// Unknowns {20,24} (Jun/Jul) match nothing left on the table.
    fn two_world_state() -> KoiGameState {
        let south_hand = CardSet::from_card(Card::new_unchecked(0));
        let field = CardSet::from_card(Card::new_unchecked(44));
        let taken = south_hand
            .union(field)
            .insert(Card::new_unchecked(20))
            .insert(Card::new_unchecked(24));
        let mut pile_south = CardSet::EMPTY;
        for card in CardSet::ALL.difference(taken) {
            if pile_south.count() >= 22 {
                break;
            }
            pile_south = pile_south.insert(card);
        }
        let pile_north = CardSet::ALL.difference(taken).difference(pile_south);
        let view = PublicView {
            observer: Player::South,
            rules: Ruleset::nintendo(),
            dealer: Player::South,
            active: Player::South,
            turn: 15,
            round: 0,
            score: [0, 0],
            phase: TurnPhase::AwaitingHandAction,
            own_hand: south_hand,
            opponent_hand_count: 1,
            field,
            captured: [pile_south, pile_north],
            stock_count: 1,
            koi_koi_caller: None,
            koi_koi_calls: [0, 0],
            last_yaku_score: [u32::MAX, u32::MAX],
        };
        KoiGameState::from_public_view(&view).unwrap()
    }

    /// A one-turn-left subgame solved by LP must return a normalized root
    /// strategy and a finite value.
    #[test]
    fn lp_solves_a_small_subgame() {
        let state = two_world_state();
        let ctx = SubgameContext {
            variant: &KoiVariant::NANO_6,
            initial_field: state.field,
            ..Default::default()
        };
        let solution = solve_subgame(&state, Player::South, &ctx).unwrap();
        assert_eq!(solution.method, SubgameMethod::SequenceFormLp);
        assert!(solution.value.is_finite());
        let root = solution.root_strategy();
        assert!(
            (root.iter().sum::<f64>() - 1.0).abs() < 1e-6,
            "root strategy {root:?} must normalize"
        );
    }

    /// The enumeration budget is fail-closed: a cap below the world count
    /// refuses rather than returning partial evidence.
    #[test]
    fn world_cap_fails_closed() {
        let state = two_world_state();
        // Two worlds exist; a cap of 1 must refuse.
        let ctx = SubgameContext {
            variant: &KoiVariant::NANO_6,
            initial_field: state.field,
            max_worlds: 1,
            cfr_iterations: 1,
            ..Default::default()
        };
        assert!(matches!(
            solve_subgame(&state, Player::South, &ctx),
            Err(SubgameError::Endgame(EndgameError::TooManyWorlds { .. }))
        ));
    }
}
