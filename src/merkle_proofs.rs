use crate::circuit_builder::CircuitBuilder;
use crate::field::field::Field;
use crate::gates::gmimc::GMiMCGate;
use crate::hash::{compress, hash_or_noop};
use crate::hash::{merkle_root_inner, GMIMC_ROUNDS};
use crate::proof::{Hash, HashTarget};
use crate::target::Target;
use crate::util::reverse_index_bits_in_place;
use crate::wire::Wire;
use anyhow::{ensure, Result};

#[derive(Clone, Debug)]
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
    proof: &MerkleProof<F>,
    reverse_bits: bool,
) -> Result<()> {
    let index = if reverse_bits {
        crate::util::reverse_bits(leaf_index, proof.siblings.len())
    } else {
        leaf_index
    };
    let mut current_digest = hash_or_noop(leaf_data);
    for (i, &sibling_digest) in proof.siblings.iter().enumerate() {
        let bit = (index >> i & 1) == 1;
        current_digest = if bit {
            compress(sibling_digest, current_digest)
        } else {
            compress(current_digest, sibling_digest)
        }
    }
    ensure!(current_digest == merkle_root, "Invalid Merkle proof.");

    Ok(())
}

/// Verifies that the given subtree is present at the given index in the Merkle tree with the
/// given root.
pub(crate) fn verify_merkle_proof_subtree<F: Field>(
    mut subtree_leaves_data: Vec<Vec<F>>,
    subtree_index: usize,
    merkle_root: Hash<F>,
    proof: &MerkleProof<F>,
    reverse_bits: bool,
) -> Result<()> {
    let index = if reverse_bits {
        // reverse_index_bits_in_place(&mut subtree_leaves_data);
        crate::util::reverse_bits(subtree_index, proof.siblings.len())
    } else {
        subtree_index
    };
    dbg!(&subtree_leaves_data);
    let mut current_digest = merkle_root_inner(subtree_leaves_data);
    dbg!(current_digest);
    for (i, &sibling_digest) in proof.siblings.iter().enumerate() {
        let bit = (index >> i & 1) == 1;
        current_digest = if bit {
            compress(sibling_digest, current_digest)
        } else {
            compress(current_digest, sibling_digest)
        }
    }
    ensure!(current_digest == merkle_root, "Invalid Merkle proof.");

    Ok(())
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

        let mut state: HashTarget = self.hash_or_noop(leaf_data);
        let mut acc_leaf_index = zero;

        for (bit, sibling) in purported_index_bits.into_iter().zip(proof.siblings) {
            let gate = self
                .add_gate_no_constants(GMiMCGate::<F, GMIMC_ROUNDS>::with_automatic_constants());

            let swap_wire = GMiMCGate::<F, GMIMC_ROUNDS>::WIRE_SWAP;
            let swap_wire = Target::Wire(Wire {
                gate,
                input: swap_wire,
            });
            self.generate_copy(bit, swap_wire);

            let old_acc_wire = GMiMCGate::<F, GMIMC_ROUNDS>::WIRE_INDEX_ACCUMULATOR_OLD;
            let old_acc_wire = Target::Wire(Wire {
                gate,
                input: old_acc_wire,
            });
            self.route(acc_leaf_index, old_acc_wire);

            let new_acc_wire = GMiMCGate::<F, GMIMC_ROUNDS>::WIRE_INDEX_ACCUMULATOR_NEW;
            let new_acc_wire = Target::Wire(Wire {
                gate,
                input: new_acc_wire,
            });
            acc_leaf_index = new_acc_wire;

            let input_wires = (0..12)
                .map(|i| {
                    Target::Wire(Wire {
                        gate,
                        input: GMiMCGate::<F, GMIMC_ROUNDS>::wire_input(i),
                    })
                })
                .collect::<Vec<_>>();

            for i in 0..4 {
                self.route(state.elements[i], input_wires[i]);
                self.route(sibling.elements[i], input_wires[4 + i]);
                self.route(zero, input_wires[8 + i]);
            }

            state = HashTarget::from_vec(
                (0..4)
                    .map(|i| {
                        Target::Wire(Wire {
                            gate,
                            input: GMiMCGate::<F, GMIMC_ROUNDS>::wire_output(i),
                        })
                    })
                    .collect(),
            )
        }

        self.assert_equal(acc_leaf_index, leaf_index);

        self.assert_hashes_equal(state, merkle_root)
    }

    pub(crate) fn assert_hashes_equal(&mut self, x: HashTarget, y: HashTarget) {
        for i in 0..4 {
            self.assert_equal(x.elements[i], y.elements[i]);
        }
    }
}
