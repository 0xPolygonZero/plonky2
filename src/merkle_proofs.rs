use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};

use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::Extendable;
use crate::field::field::Field;
use crate::gates::gmimc::GMiMCGate;
use crate::hash::GMIMC_ROUNDS;
use crate::hash::{compress, hash_or_noop};
use crate::proof::{Hash, HashTarget};
use crate::target::Target;
use crate::wire::Wire;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct MerkleProof<F: Field> {
    /// The Merkle digest of each sibling subtree, staying from the bottommost layer.
    pub siblings: Vec<Hash<F>>,
}

#[derive(Clone)]
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
    ensure!(
        leaf_index >> proof.siblings.len() == 0,
        "Merkle leaf index is too large."
    );

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

impl<F: Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    /// Verifies that the given leaf data is present at the given index in the Merkle tree with the
    /// given root. The index is given by it's little-endian bits.
    pub(crate) fn verify_merkle_proof(
        &mut self,
        leaf_data: Vec<Target>,
        leaf_index_bits: &[Target],
        merkle_root: HashTarget,
        proof: &MerkleProofTarget,
    ) {
        let zero = self.zero();

        let mut state: HashTarget = self.hash_or_noop(leaf_data);

        for (&bit, &sibling) in leaf_index_bits.iter().zip(&proof.siblings) {
            let gate_type = GMiMCGate::<F, D, GMIMC_ROUNDS>::new_automatic_constants();
            let gate = self.add_gate(gate_type, vec![]);

            let swap_wire = GMiMCGate::<F, D, GMIMC_ROUNDS>::WIRE_SWAP;
            let swap_wire = Target::Wire(Wire {
                gate,
                input: swap_wire,
            });
            self.generate_copy(bit, swap_wire);

            let input_wires = (0..12)
                .map(|i| {
                    Target::Wire(Wire {
                        gate,
                        input: GMiMCGate::<F, D, GMIMC_ROUNDS>::wire_input(i),
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
                            input: GMiMCGate::<F, D, GMIMC_ROUNDS>::wire_output(i),
                        })
                    })
                    .collect(),
            )
        }

        self.named_assert_hashes_equal(state, merkle_root, "check Merkle root".into())
    }

    pub(crate) fn assert_hashes_equal(&mut self, x: HashTarget, y: HashTarget) {
        for i in 0..4 {
            self.assert_equal(x.elements[i], y.elements[i]);
        }
    }

    pub(crate) fn named_assert_hashes_equal(&mut self, x: HashTarget, y: HashTarget, name: String) {
        for i in 0..4 {
            self.named_assert_equal(
                x.elements[i],
                y.elements[i],
                format!("{}: {}-th hash element", name, i),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use rand::{thread_rng, Rng};

    use super::*;
    use crate::circuit_data::CircuitConfig;
    use crate::field::crandall_field::CrandallField;
    use crate::merkle_tree::MerkleTree;
    use crate::verifier::verify;
    use crate::witness::PartialWitness;

    fn random_data<F: Field>(n: usize, k: usize) -> Vec<Vec<F>> {
        (0..n).map(|_| F::rand_vec(k)).collect()
    }

    #[test]
    fn test_recursive_merkle_proof() -> Result<()> {
        type F = CrandallField;
        let config = CircuitConfig::large_config();
        let mut builder = CircuitBuilder::<F, 4>::new(config);
        let mut pw = PartialWitness::new();

        let log_n = 8;
        let n = 1 << log_n;
        let leaves = random_data::<F>(n, 7);
        let tree = MerkleTree::new(leaves, false);
        let i: usize = thread_rng().gen_range(0, n);
        let proof = tree.prove(i);

        let proof_t = MerkleProofTarget {
            siblings: builder.add_virtual_hashes(proof.siblings.len()),
        };
        for i in 0..proof.siblings.len() {
            pw.set_hash_target(proof_t.siblings[i], proof.siblings[i]);
        }

        let root_t = builder.add_virtual_hash();
        pw.set_hash_target(root_t, tree.root);

        let i_c = builder.constant(F::from_canonical_usize(i));
        let i_bits = builder.split_le(i_c, log_n);

        let data = builder.add_virtual_targets(tree.leaves[i].len());
        for j in 0..data.len() {
            pw.set_target(data[j], tree.leaves[i][j]);
        }

        builder.verify_merkle_proof(data, &i_bits, root_t, &proof_t);

        let data = builder.build();
        let proof = data.prove(pw)?;

        verify(proof, &data.verifier_only, &data.common)
    }
}
