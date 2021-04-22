use crate::field::field::Field;
use crate::hash::{compress, hash_or_noop};
use crate::merkle_proofs::MerkleProof;
use crate::proof::Hash;
use crate::util::{log2_strict, reverse_bits, reverse_index_bits_in_place};

#[derive(Clone, Debug)]
pub struct MerkleTree<F: Field> {
    /// The data in the leaves of the Merkle tree.
    pub leaves: Vec<Vec<F>>,

    /// The layers of hashes in the tree. The first layer is the one at the bottom.
    pub layers: Vec<Vec<Hash<F>>>,

    /// The Merkle root.
    pub root: Hash<F>,

    /// If true, the indices are in bit-reversed form, so that the leaf at index `i`
    /// contains the leaf originally at index `reverse_bits(i)`.
    pub reverse_bits: bool,
}

impl<F: Field> MerkleTree<F> {
    pub fn new(mut leaves: Vec<Vec<F>>, reverse_bits: bool) -> Self {
        if reverse_bits {
            reverse_index_bits_in_place(&mut leaves);
        }
        let mut layers = vec![leaves
            .iter()
            .map(|l| hash_or_noop(l.clone()))
            .collect::<Vec<_>>()];
        while let Some(l) = layers.last() {
            if l.len() == 1 {
                break;
            }
            let next_layer = l
                .chunks(2)
                .map(|chunk| compress(chunk[0], chunk[1]))
                .collect::<Vec<_>>();
            layers.push(next_layer);
        }
        let root = layers.pop().unwrap()[0];
        Self {
            leaves,
            layers,
            root,
            reverse_bits,
        }
    }

    pub fn get(&self, i: usize) -> &[F] {
        let n = log2_strict(self.leaves.len());
        &self.leaves[if self.reverse_bits {
            reverse_bits(i, n)
        } else {
            i
        }]
    }

    /// Create a Merkle proof from a leaf index.
    pub fn prove(&self, leaf_index: usize) -> MerkleProof<F> {
        let index = if self.reverse_bits {
            reverse_bits(leaf_index, log2_strict(self.leaves.len()))
        } else {
            leaf_index
        };
        MerkleProof {
            siblings: self
                .layers
                .iter()
                .scan(index, |acc, layer| {
                    let index = *acc ^ 1;
                    *acc >>= 1;
                    Some(layer[index])
                })
                .collect(),
        }
    }

    /// Create a Merkle proof for an entire subtree.
    /// Example:
    /// ```
    ///         G
    ///        / \
    ///       /   \
    ///      /     \
    ///     E       F
    ///    / \     / \
    ///   A   B   C   D
    /// ```
    /// `self.prove_subtree(0, 1)` gives a Merkle proof for the subtree E->(A,B), i.e., the
    /// path (F,).
    pub fn prove_subtree(&self, subtree_index: usize, subtree_height: usize) -> MerkleProof<F> {
        let index = if self.reverse_bits {
            reverse_bits(
                subtree_index,
                log2_strict(self.leaves.len()) - subtree_height,
            )
        } else {
            subtree_index
        };
        MerkleProof {
            siblings: self
                .layers
                .iter()
                .skip(subtree_height)
                .scan(index, |acc, layer| {
                    let index = *acc ^ 1;
                    *acc >>= 1;
                    Some(layer[index])
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::crandall_field::CrandallField;
    use crate::merkle_proofs::{verify_merkle_proof, verify_merkle_proof_subtree};
    use crate::polynomial::division::divide_by_z_h;
    use anyhow::Result;

    fn random_data<F: Field>(n: usize, k: usize) -> Vec<Vec<F>> {
        (0..n)
            .map(|_| (0..k).map(|_| F::rand()).collect())
            .collect()
    }

    #[test]
    fn test_merkle_trees() -> Result<()> {
        type F = CrandallField;

        let log_n = 3;
        let n = 1 << log_n;
        let leaves = random_data::<F>(n, 7);

        let tree = MerkleTree::new(leaves.clone(), false);
        for i in 0..n {
            let proof = tree.prove(i);
            verify_merkle_proof(tree.leaves[i].clone(), i, tree.root, &proof, false)?;
        }

        for height in 0..=log_n {
            for i in 0..(n >> height) {
                let subtree_proof = tree.prove_subtree(i, height);
                verify_merkle_proof_subtree(
                    tree.leaves[i << height..(i + 1) << height].to_vec(),
                    i,
                    tree.root,
                    &subtree_proof,
                    false,
                )?;
            }
        }

        let tree_reversed_bits = MerkleTree::new(leaves.clone(), true);
        for i in 0..n {
            let proof = tree_reversed_bits.prove(i);
            verify_merkle_proof(leaves[i].clone(), i, tree_reversed_bits.root, &proof, true)?;
        }

        let (height, i) = (1, 0);
        dbg!(height, i);
        let subtree_proof = tree_reversed_bits.prove_subtree(i, height);
        dbg!(&tree_reversed_bits, &subtree_proof);
        verify_merkle_proof_subtree(
            (i << height..(i + 1) << height)
                .map(|j| tree_reversed_bits.leaves[j].to_vec())
                .collect(),
            i,
            tree_reversed_bits.root,
            &subtree_proof,
            true,
        )?;
        for height in 1..=log_n {
            for i in 0..(n >> height) {
                dbg!(height, i);
                let subtree_proof = tree_reversed_bits.prove_subtree(i, height);
                verify_merkle_proof_subtree(
                    (i << height..(i + 1) << height)
                        .map(|j| tree_reversed_bits.leaves[j].to_vec())
                        .collect(),
                    i,
                    tree_reversed_bits.root,
                    &subtree_proof,
                    true,
                )?;
            }
        }

        Ok(())
    }
}
