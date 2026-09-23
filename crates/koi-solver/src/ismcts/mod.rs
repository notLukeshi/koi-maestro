//! Information-set Monte Carlo tree search for Koi-Koi.
//!
//! Each iteration determinizes the hidden zones (opponent hand + stock
//! order) from the root observer's view, walks a tree whose nodes are keyed
//! by the *acting player's* information set — their observable public view,
//! never the sampled hidden cards — then rolls out greedily and propagates
//! the root-observer-relative reward. Statistics merge across
//! determinizations at the information set, which is what makes the search
//! correct under imperfect information: a player cannot distinguish worlds
//! inside its own info set, so the action value must aggregate them.
//!
//! Parallel search is a root-split over a fixed number of trees whose
//! results merge deterministically — the tree count is a constant, so the
//! outcome is schedule-invariant (decision E5/ARCH 11.4).

mod search;
mod tree;

pub(crate) use search::find_best_action_ismcts;
