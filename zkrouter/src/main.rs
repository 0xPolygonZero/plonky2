use plonky2_field::field_types::Field;
use rand::{thread_rng, Rng};

use plonky2::hash::merkle_tree::MerkleTree;
use plonky2::iop::witness::{PartialWitness, Witness};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::circuit_data::CircuitConfig;
use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig, Hasher};
use plonky2::plonk::verifier::verify;
use plonky2::hash::merkle_proofs::MerkleProofTarget;
use plonky2_field::goldilocks_field::GoldilocksField;
use plonky2::hash::poseidon::PoseidonHash;
use std::time::Instant;


const D: usize = 2;
// This is some optimization used mainly for recursive proofs / internally by FRI - irrelevant ro the zkrouter for now
const CAP_HEIGHT: usize = 1;
const SECRET_SIZE: usize = 2;
type C = PoseidonGoldilocksConfig;
type F = <C as GenericConfig<D>>::F;

// This is the private key used for withdrawing the funds after deposit
#[derive(Clone)]
struct DepositPrivateKey {
    secret: [F; SECRET_SIZE],
    nonce: [F; SECRET_SIZE]
}

impl DepositPrivateKey {
    fn commit_to_key(self) -> Vec<F> {
        // Hardcoded SECRET_SIZE = 2 to save some clones
        let hash_in = [self.secret[0], self.secret[1], self.nonce[0], self.nonce[1]];
        PoseidonHash::hash_no_pad(&hash_in).elements.to_vec()
    }

    fn generate_random_private_key() -> Self {
        Self {
            secret: F::rand_arr::<SECRET_SIZE>(),
            nonce: F::rand_arr::<SECRET_SIZE>()
        }
    }
}

fn random_data(n: usize) -> Vec<DepositPrivateKey> {
    (0..n).map(|_| DepositPrivateKey::generate_random_private_key()).collect()
}

fn generate_merkle_tree(tree_size: usize, deposit_priv_key: DepositPrivateKey, index: usize) -> MerkleTree<GoldilocksField, PoseidonHash> {
    let leaves_pt = random_data(tree_size);
    let mut leaves: Vec<Vec<GoldilocksField>> = leaves_pt.into_iter().map(|privkey| privkey.commit_to_key()).collect();
    leaves[index] = deposit_priv_key.commit_to_key();
    MerkleTree::<F, <C as GenericConfig<D>>::Hasher>::new(leaves, CAP_HEIGHT)
}

fn main() {
    let mut start = Instant::now();
    
    let config = CircuitConfig::standard_recursion_zk_config();
    let mut pw = PartialWitness::new();

    let log_n = 20;
    println!("Testing Merkle Tree with 2^{:?} leaves", log_n);
    let n = 1 << log_n;
    let my_private_key = DepositPrivateKey::generate_random_private_key();
    let i: usize = thread_rng().gen_range(0..n);
    let tree = generate_merkle_tree(n, my_private_key.clone(), i);
    let proof = tree.prove(i);

    let mut duration = start.elapsed();
    println!("Plaintext Merkling took: {:?}", duration);
    
    // Starting to build the circuit with inputs
    start = Instant::now();
    let mut builder = CircuitBuilder::<F, D>::new(config);

    let proof_t = MerkleProofTarget {
        siblings: builder.add_virtual_hashes(proof.siblings.len()),
    };
    for i in 0..proof.siblings.len() {
        pw.set_hash_target(proof_t.siblings[i], proof.siblings[i]);
    }

    let cap_t = builder.add_virtual_cap(CAP_HEIGHT);
    pw.set_cap_target(&cap_t, &tree.cap);

    let i_c = builder.constant(F::from_canonical_usize(i));
    let i_bits = builder.split_le(i_c, log_n);

    let data = builder.add_virtual_targets(tree.leaves[i].len());
    for j in 0..data.len() {
        pw.set_target(data[j], tree.leaves[i][j]);
    }

    builder.verify_merkle_proof::<<C as GenericConfig<D>>::InnerHasher>(
        data, &i_bits, &cap_t, &proof_t,
    );

    let data = builder.build::<C>();
    duration = start.elapsed();
    println!("Circuit building: {:?}", duration);
    start = Instant::now();
    let proof = data.prove(pw).unwrap();
    let compressed_proof = proof.to_owned().compress(&data.common).unwrap();
    println!("Proof size: {:?}", proof.to_bytes().unwrap().len());
    println!("Compressed proof size: {:?}", compressed_proof.to_bytes().unwrap().len());
    duration = start.elapsed();
    println!("Proof generation: {:?}", duration);
    start = Instant::now();
    let resc = compressed_proof.verify(&data.verifier_only, &data.common);
    duration = start.elapsed();
    println!("Verify compressed: {:?}, took: {:?}", resc, duration);
    start = Instant::now();
    let res = verify(proof, &data.verifier_only, &data.common);
    duration = start.elapsed();
    println!("Verify: {:?}, took: {:?}", res, duration);
}
