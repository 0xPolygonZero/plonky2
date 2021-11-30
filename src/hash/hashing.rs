//! Concrete instantiation of a hash function.

use crate::field::extension_field::Extendable;
use crate::field::field_types::RichField;
use crate::hash::hash_types::{HashOut, HashOutTarget};
use crate::iop::target::Target;
use crate::plonk::circuit_builder::CircuitBuilder;

pub(crate) const SPONGE_RATE: usize = 8;
pub(crate) const SPONGE_CAPACITY: usize = 4;
pub const SPONGE_WIDTH: usize = SPONGE_RATE + SPONGE_CAPACITY;

pub(crate) const HASH_FAMILY: HashFamily = HashFamily::Poseidon;

pub(crate) enum HashFamily {
    #[allow(dead_code)]
    GMiMC,
    Poseidon,
}

/// Hash the vector if necessary to reduce its length to ~256 bits. If it already fits, this is a
/// no-op.
pub fn hash_or_noop<F: RichField>(inputs: Vec<F>) -> HashOut<F> {
    if inputs.len() <= 4 {
        HashOut::from_partial(inputs)
    } else {
        hash_n_to_hash(inputs, false)
    }
}

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    pub fn hash_or_noop(&mut self, inputs: Vec<Target>) -> HashOutTarget {
        let zero = self.zero();
        if inputs.len() <= 4 {
            HashOutTarget::from_partial(inputs, zero)
        } else {
            self.hash_n_to_hash(inputs, false)
        }
    }

    pub fn hash_n_to_hash(&mut self, inputs: Vec<Target>, pad: bool) -> HashOutTarget {
        HashOutTarget::from_vec(self.hash_n_to_m(inputs, 4, pad))
    }

    pub fn hash_n_to_m(
        &mut self,
        mut inputs: Vec<Target>,
        num_outputs: usize,
        pad: bool,
    ) -> Vec<Target> {
        let zero = self.zero();
        let one = self.one();

        if pad {
            inputs.push(zero);
            while (inputs.len() + 1) % SPONGE_WIDTH != 0 {
                inputs.push(one);
            }
            inputs.push(zero);
        }

        let mut state = [zero; SPONGE_WIDTH];

        // Absorb all input chunks.
        for input_chunk in inputs.chunks(SPONGE_RATE) {
            // Overwrite the first r elements with the inputs. This differs from a standard sponge,
            // where we would xor or add in the inputs. This is a well-known variant, though,
            // sometimes called "overwrite mode".
            state[..input_chunk.len()].copy_from_slice(input_chunk);
            state = self.permute(state);
        }

        // Squeeze until we have the desired number of outputs.
        let mut outputs = Vec::new();
        loop {
            for i in 0..SPONGE_RATE {
                outputs.push(state[i]);
                if outputs.len() == num_outputs {
                    return outputs;
                }
            }
            state = self.permute(state);
        }
    }
}

/// A one-way compression function which takes two ~256 bit inputs and returns a ~256 bit output.
pub fn compress<F: RichField>(x: HashOut<F>, y: HashOut<F>) -> HashOut<F> {
    let mut perm_inputs = [F::ZERO; SPONGE_WIDTH];
    perm_inputs[..4].copy_from_slice(&x.elements);
    perm_inputs[4..8].copy_from_slice(&y.elements);
    HashOut {
        elements: permute(perm_inputs)[..4].try_into().unwrap(),
    }
}

/// If `pad` is enabled, the message is padded using the pad10*1 rule. In general this is required
/// for the hash to be secure, but it can safely be disabled in certain cases, like if the input
/// length is fixed.
pub fn hash_n_to_m<F: RichField>(mut inputs: Vec<F>, num_outputs: usize, pad: bool) -> Vec<F> {
    if pad {
        inputs.push(F::ZERO);
        while (inputs.len() + 1) % SPONGE_WIDTH != 0 {
            inputs.push(F::ONE);
        }
        inputs.push(F::ZERO);
    }

    let mut state = [F::ZERO; SPONGE_WIDTH];

    // Absorb all input chunks.
    for input_chunk in inputs.chunks(SPONGE_RATE) {
        state[..input_chunk.len()].copy_from_slice(input_chunk);
        state = permute(state);
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
        state = permute(state);
    }
}

pub fn hash_n_to_hash<F: RichField>(inputs: Vec<F>, pad: bool) -> HashOut<F> {
    HashOut::from_vec(hash_n_to_m(inputs, 4, pad))
}

pub fn hash_n_to_1<F: RichField>(inputs: Vec<F>, pad: bool) -> F {
    hash_n_to_m(inputs, 1, pad)[0]
}

pub(crate) fn permute<F: RichField>(inputs: [F; SPONGE_WIDTH]) -> [F; SPONGE_WIDTH] {
    match HASH_FAMILY {
        HashFamily::GMiMC => F::gmimc_permute(inputs),
        HashFamily::Poseidon => F::poseidon(inputs),
    }
}
