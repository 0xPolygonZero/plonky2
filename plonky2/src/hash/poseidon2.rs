//! Implementation of the Poseidon2 hash function, as described in
//! https://eprint.iacr.org/2023/323.pdf
//!
//! NOTE: This and related work like Poseidon2Gate and benchmarks are based on OlaVM's work
//! at https://github.com/Sin7Y/olavm/blob/main/plonky2/plonky2/src/hash/poseidon2.rs
//!
#[cfg(not(feature = "std"))]
use alloc::vec;
use core::fmt::Debug;

use plonky2_field::extension::{Extendable, FieldExtension};
use plonky2_field::types::{Field, PrimeField64};
use unroll::unroll_for_loops;

use crate::gates::poseidon2::Poseidon2Gate;
use crate::hash::hash_types::{HashOut, RichField};
use crate::hash::hashing::{compress, hash_n_to_hash_no_pad, PlonkyPermutation};
use crate::iop::ext_target::ExtensionTarget;
use crate::iop::target::{BoolTarget, Target};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::config::{AlgebraicHasher, Hasher};

// The number offull rounds and partial rounds is given by the
// calc_round_numbers.py script. They happen to be the same for both
// width 8 and width 12 with s-box x^7.
//
// NB: Changing any of these values will require regenerating all of
// the precomputed constant arrays in this file.

pub const ROUND_F_BEGIN: usize = 4;
pub const ROUND_F_END: usize = 2 * ROUND_F_BEGIN;
pub const ROUND_P: usize = 22;
pub const ROUNDS: usize = ROUND_F_END + ROUND_P;
pub const WIDTH: usize = 12; // we only have width 8 and 12, and 12 is bigger. :)

pub trait Poseidon2: PrimeField64 {
    const MAT_DIAG12_M_1: [u64; WIDTH];
    const RC12: [u64; WIDTH * ROUND_F_END];
    const RC12_MID: [u64; ROUND_P];

    // The more info of poseidon2 refer to the paper: https://eprint.iacr.org/2023/323.pdf
    // Paras:
    //      R_F = 8
    //      R_P = 22
    //      d = 7
    //      x -- input vector
    // P2_output  = External_0(M_E * x) * External_1 * ... * External_{R_F / 2 -1}  -- 4
    //            * Internal_0 * Internal_1 * ... * Internal_{R_P} -- 22
    //            * External_{R_F/2} * ... * External_{R_F - 1} -- 4
    // Preprocess
    //      M_E * x
    // External_i = M_E * ((x_0 + c_0 ^ {i}) ^ 7, (x_1 + c_1 ^ {i}) ^ 7, ..., (x_{t - 1}} + c_{t - 1} ^ {i}) ^ 7)
    // Note:
    //      x_0 + c_0^{i} -- Add roundconstant
    //      _ ^ 7 -- Sbox
    //      M_E * -- Linear layer
    // Internal_I = M_I * ((x_0 + c_0 ^ {i}) ^ 7, x_1, x-2, ..., x_{t - 1})
    #[inline]
    fn poseidon2(input: [Self; WIDTH]) -> [Self; WIDTH] {
        // vector x
        let mut current_state = input;

        // M_E * X
        Self::matmul_external(&mut current_state);

        // External_i, i in {0 - R_F/2 -1}
        for round_ctr in 0..ROUND_F_BEGIN {
            Self::constant_layer(&mut current_state, round_ctr);
            Self::sbox_layer(&mut current_state);
            Self::matmul_external(&mut current_state);
        }

        // Internal_i
        for r in 0..ROUND_P {
            // t_0 = x_0 + c_0^i
            current_state[0] += Self::from_canonical_u64(Self::RC12_MID[r]);
            // t_1 = t_0^7
            current_state[0] = Self::sbox_monomial(current_state[0]);
            // M_I * t_1
            Self::matmul_internal(&mut current_state, &Self::MAT_DIAG12_M_1);
        }

        // External_i, i in {R_F/2 = R/F - 1}
        for round_ctr in ROUND_F_BEGIN..ROUND_F_END {
            Self::constant_layer(&mut current_state, round_ctr);
            Self::sbox_layer(&mut current_state);
            Self::matmul_external(&mut current_state);
        }

        current_state
    }

    #[inline]
    #[unroll_for_loops]
    fn constant_layer(state: &mut [Self; WIDTH], round_ctr: usize) {
        for i in 0..WIDTH {
            let round_constant = Self::RC12[round_ctr + i];
            unsafe {
                state[i] = state[i].add_canonical_u64(round_constant);
            }
        }
    }

    #[inline]
    #[unroll_for_loops]
    fn sbox_layer(state: &mut [Self; WIDTH]) {
        for i in 0..WIDTH {
            state[i] = Self::sbox_monomial(state[i]);
        }
    }

    #[inline(always)]
    fn sbox_monomial<F: FieldExtension<D, BaseField = Self>, const D: usize>(x: F) -> F {
        // x |--> x^7
        let x2 = x.square();
        let x4 = x2.square();
        let x3 = x * x2;
        x3 * x4
    }

    // M_E * x
    // M_E = circ[2*M4, M4,...,M4] * x
    //     = [M4, M4, M4] * x + circ[M4,0,0] * X
    #[inline]
    #[unroll_for_loops]
    fn matmul_external(input: &mut [Self]) {
        // Applying cheap 4x4 MDS matrix to each 4-element part of the state
        Self::matmul_m4(input);

        // Applying second cheap matrix for t > 4
        // Compute store = [M4, M4, M4] * x
        let t4: usize = WIDTH / 4;
        let mut stored = [Self::ZERO; 4];
        for l in 0..4 {
            stored[l] = input[l];
            for j in 1..t4 {
                stored[l] = stored[l].add(input[4 * j + l]);
            }
        }
        // Compute store + circ[M4,0,0] * X
        for i in 0..input.len() {
            input[i] = input[i].add(stored[i % 4]);
        }
    }

    // M_I * x =
    //      [u_0,1,1,1,1,1,1,1,1,1,1,1]
    //      [1,u_1,1,1,1,1,1,1,1,1,1,1]
    //      [1,1,u_2,1,1,1,1,1,1,1,1,1]
    //      [1,1,1,u_3,1,1,1,1,1,1,1,1]
    //      [1,1,1,1,u_4,1,1,1,1,1,1,1]
    //      [1,1,1,1,1,u_5,1,1,1,1,1,1]    * [x_0, x_1,..., x_11]
    //      [1,1,1,1,1,1,u_6,1,1,1,1,1]
    //      [1,1,1,1,1,1,1,u_7,1,1,1,1]
    //      [1,1,1,1,1,1,1,1,u_8,1,1,1]
    //      [1,1,1,1,1,1,1,1,1,u_9,1,1]
    //      [1,1,1,1,1,1,1,1,1,1,u_10,1]
    //      [1,1,1,1,1,1,1,1,1,1,1,u_11]
    // = Sum_i (u_i - 1) * x_i + Sum(x_0 + x_1 +...+ x_11)
    #[inline]
    #[unroll_for_loops]
    fn matmul_internal(input: &mut [Self], mat_internal_diag_m_1: &[u64]) {
        ////Compute input Sum
        let mut state = [0u128; WIDTH];
        let mut sum = 0_u128;
        for r in 0..WIDTH {
            state[r] = input[r].to_noncanonical_u64() as u128;
            sum += state[r];
        }

        // Add sum + diag entry * element to each element
        for i in 0..WIDTH {
            let mat_internal_diag = mat_internal_diag_m_1[i] - 1;
            let mut multi = (mat_internal_diag as u128) * state[i];
            multi += sum;
            input[i] = Self::from_noncanonical_u128(multi);
        }
    }

    #[inline]
    #[unroll_for_loops]
    fn matmul_m4(input: &mut [Self]) {
        let t4 = WIDTH / 4;

        for i in 0..t4 {
            let start_index = i * 4;
            let mut t_0 = input[start_index];

            t_0 = t_0.add(input[start_index + 1]);
            let mut t_1 = input[start_index + 2];

            t_1 = t_1.add(input[start_index + 3]);
            let mut t_2 = t_1;
            // let mut t_2 = input[start_index + 1];
            //t_2 *= Self::from_canonical_u8(2);
            //t_2 = t_2.add(t_2);
            //t_2 = t_2.add(t_1);
            t_2 = t_2.multiply_accumulate(input[start_index + 1], Self::TWO);
            //t_2 += t_1;
            //let mut t_3 = input[start_index + 3];
            let mut t_3 = t_0;
            //t_3 *= Self::from_canonical_u8(2);
            //t_3 += t_0;
            //t_3 = t_3.add(t_3);
            //t_3 = t_3.add(t_0);
            t_3 = t_3.multiply_accumulate(input[start_index + 3], Self::TWO);
            //let mut t_4 = t_1;
            let mut t_4 = t_3;
            //t_4 *= (F::from_canonical_u8(2));
            //t_4 *= (F::from_canonical_u8(2));
            //t_4 *= Self::from_canonical_u8(4);
            //t_4 = t_4.add(t_4);
            //t_4 = t_4.add(t_4);
            //t_4 = t_4.add(t_3);
            t_4 = t_4.multiply_accumulate(t_1, Self::TWO.double());
            //t_4 += t_3;
            //let mut t_5 = t_0;
            let mut t_5 = t_2;
            //t_5 *= (F::from_canonical_u8(2));
            //t_5 *= (F::from_canonical_u8(2));
            //t_5 *= Self::from_canonical_u8(4);
            //t_5 += t_2;
            //t_5 = t_5.add(t_5);
            //t_5 = t_5.add(t_5);
            //t_5 = t_5.add(t_2);
            t_5 = t_5.multiply_accumulate(t_0, Self::TWO.double());

            //let mut t_6 = t_3;
            //t_6 += t_5;
            //t_6 = t_6.add(t_5);
            //let mut t_7 = t_2;
            //t_7 += t_4;
            //t_7 = t_7.add(t_4);

            input[start_index] = t_3.add(t_5);
            input[start_index + 1] = t_5;
            input[start_index + 2] = t_2.add(t_4);
            input[start_index + 3] = t_4;
        }
    }

    // -------------------------------------- field ------------------------------------------
    #[inline]
    fn matmul_external_field<F: FieldExtension<D, BaseField = Self>, const D: usize>(
        input: &mut [F],
    ) {
        // Applying cheap 4x4 MDS matrix to each 4-element part of the state
        Self::matmul_m4_field(input);

        // Applying second cheap matrix for t > 4
        let t4: usize = WIDTH / 4;
        let mut stored = [F::ZERO; 4];
        for l in 0..4 {
            stored[l] = input[l];
            for j in 1..t4 {
                stored[l] += input[4 * j + l];
            }
        }
        for i in 0..input.len() {
            input[i] += stored[i % 4];
        }
    }

    #[inline]
    fn matmul_internal_field<F: FieldExtension<D, BaseField = Self>, const D: usize>(
        input: &mut [F],
        mat_internal_diag_m_1: &[u64],
    ) {
        //let t: usize = WIDTH;

        /*// Compute input sum
        let mut sum = input[0];
        input
            .iter()
            .skip(1)
            .take(t-1)
            .for_each(|el| sum += (*el));
        */
        //Compute input sum
        let mut sum = input[0];
        for i in 1..WIDTH {
            sum += input[i];
        }
        // Add sum + diag entry * element to each element
        for i in 0..WIDTH {
            input[i] *= F::from_canonical_u64(mat_internal_diag_m_1[i] - 1);
            input[i] += sum;
        }
    }

    // M4 * x
    // M4 = [
    //    [5,7,1,3]
    //    [4,6,1,1]
    //    [1,3,5,7]
    //    [1,1,4,6]
    //  ]
    #[inline]
    fn matmul_m4_field<F: FieldExtension<D, BaseField = Self>, const D: usize>(input: &mut [F]) {
        let t4 = WIDTH / 4;
        for i in 0..t4 {
            let start_index = i * 4;
            let mut t_0 = input[start_index];

            t_0 = t_0.add(input[start_index + 1]);
            let mut t_1 = input[start_index + 2];

            t_1 = t_1.add(input[start_index + 3]);
            let mut t_2 = t_1;
            // let mut t_2 = input[start_index + 1];
            //t_2 *= Self::from_canonical_u8(2);
            //t_2 = t_2.add(t_2);
            //t_2 = t_2.add(t_1);
            t_2 = t_2.multiply_accumulate(input[start_index + 1], F::TWO);
            //t_2 += t_1;
            //let mut t_3 = input[start_index + 3];
            let mut t_3 = t_0;
            //t_3 *= Self::from_canonical_u8(2);
            //t_3 += t_0;
            //t_3 = t_3.add(t_3);
            //t_3 = t_3.add(t_0);
            t_3 = t_3.multiply_accumulate(input[start_index + 3], F::TWO);
            //let mut t_4 = t_1;
            let mut t_4 = t_3;
            //t_4 *= (F::from_canonical_u8(2));
            //t_4 *= (F::from_canonical_u8(2));
            //t_4 *= Self::from_canonical_u8(4);
            //t_4 = t_4.add(t_4);
            //t_4 = t_4.add(t_4);
            //t_4 = t_4.add(t_3);
            t_4 = t_4.multiply_accumulate(t_1, F::TWO.double());
            //t_4 += t_3;
            //let mut t_5 = t_0;
            let mut t_5 = t_2;
            //t_5 *= (F::from_canonical_u8(2));
            //t_5 *= (F::from_canonical_u8(2));
            //t_5 *= Self::from_canonical_u8(4);
            //t_5 += t_2;
            //t_5 = t_5.add(t_5);
            //t_5 = t_5.add(t_5);
            //t_5 = t_5.add(t_2);
            t_5 = t_5.multiply_accumulate(t_0, F::TWO.double());

            //let mut t_6 = t_3;
            //t_6 += t_5;
            //t_6 = t_6.add(t_5);
            //let mut t_7 = t_2;
            //t_7 += t_4;
            //t_7 = t_7.add(t_4);

            input[start_index] = t_3.add(t_5);
            input[start_index + 1] = t_5;
            input[start_index + 2] = t_2.add(t_4);
            input[start_index + 3] = t_4;
        }
    }

    fn constant_layer_field<F: FieldExtension<D, BaseField = Self>, const D: usize>(
        state: &mut [F; WIDTH],
        round_ctr: usize,
    ) {
        for i in 0..WIDTH {
            let round_constant = Self::RC12[round_ctr + i];
            state[i] += F::from_canonical_u64(round_constant);
        }
    }

    fn sbox_layer_field<F: FieldExtension<D, BaseField = Self>, const D: usize>(
        state: &mut [F; WIDTH],
    ) {
        for i in 0..WIDTH {
            state[i] = Self::sbox_monomial(state[i]);
        }
    }
    // -------------------------------------- circuit ----------------------------------------
    // matmul_external_circuit
    fn matmul_external_circuit<const D: usize>(
        builder: &mut CircuitBuilder<Self, D>,
        input: &mut [ExtensionTarget<D>; WIDTH],
    ) -> [ExtensionTarget<D>; WIDTH]
    where
        Self: RichField + Extendable<D>,
    {
        let mut result = [builder.zero_extension(); WIDTH];

        Self::matmul_m4_circuit(builder, input);

        result[0] = builder.add_many_extension([input[0], input[0], input[4], input[8]]);
        result[1] = builder.add_many_extension([input[1], input[1], input[5], input[9]]);
        result[2] = builder.add_many_extension([input[2], input[2], input[6], input[10]]);
        result[3] = builder.add_many_extension([input[3], input[3], input[7], input[11]]);

        result[4] = builder.add_many_extension([input[0], input[4], input[4], input[8]]);
        result[5] = builder.add_many_extension([input[1], input[5], input[5], input[9]]);
        result[6] = builder.add_many_extension([input[2], input[6], input[6], input[10]]);
        result[7] = builder.add_many_extension([input[3], input[7], input[7], input[11]]);

        result[8] = builder.add_many_extension([input[0], input[4], input[8], input[8]]);
        result[9] = builder.add_many_extension([input[1], input[5], input[9], input[9]]);
        result[10] = builder.add_many_extension([input[2], input[6], input[10], input[10]]);
        result[11] = builder.add_many_extension([input[3], input[7], input[11], input[11]]);

        result
    }

    // matmul_m4_circuit
    fn matmul_m4_circuit<const D: usize>(
        builder: &mut CircuitBuilder<Self, D>,
        input: &mut [ExtensionTarget<D>; WIDTH],
    ) where
        Self: RichField + Extendable<D>,
    {
        for i in 0..3 {
            let t_0 = builder.mul_const_add_extension(Self::ONE, input[i * 4], input[i * 4 + 1]);
            let t_1 =
                builder.mul_const_add_extension(Self::ONE, input[i * 4 + 2], input[i * 4 + 3]);
            let t_2 = builder.mul_const_add_extension(Self::TWO, input[i * 4 + 1], t_1);
            let t_3 = builder.mul_const_add_extension(Self::TWO, input[i * 4 + 3], t_0);

            let four = Self::TWO + Self::TWO;

            let t_4 = builder.mul_const_add_extension(four, t_1, t_3);
            let t_5 = builder.mul_const_add_extension(four, t_0, t_2);
            let t_6 = builder.mul_const_add_extension(Self::ONE, t_3, t_5);
            let t_7 = builder.mul_const_add_extension(Self::ONE, t_2, t_4);

            input[i * 4] = t_6;
            input[i * 4 + 1] = t_5;
            input[i * 4 + 2] = t_7;
            input[i * 4 + 3] = t_4;
        }
    }

    // add round_constant layer circuit
    fn constant_layer_circuit<const D: usize>(
        builder: &mut CircuitBuilder<Self, D>,
        input: &mut [ExtensionTarget<D>; WIDTH],
        rc_index: usize,
    ) where
        Self: RichField + Extendable<D>,
    {
        for i in 0..WIDTH {
            let round_constant = Self::Extension::from_canonical_u64(Self::RC12[rc_index + i]);
            let round_constant = builder.constant_extension(round_constant);
            input[i] = builder.add_extension(input[i], round_constant);
        }
    }

    // sbox_layer_circuit circuit
    fn sbox_layer_circuit<const D: usize>(
        builder: &mut CircuitBuilder<Self, D>,
        input: &mut [ExtensionTarget<D>; WIDTH],
    ) where
        Self: RichField + Extendable<D>,
    {
        for i in 0..WIDTH {
            input[i] = builder.exp_u64_extension(input[i], 7);
        }
    }

    // add round_constant layer circuit
    fn sbox_monomial_circuit<const D: usize>(
        builder: &mut CircuitBuilder<Self, D>,
        input: ExtensionTarget<D>,
    ) -> ExtensionTarget<D>
    where
        Self: RichField + Extendable<D>,
    {
        builder.exp_u64_extension(input, 7)
    }

    // matmul_internal_circuit
    fn matmul_internal_circuit<const D: usize>(
        builder: &mut CircuitBuilder<Self, D>,
        input: &mut [ExtensionTarget<D>; WIDTH],
    ) where
        Self: RichField + Extendable<D>,
    {
        let sum = builder.add_many_extension([
            input[0], input[1], input[2], input[3], input[4], input[5], input[6], input[7],
            input[8], input[9], input[10], input[11],
        ]);

        for i in 0..WIDTH {
            let round_constant = Self::Extension::from_canonical_u64(Self::MAT_DIAG12_M_1[i] - 1);
            let round_constant = builder.constant_extension(round_constant);

            input[i] = builder.mul_add_extension(round_constant, input[i], sum);
        }
    }
}

#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct Poseidon2Permutation<T> {
    state: [T; WIDTH],
}

impl<T: Eq> Eq for Poseidon2Permutation<T> {}

trait Permuter: Sized {
    fn permute(input: [Self; WIDTH]) -> [Self; WIDTH];
}

impl<F: Poseidon2> Permuter for F {
    fn permute(input: [Self; WIDTH]) -> [Self; WIDTH] {
        <F as Poseidon2>::poseidon2(input)
    }
}

impl Permuter for Target {
    fn permute(_input: [Self; WIDTH]) -> [Self; WIDTH] {
        panic!("Call `permute_swapped()` instead of `permute()`");
    }
}

impl<T> AsRef<[T]> for Poseidon2Permutation<T> {
    fn as_ref(&self) -> &[T] {
        &self.state
    }
}

impl<T: Copy + Debug + Default + Eq + Permuter + Send + Sync> PlonkyPermutation<T>
    for Poseidon2Permutation<T>
{
    const RATE: usize = 8;
    const WIDTH: usize = WIDTH;

    fn new<I: IntoIterator<Item = T>>(elts: I) -> Self {
        let mut perm = Self {
            state: [T::default(); WIDTH],
        };
        perm.set_from_iter(elts, 0);
        perm
    }

    fn set_elt(&mut self, elt: T, idx: usize) {
        self.state[idx] = elt;
    }

    fn set_from_slice(&mut self, elts: &[T], start_idx: usize) {
        let begin = start_idx;
        let end = start_idx + elts.len();
        self.state[begin..end].copy_from_slice(elts);
    }

    fn set_from_iter<I: IntoIterator<Item = T>>(&mut self, elts: I, start_idx: usize) {
        for (s, e) in self.state[start_idx..].iter_mut().zip(elts) {
            *s = e;
        }
    }

    fn permute(&mut self) {
        self.state = T::permute(self.state);
    }

    fn squeeze(&self) -> &[T] {
        &self.state[..Self::RATE]
    }
}

/// Poseidon2 hash function.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Poseidon2Hash;
impl<F: RichField> Hasher<F> for Poseidon2Hash {
    const HASH_SIZE: usize = 4 * 8;
    type Hash = HashOut<F>;
    type Permutation = Poseidon2Permutation<F>;

    fn hash_no_pad(input: &[F]) -> Self::Hash {
        hash_n_to_hash_no_pad::<F, Self::Permutation>(input)
    }

    fn two_to_one(left: Self::Hash, right: Self::Hash) -> Self::Hash {
        compress::<F, Self::Permutation>(left, right)
    }
}

impl<F: RichField> AlgebraicHasher<F> for Poseidon2Hash {
    type AlgebraicPermutation = Poseidon2Permutation<Target>;

    fn permute_swapped<const D: usize>(
        inputs: Self::AlgebraicPermutation,
        swap: BoolTarget,
        builder: &mut CircuitBuilder<F, D>,
    ) -> Self::AlgebraicPermutation
    where
        F: RichField + Extendable<D>,
    {
        let gate_type = Poseidon2Gate::<F, D>::new();
        let gate = builder.add_gate(gate_type, vec![]);

        let swap_wire = Poseidon2Gate::<F, D>::WIRE_SWAP;
        let swap_wire = Target::wire(gate, swap_wire);
        builder.connect(swap.target, swap_wire);

        // Route input wires.
        let inputs = inputs.as_ref();
        for i in 0..WIDTH {
            let in_wire = Poseidon2Gate::<F, D>::wire_input(i);
            let in_wire = Target::wire(gate, in_wire);
            builder.connect(inputs[i], in_wire);
        }

        // Collect output wires.
        Self::AlgebraicPermutation::new(
            (0..WIDTH).map(|i| Target::wire(gate, Poseidon2Gate::<F, D>::wire_output(i))),
        )
    }
}

#[cfg(test)]
pub(crate) mod test_helpers {
    #[cfg(not(feature = "std"))]
    use alloc::vec::Vec;

    use crate::hash::poseidon2::{Poseidon2, WIDTH};

    pub(crate) fn check_test_vectors<F>(test_vectors: Vec<([u64; WIDTH], [u64; WIDTH])>)
    where
        F: Poseidon2,
    {
        for (input_, expected_output_) in test_vectors.into_iter() {
            let mut input = [F::ZERO; WIDTH];
            for i in 0..WIDTH {
                input[i] = F::from_canonical_u64(input_[i]);
            }
            let output = F::poseidon2(input);
            for i in 0..WIDTH {
                let ex_output = F::from_canonical_u64(expected_output_[i]);
                // println!("{:#x}", output[i].to_canonical_u64());
                assert_eq!(output[i], ex_output);
            }
        }
    }
}
