use std::ops::RangeInclusive;

use ethereum_types::U256;

/// Create a U256, where the bits at indices inside the specified ranges are set to 1, and all other
/// bits are set to 0.
const fn u256_from_set_index_ranges<const N: usize>(ranges: &[RangeInclusive<u8>; N]) -> U256 {
    let mut j = 0;
    let mut res_limbs = [0u64; 4];
    while j < ranges.len() {
        let range = &ranges[j];
        let mut i = *range.start();
        if i > *range.end() {
            continue;
        }
        loop {
            let i_lo = i & 0x3f;
            let i_hi = i >> 6;
            res_limbs[i_hi as usize] |= 1 << i_lo;

            if i >= *range.end() {
                break;
            }
            i += 1;
        }
        j += 1;
    }
    U256(res_limbs)
}

pub(crate) const STACK_LENGTH_INCREASING_OPCODES_USER: U256 = u256_from_set_index_ranges(&[
    0x30..=0x30, // ADDRESS
    0x32..=0x34, // ORIGIN, CALLER, CALLVALUE
    0x36..=0x36, // CALLDATASIZE
    0x38..=0x38, // CODESIZE
    0x3a..=0x3a, // GASPRICE
    0x3d..=0x3d, // RETURNDATASIZE
    0x41..=0x48, // COINBASE, TIMESTAMP, NUMBER, DIFFICULTY, GASLIMIT, CHAINID, SELFBALANCE, BASEFEE
    0x58..=0x5a, // PC, MSIZE, GAS
    0x5f..=0x8f, // PUSH*, DUP*
]);

pub(crate) const INVALID_OPCODES_USER: U256 = u256_from_set_index_ranges(&[
    0x0c..=0x0f,
    0x1e..=0x1f,
    0x21..=0x2f,
    0x49..=0x4f,
    0x5c..=0x5e,
    0xa5..=0xef,
    0xf6..=0xf9,
    0xfb..=0xfc,
    0xfe..=0xfe,
]);
