use crate::circuit_builder::CircuitBuilder;
use crate::field::field::Field;
use crate::hash::{compress, hash_n_to_hash};
use crate::proof::{Hash, HashTarget};
use crate::target::Target;

#[derive(Clone, Debug)]
pub struct MerkleProof<F: Field> {
    /// The Merkle digest of each sibling subtree, staying from the bottommost layer.
    pub siblings: Vec<Hash<F>>,
}

pub struct MerkleProofTarget {
    /// The Merkle digest of each sibling subtree, staying from the bottommost layer.
    pub siblings: Vec<HashTarget>,
}

#[derive(Clone, Debug)]
pub struct MerkleTree<F: Field> {
    /// The data in the leaves of the Merkle tree.
    pub leaves: Vec<Vec<F>>,

    /// The layers of hashes in the tree. The first layer is the one at the bottom.
    pub layers: Vec<Vec<Hash<F>>>,

    /// The Merkle root.
    pub root: Hash<F>,
}

impl<F: Field> MerkleTree<F> {
    pub fn new(leaves: Vec<Vec<F>>) -> Self {
        let mut layers = vec![leaves.iter().map(|l| hash_n_to_hash(l.clone(), false)).collect::<Vec<_>>()];
        loop {
            match layers.last() {
                Some(l) if l.len() > 1 => {
                    layers.push(l.chunks(2).map(|chunk| compress(chunk[0], chunk[1])).collect::<Vec<_>>());
                },
                _ => break
            }
        }
        let root = layers.pop().unwrap()[0];
        Self {
            leaves,
            layers,
            root
        }

    }

    /// Create a Merkle proof from a leaf index.
    pub fn prove(&self, leaf_index: usize) -> MerkleProof<F> {
        MerkleProof {
            siblings: self.layers
                .iter()
                .scan(leaf_index, |acc, layer| {
                    let index = *acc ^ 1;
                    *acc >>= 1;
                    Some(layer[index])
                })
                .collect(),
        }
    }
}



/// Verifies that the given leaf data is present at the given index in the Merkle tree with the
/// given root.
pub(crate) fn verify_merkle_proof<F: Field>(
    leaf_data: Vec<F>,
    leaf_index: usize,
    merkle_root: Hash<F>,
    proof: MerkleProof<F>,
) -> bool {
    let mut index = leaf_index;
    let mut h = hash_n_to_hash(leaf_data, false);
    for s in &proof.siblings {
        h = if index & 1 == 0 {
            compress(h, *s)
        } else {
            compress(*s, h)
        };
        index >>= 1;
    }
    h == merkle_root
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::crandall_field::CrandallField;

    #[test]
    fn test_merkle_proofs() {
        type F = CrandallField;
        let num_leaves = 128;
        let leaves = (0..num_leaves).map(|_| vec![F::rand()]).collect::<Vec<_>>();
        let tree = MerkleTree::new(leaves);
        for i in 0..num_leaves {
            let proof = tree.prove(i);
            assert!(verify_merkle_proof(tree.leaves[i].clone(),i, tree.root, proof));
        }
    }
}
