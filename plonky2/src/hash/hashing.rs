//! Concrete instantiation of a hash function.

use alloc::vec::Vec;
use core::fmt::Debug;
use std::ops::{Index, IndexMut};

use crate::field::extension::Extendable;
use crate::field::types::Field;
use crate::hash::hash_types::{HashOut, HashOutTarget, RichField, NUM_HASH_OUT_ELTS};
use crate::iop::target::Target;
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::config::AlgebraicHasher;

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    pub fn hash_or_noop<H: AlgebraicHasher<F>>(&mut self, inputs: Vec<Target>) -> HashOutTarget
    where
        [(); H::Permutation::WIDTH]:,
    {
        let zero = self.zero();
        if inputs.len() <= NUM_HASH_OUT_ELTS {
            HashOutTarget::from_partial(&inputs, zero)
        } else {
            self.hash_n_to_hash_no_pad::<H>(inputs)
        }
    }

    pub fn hash_n_to_hash_no_pad<H: AlgebraicHasher<F>>(
        &mut self,
        inputs: Vec<Target>,
    ) -> HashOutTarget
    where
        [(); H::Permutation::WIDTH]:,
    {
        HashOutTarget::from_vec(self.hash_n_to_m_no_pad::<H>(inputs, NUM_HASH_OUT_ELTS))
    }

    pub fn hash_n_to_m_no_pad<H: AlgebraicHasher<F>>(
        &mut self,
        inputs: Vec<Target>,
        num_outputs: usize,
    ) -> Vec<Target>
    where
        [(); H::Permutation::WIDTH]:,
    {
        let zero = self.zero();
        let mut state = [zero; H::Permutation::WIDTH];

        // Absorb all input chunks.
        for input_chunk in inputs.chunks(H::Permutation::RATE) {
            // Overwrite the first r elements with the inputs. This differs from a standard sponge,
            // where we would xor or add in the inputs. This is a well-known variant, though,
            // sometimes called "overwrite mode".
            state[..input_chunk.len()].copy_from_slice(input_chunk);
            state = self.permute::<H>(state);
        }

        // Squeeze until we have the desired number of outputs.
        let mut outputs = Vec::with_capacity(num_outputs);
        loop {
            for i in 0..H::Permutation::RATE {
                outputs.push(state[i]);
                if outputs.len() == num_outputs {
                    return outputs;
                }
            }
            state = self.permute::<H>(state);
        }
    }
}

/// Permutation that can be used in the sponge construction for an algebraic hash.
pub trait PlonkyPermutation<F: Field> {
    const RATE: usize;
    const WIDTH: usize;

    type State: AsMut<[F]>
        + AsRef<[F]>
        + Clone
        + Debug
        + Default
        + Eq
        + Index<usize, Output = F>
        + IndexMut<usize, Output = F>
        + IntoIterator<Item = F>
        + Sync
        + Send;

    fn permute(input: Self::State) -> Self::State;
}

/// A one-way compression function which takes two ~256 bit inputs and returns a ~256 bit output.
pub fn compress<F: Field, P: PlonkyPermutation<F>>(x: HashOut<F>, y: HashOut<F>) -> HashOut<F> {
    // TODO: With some refactoring, this function could be implemented as
    // hash_n_to_m_no_pad(chain(x.elements, y.elements), NUM_HASH_OUT_ELTS).

    let mut perm_inputs = P::State::default();
    let mut_state = perm_inputs.as_mut();
    mut_state.fill(F::ZERO);

    mut_state[..NUM_HASH_OUT_ELTS].copy_from_slice(&x.elements);
    mut_state[NUM_HASH_OUT_ELTS..2 * NUM_HASH_OUT_ELTS].copy_from_slice(&y.elements);

    HashOut {
        elements: P::permute(perm_inputs).as_ref()[..NUM_HASH_OUT_ELTS]
            .try_into()
            .unwrap(),
    }
}

/// Hash a message without any padding step. Note that this can enable length-extension attacks.
/// However, it is still collision-resistant in cases where the input has a fixed length.
pub fn hash_n_to_m_no_pad<F: RichField, P: PlonkyPermutation<F>>(
    inputs: &[F],
    num_outputs: usize,
) -> Vec<F> {
    let mut state = P::State::default();
    state.as_mut().fill(F::ZERO);

    // Absorb all input chunks.
    for input_chunk in inputs.chunks(P::RATE) {
        state.as_mut()[..input_chunk.len()].copy_from_slice(input_chunk);
        state = P::permute(state);
    }

    // Squeeze until we have the desired number of outputs.

    // TODO: Replace loops below with something like this:
    //
    // (0..)
    //   .scan(initial_state, |state, _| Some(P::permute(state)))
    //   .flat_map(|state| state.into_iter().take(HC::RATE))
    //   .take(num_outputs)
    //   .collect()

    let mut outputs = Vec::new();
    loop {
        for &item in state.as_ref().iter().take(P::RATE) {
            outputs.push(item);
            if outputs.len() == num_outputs {
                return outputs;
            }
        }
        state = P::permute(state);
    }
}

pub fn hash_n_to_hash_no_pad<F: RichField, P: PlonkyPermutation<F>>(inputs: &[F]) -> HashOut<F> {
    HashOut::from_vec(hash_n_to_m_no_pad::<F, P>(inputs, NUM_HASH_OUT_ELTS))
}
