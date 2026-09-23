//! Discounted CFR (Brown & Sandholm, AAAI 2019): each iteration the
//! updater's accumulated regrets are shrunk toward zero — positive entries
//! by `t^alpha / (t^alpha + 1)`, negative by `t^beta / (t^beta + 1)` — and
//! the average strategy weights the iterate by `t^gamma`. Discounting old
//! regrets lets recent (better) iterates dominate the table, which
//! empirically beats CFR+ on most benchmark games.
//!
//! The recommended configuration `alpha = 1.5, beta = 0, gamma = 2` is used;
//! regret-matching+ clamping stays on (the paper's DCFR+ variant).
//!
//! Ported from a sibling research codebase's `solver/cfr/dcfr.rs` — game-agnostic.

use koi_core::Player;

use super::{cfr_plus, profile::Blueprint};
use crate::efg::tree::EfgTree;

pub const DCFR_ALPHA: f64 = 1.5;
pub const DCFR_BETA: f64 = 0.0;
pub const DCFR_GAMMA: f64 = 2.0;

fn discount(exponent: f64, t: f64) -> f64 {
    // `powf` is not correctly rounded by every platform libm (MSVC's `pow`
    // differs from glibc's in the last ulp), so the three fixed exponents
    // are spelled out with correctly-rounded primitives — bit-identical on
    // all three targets.
    let powered = if exponent == DCFR_ALPHA {
        t * t.sqrt()
    } else if exponent == DCFR_BETA {
        1.0
    } else {
        debug_assert_eq!(exponent, DCFR_GAMMA);
        t * t
    };
    powered / (powered + 1.0)
}

/// Trains a blueprint with DCFR for `iterations` alternating updates and
/// returns the flushed average profile.
pub fn train_dcfr(tree: &EfgTree, iterations: usize) -> Blueprint {
    let action_counts: Vec<usize> = tree.infosets().iter().map(|infoset| infoset.actions.len()).collect();
    let mut blueprint = Blueprint::new(action_counts.clone());
    // Per-traversal instant accumulator — the aggregate add_regret pattern
    // shared with CFR+ (RM+ clamps once per infoset, not per member).
    let mut instants: Vec<Vec<f64>> = action_counts.iter().map(|&count| vec![0.0; count]).collect();
    // Each player discounts on its own update count — with alternating
    // updates a player's t-th update happens every other traversal.
    let mut update_counts = [0usize; 2];
    for iteration in 0..iterations {
        blueprint.advance_iteration();
        let updater = if iteration % 2 == 0 {
            Player::South
        } else {
            Player::North
        };
        update_counts[updater.index()] += 1;
        let t = update_counts[updater.index()] as f64;
        let positive = discount(DCFR_ALPHA, t);
        let negative = discount(DCFR_BETA, t);
        for (id, infoset) in tree.infosets().iter().enumerate() {
            if infoset.player == updater {
                blueprint.discount_regrets(id, positive, negative);
            }
        }
        for row in instants.iter_mut() {
            row.fill(0.0);
        }
        cfr_plus::traverse(
            tree,
            tree.root(),
            updater,
            t * t,
            1.0,
            1.0,
            1.0,
            &mut blueprint,
            &mut |profile: &Blueprint, infoset: usize, out: &mut super::regret::ActionBuf| {
                super::regret::regret_matching_into(profile.regret_row(infoset), out)
            },
            &mut |infoset: usize, instant: &[f64]| {
                for (action, &regret) in instant.iter().enumerate() {
                    instants[infoset][action] += regret;
                }
            },
        );
        for (infoset, row) in instants.iter().enumerate() {
            for (action, &regret) in row.iter().enumerate() {
                blueprint.add_regret(infoset, action, regret, true);
            }
        }
    }
    blueprint.flush();
    blueprint
}

#[cfg(test)]
mod tests {
    use rand::{rngs::SmallRng, SeedableRng};

    use super::*;
    use crate::{
        cfr::{cfr_plus::train_cfr_plus, exploitability::exploitability, mccfr::train_external_sampling},
        efg::{compile_variant, KoiVariant},
    };

    /// DCFR must converge on the smallest honest verification domain.
    #[test]
    fn dcfr_converges_on_the_reduced_domain() {
        let tree = compile_variant(&KoiVariant::MICRO_8, Player::South).unwrap();
        let profile = train_dcfr(&tree, 2_000).averaged_profile();
        let expl = exploitability(&tree, &profile);
        assert!(
            expl < 1e-2,
            "DCFR exploitability after 2000 iterations should be small, got {expl}"
        );
    }

    /// The discount sweep only touches the updater's tables.
    #[test]
    fn discounting_is_scoped_to_the_updater() {
        let tree = compile_variant(&KoiVariant::MICRO_8, Player::South).unwrap();
        let mut blueprint = Blueprint::new(tree.infosets().iter().map(|i| i.actions.len()).collect());
        let south_infoset = tree
            .infosets()
            .iter()
            .position(|i| i.player == Player::South)
            .expect("south infoset");
        let north_infoset = tree
            .infosets()
            .iter()
            .position(|i| i.player == Player::North)
            .expect("north infoset");
        blueprint.add_regret(south_infoset, 0, 4.0, false);
        blueprint.add_regret(north_infoset, 0, 4.0, false);
        blueprint.discount_regrets(south_infoset, 0.5, 0.5);
        assert_eq!(blueprint.regret_row(south_infoset)[0], 2.0);
        assert_eq!(blueprint.regret_row(north_infoset)[0], 4.0);
    }

    /// Sign-selective discounting.
    #[test]
    fn discount_respects_sign() {
        let mut blueprint = Blueprint::new(vec![2]);
        blueprint.add_regret(0, 0, 4.0, false);
        blueprint.add_regret(0, 1, -4.0, false);
        blueprint.discount_regrets(0, 0.5, 0.25);
        assert_eq!(blueprint.regret_row(0), &[2.0, -1.0]);
    }

    /// The CFR baselines should be in the same convergence class.
    #[test]
    fn dcfr_matches_cfr_family_convergence_scale() {
        let tree = compile_variant(&KoiVariant::MICRO_8, Player::South).unwrap();
        let mut rng = SmallRng::seed_from_u64(0xdecf);
        let mccfr = exploitability(
            &tree,
            &train_external_sampling(&tree, 2_000, &mut rng).averaged_profile(),
        );
        let dcfr = exploitability(&tree, &train_dcfr(&tree, 2_000).averaged_profile());
        let cfr_plus = exploitability(&tree, &train_cfr_plus(&tree, 2_000).averaged_profile());
        assert!(
            dcfr < 0.05 && cfr_plus < 0.05 && mccfr < 0.2,
            "dcfr {dcfr} cfr+ {cfr_plus} mccfr {mccfr}"
        );
    }
}
