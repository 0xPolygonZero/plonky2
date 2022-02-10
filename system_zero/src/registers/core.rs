//! Core registers.

/// A cycle counter. Starts at 0; increments by 1.
pub(crate) const COL_CLOCK: usize = super::START_CORE;

/// A column which contains the values `[0, ... 2^16 - 1]`, potentially with duplicates. Used for
/// 16-bit range checks.
///
/// For ease of verification, we enforce that it must begin with 0 and end with `2^16 - 1`, and each
/// delta must be either 0 or 1.
pub(crate) const COL_RANGE_16: usize = COL_CLOCK + 1;

/// Pointer to the current instruction.
pub(crate) const COL_INSTRUCTION_PTR: usize = COL_RANGE_16 + 1;
/// Pointer to the base of the current call's stack frame.
pub(crate) const COL_FRAME_PTR: usize = COL_INSTRUCTION_PTR + 1;
/// Pointer to the tip of the current call's stack frame.
pub(crate) const COL_STACK_PTR: usize = COL_FRAME_PTR + 1;

pub(super) const END: usize = COL_STACK_PTR + 1;
