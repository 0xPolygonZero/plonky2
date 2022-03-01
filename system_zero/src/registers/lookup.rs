//! Lookup unit.
//! See https://zcash.github.io/halo2/design/proving-system/lookup.html

const START_UNIT: usize = super::START_LOOKUP;

pub(crate) const NUM_LOOKUPS: usize =
    super::range_check_16::NUM_RANGE_CHECKS + super::range_check_degree::NUM_RANGE_CHECKS;

pub(crate) const fn col_input(i: usize) -> usize {
    if i < super::range_check_16::NUM_RANGE_CHECKS {
        super::range_check_16::col_rc_16_input(i)
    } else {
        super::range_check_degree::col_rc_degree_input(i - super::range_check_16::NUM_RANGE_CHECKS)
    }
}

/// This column contains a permutation of the input values.
pub(crate) const fn col_permuted_input(i: usize) -> usize {
    debug_assert!(i < NUM_LOOKUPS);
    START_UNIT + 2 * i
}

pub(crate) const fn col_table(i: usize) -> usize {
    if i < super::range_check_16::NUM_RANGE_CHECKS {
        super::core::COL_RANGE_16
    } else {
        super::core::COL_CLOCK
    }
}

/// This column contains a permutation of the table values.
pub(crate) const fn col_permuted_table(i: usize) -> usize {
    debug_assert!(i < NUM_LOOKUPS);
    START_UNIT + 2 * i + 1
}

pub(super) const END: usize = START_UNIT + NUM_LOOKUPS * 2;
