//! Print EFG compile sizes for reduced variants — an instrument, not a gate.
//! Run: cargo run -p koi-solver --release --example efg_size_probe

use koi_core::Player;
use koi_solver::efg::{compile_variant, KoiVariant};

fn main() {
    for v in [KoiVariant::MICRO_8, KoiVariant::REDUCED_9] {
        let t = std::time::Instant::now();
        let tree = compile_variant(&v, Player::South).unwrap();
        println!(
            "{}: nodes={} infosets={} terminals={} in {:?}",
            v.name,
            tree.node_count(),
            tree.infosets().len(),
            tree.terminal_count(),
            t.elapsed()
        );
    }
}
