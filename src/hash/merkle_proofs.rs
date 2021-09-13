use std::convert::TryInto;

use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};

use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::field::field_types::{Field, RichField};
use crate::gates::gmimc::GMiMCGate;
use crate::hash::hash_types::{HashOut, HashOutTarget, MerkleCapTarget};
use crate::hash::hashing::{compress, hash_or_noop};
use crate::hash::merkle_tree::MerkleCap;
use crate::iop::target::{BoolTarget, Target};
use crate::iop::wire::Wire;
use crate::plonk::circuit_builder::CircuitBuilder;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct MerkleProof<F: Field> {
    /// The Merkle digest of each sibling subtree, staying from the bottommost layer.
    pub siblings: Vec<HashOut<F>>,
}

#[derive(Clone)]
pub struct MerkleProofTarget {
    /// The Merkle digest of each sibling subtree, staying from the bottommost layer.
    pub siblings: Vec<HashOutTarget>,
}

/// Verifies that the given leaf data is present at the given index in the Merkle tree with the
/// given cap.
pub(crate) fn verify_merkle_proof<F: RichField>(
    leaf_data: Vec<F>,
    leaf_index: usize,
    merkle_cap: &MerkleCap<F>,
    proof: &MerkleProof<F>,
) -> Result<()> {
    let mut index = leaf_index;
    let mut current_digest = hash_or_noop(leaf_data);
    for &sibling_digest in proof.siblings.iter() {
        let bit = index & 1;
        index >>= 1;
        current_digest = if bit == 1 {
            compress(sibling_digest, current_digest)
        } else {
            compress(current_digest, sibling_digest)
        }
    }
    ensure!(
        current_digest == merkle_cap.0[index],
        "Invalid Merkle proof."
    );

    Ok(())
}

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    /// Verifies that the given leaf data is present at the given index in the Merkle tree with the
    /// given cap. The index is given by it's little-endian bits.
    /// Note: Works only for D=4.
    pub(crate) fn verify_merkle_proof(
        &mut self,
        leaf_data: Vec<Target>,
        leaf_index_bits: &[BoolTarget],
        merkle_cap: &MerkleCapTarget,
        proof: &MerkleProofTarget,
    ) {
        let zero = self.zero();

        let mut state: HashOutTarget = self.hash_or_noop(leaf_data);

        for (&bit, &sibling) in leaf_index_bits.iter().zip(&proof.siblings) {
            let inputs = [state.elements, sibling.elements, [zero; 4]]
                .concat()
                .try_into()
                .unwrap();
            let outputs = self.gmimc_permute_swapped(inputs, bit);
            state = HashOutTarget::from_vec(outputs[0..4].to_vec());
        }

        let index = self.le_sum(leaf_index_bits[proof.siblings.len()..].to_vec().into_iter());
        let state_ext = state.elements[..].try_into().expect("requires D = 4");
        let state_ext = ExtensionTarget(state_ext);
        let cap_ext = merkle_cap
            .0
            .iter()
            .map(|h| {
                let tmp = h.elements[..].try_into().expect("requires D = 4");
                ExtensionTarget(tmp)
            })
            .collect();
        self.random_access(index, state_ext, cap_ext);
    }

    /// Same a `verify_merkle_proof` but with the final "cap index" as extra parameter.
    /// Note: Works only for D=4.
    pub(crate) fn verify_merkle_proof_with_cap_index(
        &mut self,
        leaf_data: Vec<Target>,
        leaf_index_bits: &[BoolTarget],
        cap_index: Target,
        merkle_cap: &MerkleCapTarget,
        proof: &MerkleProofTarget,
    ) {
        let zero = self.zero();

        let mut state: HashOutTarget = self.hash_or_noop(leaf_data);

        for (&bit, &sibling) in leaf_index_bits.iter().zip(&proof.siblings) {
            let gate_type = GMiMCGate::<F, D, 12>::new();
            let gate = self.add_gate(gate_type, vec![]);

            let swap_wire = GMiMCGate::<F, D, 12>::WIRE_SWAP;
            let swap_wire = Target::Wire(Wire {
                gate,
                input: swap_wire,
            });
            self.generate_copy(bit.target, swap_wire);

            let input_wires = (0..12)
                .map(|i| {
                    Target::Wire(Wire {
                        gate,
                        input: GMiMCGate::<F, D, 12>::wire_input(i),
                    })
                })
                .collect::<Vec<_>>();

            for i in 0..4 {
                self.connect(state.elements[i], input_wires[i]);
                self.connect(sibling.elements[i], input_wires[4 + i]);
                self.connect(zero, input_wires[8 + i]);
            }

            state = HashOutTarget::from_vec(
                (0..4)
                    .map(|i| {
                        Target::Wire(Wire {
                            gate,
                            input: GMiMCGate::<F, D, 12>::wire_output(i),
                        })
                    })
                    .collect(),
            )
        }

        let state_ext = state.elements[..].try_into().expect("requires D = 4");
        let state_ext = ExtensionTarget(state_ext);
        let cap_ext = merkle_cap
            .0
            .iter()
            .map(|h| {
                let tmp = h.elements[..].try_into().expect("requires D = 4");
                ExtensionTarget(tmp)
            })
            .collect();
        self.random_access(cap_index, state_ext, cap_ext);
    }

    pub fn assert_hashes_equal(&mut self, x: HashOutTarget, y: HashOutTarget) {
        for i in 0..4 {
            self.connect(x.elements[i], y.elements[i]);
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use rand::{thread_rng, Rng};

    use super::*;
    use crate::field::crandall_field::CrandallField;
    use crate::hash::merkle_tree::MerkleTree;
    use crate::iop::witness::{PartialWitness, Witness};
    use crate::plonk::circuit_builder::CircuitBuilder;
    use crate::plonk::circuit_data::CircuitConfig;
    use crate::plonk::verifier::verify;

    fn random_data<F: Field>(n: usize, k: usize) -> Vec<Vec<F>> {
        (0..n).map(|_| F::rand_vec(k)).collect()
    }

    #[test]
    fn test_recursive_merkle_proof() -> Result<()> {
        type F = CrandallField;
        let config = CircuitConfig::large_config();
        let mut pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, 4>::new(config);

        let log_n = 8;
        let n = 1 << log_n;
        let cap_height = 1;
        let leaves = random_data::<F>(n, 7);
        let tree = MerkleTree::new(leaves, cap_height);
        let i: usize = thread_rng().gen_range(0..n);
        let proof = tree.prove(i);

        let proof_t = MerkleProofTarget {
            siblings: builder.add_virtual_hashes(proof.siblings.len()),
        };
        for i in 0..proof.siblings.len() {
            pw.set_hash_target(proof_t.siblings[i], proof.siblings[i]);
        }

        let cap_t = builder.add_virtual_cap(cap_height);
        pw.set_cap_target(&cap_t, &tree.cap);

        let i_c = builder.constant(F::from_canonical_usize(i));
        let i_bits = builder.split_le(i_c, log_n);

        let data = builder.add_virtual_targets(tree.leaves[i].len());
        for j in 0..data.len() {
            pw.set_target(data[j], tree.leaves[i][j]);
        }

        builder.verify_merkle_proof(data, &i_bits, &cap_t, &proof_t);

        let data = builder.build();
        let proof = data.prove(pw)?;

        verify(proof, &data.verifier_only, &data.common)
    }
}
