use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use crate::field::field::Field;

/// Returns `k_i`, the multiplier used in `S_ID_i` in the context of Plonk's permutation argument.
// TODO: This is copied from plonky1, but may need revisiting. Is the random approach still OK now
// that our field is smaller?
pub(crate) fn get_subgroup_shift<F: Field>(i: usize) -> F {
    // The optimized variant of Plonk's permutation argument calls for NUM_ROUTED_WIRES shifts,
    // k_1, ..., k_n, which result in distinct cosets. The paper suggests a method which is
    // fairly straightforward when only three shifts are needed, but seems a bit complex and
    // expensive if more are needed.

    // We will "cheat" and just use random field elements. Since our subgroup has |F*|/degree
    // possible cosets, the probability of a collision is negligible for large fields.

    // Unlike what's shown in the Plonk paper, we do not set k_1=1 to "randomize" the
    // sigmas polynomials evaluations and making them fit in both fields with high probability.
    // TODO: Go back to k_1=1 if we change the way we deal with values not fitting in both fields.
    let mut rng = ChaCha8Rng::seed_from_u64(i as u64);
    F::rand_from_rng(&mut rng)
}
