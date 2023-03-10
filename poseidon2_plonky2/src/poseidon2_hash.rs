//! Implementation of the Poseidon2 hash function as described in
//! <https://eprint.iacr.org/YYYY/NNN.pdf>

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

// The number of full rounds and partial rounds is given by the
// calc_round_numbers.py script. They happen to be the same for both
// width 8 and width 12 with s-box x^7.
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
    //
    // WARNING: If these are changed in any way, then all the
    // implementations of Poseidon must be regenerated. See comments
    // in `poseidon2_goldilocks.rs`.
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

implement_poseidon2!(Poseidon2, ALL_ROUND_CONSTANTS, SPONGE_WIDTH);

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

    // Dummy
    /*fn permute_swapped_c<const D: usize>(
        inputs: [Target; COMPRESSION_WIDTH],
        _: BoolTarget,
        _: &mut CircuitBuilder<F, D>,
    ) -> [Target; COMPRESSION_WIDTH]
        where
            F: RichField + Extendable<D>,
    {
        inputs
    }*/
}

#[cfg(test)]
pub(crate) mod test_helpers {
    use plonky2::field::types::Field;
    use plonky2::hash::hashing::SPONGE_WIDTH;
    use super::Poseidon2;

    pub(crate) fn check_test_vectors<F: Field>(
        test_vectors: Vec<([u64; SPONGE_WIDTH], [u64; SPONGE_WIDTH])>,
    ) where
        F: Poseidon2,
    {
        for (input_, expected_output_) in test_vectors.into_iter() {
            let mut input = [F::ZERO; SPONGE_WIDTH];
            for i in 0..SPONGE_WIDTH {
                input[i] = F::from_canonical_u64(input_[i]);
            }
            let output = F::poseidon2(input);
            for i in 0..SPONGE_WIDTH {
                let ex_output = F::from_canonical_u64(expected_output_[i]); // Adjust!
                assert_eq!(output[i], ex_output);
            }
        }
    }

    // TODO: Remove later
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
}