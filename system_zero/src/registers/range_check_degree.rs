//! Range check unit which checks that values are in `[0, degree)`.

pub(crate) const NUM_RANGE_CHECKS: usize = 5;

/// The input of the `i`th range check, i.e. the value being range checked.
pub(crate) const fn col_rc_degree_input(i: usize) -> usize {
    debug_assert!(i < NUM_RANGE_CHECKS);
    super::START_RANGE_CHECK_DEGREE + i
}

pub(super) const END: usize = super::START_RANGE_CHECK_DEGREE + NUM_RANGE_CHECKS;
