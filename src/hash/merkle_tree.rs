use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::field::field_types::Field;
use crate::hash::hash_types::HashOut;
use crate::hash::hashing::{compress, hash_or_noop};
use crate::hash::merkle_proofs::MerkleProof;
use crate::util::{log2_strict, reverse_bits, reverse_index_bits_in_place};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct MerkleCap<F: Field>(pub Vec<HashOut<F>>);

#[derive(Clone, Debug)]
pub struct MerkleTree<F: Field> {
    /// The data in the leaves of the Merkle tree.
    pub leaves: Vec<Vec<F>>,

    /// The layers of hashes in the tree. The first layer is the one at the bottom.
    pub layers: Vec<Vec<HashOut<F>>>,

    /// The Merkle cap.
    pub root: MerkleCap<F>,

    /// If true, the indices are in bit-reversed form, so that the leaf at index `i`
    /// contains the leaf originally at index `reverse_bits(i)`.
    pub reverse_bits: bool,
}

impl<F: Field> MerkleTree<F> {
    pub fn new(mut leaves: Vec<Vec<F>>, cap_height: usize, reverse_bits: bool) -> Self {
        if reverse_bits {
            reverse_index_bits_in_place(&mut leaves);
        }
        let mut layers = vec![leaves
            .par_iter()
            .map(|l| hash_or_noop(l.clone()))
            .collect::<Vec<_>>()];
        while let Some(l) = layers.last() {
            if l.len() == 1 << cap_height {
                break;
            }
            let next_layer = l
                .par_chunks(2)
                .map(|chunk| compress(chunk[0], chunk[1]))
                .collect::<Vec<_>>();
            layers.push(next_layer);
        }
        let cap = layers.pop().unwrap();
        Self {
            leaves,
            layers,
            root: MerkleCap(cap),
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
    use anyhow::Result;

    use super::*;
    use crate::field::crandall_field::CrandallField;
    use crate::hash::merkle_proofs::verify_merkle_proof;

    fn random_data<F: Field>(n: usize, k: usize) -> Vec<Vec<F>> {
        (0..n).map(|_| F::rand_vec(k)).collect()
    }

    fn verify_all_leaves<F: Field>(
        leaves: Vec<Vec<F>>,
        n: usize,
        reverse_bits: bool,
    ) -> Result<()> {
        let tree = MerkleTree::new(leaves.clone(), 1, reverse_bits);
        for i in 0..n {
            let proof = tree.prove(i);
            verify_merkle_proof(leaves[i].clone(), i, &tree.root, &proof, reverse_bits)?;
        }
        Ok(())
    }

    #[test]
    fn test_merkle_trees() -> Result<()> {
        type F = CrandallField;

        let log_n = 8;
        let n = 1 << log_n;
        let leaves = random_data::<F>(n, 7);

        verify_all_leaves(leaves.clone(), n, false)?;
        // verify_all_leaves(leaves, n, true)?;

        Ok(())
    }
}
