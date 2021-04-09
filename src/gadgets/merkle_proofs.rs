use std::convert::TryInto;

use crate::circuit_builder::CircuitBuilder;
use crate::field::field::Field;
use crate::gates::gmimc::GMiMCGate;
use crate::hash::{compress, hash_or_noop};
use crate::hash::GMIMC_ROUNDS;
use crate::proof::{Hash, HashTarget};
use crate::target::Target;
use crate::wire::Wire;

pub struct MerkleProof<F: Field> {
    /// The Merkle digest of each sibling subtree, staying from the bottommost layer.
    pub siblings: Vec<Hash<F>>,
}

pub struct MerkleProofTarget {
    /// The Merkle digest of each sibling subtree, staying from the bottommost layer.
    pub siblings: Vec<HashTarget>,
}

/// Verifies that the given leaf data is present at the given index in the Merkle tree with the
/// given root.
pub(crate) fn verify_merkle_proof<F: Field>(
    leaf_data: Vec<F>,
    leaf_index: usize,
    merkle_root: Hash<F>,
    proof: MerkleProof<F>,
) -> bool {
    let mut current_digest = hash_or_noop(leaf_data);
    for (i, sibling_digest) in proof.siblings.into_iter().enumerate() {
        let bit = (leaf_index >> i & 1) == 1;
        current_digest = if bit {
            compress(sibling_digest, current_digest)
        } else {
            compress(current_digest, sibling_digest)
        }
    }
    current_digest == merkle_root
}

impl<F: Field> CircuitBuilder<F> {
    /// Verifies that the given leaf data is present at the given index in the Merkle tree with the
    /// given root.
    pub(crate) fn verify_merkle_proof(
        &mut self,
        leaf_data: Vec<Target>,
        leaf_index: Target,
        merkle_root: HashTarget,
        proof: MerkleProofTarget,
    ) {
        let zero = self.zero();
        let height = proof.siblings.len();
        let purported_index_bits = self.split_le_virtual(leaf_index, height);

        let mut state: Vec<Target> = todo!(); // hash leaf data

        for (bit, sibling) in purported_index_bits.into_iter().zip(proof.siblings) {
            let gate = self.add_gate_no_constants(
                GMiMCGate::<F, GMIMC_ROUNDS>::with_automatic_constants());

            let swap_wire = GMiMCGate::<F, GMIMC_ROUNDS>::WIRE_SWAP;
            let swap_wire = Target::Wire(Wire { gate, input: swap_wire });
            self.generate_copy(bit, swap_wire);

            let input_wires = (0..12)
                .map(|i| Target::Wire(
                    Wire { gate, input: GMiMCGate::<F, GMIMC_ROUNDS>::wire_input(i) }))
                .collect::<Vec<_>>();

            for i in 0..4 {
                self.route(state[i], input_wires[i]);
                self.route(sibling.elements[i], input_wires[4 + i]);
                self.route(zero, input_wires[8 + i]);
            }

            state = (0..4)
                .map(|i| Target::Wire(
                    Wire { gate, input: GMiMCGate::<F, GMIMC_ROUNDS>::wire_output(i) }))
                .collect::<Vec<_>>()
                .try_into()
                .unwrap();
        }

        // TODO: Verify that weighted sum of bits matches index.
        // TODO: Verify that state matches merkle root.
    }
}
