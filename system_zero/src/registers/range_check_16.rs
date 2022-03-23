//! Range check unit which checks that values are in `[0, 2^16)`.

pub(super) const NUM_RANGE_CHECKS: usize = 6;

/// The input of the `i`th range check, i.e. the value being range checked.
pub(crate) const fn col_rc_16_input(i: usize) -> usize {
    debug_assert!(i < NUM_RANGE_CHECKS);
    super::START_RANGE_CHECK_16 + i
}

pub(super) const END: usize = super::START_RANGE_CHECK_16 + NUM_RANGE_CHECKS;
