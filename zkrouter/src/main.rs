use std::time::Instant;

use plonky2::hash::merkle_proofs::MerkleProofTarget;
use plonky2::hash::merkle_tree::MerkleTree;
use plonky2::hash::poseidon::PoseidonHash;
use plonky2::iop::witness::{PartialWitness, Witness};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::circuit_data::CircuitConfig;
use plonky2::plonk::config::{GenericConfig, Hasher, PoseidonGoldilocksConfig};
use plonky2::plonk::proof::{CompressedProofWithPublicInputs, ProofWithPublicInputs};
use plonky2::plonk::verifier::verify;
use plonky2_field::field_types::Field;
use plonky2_field::goldilocks_field::GoldilocksField;
use rand::{thread_rng, Rng};

const D: usize = 2;
// This is some optimization used mainly for recursive proofs / internally by FRI - irrelevant to the zkrouter for now
const CAP_HEIGHT: usize = 0;
const SECRET_SIZE: usize = 4;
type C = PoseidonGoldilocksConfig;
type F = <C as GenericConfig<D>>::F;

// This is the private key used for withdrawing the funds after deposit
#[derive(Clone)]
struct DepositPrivateKey {
    secret: [F; SECRET_SIZE],
    nonce: [F; SECRET_SIZE],
}

impl DepositPrivateKey {
    fn commit_to_key(self) -> Vec<F> {
        let hash_in= Self::as_slice(&self);
        PoseidonHash::hash_no_pad(&hash_in).elements.to_vec()
    }

    fn generate_random_private_key() -> Self {
        Self {
            secret: F::rand_arr::<SECRET_SIZE>(),
            nonce: F::rand_arr::<SECRET_SIZE>(),
        }
    }

    fn as_slice(privkey: &Self) -> [F; SECRET_SIZE*2] {
        let mut slice = [F::ZERO; SECRET_SIZE*2];
        slice[..SECRET_SIZE].copy_from_slice(&privkey.secret);
        slice[SECRET_SIZE..].copy_from_slice(&privkey.nonce);
        slice
    }
}

fn random_data(n: usize) -> Vec<DepositPrivateKey> {
    (0..n)
        .map(|_| DepositPrivateKey::generate_random_private_key())
        .collect()
}

/// Generates merkle tree s.t. the leaves are of hash commitments of random data and in the given index is the commitment of the given DepositPrivateKey
fn generate_merkle_tree(
    tree_size: usize,
    deposit_priv_key: DepositPrivateKey,
    index: usize,
) -> MerkleTree<GoldilocksField, PoseidonHash> {
    let leaves_pt = random_data(tree_size);
    let mut leaves: Vec<Vec<GoldilocksField>> = leaves_pt
        .into_iter()
        .map(|privkey| privkey.commit_to_key())
        .collect();
    leaves[index] = deposit_priv_key.commit_to_key();
    MerkleTree::<F, <C as GenericConfig<D>>::Hasher>::new(leaves, CAP_HEIGHT)
}

fn main() {
    let mut start = Instant::now();

    let config = CircuitConfig::standard_recursion_zk_config();
    let log_n = 20;
    println!("Testing Merkle Tree with 2^{:?} leaves", log_n);
    let n = 1 << log_n;
    let my_private_key = DepositPrivateKey::generate_random_private_key();
    println!("my_private_key - nonce: {:?}", my_private_key.nonce);
    let private_deposit_index: usize = thread_rng().gen_range(0..n);
    let tree = generate_merkle_tree(n, my_private_key.clone(), private_deposit_index);
    println!("tree cap: {:?}", tree.cap);
    let proof = tree.prove(private_deposit_index);

    let mut duration = start.elapsed();
    println!("Plaintext Merkling took: {:?}", duration);

    // Starting to build the circuit with inputs
    start = Instant::now();
    let mut builder = CircuitBuilder::<F, D>::new(config);

    let cap_t = builder.add_virtual_cap(CAP_HEIGHT);
    // Register the root of the merkle tree as public input
    builder.register_public_inputs(&cap_t.0[0].elements);
    
    let proof_t = MerkleProofTarget {
        siblings: builder.add_virtual_hashes(proof.siblings.len()),
    };

    let i_c = builder.constant(F::from_canonical_usize(private_deposit_index));
    let i_bits = builder.split_le(i_c, log_n);

    let targets_for_deposit_privkey = builder.add_virtual_targets(SECRET_SIZE * 2);
    // Register the nonce used as a public input
    builder.register_public_inputs(&targets_for_deposit_privkey[SECRET_SIZE..]);
    
    let deposit_privkey_hash = builder.hash_n_to_hash_no_pad::<<C as GenericConfig<D>>::Hasher>(targets_for_deposit_privkey.clone());

    // Creates the logic that verifies the merkle proof inputs
    builder.verify_merkle_proof::<<C as GenericConfig<D>>::InnerHasher>(
        deposit_privkey_hash.elements.to_vec(),
        &i_bits,
        &cap_t,
        &proof_t,
    );

    // Building the circuit
    let circuit_data = builder.build::<C>();

    // Creating the private witness
    let mut pw = PartialWitness::new();

    // Set the root of the merkle tree (privately)
    pw.set_cap_target(&cap_t, &tree.cap);

    // Set the sibling wires to their values (privately)
    for i in 0..proof.siblings.len() {
        pw.set_hash_target(proof_t.siblings[i], proof.siblings[i]);
    }

    // Set the FEs that comprise our deposit private key (privately)
    let privkey_slice = DepositPrivateKey::as_slice(&my_private_key);
    for j in 0..targets_for_deposit_privkey.len() {
        pw.set_target(
            targets_for_deposit_privkey[j],
            privkey_slice[j],
        );
    }

    duration = start.elapsed();
    println!("Circuit building: {:?}", duration);
    
    // Starting to generate the proof
    start = Instant::now();
    let proof = circuit_data.prove(pw).unwrap();
    let proof_bytes = proof.to_bytes().unwrap();
    println!("public_inputs: {:?}", proof.public_inputs);
    let compressed_proof = proof.to_owned().compress(&circuit_data.common).unwrap();
    let compressed_proof_bytes = compressed_proof.to_bytes().unwrap();
    println!("compressed public_inputs: {:?}", compressed_proof.public_inputs);
    println!("Proof size: {:?}", proof_bytes.len());
    println!(
        "Compressed proof size: {:?}",
        compressed_proof_bytes.len()
    );
    duration = start.elapsed();
    println!("Proof generation: {:?}", duration);
    
    // Verifying the proof
    start = Instant::now();
    let unpacked_compressed_proof = CompressedProofWithPublicInputs::from_bytes(compressed_proof_bytes, &circuit_data.common).unwrap();
    let resc = unpacked_compressed_proof.verify(&circuit_data.verifier_only, &circuit_data.common);
    duration = start.elapsed();
    println!("Verify compressed: {:?}, took: {:?}", resc, duration);
    let proof_from_bytes = ProofWithPublicInputs::from_bytes(proof_bytes, &circuit_data.common).unwrap();
    start = Instant::now();
    let res = verify(proof_from_bytes, &circuit_data.verifier_only, &circuit_data.common);
    duration = start.elapsed();
    println!("Verify: {:?}, took: {:?}", res, duration);
}
