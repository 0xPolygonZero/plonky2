use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::field::field_types::RichField;
use crate::hash::merkle_proofs::MerkleProof;
use crate::plonk::config::Hasher;

/// The Merkle cap of height `h` of a Merkle tree is the `h`-th layer (from the root) of the tree.
/// It can be used in place of the root to verify Merkle paths, which are `h` elements shorter.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(bound = "")]
pub struct MerkleCap<F: RichField, H: Hasher<F>>(pub Vec<H::Hash>);

impl<F: RichField, H: Hasher<F>> MerkleCap<F, H> {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn flatten(&self) -> Vec<F> {
        self.0
            .iter()
            .flat_map(|&h| {
                let felts: Vec<F> = h.into();
                felts
            })
            .collect()
    }
}

#[derive(Clone, Debug)]
pub struct MerkleTree<F: RichField, H: Hasher<F>> {
    /// The data in the leaves of the Merkle tree.
    pub leaves: Vec<Vec<F>>,

    /// The layers of hashes in the tree. The first layer is the one at the bottom.
    pub layers: Vec<Vec<H::Hash>>,

    /// The Merkle cap.
    pub cap: MerkleCap<F, H>,
}

impl<F: RichField, H: Hasher<F>> MerkleTree<F, H> {
    pub fn new(leaves: Vec<Vec<F>>, cap_height: usize) -> Self {
        let mut layers = vec![leaves
            .par_iter()
            .map(|l| H::hash(l.clone(), false))
            .collect::<Vec<_>>()];
        while let Some(l) = layers.last() {
            if l.len() == 1 << cap_height {
                break;
            }
            let next_layer = l
                .par_chunks(2)
                .map(|chunk| H::two_to_one(chunk[0], chunk[1]))
                .collect::<Vec<_>>();
            layers.push(next_layer);
        }
        let cap = layers.pop().unwrap();
        Self {
            leaves,
            layers,
            cap: MerkleCap(cap),
        }
    }

    pub fn get(&self, i: usize) -> &[F] {
        &self.leaves[i]
    }

    /// Create a Merkle proof from a leaf index.
    pub fn prove(&self, leaf_index: usize) -> MerkleProof<F, H> {
        MerkleProof {
            siblings: self
                .layers
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

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::*;
    use crate::field::extension_field::Extendable;
    use crate::field::goldilocks_field::GoldilocksField;
    use crate::hash::merkle_proofs::verify_merkle_proof;
    use crate::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};

    fn random_data<F: RichField>(n: usize, k: usize) -> Vec<Vec<F>> {
        (0..n).map(|_| F::rand_vec(k)).collect()
    }

    fn verify_all_leaves<F: Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>(
        leaves: Vec<Vec<F>>,
        n: usize,
    ) -> Result<()> {
        let tree = MerkleTree::<F, C::Hasher>::new(leaves.clone(), 1);
        for i in 0..n {
            let proof = tree.prove(i);
            verify_merkle_proof(leaves[i].clone(), i, &tree.cap, &proof)?;
        }
        Ok(())
    }

    #[test]
    fn test_merkle_trees() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;

        let log_n = 8;
        let n = 1 << log_n;
        let leaves = random_data::<F>(n, 7);

        verify_all_leaves::<F, C, D>(leaves, n)?;

        Ok(())
    }
}
