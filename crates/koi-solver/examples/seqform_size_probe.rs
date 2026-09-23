//! Print sequence-form dimensions for reduced variants — an instrument,
//! not a gate. Run: cargo run -p koi-solver --release --example seqform_size_probe

use koi_core::Player;
use koi_solver::efg::{compile_variant, KoiVariant, SequenceForm};

fn main() {
    for (name, v) in [
        ("NANO_6", KoiVariant::NANO_6),
        ("MICRO_8", KoiVariant::MICRO_8),
        ("FIELDVOID_10", KoiVariant::FIELDVOID_10),
    ] {
        let tree = compile_variant(&v, Player::South).unwrap();
        print!(
            "{name}: nodes={} infosets={} ",
            tree.node_count(),
            tree.infosets().len()
        );
        match SequenceForm::compile(&tree) {
            Ok(f) => println!(
                "south_seq={} north_seq={} cells={}",
                f.south_sequence_count(),
                f.north_sequence_count(),
                f.payoff.len()
            ),
            Err(e) => println!("seqform={e}"),
        }
    }
}
