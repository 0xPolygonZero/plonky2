use std::collections::HashSet;

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use crate::field::field::Field;

/// Finds a set of shifts that result in unique cosets for the subgroup of size `2^subgroup_bits`.
pub(crate) fn get_unique_coset_shifts<F: Field>(
    subgroup_bits: usize,
    num_shifts: usize,
) -> Vec<F> {
    let mut rng = ChaCha8Rng::seed_from_u64(0);

    let generator = F::primitive_root_of_unity(subgroup_bits);
    let subgroup_size = 1 << subgroup_bits;

    let mut shifts = Vec::with_capacity(num_shifts);

    // We start with the trivial coset. This isn't necessary, but there may be a slight cost
    // savings, since multiplication by 1 can be free in some settings.
    shifts.push(F::ONE);

    let subgroup = F::cyclic_subgroup_known_order(generator, subgroup_size)
        .into_iter()
        .collect::<HashSet<F>>();

    while shifts.len() < num_shifts {
        let candidate_shift = F::rand_from_rng(&mut rng);
        if candidate_shift.is_zero() {
            continue;
        }
        let candidate_shift_inv = candidate_shift.inverse();

        // If this coset was not disjoint from the others, then there would exist some i, j with
        //     candidate_shift g^i = existing_shift g^j
        // or
        //     existing_shift / candidate_shift = g^(i - j).
        // In other words, `existing_shift / candidate_shift` would be in the subgroup.
        let quotients = shifts.iter()
            .map(|&shift| shift * candidate_shift_inv)
            .collect::<HashSet<F>>();

        if quotients.is_disjoint(&subgroup) {
            shifts.push(candidate_shift);
        }
    }

    shifts
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::field::cosets::get_unique_coset_shifts;
    use crate::field::crandall_field::CrandallField;
    use crate::field::field::Field;

    #[test]
    fn distinct_cosets() {
        // TODO: Switch to a smaller test field so that collision rejection is likely to occur.

        type F = CrandallField;
        const SUBGROUP_BITS: usize = 5;
        const NUM_SHIFTS: usize = 50;

        let generator = F::primitive_root_of_unity(SUBGROUP_BITS);
        let subgroup_size = 1 << SUBGROUP_BITS;

        let shifts = get_unique_coset_shifts::<F>(SUBGROUP_BITS, NUM_SHIFTS);

        let mut union = HashSet::new();
        for shift in shifts {
            let coset = F::cyclic_subgroup_coset_known_order(generator, shift, subgroup_size);
            assert!(
                coset.into_iter().all(|x| union.insert(x)),
                "Duplicate element!");
        }
    }
}
