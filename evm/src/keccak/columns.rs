use plonky2::field::types::Field;

use crate::cross_table_lookup::Column;
use crate::keccak::keccak_stark::{NUM_INPUTS, NUM_ROUNDS};

/// A register which is set to 1 if we are in the `i`th round, otherwise 0.
pub const fn reg_step(i: usize) -> usize {
    debug_assert!(i < NUM_ROUNDS);
    i
}

/// Registers to hold permutation inputs.
/// `reg_input_limb(2*i) -> input[i] as u32`
/// `reg_input_limb(2*i+1) -> input[i] >> 32`
pub fn reg_input_limb<F: Field>(i: usize) -> Column<F> {
    debug_assert!(i < 2 * NUM_INPUTS);
    let i_u64 = i / 2; // The index of the 64-bit chunk.

    // The 5x5 state is treated as y-major, as per the Keccak spec.
    let y = i_u64 / 5;
    let x = i_u64 % 5;

    let reg_low_limb = reg_a(x, y);
    let is_high_limb = i % 2;
    Column::single(reg_low_limb + is_high_limb)
}

/// Registers to hold permutation outputs.
/// `reg_output_limb(2*i) -> output[i] as u32`
/// `reg_output_limb(2*i+1) -> output[i] >> 32`
pub const fn reg_output_limb(i: usize) -> usize {
    debug_assert!(i < 2 * NUM_INPUTS);
    let i_u64 = i / 2; // The index of the 64-bit chunk.

    // The 5x5 state is treated as y-major, as per the Keccak spec.
    let y = i_u64 / 5;
    let x = i_u64 % 5;

    let is_high_limb = i % 2;
    reg_a_prime_prime_prime(x, y) + is_high_limb
}

const R: [[u8; 5]; 5] = [
    [0, 36, 3, 41, 18],
    [1, 44, 10, 45, 2],
    [62, 6, 43, 15, 61],
    [28, 55, 25, 21, 56],
    [27, 20, 39, 8, 14],
];

const START_A: usize = NUM_ROUNDS;
pub(crate) const fn reg_a(x: usize, y: usize) -> usize {
    debug_assert!(x < 5);
    debug_assert!(y < 5);
    START_A + (x * 5 + y) * 2
}

// C[x] = xor(A[x, 0], A[x, 1], A[x, 2], A[x, 3], A[x, 4])
const START_C: usize = START_A + 5 * 5 * 2;
pub(crate) const fn reg_c(x: usize, z: usize) -> usize {
    debug_assert!(x < 5);
    debug_assert!(z < 64);
    START_C + x * 64 + z
}

// C'[x, z] = xor(C[x, z], C[x - 1, z], C[x + 1, z - 1])
const START_C_PRIME: usize = START_C + 5 * 64;
pub(crate) const fn reg_c_prime(x: usize, z: usize) -> usize {
    debug_assert!(x < 5);
    debug_assert!(z < 64);
    START_C_PRIME + x * 64 + z
}

// Note: D is inlined, not stored in the witness.

// A'[x, y] = xor(A[x, y], D[x])
//          = xor(A[x, y], C[x - 1], ROT(C[x + 1], 1))
const START_A_PRIME: usize = START_C_PRIME + 5 * 64;
pub(crate) const fn reg_a_prime(x: usize, y: usize, z: usize) -> usize {
    debug_assert!(x < 5);
    debug_assert!(y < 5);
    debug_assert!(z < 64);
    START_A_PRIME + x * 64 * 5 + y * 64 + z
}

pub(crate) const fn reg_b(x: usize, y: usize, z: usize) -> usize {
    debug_assert!(x < 5);
    debug_assert!(y < 5);
    debug_assert!(z < 64);
    // B is just a rotation of A', so these are aliases for A' registers.
    // From the spec,
    //     B[y, (2x + 3y) % 5] = ROT(A'[x, y], r[x, y])
    // So,
    //     B[x, y] = f((x + 3y) % 5, x)
    // where f(a, b) = ROT(A'[a, b], r[a, b])
    let a = (x + 3 * y) % 5;
    let b = x;
    let rot = R[a][b] as usize;
    reg_a_prime(a, b, (z + 64 - rot) % 64)
}

// A''[x, y] = xor(B[x, y], andn(B[x + 1, y], B[x + 2, y])).
const START_A_PRIME_PRIME: usize = START_A_PRIME + 5 * 5 * 64;
pub(crate) const fn reg_a_prime_prime(x: usize, y: usize) -> usize {
    debug_assert!(x < 5);
    debug_assert!(y < 5);
    START_A_PRIME_PRIME + x * 2 * 5 + y * 2
}

const START_A_PRIME_PRIME_0_0_BITS: usize = START_A_PRIME_PRIME + 5 * 5 * 2;
pub(crate) const fn reg_a_prime_prime_0_0_bit(i: usize) -> usize {
    debug_assert!(i < 64);
    START_A_PRIME_PRIME_0_0_BITS + i
}

const REG_A_PRIME_PRIME_PRIME_0_0_LO: usize = START_A_PRIME_PRIME_0_0_BITS + 64;
const REG_A_PRIME_PRIME_PRIME_0_0_HI: usize = REG_A_PRIME_PRIME_PRIME_0_0_LO + 1;

// A'''[0, 0] is additionally xor'd with RC.
pub(crate) const fn reg_a_prime_prime_prime(x: usize, y: usize) -> usize {
    debug_assert!(x < 5);
    debug_assert!(y < 5);
    if x == 0 && y == 0 {
        REG_A_PRIME_PRIME_PRIME_0_0_LO
    } else {
        reg_a_prime_prime(x, y)
    }
}

pub(crate) const NUM_COLUMNS: usize = REG_A_PRIME_PRIME_PRIME_0_0_HI + 1;
