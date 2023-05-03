//! Concrete instantiation of a hash function.

use alloc::vec::Vec;
use core::fmt::Debug;

use crate::field::extension::Extendable;
use crate::hash::hash_types::{HashOut, HashOutTarget, RichField, NUM_HASH_OUT_ELTS};
use crate::iop::target::Target;
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::config::AlgebraicHasher;

pub trait HashConfig: Clone + Debug + Eq + PartialEq {
    const RATE: usize;
    const WIDTH: usize;
}

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    pub fn hash_or_noop<HC: HashConfig, H: AlgebraicHasher<F, HC>>(
        &mut self,
        inputs: Vec<Target>,
    ) -> HashOutTarget
    where
        [(); HC::WIDTH]:,
    {
        let zero = self.zero();
        if inputs.len() <= NUM_HASH_OUT_ELTS {
            HashOutTarget::from_partial(&inputs, zero)
        } else {
            self.hash_n_to_hash_no_pad::<HC, H>(inputs)
        }
    }

    pub fn hash_n_to_hash_no_pad<HC: HashConfig, H: AlgebraicHasher<F, HC>>(
        &mut self,
        inputs: Vec<Target>,
    ) -> HashOutTarget
    where
        [(); HC::WIDTH]:,
    {
        HashOutTarget::from_vec(self.hash_n_to_m_no_pad::<HC, H>(inputs, NUM_HASH_OUT_ELTS))
    }

    pub fn hash_n_to_m_no_pad<HC: HashConfig, H: AlgebraicHasher<F, HC>>(
        &mut self,
        inputs: Vec<Target>,
        num_outputs: usize,
    ) -> Vec<Target>
    where
        [(); HC::WIDTH]:,
    {
        let zero = self.zero();

        let mut state = [zero; HC::WIDTH];

        // Absorb all input chunks.
        for input_chunk in inputs.chunks(HC::RATE) {
            // Overwrite the first r elements with the inputs. This differs from a standard sponge,
            // where we would xor or add in the inputs. This is a well-known variant, though,
            // sometimes called "overwrite mode".
            state[..input_chunk.len()].copy_from_slice(input_chunk);
            state = self.permute::<HC, H>(state);
        }

        // Squeeze until we have the desired number of outputs.
        let mut outputs = Vec::with_capacity(num_outputs);
        loop {
            for i in 0..HC::RATE {
                outputs.push(state[i]);
                if outputs.len() == num_outputs {
                    return outputs;
                }
            }
            state = self.permute::<HC, H>(state);
        }
    }
}

/// A one-way compression function which takes two ~256 bit inputs and returns a ~256 bit output.
pub fn compress<F: RichField, HC: HashConfig, P: PlonkyPermutation<F, HC>>(
    x: HashOut<F>,
    y: HashOut<F>,
) -> HashOut<F>
where
    [(); HC::WIDTH]:,
{
    let mut perm_inputs = [F::ZERO; HC::WIDTH];
    perm_inputs[..NUM_HASH_OUT_ELTS].copy_from_slice(&x.elements);
    perm_inputs[NUM_HASH_OUT_ELTS..2 * NUM_HASH_OUT_ELTS].copy_from_slice(&y.elements);
    HashOut {
        elements: P::permute(perm_inputs)[..NUM_HASH_OUT_ELTS]
            .try_into()
            .unwrap(),
    }
}

/// Permutation that can be used in the sponge construction for an algebraic hash.
pub trait PlonkyPermutation<F: RichField, HC: HashConfig> {
    fn permute(input: [F; HC::WIDTH]) -> [F; HC::WIDTH]
    where
        [(); HC::WIDTH]:;
}

/// Hash a message without any padding step. Note that this can enable length-extension attacks.
/// However, it is still collision-resistant in cases where the input has a fixed length.
pub fn hash_n_to_m_no_pad<F: RichField, HC: HashConfig, P: PlonkyPermutation<F, HC>>(
    inputs: &[F],
    num_outputs: usize,
) -> Vec<F>
where
    [(); HC::WIDTH]:,
{
    let mut state = [F::ZERO; HC::WIDTH];

    // Absorb all input chunks.
    for input_chunk in inputs.chunks(HC::RATE) {
        state[..input_chunk.len()].copy_from_slice(input_chunk);
        state = P::permute(state);
    }

    // Squeeze until we have the desired number of outputs.
    let mut outputs = Vec::new();
    loop {
        for &item in state.iter().take(HC::RATE) {
            outputs.push(item);
            if outputs.len() == num_outputs {
                return outputs;
            }
        }
        state = P::permute(state);
    }
}

pub fn hash_n_to_hash_no_pad<F: RichField, HC: HashConfig, P: PlonkyPermutation<F, HC>>(
    inputs: &[F],
) -> HashOut<F>
where
    [(); HC::WIDTH]:,
{
    HashOut::from_vec(hash_n_to_m_no_pad::<F, HC, P>(inputs, NUM_HASH_OUT_ELTS))
}
