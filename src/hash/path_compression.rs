use std::collections::HashMap;

use anyhow::{ensure, Result};
use num::Integer;
use serde::{Deserialize, Serialize};

use crate::field::field_types::Field;
use crate::hash::hash_types::HashOut;
use crate::hash::hashing::{compress, hash_or_noop};
use crate::hash::merkle_proofs::MerkleProof;
use crate::hash::merkle_tree::{MerkleCap, MerkleTree};
use crate::util::log2_strict;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct CompressedMerkleProof<F: Field> {
    pub proof: Vec<HashOut<F>>,
}

pub(crate) fn compress_merkle_proofs<F: Field>(
    cap_height: usize,
    mut proofs: Vec<(usize, MerkleProof<F>)>,
) -> CompressedMerkleProof<F> {
    let height = cap_height + proofs[0].1.siblings.len();
    let num_leaves = 1 << height;
    let mut proof = Vec::new();
    let mut known = vec![false; 2 * num_leaves];
    for (i, _) in &proofs {
        known[*i + num_leaves] = true;
    }
    for (i, p) in proofs {
        let mut index = i + num_leaves;
        for sibling in p.siblings {
            let sibling_index = index ^ 1;
            if known[sibling_index] {
                index >>= 1;
                continue;
            } else if (sibling_index < num_leaves)
                && (known[2 * sibling_index] && known[2 * sibling_index + 1])
            {
                index >>= 1;
                known[sibling_index] = true;
                continue;
            }
            proof.push(sibling);
            index >>= 1;
            known[sibling_index] = true;
            known[index] = true;
        }
    }

    CompressedMerkleProof { proof }
}

pub(crate) fn verify_compressed_merkle_proof<F: Field>(
    leaves_data: &[Vec<F>],
    leaves_indices: &[usize],
    proof: &CompressedMerkleProof<F>,
    height: usize,
    cap: &MerkleCap<F>,
) -> Result<()> {
    let cap_height = log2_strict(cap.0.len());
    let mut proof = proof.proof.clone();
    let mut seen = HashMap::new();
    let mut leaves = leaves_indices
        .iter()
        .zip(leaves_data)
        .map(|(&i, v)| (i, v.clone()))
        .collect::<Vec<_>>();

    for (i, v) in &leaves {
        seen.insert(i + (1 << height), hash_or_noop(v.to_vec()));
    }
    for (i, v) in leaves {
        let mut index = i + (1 << height);
        let mut current_digest = seen[&index];
        for _ in 0..height - cap_height {
            let sibling_index = index ^ 1;
            let h = if seen.contains_key(&sibling_index) {
                seen[&sibling_index]
            } else if seen.contains_key(&(2 * sibling_index))
                && seen.contains_key(&(2 * sibling_index + 1))
            {
                let a = seen[&(2 * sibling_index)];
                let b = seen[&(2 * sibling_index + 1)];
                compress(a, b)
            } else {
                proof.remove(0)
            };
            seen.insert(sibling_index, h);
            if index.is_even() {
                current_digest = compress(current_digest, h);
            } else {
                current_digest = compress(h, current_digest);
            }
            index >>= 1;
            seen.insert(index, current_digest);
        }
        ensure!(
            current_digest == cap.0[index - cap.0.len()],
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
        let h = 16;
        let cap_height = 3;
        let vs = (0..1 << h).map(|i| vec![F::rand()]).collect::<Vec<_>>();
        let mt = MerkleTree::new(vs.clone(), cap_height);

        let mut rng = thread_rng();
        let k = rng.gen_range(0..1 << h);
        let k = 27;
        let indices = (0..k).map(|_| rng.gen_range(0..1 << h)).collect::<Vec<_>>();
        let proofs: Vec<(usize, MerkleProof<_>)> =
            indices.iter().map(|&i| (i, mt.prove(i))).collect();

        let cp = compress_merkle_proofs(cap_height, proofs.clone());

        verify_compressed_merkle_proof(
            &indices.iter().map(|&i| vs[i].clone()).collect::<Vec<_>>(),
            &indices,
            &cp,
            h,
            &mt.cap,
        )
        .unwrap();

        println!("Proof length: {} ", proofs.len());
        let proof_bytes = serde_cbor::to_vec(&cp).unwrap();
        println!("Proof length: {} bytes", proof_bytes.len());
        let proof_bytes = serde_cbor::to_vec(&proofs).unwrap();
        println!("Proof length: {} bytes", proof_bytes.len());
    }
}
