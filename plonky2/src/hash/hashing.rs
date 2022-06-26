//! Concrete instantiation of a hash function.

use plonky2_field::extension::Extendable;

use crate::hash::hash_types::RichField;
use crate::hash::hash_types::{HashOut, HashOutTarget};
use crate::iop::target::Target;
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::config::AlgebraicHasher;

pub(crate) const SPONGE_RATE: usize = 8;
pub(crate) const SPONGE_CAPACITY: usize = 4;
pub const SPONGE_WIDTH: usize = SPONGE_RATE + SPONGE_CAPACITY;

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    pub fn hash_or_noop<H: AlgebraicHasher<F>>(&mut self, inputs: Vec<Target>) -> HashOutTarget {
        let zero = self.zero();
        if inputs.len() <= 4 {
            HashOutTarget::from_partial(&inputs, zero)
        } else {
            self.hash_n_to_hash_no_pad::<H>(inputs)
        }
    }

    pub fn hash_n_to_hash_no_pad<H: AlgebraicHasher<F>>(
        &mut self,
        inputs: Vec<Target>,
    ) -> HashOutTarget {
        HashOutTarget::from_vec(self.hash_n_to_m_no_pad::<H>(inputs, 4))
    }

    pub fn hash_n_to_m_no_pad<H: AlgebraicHasher<F>>(
        &mut self,
        inputs: Vec<Target>,
        num_outputs: usize,
    ) -> Vec<Target> {
        let zero = self.zero();

        let mut state = [zero; SPONGE_WIDTH];

        // Absorb all input chunks.
        for input_chunk in inputs.chunks(SPONGE_RATE) {
            // Overwrite the first r elements with the inputs. This differs from a standard sponge,
            // where we would xor or add in the inputs. This is a well-known variant, though,
            // sometimes called "overwrite mode".
            state[..input_chunk.len()].copy_from_slice(input_chunk);
            state = self.permute::<H>(state);
        }

        // Squeeze until we have the desired number of outputs.
        let mut outputs = Vec::with_capacity(num_outputs);
        loop {
            for i in 0..SPONGE_RATE {
                outputs.push(state[i]);
                if outputs.len() == num_outputs {
                    return outputs;
                }
            }
            state = self.permute::<H>(state);
        }
    }
}

/// A one-way compression function which takes two ~256 bit inputs and returns a ~256 bit output.
pub fn compress<F: RichField, P: PlonkyPermutation<F>>(x: HashOut<F>, y: HashOut<F>) -> HashOut<F> {
    let mut perm_inputs = [F::ZERO; SPONGE_WIDTH];
    perm_inputs[..4].copy_from_slice(&x.elements);
    perm_inputs[4..8].copy_from_slice(&y.elements);
    HashOut {
        elements: P::permute(perm_inputs)[..4].try_into().unwrap(),
    }
}

/// Permutation that can be used in the sponge construction for an algebraic hash.
pub trait PlonkyPermutation<F: RichField> {
    fn permute(input: [F; SPONGE_WIDTH]) -> [F; SPONGE_WIDTH];
}

/// Hash a message without any padding step. Note that this can enable length-extension attacks.
/// However, it is still collision-resistant in cases where the input has a fixed length.
pub fn hash_n_to_m_no_pad<F: RichField, P: PlonkyPermutation<F>>(
    inputs: &[F],
    num_outputs: usize,
) -> Vec<F> {
    let mut state = [F::ZERO; SPONGE_WIDTH];

    // Absorb all input chunks.
    for input_chunk in inputs.chunks(SPONGE_RATE) {
        state[..input_chunk.len()].copy_from_slice(input_chunk);
        state = P::permute(state);
    }

    // Squeeze until we have the desired number of outputs.
    let mut outputs = Vec::new();
    loop {
        for &item in state.iter().take(SPONGE_RATE) {
            outputs.push(item);
            if outputs.len() == num_outputs {
                return outputs;
            }
        }
        state = P::permute(state);
    }
}

pub fn hash_n_to_hash_no_pad<F: RichField, P: PlonkyPermutation<F>>(inputs: &[F]) -> HashOut<F> {
    HashOut::from_vec(hash_n_to_m_no_pad::<F, P>(inputs, 4))
}
