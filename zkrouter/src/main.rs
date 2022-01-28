use plonky2_field::field_types::Field;
use rand::{thread_rng, Rng};

use plonky2::hash::merkle_tree::MerkleTree;
use plonky2::iop::witness::{PartialWitness, Witness};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::circuit_data::CircuitConfig;
use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
use plonky2::plonk::verifier::verify;
use plonky2::hash::merkle_proofs::MerkleProofTarget;
use std::time::Instant;



fn random_data<F: Field>(n: usize, k: usize) -> Vec<Vec<F>> {
    (0..n).map(|_| F::rand_vec(k)).collect()
}

fn main() {
    let mut start = Instant::now();
    const D: usize = 2;
    type C = PoseidonGoldilocksConfig;
    type F = <C as GenericConfig<D>>::F;
    let config = CircuitConfig::standard_recursion_zk_config();
    let mut pw = PartialWitness::new();

    let log_n = 26;
    println!("Testing Merkle Tree with 2^{:?} leaves", log_n);
    let n = 1 << log_n;
    let cap_height = 1;
    let leaves = random_data::<F>(n, 7);
    let tree = MerkleTree::<F, <C as GenericConfig<D>>::Hasher>::new(leaves, cap_height);
    let i: usize = thread_rng().gen_range(0..n);
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

    let cap_t = builder.add_virtual_cap(cap_height);
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
