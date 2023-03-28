//! Implementation of the Poseidon2 hash function and the traits necessary to employ it in Plonky2

use alloc::vec;
use alloc::vec::Vec;

use unroll::unroll_for_loops;

use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::types::{Field, PrimeField64};
use crate::poseidon2_gate::Poseidon2Gate;
use plonky2::hash::hash_types::{HashOut, RichField};
use plonky2::hash::hashing::{compress, hash_n_to_hash_no_pad, PlonkyPermutation, SPONGE_WIDTH};
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::iop::target::{BoolTarget, Target};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::config::{AlgebraicHasher, Hasher};

/// The number of full rounds and partial rounds is given by the
/// calc_round_numbers.py script. They happen to be the same for both
/// width 8 and width 12 with s-box x^7.
//
// NB: Changing any of these values will require regenerating all of
// the precomputed constant arrays in this file.
pub const HALF_N_FULL_ROUNDS: usize = 4;
pub(crate) const N_FULL_ROUNDS_TOTAL: usize = 2 * HALF_N_FULL_ROUNDS;
pub const N_PARTIAL_ROUNDS: usize = 22;
pub const N_ROUNDS: usize = N_FULL_ROUNDS_TOTAL + N_PARTIAL_ROUNDS;
const MAX_WIDTH: usize = 12; // we only have width 8 and 12, and 12 is bigger. :)

// Round constants for Poseidon and Poseidon2 are the same (given a specific instance)
#[rustfmt::skip]
pub const ALL_ROUND_CONSTANTS: [u64; MAX_WIDTH * N_ROUNDS]  = [
    // WARNING: The AVX2 Goldilocks specialization relies on all round constants being in
    // 0..0xfffeeac900011537. If these constants are randomly regenerated, there is a ~.6% chance
    // that this condition will no longer hold.
    0xe034a8785fd284a7, 0xe2463f1ea42e1b80, 0x048742e681ae290a, 0xe4af50ade990154c,
    0x8b13ffaaf4f78f8a, 0xe3fbead7dccd8d63, 0x631a47705eb92bf8, 0x88fbbb8698548659,
    0x74cd2003b0f349c9, 0xe16a3df6764a3f5d, 0x57ce63971a71aaa2, 0xdc1f7fd3e7823051,
    0xbb8423be34c18d7a, 0xf8bc5a2a0c1b3d6d, 0xf1a01bbd6f7123e5, 0xed960a080f5e348b,
    0x1b9c0c1e87e2390e, 0x18c83caf729a613e, 0x671ab9fe037a72c4, 0x508565f67d4c276a,
    0x4d2cd8827a482590, 0xa48e11e84dd3500b, 0x825a8c955fc2442b, 0xf573a6ee07cddc68,
    0x7dd3f19c73a39e0b, 0xcc0f13537a796fa6, 0x1d9006bfaedac57f, 0x4705f69b68b0b7de,
    0x5b62bfb718bcc57f, 0x879d821770563827, 0x3da5ccb7f8dff0e3, 0xb49d6a706923fc5b,
    0xb6a0babe883a969d, 0x2984f9b055401960, 0xcd3496f05511d79d, 0x4791da5d63854fc5,
    0xdb7344d0580a39d4, 0x5aedc4dad1de120a, 0x5e1bdc1fb8e1abf0, 0x3904c09a0e46747c,
    0xb54a0e23ab85ddcd, 0xc0c3cf05bccbdb3a, 0xb362076a73baf7e9, 0x212c953d81a5d5ba,
    0x212d4cc965d898bd, 0xdd44ddd0f41509b9, 0x8931329fa67823c0, 0xc65510f4d2a873be,
    0xe3ecbb6ba1e16211, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x70f5b3266792bbb6, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0xe7560e690634757e, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0xafd0202bc7eaf66e, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x349f4c5871f220fd, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x3697eb3e31529e0d, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x7735d5b0622d9900, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x5f5b58b9cf997668, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x645534b6548af9d9, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x4232d29d91a426a8, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0xb987278aed485d35, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x6dabeef669bb406e, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x35ee78288b749d40, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x6dcd560f14af0fc3, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x71ed3dc007ea6383, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x8b6b51caab7f5b6f, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0xcf2e8cc4181dbfa8, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0xa01d3f1c306f825a, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0xccee646a5d8ddb87, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x70df6f277cbaffeb, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x64ec0a6556b8f45c, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x6f68c9664fda6e37, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x387356e4516fab6f, 0x35310dce33903e67, 0x45f3e5251d30f912, 0x7c97f480ca428f45,
    0x74d5874c20b50de2, 0xff1d5b7cee3dc67f, 0xa04d5d5ac0ff3de9, 0x1cefb5eb7d24580e,
    0xf685e1bfcc0104ad, 0x6204dd95db22ead4, 0x8265c6c57c73c440, 0x4f708ab0b4e1e382,
    0xcfc60c7a52fbffa7, 0x9c0c1951d8910306, 0x4d06df27c89819f2, 0x621bdb0e75eca660,
    0x343adffd079cee57, 0xa760f0e5debde398, 0xe3110fefd97b188a, 0x0ed6584e6b150297,
    0x2b10e625d0d079c0, 0xefa493442057264f, 0xebcfaa7b3f26a2b6, 0xf36bcda28e343e2a,
    0xa1183cb63b67aa9e, 0x40f3e415d5e5b0ba, 0xc51fc2367eff7b15, 0xe07fe5f3aebc649f,
    0xc9cb2be56968e8aa, 0x648600db69078a0e, 0x4e9135ab1256edb9, 0x00382c73435556c2,
    0x1d78cafac9150ddf, 0xb8df60ab6215a233, 0xa7a65ba31f8fcd9a, 0x907d436dd964006b,
    0x3bdf7fd528633b97, 0x265adb359c0cc0f8, 0xf16cfc4034b39614, 0x71f0751b08fa0947,
    0x3165eda4b5403a37, 0xca30fc5680467e46, 0x4c743354d37777c5, 0x3d1f0a4e6bba4a09,
    0xc0c2e289afa75181, 0x1e4fa2ad948978b7, 0x2a226a127a0bb26a, 0xe61738a70357ce76,
];

const WIDTH: usize = SPONGE_WIDTH;
pub trait Poseidon2: PrimeField64 {
    /// Total number of round constants required: width of the input
    /// times number of rounds.
    const N_ROUND_CONSTANTS: usize = WIDTH * N_ROUNDS;

    /// We only need INTERNAL_MATRIX_DIAG_M_1 here, specifying the diagonal - 1 of the internal matrix
    const INTERNAL_MATRIX_DIAG_M_1: [u64; WIDTH];

    /// Compute the product between the state vector and the matrix employed in full rounds of
    /// the permutation
    #[inline(always)]
    #[unroll_for_loops]
    fn external_matrix(state: &mut[Self; WIDTH]) {
        // Applying cheap 4x4 MDS matrix to each 4-element part of the state
        // The matrix in this case is:
        // M_4 =
        // [5   7   1   3]
        // [4   6   1   1]
        // [1   3   5   7]
        // [1   1   4   6]
        // The computation is shown in more detail in https://tosc.iacr.org/index.php/ToSC/article/view/888/839, Figure 13 (M_{4,4}^{8,4} with alpha = 2)
        let t4 = WIDTH / 4;
        let mut state_u128: Vec<u128> = state.iter().map(|&e| e.to_canonical_u64() as u128).collect();
        for i in 0..t4 {
            let start_index = i * 4;
            let mut t_0 = state_u128[start_index];
            t_0 += state_u128[start_index + 1];
            let mut t_1 = state_u128[start_index + 2];
            t_1 += state_u128[start_index + 3];
            let mut t_2 = state_u128[start_index + 1];
            t_2 = t_2 + t_2;
            t_2 += t_1;
            let mut t_3 = state_u128[start_index + 3];
            t_3 = t_3 + t_3;
            t_3 += t_0;
            let mut t_4 = t_1;
            t_4 = 4 * t_4;
            t_4 += t_3;
            let mut t_5 = t_0;
            t_5 = 4 * t_5;
            t_5 += t_2;
            let mut t_6 = t_3;
            t_6 += t_5;
            let mut t_7 = t_2;
            t_7 += t_4;
            state_u128[start_index] = t_6;
            state_u128[start_index + 1] = t_5;
            state_u128[start_index + 2] = t_7;
            state_u128[start_index + 3] = t_4;
        }

        // Applying second cheap matrix
        // This completes the multiplication by
        // M_E =
        // [2*M_4    M_4    M_4]
        // [  M_4  2*M_4    M_4]
        // [  M_4    M_4  2*M_4]
        // using the results with M_4 obtained above
        let mut stored = [0 as u128; 4];
        for l in 0..4 {
            stored[l] = state_u128[l];
            for j in 1..t4 {
                stored[l] += state_u128[4 * j + l];
            }
        }
        for i in 0..WIDTH {
            state_u128[i] += stored[i % 4];
            state[i] = Self::from_noncanonical_u128(state_u128[i]);
        }
    }

    /// Same as `external_matrix` for field extensions of `Self`.
    fn external_matrix_field<F: FieldExtension<D, BaseField = Self>, const D: usize>(
        state: &mut [F; WIDTH],
    ) {
        // Applying cheap 4x4 MDS matrix to each 4-element part of the state
        let t4 = WIDTH / 4;
        for i in 0..t4 {
            let start_index = i * 4;
            let mut t_0 = state[start_index];
            t_0 += state[start_index + 1];
            let mut t_1 = state[start_index + 2];
            t_1 += state[start_index + 3];
            let mut t_2 = state[start_index + 1];
            t_2 = t_2 + t_2;
            t_2 += t_1;
            let mut t_3 = state[start_index + 3];
            t_3 = t_3 + t_3;
            t_3 += t_0;
            let mut t_4 = t_1;
            t_4 = F::from_canonical_u64(4) * t_4;
            t_4 += t_3;
            let mut t_5 = t_0;
            t_5 = F::from_canonical_u64(4) * t_5;
            t_5 += t_2;
            let mut t_6 = t_3;
            t_6 += t_5;
            let mut t_7 = t_2;
            t_7 += t_4;
            state[start_index] = t_6;
            state[start_index + 1] = t_5;
            state[start_index + 2] = t_7;
            state[start_index + 3] = t_4;
        }

        // Applying second cheap matrix
        let mut stored = [F::ZERO; 4];
        for l in 0..4 {
            stored[l] = state[l];
            for j in 1..t4 {
                stored[l] += state[4 * j + l];
            }
        }
        for i in 0..WIDTH {
            state[i] += stored[i % 4];
        }

    }

    /// Recursive version of `external_matrix`.
    fn external_matrix_circuit<const D: usize>(
        builder: &mut CircuitBuilder<Self, D>,
        state: &mut [ExtensionTarget<D>; WIDTH],
    )
        where
            Self: RichField + Extendable<D>,
    {
        // In contrast to the Poseidon circuit, we *may not need* PoseidonMdsGate, because the number of constraints will fit regardless
        // Check!
        let four = Self::from_canonical_u64(0x4);
        // let four = builder.constant_extension(Self::Extension::from_canonical_u64(0x4));

        // Applying cheap 4x4 MDS matrix to each 4-element part of the state
        let t4 = WIDTH / 4;
        for i in 0..t4 {
            let start_index = i * 4;
            let mut t_0 = state[start_index];
            t_0 = builder.add_extension(t_0, state[start_index + 1]);
            let mut t_1 = state[start_index + 2];
            t_1 = builder.add_extension(t_1, state[start_index + 3]);
            let mut t_2 = state[start_index + 1];
            t_2 = builder.add_extension(t_2, t_2); // Double
            t_2 = builder.add_extension(t_2, t_1);
            let mut t_3 = state[start_index + 3];
            t_3 = builder.add_extension(t_3, t_3); // Double
            t_3 = builder.add_extension(t_3, t_0);
            let mut t_4 = t_1;
            t_4 = builder.mul_const_extension(four, t_4); // times 4
            t_4 = builder.add_extension(t_4, t_3);
            let mut t_5 = t_0;
            t_5 = builder.mul_const_extension(four, t_5); // times 4
            t_5 = builder.add_extension(t_5, t_2);
            let mut t_6 = t_3;
            t_6 = builder.add_extension(t_6, t_5);
            let mut t_7 = t_2;
            t_7 = builder.add_extension(t_7, t_4);
            state[start_index] = t_6;
            state[start_index + 1] = t_5;
            state[start_index + 2] = t_7;
            state[start_index + 3] = t_4;
        }

        // Applying second cheap matrix
        let mut stored = [builder.zero_extension(); 4];
        for l in 0..4 {
            stored[l] = state[l];
            for j in 1..t4 {
                stored[l] = builder.add_extension(stored[l], state[4 * j + l]);
            }
        }
        for i in 0..WIDTH {
            state[i] = builder.add_extension(state[i], stored[i % 4]);
        }
    }

    /// Compute the product between the state vector and the matrix employed in partial rounds of
    /// the permutation
    #[inline(always)]
    #[unroll_for_loops]
    fn internal_matrix(state: &mut [Self; WIDTH]) {

        // This computes the mutliplication with the matrix
        // M_I =
        // [r_1     1   1   ...     1]
        // [  1   r_2   1   ...     1]
        // ...
        // [  1     1   1   ...   r_t]
        // for pseudo-random values r_1, r_2, ..., r_t. Note that for efficiency in Self::INTERNAL_MATRIX_DIAG_M_1 only r_1 - 1, r_2 - 1, ..., r_t - 1 are stored
        // Compute input sum
        let f_sum = Self::from_noncanonical_u128(
            state.iter().fold(0u128, |sum, el| sum + el.to_noncanonical_u64() as u128)
        );
        // Add sum + diag entry * element to each element
        for i in 0..WIDTH {
            state[i] *= Self::from_canonical_u64(Self::INTERNAL_MATRIX_DIAG_M_1[i]);
            state[i] += f_sum;
        }
    }

    /// Same as `internal_matrix` for field extensions of `Self`.
    fn internal_matrix_field<F: FieldExtension<D, BaseField = Self>, const D: usize>(
        state: &mut [F; WIDTH],
    ) {
        // Compute input sum
        let sum = state.iter().fold(F::ZERO, |sum, el| sum + *el);
        // Add sum + diag entry * element to each element
        for i in 0..state.len() {
            state[i] *= F::from_canonical_u64(Self::INTERNAL_MATRIX_DIAG_M_1[i]);
            state[i] += sum;
        }
    }

    /// Recursive version of `internal_matrix`.
    fn internal_matrix_circuit<const D: usize>(
        builder: &mut CircuitBuilder<Self, D>,
        state: &mut [ExtensionTarget<D>; WIDTH],
    )
        where
            Self: RichField + Extendable<D>,
    {
        // In contrast to the Poseidon circuit, we *may not need* PoseidonMdsGate, because the number of constraints will fit regardless
        // Check!

        // Compute input sum
        let mut sum = state[0];
        for i in 1..state.len() {
            sum = builder.add_extension(sum, state[i]);
        }
        // Add sum + diag entry * element to each element
        for i in 0..state.len() {
            // Computes `C * x + y`
            state[i] = builder.mul_const_add_extension(Self::from_canonical_u64(<Self as Poseidon2>::INTERNAL_MATRIX_DIAG_M_1[i]), state[i], sum);
        }
    }

    /// Add round constant to `state` for round `round_ctr`
    #[inline(always)]
    #[unroll_for_loops]
    fn constant_layer(state: &mut [Self; WIDTH], round_ctr: usize) {
        for i in 0..12 {
            if i < WIDTH {
                let round_constant = ALL_ROUND_CONSTANTS[i + WIDTH * round_ctr];
                unsafe {
                    state[i] = state[i].add_canonical_u64(round_constant);
                }
            }
        }
    }

    /// Same as `constant_layer` for field extensions of `Self`.
    fn constant_layer_field<F: FieldExtension<D, BaseField = Self>, const D: usize>(
        state: &mut [F; WIDTH],
        round_ctr: usize,
    ) {
        for i in 0..WIDTH {
            state[i] += F::from_canonical_u64(ALL_ROUND_CONSTANTS[i + WIDTH * round_ctr]);
        }
    }

    /// Recursive version of `constant_layer`.
    fn constant_layer_circuit<const D: usize>(
        builder: &mut CircuitBuilder<Self, D>,
        state: &mut [ExtensionTarget<D>; WIDTH],
        round_ctr: usize,
    ) where
        Self: RichField + Extendable<D>,
    {
        for i in 0..WIDTH {
            let c = ALL_ROUND_CONSTANTS[i + WIDTH * round_ctr];
            let c = Self::Extension::from_canonical_u64(c);
            let c = builder.constant_extension(c);
            state[i] = builder.add_extension(state[i], c);
        }
    }

    /// Apply the sbox to a single state element
    #[inline(always)]
    fn sbox_monomial<F: FieldExtension<D, BaseField = Self>, const D: usize>(x: F) -> F {
        // x |--> x^7
        let x2 = x.square();
        let x4 = x2.square();
        let x3 = x * x2;
        x3 * x4
    }

    /// Recursive version of `sbox_monomial`.
    fn sbox_monomial_circuit<const D: usize>(
        builder: &mut CircuitBuilder<Self, D>,
        x: ExtensionTarget<D>,
    ) -> ExtensionTarget<D>
        where
            Self: RichField + Extendable<D>,
    {
        // x |--> x^7
        builder.exp_u64_extension(x, 7)
    }

    /// Apply the sbox-layer to the whole state of the permutation
    #[inline(always)]
    #[unroll_for_loops]
    fn sbox_layer(state: &mut [Self; WIDTH]) {
        for i in 0..12 {
            if i < WIDTH {
                state[i] = Self::sbox_monomial(state[i]);
            }
        }
    }

    /// Same as `sbox_layer` for field extensions of `Self`.
    fn sbox_layer_field<F: FieldExtension<D, BaseField = Self>, const D: usize>(
        state: &mut [F; WIDTH],
    ) {
        for i in 0..WIDTH {
            state[i] = Self::sbox_monomial(state[i]);
        }
    }

    /// Recursive version of `sbox_layer`.
    fn sbox_layer_circuit<const D: usize>(
        builder: &mut CircuitBuilder<Self, D>,
        state: &mut [ExtensionTarget<D>; WIDTH],
    ) where
        Self: RichField + Extendable<D>,
    {
        for i in 0..WIDTH {
            state[i] = <Self as Poseidon2>::sbox_monomial_circuit(builder, state[i]);
        }
    }

    /// Apply half of the overall full rounds of the permutation. It can be employed to perform
    /// both the first and the second half of the full rounds, depending on the value of `round_ctr`
    #[inline]
    fn full_rounds(state: &mut [Self; WIDTH], round_ctr: &mut usize) {
        for _ in 0..HALF_N_FULL_ROUNDS {
            Self::constant_layer(state, *round_ctr);
            Self::sbox_layer(state);
            Self::external_matrix(state);
            *round_ctr += 1;
        }
    }

    /// Apply the partial rounds of the permutation
    #[inline]
    fn partial_rounds(state: &mut [Self; WIDTH], round_ctr: &mut usize) {
        let mut constant_counter = HALF_N_FULL_ROUNDS * WIDTH;
        for _ in 0..N_PARTIAL_ROUNDS {
            unsafe {
                state[0] = state[0].add_canonical_u64(ALL_ROUND_CONSTANTS[constant_counter]);
                constant_counter += WIDTH;
            }
            state[0] = Self::sbox_monomial(state[0]);
            Self::internal_matrix(state);
        }
        *round_ctr += N_PARTIAL_ROUNDS;
    }
    /// Apply the entire Poseidon2 permutation to `input`
    ///
    /// ```rust
    /// use plonky2::field::goldilocks_field::GoldilocksField as F;
    /// use plonky2::field::types::Sample;
    /// use poseidon2_plonky2::poseidon2_hash::Poseidon2;
    ///
    /// let output = F::poseidon2(F::rand_array());
    /// ```
    #[inline]
    fn poseidon2(input: [Self; WIDTH]) -> [Self; WIDTH] {
        let mut state = input;
        let mut round_ctr = 0;

        // First external matrix
        Self::external_matrix(&mut state);

        Self::full_rounds(&mut state, &mut round_ctr);
        Self::partial_rounds(&mut state, &mut round_ctr);
        Self::full_rounds(&mut state, &mut round_ctr);
        debug_assert_eq!(round_ctr, N_ROUNDS);

        state
    }
}

/// Poseidon2 permutation
pub struct Poseidon2Permutation;
impl<F: RichField + Poseidon2> PlonkyPermutation<F> for Poseidon2Permutation {
    fn permute(input: [F; SPONGE_WIDTH]) -> [F; SPONGE_WIDTH] {
        F::poseidon2(input)
    }
}

/// Poseidon2 hash function.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Poseidon2Hash;
impl<F: RichField + Poseidon2> Hasher<F> for Poseidon2Hash {
    const HASH_SIZE: usize = 4 * 8;
    type Hash = HashOut<F>;
    type Permutation = Poseidon2Permutation;

    fn hash_no_pad(input: &[F]) -> Self::Hash {
        hash_n_to_hash_no_pad::<F, Self::Permutation>(input)
    }

    fn two_to_one(left: Self::Hash, right: Self::Hash) -> Self::Hash {
        compress::<F, Self::Permutation>(left, right)
    }
}

impl<F: RichField + Poseidon2> AlgebraicHasher<F> for Poseidon2Hash {
    fn permute_swapped<const D: usize>(
        inputs: [Target; SPONGE_WIDTH],
        swap: BoolTarget,
        builder: &mut CircuitBuilder<F, D>,
    ) -> [Target; SPONGE_WIDTH]
        where
            F: RichField + Extendable<D>,
    {
        let gate_type = Poseidon2Gate::<F, D>::new();
        let gate = builder.add_gate(gate_type, vec![]);

        let swap_wire = Poseidon2Gate::<F, D>::WIRE_SWAP;
        let swap_wire = Target::wire(gate, swap_wire);
        builder.connect(swap.target, swap_wire);

        // Route input wires.
        for i in 0..SPONGE_WIDTH {
            let in_wire = Poseidon2Gate::<F, D>::wire_input(i);
            let in_wire = Target::wire(gate, in_wire);
            builder.connect(inputs[i], in_wire);
        }

        // Collect output wires.
        (0..SPONGE_WIDTH)
            .map(|i| Target::wire(gate, Poseidon2Gate::<F, D>::wire_output(i)))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
    }
}

#[cfg(test)]
pub(crate) mod test_helpers {
    use plonky2::field::types::Field;
    use plonky2::hash::hash_types::RichField;
    use plonky2::hash::hashing::SPONGE_WIDTH;
    use plonky2::plonk::circuit_builder::CircuitBuilder;
    use plonky2::plonk::circuit_data::{CircuitConfig, CircuitData};
    use plonky2::plonk::config::{AlgebraicHasher, GenericConfig, Hasher};
    use plonky2::plonk::proof::ProofWithPublicInputs;
    use super::Poseidon2;
    use anyhow::Result;
    use plonky2::field::extension::Extendable;
    use plonky2::iop::witness::{PartialWitness, WitnessWrite};
    use plonky2::plonk::prover::prove;
    use plonky2::util::timing::TimingTree;
    use log::Level;

    pub(crate) fn check_test_vectors<F: Field>(
        test_vectors: Vec<([u64; SPONGE_WIDTH], [u64; SPONGE_WIDTH])>,
    ) where
        F: Poseidon2,
    {
        for (input, expected_output) in test_vectors.into_iter() {
            let mut state = [F::ZERO; SPONGE_WIDTH];
            for i in 0..SPONGE_WIDTH {
                state[i] = F::from_canonical_u64(input[i]);
            }
            let output = F::poseidon2(state);
            for i in 0..SPONGE_WIDTH {
                let ex_output = F::from_canonical_u64(expected_output[i]); // Adjust!
                assert_eq!(output[i], ex_output);
            }
        }
    }

    pub(crate) fn check_consistency<F: Field>()
        where
            F: Poseidon2,
    {
        let mut input = [F::ZERO; SPONGE_WIDTH];
        for i in 0..SPONGE_WIDTH {
            input[i] = F::from_canonical_u64(i as u64);
        }
        let output = F::poseidon2(input);
        for i in 0..SPONGE_WIDTH {
            assert_eq!(output[i], output[i]); // Dummy check
        }
    }

    pub(crate) fn prove_circuit_with_poseidon2
    <
        F: RichField + Poseidon2 + Extendable<D>,
        C: GenericConfig<D, F=F>,
        const D: usize,
        H: Hasher<F> + AlgebraicHasher<F>
    >
    (
        config: CircuitConfig,
        num_ops: usize,
        _hasher: H,
        print_timing: bool,
    ) -> Result<(CircuitData<F,C,D>, ProofWithPublicInputs<F,C,D>)> {
        let mut builder = CircuitBuilder::<F,D>::new(config);
        let init_t = builder.add_virtual_public_input();
        let mut res_t = builder.add_virtual_target();
        builder.connect(init_t, res_t);
        let hash_targets = (0..SPONGE_WIDTH-1).map(|_|
            builder.add_virtual_target()
        ).collect::<Vec<_>>();
        for _ in 0..num_ops {
            res_t = builder.mul(res_t, res_t);
            let mut to_be_hashed_elements = vec![res_t];
            to_be_hashed_elements.extend_from_slice(hash_targets.as_slice());
            res_t = builder.hash_or_noop::<H>(to_be_hashed_elements).elements[0]
        }
        let out_t = builder.add_virtual_public_input();
        let is_eq_t = builder.is_equal(out_t, res_t);
        builder.assert_one(is_eq_t.target);

        let data = builder.build::<C>();

        let mut pw = PartialWitness::<F>::new();
        let input = F::rand();
        pw.set_target(init_t, input);

        let input_hash_elements = hash_targets.iter().map(|&hash_t| {
            let elem = F::rand();
            pw.set_target(hash_t, elem);
            elem
        }).collect::<Vec<_>>();

        let mut res = input.clone();
        for _ in 0..num_ops {
            res = res.mul(res);
            let mut to_be_hashed_elements = vec![res];
            to_be_hashed_elements.extend_from_slice(input_hash_elements.as_slice());
            res = H::hash_no_pad(to_be_hashed_elements.as_slice()).elements[0]
        }

        pw.set_target(out_t, res);

        let proof = if print_timing {
            let mut timing = TimingTree::new("prove", Level::Debug);
            let proof = prove(&data.prover_only, &data.common, pw, &mut timing)?;
            timing.print();
            proof
        } else {
            data.prove(pw)?
        };

        assert_eq!(proof.public_inputs[0], input);
        assert_eq!(proof.public_inputs[1], res);

        Ok((data, proof))
    }

    pub(crate) fn recursive_proof<
        F: RichField + Poseidon2 + Extendable<D>,
        C: GenericConfig<D, F=F>,
        InnerC: GenericConfig<D, F = F>,
        const D: usize,
    >(
        inner_proof: ProofWithPublicInputs<F, InnerC, D>,
        inner_cd: &CircuitData<F,InnerC,D>,
        config: &CircuitConfig,
    ) -> Result<(CircuitData<F,C,D>, ProofWithPublicInputs<F,C,D>)>
        where
            InnerC::Hasher: AlgebraicHasher<F>,
    {
        let mut builder = CircuitBuilder::<F, D>::new(config.clone());
        let mut pw = PartialWitness::new();
        let pt = builder.add_virtual_proof_with_pis(&inner_cd.common);
        pw.set_proof_with_pis_target(&pt, &inner_proof);

        let inner_data = builder.add_virtual_verifier_data(inner_cd.common.config.fri_config.cap_height);
        pw.set_cap_target(
            &inner_data.constants_sigmas_cap,
            &inner_cd.verifier_only.constants_sigmas_cap,
        );
        pw.set_hash_target(inner_data.circuit_digest, inner_cd.verifier_only.circuit_digest);

        for &pi_t in pt.public_inputs.iter() {
            let t = builder.add_virtual_public_input();
            builder.connect(pi_t, t);
        }
        builder.verify_proof::<InnerC>(&pt, &inner_data, &inner_cd.common);
        let data = builder.build::<C>();

        let proof = data.prove(pw)?;

        Ok((data, proof))

    }
}
