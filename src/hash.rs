//! Concrete instantiation of a hash function.

use std::convert::TryInto;

use num::traits::real::Real;
use rayon::prelude::*;

use crate::field::field::Field;
use crate::gmimc::{gmimc_compress, gmimc_permute_array};
use crate::proof::Hash;
use crate::util::{reverse_index_bits, transpose, reverse_index_bits_in_place};

const RATE: usize = 8;
const CAPACITY: usize = 4;
const WIDTH: usize = RATE + CAPACITY;

const GMIMC_ROUNDS: usize = 101;
const GMIMC_CONSTANTS: [u64; GMIMC_ROUNDS] = [11875528958976719239, 6107683892976199900, 7756999550758271958, 14819109722912164804, 9716579428412441110, 13627117528901194436, 16260683900833506663, 5942251937084147420, 3340009544523273897, 5103423085715007461, 17051583366444092101, 11122892258227244197, 16564300648907092407, 978667924592675864, 17676416205210517593, 1938246372790494499, 8857737698008340728, 1616088456497468086, 15961521580811621978, 17427220057097673602, 14693961562064090188, 694121596646283736, 554241305747273747, 5783347729647881086, 14933083198980931734, 2600898787591841337, 9178797321043036456, 18068112389665928586, 14493389459750307626, 1650694762687203587, 12538946551586403559, 10144328970401184255, 4215161528137084719, 17559540991336287827, 1632269449854444901, 986434918028205468, 14921385763379308253, 4345141219277982730, 2645897826751167170, 9815223670029373528, 7687983869685434132, 13956100321958014639, 519639453142393369, 15617837024229225911, 1557446238053329052, 8130006133842942201, 864716631341688017, 2860289738131495304, 16723700803638270299, 8363528906277648001, 13196016034228493087, 2514677332206134618, 15626342185220554936, 466271571343554681, 17490024028988898434, 6454235936129380878, 15187752952940298536, 18043495619660620405, 17118101079533798167, 13420382916440963101, 535472393366793763, 1071152303676936161, 6351382326603870931, 12029593435043638097, 9983185196487342247, 414304527840226604, 1578977347398530191, 13594880016528059526, 13219707576179925776, 6596253305527634647, 17708788597914990288, 7005038999589109658, 10171979740390484633, 1791376803510914239, 2405996319967739434, 12383033218117026776, 17648019043455213923, 6600216741450137683, 5359884112225925883, 1501497388400572107, 11860887439428904719, 64080876483307031, 11909038931518362287, 14166132102057826906, 14172584203466994499, 593515702472765471, 3423583343794830614, 10041710997716717966, 13434212189787960052, 9943803922749087030, 3216887087479209126, 17385898166602921353, 617799950397934255, 9245115057096506938, 13290383521064450731, 10193883853810413351, 14648839921475785656, 14635698366607946133, 9134302981480720532, 10045888297267997632, 10752096344939765738];

/// Hash the vector if necessary to reduce its length to ~256 bits. If it already fits, this is a
/// no-op.
pub fn hash_or_noop<F: Field>(mut inputs: Vec<F>) -> Hash<F> {
    if inputs.len() <= 4 {
        Hash::from_partial(inputs)
    } else {
        hash_n_to_hash(inputs, false)
    }
}

/// A one-way compression function which takes two ~256 bit inputs and returns a ~256 bit output.
pub fn compress<F: Field>(x: Hash<F>, y: Hash<F>) -> Hash<F> {
    let mut inputs = Vec::with_capacity(8);
    inputs.extend(&x.elements);
    inputs.extend(&y.elements);
    hash_n_to_hash(inputs, false)
}

/// If `pad` is enabled, the message is padded using the pad10*1 rule. In general this is required
/// for the hash to be secure, but it can safely be disabled in certain cases, like if the input
/// length is fixed.
pub fn hash_n_to_m<F: Field>(mut inputs: Vec<F>, num_outputs: usize, pad: bool) -> Vec<F> {
    if pad {
        inputs.push(F::ZERO);
        while (inputs.len() + 1) % WIDTH != 0 {
            inputs.push(F::ONE);
        }
        inputs.push(F::ZERO);
    }

    let mut state = [F::ZERO; WIDTH];

    // Absorb all input chunks.
    for input_chunk in inputs.chunks(WIDTH - 1) {
        for i in 0..input_chunk.len() {
            state[i] = state[i] + input_chunk[i];
        }
        state = gmimc_permute_array(state, GMIMC_CONSTANTS);
    }

    // Squeeze until we have the desired number of outputs.
    let mut outputs = Vec::new();
    loop {
        for i in 0..(WIDTH - 1) {
            outputs.push(state[i]);
            if outputs.len() == num_outputs {
                return outputs;
            }
        }
        state = gmimc_permute_array(state, GMIMC_CONSTANTS);
    }
}

pub fn hash_n_to_hash<F: Field>(inputs: Vec<F>, pad: bool) -> Hash<F> {
    let elements = hash_n_to_m(inputs, 4, pad).try_into().unwrap();
    Hash { elements }
}

pub fn hash_n_to_1<F: Field>(inputs: Vec<F>, pad: bool) -> F {
    hash_n_to_m(inputs, 1, pad)[0]
}

/// Like `merkle_root`, but first reorders each vector so that `new[i] = old[i.reverse_bits()]`.
pub(crate) fn merkle_root_bit_rev_order<F: Field>(mut vecs: Vec<Vec<F>>) -> Hash<F> {
    reverse_index_bits_in_place(&mut vecs);
    merkle_root(vecs)
}

/// Given `n` vectors, each of length `l`, constructs a Merkle tree with `l` leaves, where each leaf
/// is a hash obtained by hashing a "leaf set" consisting of `n` elements. If `n <= 4`, this hashing
/// is skipped, as there is no need to compress leaf data.
pub(crate) fn merkle_root<F: Field>(vecs: Vec<Vec<F>>) -> Hash<F> {
    // TODO: Parallelize.
    let mut hashes = vecs.into_iter()
        .map(|leaf_set| hash_or_noop(leaf_set))
        .collect::<Vec<_>>();
    while hashes.len() > 1 {
        hashes = hashes.chunks(2)
            .map(|pair| compress(pair[0], pair[1]))
            .collect();
    }
    hashes[0]
}
