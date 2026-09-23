/// Derives a deterministic, statistically separated RNG stream from a master seed.
pub const fn derive_named_seed(master_seed: u64, stream_id: u64) -> u64 {
    let mut value = master_seed ^ stream_id;
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

/// Domain-separation tag for the canonical deal stream.
const DEAL_STREAM_TAG: u64 = 0x4b4f_4944_4541_4c01;

/// The canonical seeded deal: a deterministic Fisher-Yates shuffle of all 48
/// cards driven by ChaCha8, so every platform and every build produces the
/// identical deck for a given seed.
///
/// The returned order follows the `new_deal` layout: `deck[0..8]` is the
/// dealer's (South) hand, `deck[8..16]` the field, `deck[16..24]` the
/// opponent's hand, and `deck[24..48]` the stock top-to-bottom.
pub fn deal_from_seed(seed: u64) -> Vec<crate::card::Card> {
    use rand::{RngExt, SeedableRng};
    use rand_chacha::ChaCha8Rng;

    let mut rng = ChaCha8Rng::seed_from_u64(derive_named_seed(seed, DEAL_STREAM_TAG));
    let mut deck: Vec<crate::card::Card> = (0..crate::card::Card::COUNT)
        .map(crate::card::Card::new_unchecked)
        .collect();
    for index in (1..deck.len()).rev() {
        let j = rng.random_range(0..=index);
        deck.swap(index, j);
    }
    deck
}

/// Resolves an omitted boundary seed once while reserving zero as the sentinel.
pub fn random_nonzero_seed() -> u64 {
    loop {
        let seed = rand::random();
        if seed != 0 {
            return seed;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_streams_are_stable_and_separated() {
        assert_eq!(derive_named_seed(11, 4), 9_753_551_079_159_975_941);
        assert_ne!(derive_named_seed(11, 4), derive_named_seed(11, 5));
    }
}
