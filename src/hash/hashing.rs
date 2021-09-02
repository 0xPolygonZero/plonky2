//! Concrete instantiation of a hash function.

use crate::field::extension_field::Extendable;
use crate::field::field_types::{Field, RichField};
use crate::hash::gmimc::GMiMCInterface;
use crate::hash::hash_types::{HashOut, HashOutTarget};
use crate::iop::target::Target;
use crate::plonk::circuit_builder::CircuitBuilder;

pub(crate) const SPONGE_RATE: usize = 8;
pub(crate) const SPONGE_CAPACITY: usize = 4;
pub(crate) const SPONGE_WIDTH: usize = SPONGE_RATE + SPONGE_CAPACITY;

pub const NUM_ROUNDS: usize = 101;

/// This is the result of `gmimc_automatic_constants`; i.e. it's from ChaCha20 seeded with 0.
#[rustfmt::skip]
pub const ROUND_CONSTANTS: [u64; NUM_ROUNDS] = [
    0xb585f767417ee042, 0x7746a55f77c10331, 0xb2fb0d321d356f7a, 0x0f6760a486f1621f,
    0xe10d6666b36abcdf, 0x8cae14cb455cc50b, 0xd438539cf2cee334, 0xef781c7d4c1fd8b4,
    0xcdc4a23a0aca4b1f, 0x277fa208d07b52e3, 0xe17653a300493d38, 0xc54302f27c287dc1,
    0x8628782231d47d10, 0x59cd1a8a690b49f2, 0xc3b919ad9efec0b0, 0xa484c4c637641d97,
    0x308bbd23f191398b, 0x6e4a40c1bf713cf1, 0x9a2eedb7510414fb, 0xe360c6e111c2c63b,
    0xd5c771901d4d89aa, 0xc35eae076e7d6b2f, 0x849c2656d0a09cad, 0xc0572c8c5cf1df2b,
    0xe9fa634a883b8bf3, 0xf56f6d4900fb1fdd, 0xf7d713e872a72a1b, 0x8297132b6ba47612,
    0xad6805e12ee8af1c, 0xac51d9f6485c22b9, 0x502ad7dc3bd56bf8, 0x57a1550c3761c577,
    0x66bbd30e99d311da, 0x0da2abef5e948f87, 0xf0612750443f8e94, 0x28b8ec3afb937d8c,
    0x92a756e6be54ca18, 0x70e741ec304e925d, 0x019d5ee2b037c59f, 0x6f6f2ed7a30707d1,
    0x7cf416d01e8c169c, 0x61df517bb17617df, 0x85dc499b4c67dbaa, 0x4b959b48dad27b23,
    0xe8be3e5e0dd779a0, 0xf5c0bc1e525ed8e6, 0x40b12cbf263cf853, 0xa637093f13e2ea3c,
    0x3cc3f89232e3b0c8, 0x2e479dc16bfe86c0, 0x6f49de07d6d39469, 0x213ce7beecc232de,
    0x5b043134851fc00a, 0xa2de45784a861506, 0x7103aaf97bed8dd5, 0x5326fc0dbb88a147,
    0xa9ceb750364cb77a, 0x27f8ec88cc9e991f, 0xfceb4fda8c93fb83, 0xfac6ff13b45b260e,
    0x7131aa455813380b, 0x93510360d5d68119, 0xad535b24fb96e3db, 0x4627f5c6b7efc045,
    0x645cf794e4da78a9, 0x241c70ed1ac2877f, 0xacb8e076b009e825, 0x3737e9db6477bd9d,
    0xe7ea5e344cd688ed, 0x90dee4a009214640, 0xd1b1edf7c77e74af, 0x0b65481bab42158e,
    0x99ad1aab4b4fe3e7, 0x438a7c91f1a360cd, 0xb60de3bd159088bf, 0xc99cab6b47a3e3bb,
    0x69a5ed92d5677cef, 0x5e7b329c482a9396, 0x5fc0ac0829f893c9, 0x32db82924fb757ea,
    0x0ade699c5cf24145, 0x7cc5583b46d7b5bb, 0x85df9ed31bf8abcb, 0x6604df501ad4de64,
    0xeb84f60941611aec, 0xda60883523989bd4, 0x8f97fe40bf3470bf, 0xa93f485ce0ff2b32,
    0x6704e8eebc2afb4b, 0xcee3e9ac788ad755, 0x510d0e66062a270d, 0xf6323f48d74634a0,
    0x0b508cdf04990c90, 0xf241708a4ef7ddf9, 0x60e75c28bb368f82, 0xa6217d8c3f0f9989,
    0x7159cd30f5435b53, 0x839b4e8fe97ec79f, 0x0d3f3e5e885db625, 0x8f7d83be1daea54b,
    0x780f22441e8dbc04,
];

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
    let mut inputs = Vec::with_capacity(8);
    inputs.extend(&x.elements);
    inputs.extend(&y.elements);
    hash_n_to_hash(inputs, false)
}

/// If `pad` is enabled, the message is padded using the pad10*1 rule. In general this is required
/// for the hash to be secure, but it can safely be disabled in certain cases, like if the input
/// length is fixed.
pub fn hash_n_to_m<F: Field + GMiMCInterface<12>>(
    mut inputs: Vec<F>,
    num_outputs: usize,
    pad: bool,
) -> Vec<F> {
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
        for i in 0..input_chunk.len() {
            state[i] = input_chunk[i];
        }
        state = F::gmimc_permute(state);
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
        state = F::gmimc_permute(state);
    }
}

pub fn hash_n_to_hash<F: RichField>(inputs: Vec<F>, pad: bool) -> HashOut<F> {
    HashOut::from_vec(hash_n_to_m(inputs, 4, pad))
}

pub fn hash_n_to_1<F: RichField>(inputs: Vec<F>, pad: bool) -> F {
    hash_n_to_m(inputs, 1, pad)[0]
}
