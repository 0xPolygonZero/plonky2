use crate::field::field::Field;
use crate::hash::{compress, hash_n_to_hash, hash_or_noop};
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
            layers.push(
                l.chunks(2)
                    .map(|chunk| compress(chunk[0], chunk[1]))
                    .collect::<Vec<_>>(),
            );
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::crandall_field::CrandallField;
    use crate::merkle_proofs::verify_merkle_proof;
    use crate::polynomial::division::divide_by_z_h;
    use anyhow::Result;

    #[test]
    fn test_merkle_trees() -> Result<()> {
        type F = CrandallField;

        let n = 1 << 10;
        let leaves: Vec<Vec<F>> = (0..n)
            .map(|_| (0..10).map(|_| F::rand()).collect())
            .collect();

        let tree = MerkleTree::new(leaves.clone(), false);
        for i in 0..n {
            let proof = tree.prove(i);
            verify_merkle_proof(tree.leaves[i].clone(), i, tree.root, &proof, false)?;
        }

        let tree_reversed_bits = MerkleTree::new(leaves.clone(), true);
        for i in 0..n {
            let proof = tree_reversed_bits.prove(i);
            verify_merkle_proof(leaves[i].clone(), i, tree_reversed_bits.root, &proof, true)?;
        }

        Ok(())
    }
}
