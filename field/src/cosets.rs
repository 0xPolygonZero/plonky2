use alloc::vec::Vec;

use num::bigint::BigUint;

use crate::types::Field;

/// Finds a set of shifts that result in unique cosets for the multiplicative subgroup of size
/// `2^subgroup_bits`.
pub fn get_unique_coset_shifts<F: Field>(subgroup_size: usize, num_shifts: usize) -> Vec<F> {
    // From Lagrange's theorem.
    let num_cosets = (F::order() - 1u32) / (subgroup_size as u32);
    assert!(
        BigUint::from(num_shifts) <= num_cosets,
        "The subgroup does not have enough distinct cosets"
    );

    // Let g be a generator of the entire multiplicative group. Let n be the order of the subgroup.
    // The subgroup can be written as <g^(|F*| / n)>. We can use g^0, ..., g^(num_shifts - 1) as our
    // shifts, since g^i <g^(|F*| / n)> are distinct cosets provided i < |F*| / n, which we checked.
    F::MULTIPLICATIVE_GROUP_GENERATOR
        .powers()
        .take(num_shifts)
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::cosets::get_unique_coset_shifts;
    use crate::goldilocks_field::GoldilocksField;
    use crate::types::Field;

    #[test]
    fn distinct_cosets() {
        type F = GoldilocksField;
        const SUBGROUP_BITS: usize = 5;
        const NUM_SHIFTS: usize = 50;

        let generator = F::primitive_root_of_unity(SUBGROUP_BITS);
        let subgroup_size = 1 << SUBGROUP_BITS;

        let shifts = get_unique_coset_shifts::<F>(subgroup_size, NUM_SHIFTS);

        let mut union = HashSet::new();
        for shift in shifts {
            let coset = F::cyclic_subgroup_coset_known_order(generator, shift, subgroup_size);
            assert!(
                coset.into_iter().all(|x| union.insert(x)),
                "Duplicate element!"
            );
        }
    }
}
