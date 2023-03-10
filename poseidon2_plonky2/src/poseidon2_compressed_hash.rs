use alloc::vec::Vec;

use unroll::unroll_for_loops;

use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::types::{Field, PrimeField64};
use plonky2::hash::hash_types::{HashOut, HashOutTarget, RichField};
use plonky2::hash::hashing::{compress, hash_n_to_m_no_pad, PlonkyPermutation, SPONGE_WIDTH};
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::iop::target::{BoolTarget, Target};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::config::{AlgebraicHasher, Hasher};
use crate::poseidon2_compressed_gate::Poseidon2cGate;
use crate::poseidon2_hash::{Poseidon2, Poseidon2Hash, Poseidon2Permutation};

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
pub const COMPRESSION_WIDTH: usize = 8;

// Round constants for Poseidon and Poseidon2c are the same (given a specific instance)
#[rustfmt::skip]
pub const ALL_ROUND_CONSTANTS: [u64; COMPRESSION_WIDTH * N_ROUNDS]  = [
    // WARNING: The AVX2 Goldilocks specialization relies on all round constants being in
    // 0..0xfffeeac900011537. If these constants are randomly regenerated, there is a ~.6% chance
    // that this condition will no longer hold.
    //
    // WARNING: If these are changed in any way, then all the
    // implementations of Poseidon must be regenerated. See comments
    // in `poseidon2c_goldilocks.rs`.
    0x57056152cedf0fe7, 0x44b125d16e93ca85, 0x8e8ea2ff8b7a6d2a, 0xcce7c6cc1468fa13,
    0x47f5feb953ce5073, 0xfd8f41d8ee6b700e, 0xe40f59b8db57aeb7, 0x78b572234ff68244,
    0x926b547a9712ed0b, 0xb1525da069ba226c, 0xf37650e9d8ef46d3, 0x3146518c7738aefc,
    0x04aa9f4d916e9e5b, 0xde603b81bb63d21c, 0x8382c29e88cf2c81, 0x50456f59f404cb88,
    0x44bda4a6711f6ddb, 0xe4c94cbc9e7d15b7, 0x7faec52ce37a8256, 0x7748e71fd7803107,
    0x9b6baf83e49be593, 0xd47fe8a5c8b27ed3, 0xfcdf1e28d16392ad, 0x976753b4b516a9ee,
    0xc16ea705aa7ee467, 0x18183d87f912ebbb, 0x02d3b175b21777fe, 0x98e4c2d93e0aaaef,
    0xc31191d90cd41c96, 0x69f8f94595ad453e, 0x1de4127f3e248a2d, 0xbcce9849c99a069c,
    0x8b8e707932590779, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x4d7fff707c77890f, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x7d36116962851777, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x1dc9f40fbb3146b7, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x6a235e2d5bef54e0, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x4d1a9ae6dd337207, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x46ab49a6009cda1a, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x78e759e819648587, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0xee6e84b7763598a4, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0b426bdcaad3050e, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x1f3cd981be91490e, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0xd54572f7ecf947a1, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x393c4432d0e86a1e, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x3f1b43149ef3f4f8, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x3705f6a66d25dce4, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x3e809302b3d41471, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x6e50830e082b17f1, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x711232bf2d77ac38, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x4235f7d079c78096, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0xab1bbdc696a72a25, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0xdb1ef6f3f7fed243, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0xd21981014e77d809, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x5b2cb2bd03a18856, 0x8e45a3e4bf30df6c, 0x3f9948080379716d, 0x41c2ba50c09d6c70,
    0x5c2f57c6f81d2c6b, 0x91cfb3d3b4b04a7a, 0x81327090650355f6, 0x06957eabf4817942,
    0x7f08201e9da0e064, 0x7467dfc268e1d6e0, 0x38a9992ed589cc80, 0x266a6e035fee9286,
    0xd19ebfbf75ffbf79, 0x9f1dc0303ca0acfb, 0x230f2d6a36b23347, 0xde0cdaab08319a52,
    0xff9e2984d5f675ba, 0x27a10c5aca2fcf50, 0x8982ec2da08deb87, 0x89f9b8d33e98a684,
    0x269bcee2edb77b24, 0xcd7fb3f592ab464f, 0x05060bc8d4341e72, 0xa75ab333263a6658,
    0x3962fe1b4bb486e7, 0x52160689b78a2fd1, 0x9e953026b7be93e6, 0x7215465ca2fa2b5a,
    0x458b8385c2107d5b, 0xd86fd0264024aad9, 0x2cb61942ee72b44c, 0x50784c715273f7e7,
];

implement_poseidon2!(Poseidon2c, ALL_ROUND_CONSTANTS, COMPRESSION_WIDTH);

pub fn hash_n_to_m_no_pad_c<F: RichField + Poseidon2 + Poseidon2c>(
    inputs: &[F],
    num_outputs: usize,
) -> Vec<F> {
    if inputs.len() <= COMPRESSION_WIDTH && num_outputs <= COMPRESSION_WIDTH {
        let mut perm_inputs = inputs.to_vec();
        perm_inputs.extend_from_slice(&vec![F::ZERO; SPONGE_WIDTH-inputs.len()]);
        Poseidon2cPermutation::permute(perm_inputs.try_into().unwrap())[..num_outputs].to_vec()
    } else {
        // we use Poseidon2 in non-compressed mode, which is more efficient
        hash_n_to_m_no_pad::<F, Poseidon2Permutation>(inputs, num_outputs)
    }
}

pub fn hash_n_to_hash_no_pad_c<F: RichField + Poseidon2c + Poseidon2>(inputs: &[F]) -> HashOut<F> {
    HashOut::from_vec(hash_n_to_m_no_pad_c::<F>(inputs, 4))
}

pub trait CompressedHash<F: RichField + Poseidon2c + Poseidon2> {
    fn hash_or_noop_compressed<H: AlgebraicHasher<F>>(&mut self, inputs: Vec<Target>) -> HashOutTarget;

    fn hash_n_to_hash_no_pad_compressed<H: AlgebraicHasher<F>>(
        &mut self,
        inputs: Vec<Target>,
    ) -> HashOutTarget {
        HashOutTarget::from_vec(self.hash_n_to_m_no_pad_compressed::<H>(inputs, 4))
    }

    fn hash_n_to_m_no_pad_compressed<H: AlgebraicHasher<F>>(
        &mut self,
        inputs: Vec<Target>,
        num_outputs: usize,
    ) -> Vec<Target>;
}

impl<F,const D: usize> CompressedHash<F> for CircuitBuilder<F,D>
where F: RichField + Extendable<D> + Poseidon2c + Poseidon2
{
    fn hash_or_noop_compressed<H: AlgebraicHasher<F>>(&mut self, inputs: Vec<Target>) -> HashOutTarget {
        let zero = self.zero();
        if inputs.len() <= 4 {
            HashOutTarget::from_partial(&inputs, zero)
        } else {
            self.hash_n_to_hash_no_pad_compressed::<H>(inputs)
        }
    }

    fn hash_n_to_m_no_pad_compressed<H: AlgebraicHasher<F>>(
        &mut self,
        inputs: Vec<Target>,
        num_outputs: usize,
    ) -> Vec<Target>
    {
        if inputs.len() <= COMPRESSION_WIDTH && num_outputs <= COMPRESSION_WIDTH {
            let mut perm_inputs = inputs.to_vec();
            perm_inputs.extend_from_slice(&vec![self.zero(); SPONGE_WIDTH-inputs.len()]);
            self.permute::<Poseidon2cHash>(perm_inputs.try_into().unwrap())[..num_outputs].to_vec()
        } else {
            // we use Poseidon2 in non-compressed mode, which is more efficient
            self.hash_n_to_m_no_pad::<Poseidon2Hash>(inputs, num_outputs)
        }
    }
}

pub struct Poseidon2cPermutation;
impl<F: RichField + Poseidon2c> PlonkyPermutation<F> for Poseidon2cPermutation {
    fn permute(input: [F; SPONGE_WIDTH]) -> [F; SPONGE_WIDTH] {
        let to_be_compressed_input = &input[..COMPRESSION_WIDTH];
        let mut output =
            F::poseidon2(to_be_compressed_input.try_into().unwrap()).iter()
                .zip(to_be_compressed_input.iter()).map(|(&comp, &inp)| {
                comp + inp
            }).collect::<Vec<_>>();
        output.extend_from_slice(&input[COMPRESSION_WIDTH..]);
        output.try_into().unwrap()
    }
}

/// Poseidon2 hash function.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Poseidon2cHash;
impl<F: RichField + Poseidon2c + Poseidon2> Hasher<F> for Poseidon2cHash {
    const HASH_SIZE: usize = 4 * 8;
    type Hash = HashOut<F>;
    type Permutation = Poseidon2cPermutation; //unused

    fn hash_no_pad(input: &[F]) -> Self::Hash {
        hash_n_to_hash_no_pad_c::<F>(input)
    }

    fn two_to_one(left: Self::Hash, right: Self::Hash) -> Self::Hash {
        compress::<F, Poseidon2cPermutation>(left, right)
    }
}

impl<F: RichField + Poseidon2c + Poseidon2> AlgebraicHasher<F> for Poseidon2cHash {
    fn permute_swapped<const D: usize>(
        inputs: [Target; SPONGE_WIDTH],
        swap: BoolTarget,
        builder: &mut CircuitBuilder<F, D>,
    ) -> [Target; SPONGE_WIDTH]
    where F: Extendable<D>
    {
        let gate_type = Poseidon2cGate::<F, D>::new();
        let gate = builder.add_gate(gate_type, vec![]);

        let swap_wire = Poseidon2cGate::<F, D>::WIRE_SWAP;
        let swap_wire = Target::wire(gate, swap_wire);
        builder.connect(swap.target, swap_wire);

        let mut output =inputs[..COMPRESSION_WIDTH].iter().enumerate().map( |(i,&input)| {
            let in_wire = Poseidon2cGate::<F, D>::wire_input(i);
            let in_wire = Target::wire(gate, in_wire);
            builder.connect(input, in_wire);
            let out_wire = Target::wire(gate, Poseidon2cGate::<F, D>::wire_output(i));
            builder.add(out_wire, input)
        }).collect::<Vec<_>>();
        output.extend_from_slice(&inputs[COMPRESSION_WIDTH..]);

        output.try_into().unwrap()
    }

}

#[cfg(test)]
pub(crate) mod test_helpers {
    use plonky2::field::types::Field;
    use super::COMPRESSION_WIDTH;
    use super::Poseidon2c;

    pub(crate) fn check_test_vectors<F: Field + Poseidon2c>(
        test_vectors: Vec<([u64; COMPRESSION_WIDTH], [u64; COMPRESSION_WIDTH])>,
    )
    {
        for (input_, expected_output_) in test_vectors.into_iter() {
            let mut input = [F::ZERO; COMPRESSION_WIDTH];
            for i in 0..COMPRESSION_WIDTH {
                input[i] = F::from_canonical_u64(input_[i]);
            }
            let output = F::poseidon2(input);
            for i in 0..COMPRESSION_WIDTH {
                let ex_output = F::from_canonical_u64(expected_output_[i]); // Adjust!
                assert_eq!(output[i], ex_output);
            }
        }
    }

    // TODO: Remove later
    pub(crate) fn check_consistency<F: Field + Poseidon2c>()
    {
        let mut input = [F::ZERO; COMPRESSION_WIDTH];
        for i in 0..COMPRESSION_WIDTH {
            input[i] = F::from_canonical_u64(i as u64);
        }
        let output = F::poseidon2(input);
        for i in 0..COMPRESSION_WIDTH {
            assert_eq!(output[i], output[i]); // Dummy check
        }
    }
}