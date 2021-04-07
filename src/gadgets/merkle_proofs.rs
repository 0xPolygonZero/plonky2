use crate::circuit_builder::CircuitBuilder;
use crate::field::field::Field;
use crate::proof::{Hash, HashTarget};
use crate::target::Target;

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
) {
    todo!()
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
        todo!()
    }
}
