#[cfg(not(feature = "std"))]
use alloc::vec;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use crate::hash::hash_types::RichField;
use crate::hash::merkle_proofs::MerkleProof;
use crate::hash::merkle_tree::{capacity_up_to_mut, fill_digests_buf, MerkleCap};
use crate::plonk::config::{GenericHashOut, Hasher};
use crate::util::log2_strict;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldMerkleTree<F: RichField, H: Hasher<F>> {
    /// The data in the leaves of the Merkle tree.
    pub leaves: Vec<Vec<Vec<F>>>,

    /// Same as the `digests` in `MerkleTree`.
    pub digests: Vec<H::Hash>,

    /// The Merkle cap.
    pub cap: MerkleCap<F, H>,
}

impl<F: RichField, H: Hasher<F>> Default for FieldMerkleTree<F, H> {
    fn default() -> Self {
        Self {
            leaves: Vec::new(),
            digests: Vec::new(),
            cap: MerkleCap::default(),
        }
    }
}

impl<F: RichField, H: Hasher<F>> FieldMerkleTree<F, H> {
    /// `leaves` is a matrix (vector of vectors).
    /// All `leaves` should have a power of two height.
    /// All `leaves` should have different heights.
    /// The vector of `leaves` should be sorted by height, from tallest to shortest.
    pub fn new(mut leaves: Vec<Vec<Vec<F>>>, cap_height: usize) -> Self {
        assert!(leaves.iter().all(|leaf| leaf.len().is_power_of_two()));
        assert!(leaves.windows(2).all(|pair| {
            pair[0].len() > pair[1].len()
        }));

        let log2_leaves_len = log2_strict(leaves[0].len());
        assert!(
            cap_height <= log2_leaves_len,
            "cap_height={} should be at most log2(leaves.len())={}",
            cap_height,
            log2_leaves_len
        );

        let num_digests = 2 * (leaves[0].len() - (1 << cap_height));
        let mut digests = Vec::with_capacity(num_digests);
        let digests_buf = capacity_up_to_mut(&mut digests, num_digests);
        let mut digests_buf_pos = 0;

        let mut cap = vec![];
        let dummy_leaves = vec![vec![F::ZERO]; 1 << cap_height];
        leaves.push(dummy_leaves);
        for window in leaves.windows(2) {
            let cur = &window[0];
            let next = &window[1];

            let len_next_cap = next.len();
            let num_tmp_digests = 2 * (cur.len() - len_next_cap);

            if cur.len() == leaves[0].len() {
                cap = Vec::with_capacity(len_next_cap);
                let tmp_cap_buf = capacity_up_to_mut(&mut cap, len_next_cap);
                fill_digests_buf::<F, H>(&mut digests_buf[digests_buf_pos..(digests_buf_pos + num_tmp_digests)], tmp_cap_buf, &cur[..], log2_strict(next.len()));
            } else {
                //TODO: try to optimize it?
                let mut new_leaves: Vec<Vec<F>> = Vec::with_capacity(cur.len());
                for (i, cur_leaf) in cur.iter().enumerate() {
                    let mut tmp_leaf = cap[i].to_vec();
                    tmp_leaf.extend_from_slice(cur_leaf);
                    new_leaves.push(tmp_leaf);
                }
                cap = Vec::with_capacity(len_next_cap);
                let tmp_cap_buf = capacity_up_to_mut(&mut cap, len_next_cap);
                fill_digests_buf::<F, H>(&mut digests_buf[digests_buf_pos..(digests_buf_pos + num_tmp_digests)], tmp_cap_buf, &new_leaves[..], log2_strict(next.len()));
            }

            unsafe {
                cap.set_len(len_next_cap);
            }

            digests_buf_pos = digests_buf_pos + num_tmp_digests;
        }

        unsafe {
            // SAFETY: `fill_digests_buf` and `cap` initialized the spare capacity up to
            // `num_digests` and `len_cap`, resp.
            digests.set_len(num_digests);
        }

        Self {
            leaves,
            digests,
            cap: MerkleCap(cap),
        }
    }

    pub fn get(&self, table_index: usize, leaf_index: usize) -> &[F] {
        &self.leaves[table_index][leaf_index]
    }

    /// Create a Merkle proof from a leaf index.
    pub fn open_batch(&self, _leaf_index: usize) -> MerkleProof<F, H> {
        // let cap_height = log2_strict(self.cap.len());
        // let num_layers = log2_strict(self.leaves.len()) - cap_height;
        // debug_assert_eq!(leaf_index >> (cap_height + num_layers), 0);
        //
        // let digest_tree = {
        //     let tree_index = leaf_index >> num_layers;
        //     let tree_len = self.digests.len() >> cap_height;
        //     &self.digests[tree_len * tree_index..tree_len * (tree_index + 1)]
        // };
        //
        // // Mask out high bits to get the index within the sub-tree.
        // let mut pair_index = leaf_index & ((1 << num_layers) - 1);
        // let siblings = (0..num_layers)
        //     .map(|i| {
        //         let parity = pair_index & 1;
        //         pair_index >>= 1;
        //
        //         // The layers' data is interleaved as follows:
        //         // [layer 0, layer 1, layer 0, layer 2, layer 0, layer 1, layer 0, layer 3, ...].
        //         // Each of the above is a pair of siblings.
        //         // `pair_index` is the index of the pair within layer `i`.
        //         // The index of that the pair within `digests` is
        //         // `pair_index * 2 ** (i + 1) + (2 ** i - 1)`.
        //         let siblings_index = (pair_index << (i + 1)) + (1 << i) - 1;
        //         // We have an index for the _pair_, but we want the index of the _sibling_.
        //         // Double the pair index to get the index of the left sibling. Conditionally add `1`
        //         // if we are to retrieve the right sibling.
        //         let sibling_index = 2 * siblings_index + (1 - parity);
        //         digest_tree[sibling_index]
        //     })
        //     .collect();

        // MerkleProof { siblings }
        todo!()
    }

    pub fn verify_batch() -> anyhow::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[cfg(not(feature = "std"))]
    use alloc::vec;
    use anyhow::Result;
    use plonky2_field::goldilocks_field::GoldilocksField;
    use plonky2_field::types::Field;

    use super::*;
    use crate::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};

    fn random_data<F: RichField>(n: usize, k: usize) -> Vec<Vec<F>> {
        (0..n).map(|_| F::rand_vec(k)).collect()
    }

    #[test]
    fn commit_single() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type H = <C as GenericConfig<D>>::Hasher;

        // mat_1 = [
        //   0 1
        //   2 1
        //   2 2
        //   0 0
        // ]
        let mat_1 = vec![
            vec![F::ZERO, F::ONE],
            vec![F::TWO, F::ONE],
            vec![F::TWO, F::TWO],
            vec![F::ZERO, F::ZERO],
        ];

        let fmt: FieldMerkleTree<GoldilocksField, H> = FieldMerkleTree::new(vec![mat_1], 0);
        let mat_1_leaf_hashes = [
            H::hash_or_noop(&[F::ZERO, F::ONE]),
            H::hash_or_noop(&[F::TWO, F::ONE]),
            H::hash_or_noop(&[F::TWO, F::TWO]),
            H::hash_or_noop(&[F::ZERO, F::ZERO]),
        ];
        assert_eq!(mat_1_leaf_hashes[0..2], fmt.digests[0..2]);
        assert_eq!(mat_1_leaf_hashes[2..4], fmt.digests[4..6]);
        let mut layer_1 = [
            H::two_to_one(mat_1_leaf_hashes[0], mat_1_leaf_hashes[1]),
            H::two_to_one(mat_1_leaf_hashes[2], mat_1_leaf_hashes[3]),
        ];
        assert_eq!(layer_1, fmt.digests[2..4]);
        let root = H::two_to_one(layer_1[0],layer_1[1]);
        assert_eq!(fmt.cap.flatten(), root.to_vec());

        Ok(())
    }

    #[test]
    fn commit_mixed() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type H = <C as GenericConfig<D>>::Hasher;

        // mat_1 = [
        //   0 1
        //   2 1
        //   2 2
        //   0 0
        // ]
        let mat_1 = vec![
            vec![F::ZERO, F::ONE],
            vec![F::TWO, F::ONE],
            vec![F::TWO, F::TWO],
            vec![F::ZERO, F::ZERO],
        ];

        // mat_2 = [
        //   1 2 1
        //   0 2 2
        // ]
        let mat_2 = vec![
            vec![F::ONE, F::TWO, F::ONE],
            vec![F::ZERO, F::TWO, F::TWO],
        ];

        let fmt: FieldMerkleTree<GoldilocksField, H> = FieldMerkleTree::new(vec![mat_1, mat_2.clone()], 0);
        let mat_1_leaf_hashes = [
            H::hash_or_noop(&[F::ZERO, F::ONE]),
            H::hash_or_noop(&[F::TWO, F::ONE]),
            H::hash_or_noop(&[F::TWO, F::TWO]),
            H::hash_or_noop(&[F::ZERO, F::ZERO]),
        ];
        assert_eq!(mat_1_leaf_hashes, fmt.digests[0..4]);
        let mut hidden_layer = [
            H::two_to_one(mat_1_leaf_hashes[0], mat_1_leaf_hashes[1]).to_vec(),
            H::two_to_one(mat_1_leaf_hashes[2], mat_1_leaf_hashes[3]).to_vec(),
        ];
        let new_leaves = hidden_layer.iter().zip(mat_2.iter()).map(|(row1, row2)| {
            let mut new_row = row1.clone();
            new_row.extend_from_slice(row2);
            new_row
        }).collect::<Vec<Vec<F>>>();
        let layer_1 = [
            H::hash_or_noop(&new_leaves[0]),
            H::hash_or_noop(&new_leaves[1]),
        ];
        assert_eq!(layer_1, fmt.digests[4..]);
        let root = H::two_to_one(layer_1[0],layer_1[1]);
        assert_eq!(fmt.cap.flatten(), root.to_vec());

        Ok(())
    }
}
