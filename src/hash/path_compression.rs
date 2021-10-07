use std::collections::HashMap;

use num::Integer;

use crate::field::field_types::{Field, RichField};
use crate::hash::hashing::{compress, hash_or_noop};
use crate::hash::merkle_proofs::MerkleProof;

/// Compress multiple Merkle proofs on the same tree by removing redundancy in the Merkle paths.
pub(crate) fn compress_merkle_proofs<F: Field>(
    cap_height: usize,
    indices: &[usize],
    proofs: &[MerkleProof<F>],
) -> Vec<MerkleProof<F>> {
    assert!(!proofs.is_empty());
    let height = cap_height + proofs[0].siblings.len();
    let num_leaves = 1 << height;
    let mut compressed_proofs = Vec::with_capacity(proofs.len());
    // Holds the known nodes in the tree at a given time. The root is at index 1.
    // Valid indices are 1 through n, and each element at index `i` has
    // children at indices `2i` and `2i +1` its parent at index `floor(i ∕ 2)`.
    let mut known = vec![false; 2 * num_leaves];
    for &i in indices {
        // The leaves are known.
        for j in 0..(height - cap_height) {
            known[(i + num_leaves) >> j] = true;
        }
    }
    // For each proof collect all the unknown proof elements.
    for (&i, p) in indices.iter().zip(proofs) {
        let mut compressed_proof = MerkleProof {
            siblings: Vec::new(),
        };
        let mut index = i + num_leaves;
        for &sibling in &p.siblings {
            let sibling_index = index ^ 1;
            if !known[sibling_index] {
                // If the sibling is not yet known, add it to the proof and set it to known.
                compressed_proof.siblings.push(sibling);
                known[sibling_index] = true;
            }
            // Go up the tree and set the parent to known.
            index >>= 1;
            known[index] = true;
        }
        compressed_proofs.push(compressed_proof);
    }

    compressed_proofs
}

/// Decompress compressed Merkle proofs.
/// Note: The data and indices must be in the same order as in `compress_merkle_proofs`.
pub(crate) fn decompress_merkle_proofs<F: RichField>(
    leaves_data: &[Vec<F>],
    leaves_indices: &[usize],
    compressed_proofs: &[MerkleProof<F>],
    height: usize,
    cap_height: usize,
) -> Vec<MerkleProof<F>> {
    let num_leaves = 1 << height;
    let compressed_proofs = compressed_proofs.to_vec();
    let mut decompressed_proofs = Vec::with_capacity(compressed_proofs.len());
    // Holds the already seen nodes in the tree along with their value.
    let mut seen = HashMap::new();

    for (&i, v) in leaves_indices.iter().zip(leaves_data) {
        // Observe the leaves.
        seen.insert(i + num_leaves, hash_or_noop(v.to_vec()));
    }
    let mut proofs = compressed_proofs
        .iter()
        .map(|p| p.siblings.iter())
        .collect::<Vec<_>>();
    for depth in 0..height - cap_height {
        for (&i, p) in leaves_indices.iter().zip(proofs.iter_mut()) {
            let index = (i + num_leaves) >> depth;
            let sibling_index = index ^ 1;
            // dbg!(i, depth, index, sibling_index);
            let h = *seen
                .entry(sibling_index)
                .or_insert_with(|| *p.next().unwrap());
            seen.insert(sibling_index, h);
            let current_digest = seen[&index];
            let current_digest = if index.is_even() {
                compress(current_digest, h)
            } else {
                compress(h, current_digest)
            };
            seen.insert(index >> 1, current_digest);
        }
    }
    // For every index, go up the tree by querying `seen` to get node values, or if they are unknown
    // get them from the compressed proof.
    for (&i, p) in leaves_indices.iter().zip(compressed_proofs) {
        let mut decompressed_proof = MerkleProof {
            siblings: Vec::new(),
        };
        let mut index = i + num_leaves;
        for _ in 0..height - cap_height {
            let sibling_index = index ^ 1;
            // dbg!(index, sibling_index);
            let h = seen[&sibling_index];
            decompressed_proof.siblings.push(h);
            index >>= 1;
        }

        decompressed_proofs.push(decompressed_proof);
    }

    decompressed_proofs
}

#[cfg(test)]
mod tests {
    use rand::{thread_rng, Rng};

    use super::*;
    use crate::field::crandall_field::CrandallField;
    use crate::field::field_types::Field;
    use crate::hash::merkle_tree::MerkleTree;

    #[test]
    fn test_path_compression() {
        type F = CrandallField;
        let h = 5;
        let cap_height = 0;
        let vs = (0..1 << h).map(|_| vec![F::rand()]).collect::<Vec<_>>();
        let mt = MerkleTree::new(vs.clone(), cap_height);

        let mut rng = thread_rng();
        let k = rng.gen_range(1..=1 << h);
        let k = 8;
        let indices = (0..k).map(|_| rng.gen_range(0..1 << h)).collect::<Vec<_>>();
        let indices = [14, 8, 15, 2, 20, 3, 7, 30];
        let proofs = indices.iter().map(|&i| mt.prove(i)).collect::<Vec<_>>();

        let compressed_proofs = compress_merkle_proofs(cap_height, &indices, &proofs);
        // for p in &compressed_proofs {
        //     dbg!(&p.siblings.len());
        // }
        // println!(
        //     "{}",
        //     compressed_proofs
        //         .iter()
        //         .map(|p| p.siblings.len())
        //         .sum::<usize>()
        // );
        let decompressed_proofs = decompress_merkle_proofs(
            &indices.iter().map(|&i| vs[i].clone()).collect::<Vec<_>>(),
            &indices,
            &compressed_proofs,
            h,
            cap_height,
        );

        assert_eq!(proofs, decompressed_proofs);

        let compressed_proof_bytes = serde_cbor::to_vec(&compressed_proofs).unwrap();
        println!(
            "Compressed proof length: {} bytes",
            compressed_proof_bytes.len()
        );
        let proof_bytes = serde_cbor::to_vec(&proofs).unwrap();
        println!("Proof length: {} bytes", proof_bytes.len());
    }
}
