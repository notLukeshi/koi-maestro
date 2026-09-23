//! Counterfactual regret minimization over materialized EFG trees.
//!
//! This is the P2 blueprint machinery: regret matching, external-sampling
//! MCCFR (i.i.d. and correlated-chance), CFR+, DCFR, PCFR+, and exact
//! exploitability. It operates on trees compiled by the EFG compiler from
//! the single validated kernel, so every strategy and value it touches is
//! kernel-derived. Ported from a sibling research codebase's `solver/cfr/` — the trainers
//! are game-agnostic over `EfgTree`.

pub mod cfr_plus;
pub mod chance;
pub mod dcfr;
pub mod exploitability;
pub mod mccfr;
pub mod pcfr;
pub mod profile;
pub mod regret;

pub use cfr_plus::train_cfr_plus;
pub use dcfr::train_dcfr;
pub use exploitability::{best_response_value, exploitability, nash_conv, profile_value, profile_value_south};
pub use mccfr::{train_external_sampling, train_external_sampling_ccs};
pub use pcfr::train_pcfr_plus;
pub use profile::Blueprint;
pub use regret::regret_matching;

#[cfg(test)]
mod tests {
    use rand::{rngs::SmallRng, SeedableRng};

    use super::*;
    use crate::efg::{compile_variant, KoiVariant};
    use koi_core::Player;

    fn micro_tree() -> crate::efg::EfgTree {
        compile_variant(&KoiVariant::MICRO_8, Player::South).unwrap()
    }

    /// A uniform random profile must be strictly exploitable: best responses
    /// against uniform play gain more than uniform play earns.
    #[test]
    fn uniform_profile_is_exploitable() {
        let tree = micro_tree();
        let uniform: Vec<Vec<f64>> = tree
            .infosets()
            .iter()
            .map(|infoset| vec![1.0 / infoset.actions.len() as f64; infoset.actions.len()])
            .collect();
        let exploit = exploitability(&tree, &uniform);
        assert!(exploit > 0.0, "uniform play must be exploitable, got {exploit}");

        let value = profile_value_south(&tree, &uniform);
        let br_south = best_response_value(&tree, &uniform, Player::South);
        assert!(br_south > value, "best response must exceed the profile value");
    }

    /// External-sampling MCCFR must drive the exploitability of the average
    /// profile down to a preregistered tolerance on the MICRO_8 domain, with
    /// a fixed seed and a reproducible curve.
    #[test]
    fn mccfr_converges_on_the_reduced_domain() {
        let tree = micro_tree();
        let iterations = 40_000;
        let blueprint = train_external_sampling(&tree, iterations, &mut SmallRng::seed_from_u64(7));
        let exploit = exploitability(&tree, &blueprint.averaged_profile());
        assert!(
            exploit < 0.05,
            "MCCFR exploitability {exploit} must fall under the 0.05-point tolerance"
        );

        // Same seed, same curve: the trained profile is reproducible.
        let replay = train_external_sampling(&tree, iterations, &mut SmallRng::seed_from_u64(7));
        assert_eq!(blueprint.averaged_profile(), replay.averaged_profile());
    }

    /// The MCCFR average profile improves as training progresses.
    #[test]
    fn mccfr_exploitability_decreases_with_training() {
        let tree = micro_tree();
        let early = train_external_sampling(&tree, 500, &mut SmallRng::seed_from_u64(11));
        let late = train_external_sampling(&tree, 20_000, &mut SmallRng::seed_from_u64(11));
        let early_exploit = exploitability(&tree, &early.averaged_profile());
        let late_exploit = exploitability(&tree, &late.averaged_profile());
        assert!(
            late_exploit < early_exploit,
            "exploitability must decrease with training: early {early_exploit}, late {late_exploit}"
        );
    }

    /// CFR+ with regret-matching+ and alternating updates must converge at
    /// least as fast as external sampling at the same iteration count.
    #[test]
    fn cfr_plus_converges_faster_than_mccfr_at_equal_iterations() {
        let tree = micro_tree();
        let iterations = 200;
        let plus = train_cfr_plus(&tree, iterations);
        let sampled = train_external_sampling(&tree, iterations, &mut SmallRng::seed_from_u64(3));

        let plus_exploit = exploitability(&tree, &plus.averaged_profile());
        let sampled_exploit = exploitability(&tree, &sampled.averaged_profile());
        assert!(
            plus_exploit < sampled_exploit,
            "CFR+ ({plus_exploit}) must beat MCCFR ({sampled_exploit}) at {iterations} iterations"
        );
        assert!(
            plus_exploit < 0.05,
            "CFR+ exploitability {plus_exploit} must fall under the 0.05-point tolerance"
        );
    }
}
