use std::collections::HashMap;

use anyhow::{ensure, Result};
use num::Integer;
use serde::{Deserialize, Serialize};

use crate::field::field_types::Field;
use crate::hash::hash_types::HashOut;
use crate::hash::hashing::{compress, hash_or_noop};
use crate::hash::merkle_proofs::MerkleProof;
use crate::hash::merkle_tree::MerkleCap;
use crate::util::log2_strict;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct CompressedMerkleProof<F: Field> {
    pub proof: Vec<HashOut<F>>,
}

/// Compress multiple Merkle proofs on the same tree by removing redundancy in the Merkle paths.
pub(crate) fn compress_merkle_proofs<F: Field>(
    cap_height: usize,
    proofs: Vec<(usize, MerkleProof<F>)>,
) -> CompressedMerkleProof<F> {
    let height = cap_height + proofs[0].1.siblings.len();
    let num_leaves = 1 << height;
    let mut proof = Vec::new();
    // Holds the known nodes in the tree at a given time. The root is at index 1.
    let mut known = vec![false; 2 * num_leaves];
    for (i, _) in &proofs {
        // The leaves are known.
        known[*i + num_leaves] = true;
    }
    // For each proof collect all the unknown proof elements.
    for (i, p) in proofs {
        let mut index = i + num_leaves;
        for sibling in p.siblings {
            let sibling_index = index ^ 1;
            if !known[sibling_index] {
                // If the sibling is not yet known, add it to the proof and set it to known.
                proof.push(sibling);
                known[sibling_index] = true;
            }
            // Go up the tree and set the parent to known.
            index >>= 1;
            known[index] = true;
        }
    }

    CompressedMerkleProof { proof }
}

/// Verify a compressed Merkle proof.
/// Note: The data and indices must be in the same order as in `compress_merkle_proofs`.
pub(crate) fn verify_compressed_merkle_proof<F: Field>(
    leaves_data: &[Vec<F>],
    leaves_indices: &[usize],
    proof: &CompressedMerkleProof<F>,
    height: usize,
    cap: &MerkleCap<F>,
) -> Result<()> {
    let num_leaves = 1 << height;
    let cap_height = log2_strict(cap.len());
    let mut proof = proof.proof.clone();
    proof.reverse();
    // Holds the already seen nodes in the tree along with their value.
    let mut seen = HashMap::new();

    for (&i, v) in leaves_indices.iter().zip(leaves_data) {
        // Observe the leaves.
        seen.insert(i + num_leaves, hash_or_noop(v.to_vec()));
    }
    // For every index, go up the tree by querying `seen` to get node values, or if they are unknown
    // pop them from the compressed proof.
    for &i in leaves_indices {
        let mut index = i + num_leaves;
        let mut current_digest = seen[&index];
        for _ in 0..height - cap_height {
            let sibling_index = index ^ 1;
            // Get the value of the sibling node by querying it or popping it from the proof.
            let h = *seen
                .entry(sibling_index)
                .or_insert_with(|| proof.pop().unwrap());
            // Update the current digest to the value of the parent.
            current_digest = if index.is_even() {
                compress(current_digest, h)
            } else {
                compress(h, current_digest)
            };
            // Observe the parent.
            index >>= 1;
            seen.insert(index, current_digest);
        }

        ensure!(
            current_digest == cap.0[index - cap.len()],
            "Invalid Merkle proof."
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use rand::{thread_rng, Rng};

    use crate::field::crandall_field::CrandallField;
    use crate::field::field_types::Field;
    use crate::hash::merkle_proofs::MerkleProof;
    use crate::hash::merkle_tree::MerkleTree;
    use crate::hash::path_compression::{compress_merkle_proofs, verify_compressed_merkle_proof};

    #[test]
    fn test_path_compression() {
        type F = CrandallField;
        let h = 10;
        let cap_height = 3;
        let vs = (0..1 << h).map(|_| vec![F::rand()]).collect::<Vec<_>>();
        let mt = MerkleTree::new(vs.clone(), cap_height);

        let mut rng = thread_rng();
        let k = rng.gen_range(0..1 << h);
        let indices = (0..k).map(|_| rng.gen_range(0..1 << h)).collect::<Vec<_>>();
        let proofs: Vec<(usize, MerkleProof<_>)> =
            indices.iter().map(|&i| (i, mt.prove(i))).collect();

        let compressed_proof = compress_merkle_proofs(cap_height, proofs.clone());

        verify_compressed_merkle_proof(
            &indices.iter().map(|&i| vs[i].clone()).collect::<Vec<_>>(),
            &indices,
            &compressed_proof,
            h,
            &mt.cap,
        )
        .unwrap();

        let compressed_proof_bytes = serde_cbor::to_vec(&compressed_proof).unwrap();
        println!(
            "Compressed proof length: {} bytes",
            compressed_proof_bytes.len()
        );
        let proof_bytes = serde_cbor::to_vec(&proofs).unwrap();
        println!("Proof length: {} bytes", proof_bytes.len());
    }
}
