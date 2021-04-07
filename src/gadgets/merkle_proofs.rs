use crate::circuit_builder::CircuitBuilder;
use crate::field::field::Field;
use crate::target::Target;

pub struct MerkleProof<F: Field> {
    /// The Merkle digest of each sibling subtree, staying from the bottommost layer.
    pub siblings: Vec<F>,
}

pub struct MerkleProofTarget {
    /// The Merkle digest of each sibling subtree, staying from the bottommost layer.
    pub siblings: Vec<Target>,
}

pub(crate) fn verify_merkle_proof<F: Field>(
    leaf_index: usize,
    leaf_data: Vec<F>,
    proof: MerkleProof<F>,
) {
    todo!()
}

impl<F: Field> CircuitBuilder<F> {
    pub(crate) fn verify_merkle_proof(
        &mut self,
        leaf_index: Target,
        leaf_data: Vec<Target>,
        proof: MerkleProofTarget,
    ) {
        todo!()
    }
}
