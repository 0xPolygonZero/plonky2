//! Lookup unit.
//! See https://zcash.github.io/halo2/design/proving-system/lookup.html

const START_UNIT: usize = super::START_LOOKUP;

const NUM_LOOKUPS: usize =
    super::range_check_16::NUM_RANGE_CHECKS + super::range_check_degree::NUM_RANGE_CHECKS;

/// This column contains a permutation of the input values.
const fn col_permuted_input(i: usize) -> usize {
    debug_assert!(i < NUM_LOOKUPS);
    START_UNIT + 2 * i
}

/// This column contains a permutation of the table values.
const fn col_permuted_table(i: usize) -> usize {
    debug_assert!(i < NUM_LOOKUPS);
    START_UNIT + 2 * i + 1
}

pub(super) const END: usize = START_UNIT + NUM_LOOKUPS;
