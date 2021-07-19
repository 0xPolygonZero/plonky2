//! Concrete instantiation of a hash function.

use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::Extendable;
use crate::field::field::Field;
use crate::gmimc::gmimc_permute_array;
use crate::proof::{Hash, HashTarget};
use crate::target::Target;

pub(crate) const SPONGE_RATE: usize = 8;
pub(crate) const SPONGE_CAPACITY: usize = 4;
pub(crate) const SPONGE_WIDTH: usize = SPONGE_RATE + SPONGE_CAPACITY;

pub const GMIMC_ROUNDS: usize = 101;
/// This is the result of `gmimc_automatic_constants`; i.e. it's from ChaCha20 seeded with 0.
pub const GMIMC_CONSTANTS: [u64; GMIMC_ROUNDS] = [
    13080132715619999810,
    8594738768332784433,
    12896916466795114362,
    1109962092924985887,
    16216730424513838303,
    10137062674532189451,
    15292064468290167604,
    17255573296743700660,
    14827154243383347999,
    2846171648262623971,
    16246264665335217464,
    14214208089399786945,
    9667108688411000080,
    6470857421371427314,
    14103331941574951088,
    11854816474757864855,
    3498097497657653643,
    7947235693333396721,
    11110078702363612411,
    16384314114341783099,
    15404405914224921002,
    14077880832148466479,
    9555554663682579629,
    13859595359622389547,
    16859897326779206643,
    17685474422023725021,
    17858764736437889563,
    9410011023624402450,
    12495243630852222748,
    12416945299436348089,
    5776666812952701944,
    6314421663507268983,
    7402742472177291738,
    982536713292517255,
    17321168867539521172,
    2934354895304883596,
    10567510599683852824,
    8135543734546633309,
    116353493093565855,
    8029688164312877009,
    9003846638141970076,
    7052445133185619935,
    9645665433271393194,
    5446430061585660707,
    16770910636054378912,
    17708360573237778662,
    4661556288797079635,
    11977051900536351292,
    4378616569536950472,
    3334807503157233344,
    8019184736760206441,
    2395043909056213726,
    6558421058999795722,
    11735894061922784518,
    8143540539718733269,
    5991753490174091591,
    12235918792748480378,
    2880312033996085535,
    18224748117164817283,
    18070411014966027790,
    8156487614951798795,
    10615269511128318233,
    12489426406026437595,
    5055279340584943685,
    7231927320516917417,
    2602078848371820415,
    12445944370602567717,
    3978905924297801117,
    16711272946032085229,
    10439032362290464320,
    15110119873264383151,
    821141790739535246,
    11073536381779174375,
    4866839313593360589,
    13118391690850240703,
    14527674975242150843,
    7612751960041028847,
    6808090908507673494,
    6899703780195472329,
    3664666286710282218,
    783179505504239941,
    8990689242729919931,
    9646603556395461579,
    7351246026916028004,
    16970959815450893036,
    15735726859844361172,
    10347018222946250943,
    12195545879691602738,
    7423314197870213963,
    14908016118492485461,
    5840340123122280205,
    17740311464247702688,
    815306422036794512,
    17456357369997417977,
    6982651077270605698,
    11970987325834369417,
    8167785009370061651,
    9483259820363401119,
    954550221761525285,
    10339565172077536587,
    8651171085167737860,
];

/// Hash the vector if necessary to reduce its length to ~256 bits. If it already fits, this is a
/// no-op.
pub fn hash_or_noop<F: Field>(inputs: Vec<F>) -> Hash<F> {
    if inputs.len() <= 4 {
        Hash::from_partial(inputs)
    } else {
        hash_n_to_hash(inputs, false)
    }
}

impl<F: Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    pub fn hash_or_noop(&mut self, inputs: Vec<Target>) -> HashTarget {
        let zero = self.zero();
        if inputs.len() <= 4 {
            HashTarget::from_partial(inputs, zero)
        } else {
            self.hash_n_to_hash(inputs, false)
        }
    }

    pub fn hash_n_to_hash(&mut self, inputs: Vec<Target>, pad: bool) -> HashTarget {
        HashTarget::from_vec(self.hash_n_to_m(inputs, 4, pad))
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
pub fn compress<F: Field>(x: Hash<F>, y: Hash<F>) -> Hash<F> {
    let mut inputs = Vec::with_capacity(8);
    inputs.extend(&x.elements);
    inputs.extend(&y.elements);
    hash_n_to_hash(inputs, false)
}

pub fn permute<F: Field>(xs: [F; SPONGE_WIDTH]) -> [F; SPONGE_WIDTH] {
    gmimc_permute_array(xs, GMIMC_CONSTANTS)
}

/// If `pad` is enabled, the message is padded using the pad10*1 rule. In general this is required
/// for the hash to be secure, but it can safely be disabled in certain cases, like if the input
/// length is fixed.
pub fn hash_n_to_m<F: Field>(mut inputs: Vec<F>, num_outputs: usize, pad: bool) -> Vec<F> {
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

pub fn hash_n_to_hash<F: Field>(inputs: Vec<F>, pad: bool) -> Hash<F> {
    Hash::from_vec(hash_n_to_m(inputs, 4, pad))
}

pub fn hash_n_to_1<F: Field>(inputs: Vec<F>, pad: bool) -> F {
    hash_n_to_m(inputs, 1, pad)[0]
}
