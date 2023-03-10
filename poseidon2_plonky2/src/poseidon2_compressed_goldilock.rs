//! Implementations for Poseidon2 over Goldilocks field of widths 8 and 12.
//!
//! These contents of the implementations *must* be generated using the
//! `poseidon_constants.sage` script in the `mir-protocol/hash-constants`
//! repository.

use plonky2::field::{goldilocks_field::GoldilocksField, extension::quadratic::QuadraticExtension};
use plonky2::plonk::config::GenericConfig;
use crate::poseidon2_compressed_hash::{Poseidon2c, Poseidon2cHash};

#[rustfmt::skip]
impl Poseidon2c for GoldilocksField {
    // We only need INTERNAL_MATRIX_DIAG_M_1 here, specifying the diagonal - 1 of the internal matrix
    //
    // TODO: Adapt the following for the new internal matrix:
    //  - FAST_PARTIAL_FIRST_ROUND_CONSTANT
    //  - FAST_PARTIAL_ROUND_CONSTANTS

    const INTERNAL_MATRIX_DIAG_M_1: [u64; 8]  = [
        0xd57b33d215cc4805, 0xaa2238eb3ac17b62, 0x28925fe2f3895c0d, 0x3dab9370a67db22e,
        0xe5cafe41ef4eac62, 0x4c633d43f2260c06, 0x1fa5fb8a31d6369d, 0x999a460e4a706453,
    ];
}

/// Configuration using Poseidon2c over the Goldilocks field.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Poseidon2cGoldilocksConfig;
impl GenericConfig<2> for Poseidon2cGoldilocksConfig {
    type F = GoldilocksField;
    type FE = QuadraticExtension<Self::F>;
    type Hasher = Poseidon2cHash;
    type InnerHasher = Poseidon2cHash;
}

#[cfg(test)]
mod tests {
    use plonky2::field::goldilocks_field::GoldilocksField as F;
    use crate::poseidon2_compressed_hash::test_helpers::{check_consistency, check_test_vectors};

    #[test]
    fn test_vectors() {
        // Test inputs are:
        // 1. range 0..WIDTH

        #[rustfmt::skip]
            let test_vector: Vec<([u64; 8], [u64; 8])> = vec![
            ([0, 1, 2, 3, 4, 5, 6, 7],
             [0xb082d83af3972543, 0x3f0724f636c23139, 0xab505c56ecd19176, 0x65b5fc59b2a7360c,
                 0x75b9e3f88e48e325, 0x57d28525c3143db0, 0x48212160bfa5158e, 0x29555f54a2040e98,
             ]),
        ];

        check_test_vectors::<F>(test_vector);
    }

    #[test]
    fn consistency() {
        check_consistency::<F>();
    }
}