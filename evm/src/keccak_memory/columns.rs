pub(crate) const KECCAK_WIDTH_BYTES: usize = 200;

/// 1 if this row represents a real operation; 0 if it's a padding row.
pub(crate) const COL_IS_REAL: usize = 0;

// The address at which we will read inputs and write outputs.
pub(crate) const COL_CONTEXT: usize = 1;
pub(crate) const COL_SEGMENT: usize = 2;
pub(crate) const COL_VIRTUAL: usize = 3;

/// The timestamp at which inputs should be read from memory.
/// Outputs will be written at the following timestamp.
pub(crate) const COL_READ_TIMESTAMP: usize = 4;

const START_INPUT_LIMBS: usize = 5;
/// A byte of the input.
pub(crate) fn col_input_byte(i: usize) -> usize {
    debug_assert!(i < KECCAK_WIDTH_BYTES);
    START_INPUT_LIMBS + i
}

const START_OUTPUT_LIMBS: usize = START_INPUT_LIMBS + KECCAK_WIDTH_BYTES;
/// A byte of the output.
pub(crate) fn col_output_byte(i: usize) -> usize {
    debug_assert!(i < KECCAK_WIDTH_BYTES);
    START_OUTPUT_LIMBS + i
}

pub const NUM_COLUMNS: usize = START_OUTPUT_LIMBS + KECCAK_WIDTH_BYTES;
