#![feature(generic_const_exprs)]

use anyhow::Result;
use log::{info, Level};
use plonky2::plonk::circuit_data::VerifierCircuitTarget;
use plonky2::plonk::config::AlgebraicHasher;
use plonky2::{
    fri::{reduction_strategies::FriReductionStrategy, FriConfig},
    gates::noop::NoopGate,
    hash::hash_types::RichField,
    iop::witness::{PartialWitness, Witness},
    plonk::{
        circuit_builder::CircuitBuilder,
        circuit_data::{CircuitConfig, CommonCircuitData, VerifierOnlyCircuitData},
        config::{GenericConfig, Hasher, KeccakGoldilocksConfig, PoseidonGoldilocksConfig},
        proof::{CompressedProofWithPublicInputs, ProofWithPublicInputs},
        prover::prove,
    },
    util::timing::TimingTree,
};
use plonky2_field::extension_field::Extendable;

/// Creates a dummy proof which should have roughly `num_dummy_gates` gates.
fn dummy_proof<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>(
    config: &CircuitConfig,
    num_dummy_gates: u64,
) -> Result<(
    ProofWithPublicInputs<F, C, D>,
    VerifierOnlyCircuitData<C, D>,
    CommonCircuitData<F, C, D>,
)>
where
    [(); C::Hasher::HASH_SIZE]:,
{
    let mut builder = CircuitBuilder::<F, D>::new(config.clone());
    for _ in 0..num_dummy_gates {
        builder.add_gate(NoopGate, vec![]);
    }

    let data = builder.build::<C>();
    let inputs = PartialWitness::new();
    let proof = data.prove(inputs)?;
    data.verify(proof.clone())?;

    Ok((proof, data.verifier_only, data.common))
}

fn recursive_proof<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    InnerC: GenericConfig<D, F = F>,
    const D: usize,
>(
    inner_proof: ProofWithPublicInputs<F, InnerC, D>,
    inner_vd: VerifierOnlyCircuitData<InnerC, D>,
    inner_cd: CommonCircuitData<F, InnerC, D>,
    config: &CircuitConfig,
    min_degree_bits: Option<usize>,
    print_gate_counts: bool,
    print_timing: bool,
) -> Result<(
    ProofWithPublicInputs<F, C, D>,
    VerifierOnlyCircuitData<C, D>,
    CommonCircuitData<F, C, D>,
)>
where
    InnerC::Hasher: AlgebraicHasher<F>,
    [(); C::Hasher::HASH_SIZE]:,
{
    let mut builder = CircuitBuilder::<F, D>::new(config.clone());
    let mut pw = PartialWitness::new();
    let pt = builder.add_virtual_proof_with_pis(&inner_cd);
    pw.set_proof_with_pis_target(&pt, &inner_proof);

    let inner_data = VerifierCircuitTarget {
        constants_sigmas_cap: builder.add_virtual_cap(inner_cd.config.fri_config.cap_height),
    };
    pw.set_cap_target(
        &inner_data.constants_sigmas_cap,
        &inner_vd.constants_sigmas_cap,
    );

    builder.verify_proof(pt, &inner_data, &inner_cd);

    if print_gate_counts {
        builder.print_gate_counts(0);
    }

    if let Some(min_degree_bits) = min_degree_bits {
        // We don't want to pad all the way up to 2^min_degree_bits, as the builder will add a
        // few special gates afterward. So just pad to 2^(min_degree_bits - 1) + 1. Then the
        // builder will pad to the next power of two, 2^min_degree_bits.
        let min_gates = (1 << (min_degree_bits - 1)) + 1;
        for _ in builder.num_gates()..min_gates {
            builder.add_gate(NoopGate, vec![]);
        }
    }

    let data = builder.build::<C>();

    let mut timing = TimingTree::new("prove", Level::Debug);
    let proof = prove(&data.prover_only, &data.common, pw, &mut timing)?;
    if print_timing {
        timing.print();
    }

    data.verify(proof.clone())?;

    Ok((proof, data.verifier_only, data.common))
}

/// Test serialization and print some size info.
fn test_serialization<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>(
    proof: &ProofWithPublicInputs<F, C, D>,
    cd: &CommonCircuitData<F, C, D>,
) -> Result<()>
where
    [(); C::Hasher::HASH_SIZE]:,
{
    let proof_bytes = proof.to_bytes()?;
    info!("Proof length: {} bytes", proof_bytes.len());
    let proof_from_bytes = ProofWithPublicInputs::from_bytes(proof_bytes, cd)?;
    assert_eq!(proof, &proof_from_bytes);

    let now = std::time::Instant::now();
    let compressed_proof = proof.clone().compress(cd)?;
    let decompressed_compressed_proof = compressed_proof.clone().decompress(cd)?;
    info!("{:.4}s to compress proof", now.elapsed().as_secs_f64());
    assert_eq!(proof, &decompressed_compressed_proof);

    let compressed_proof_bytes = compressed_proof.to_bytes()?;
    info!(
        "Compressed proof length: {} bytes",
        compressed_proof_bytes.len()
    );
    let compressed_proof_from_bytes =
        CompressedProofWithPublicInputs::from_bytes(compressed_proof_bytes, cd)?;
    assert_eq!(compressed_proof, compressed_proof_from_bytes);

    Ok(())
}

fn main() -> Result<()> {
    let _ = env_logger::builder().format_timestamp(None).try_init();

    const D: usize = 2;
    type C = PoseidonGoldilocksConfig;
    type F = <C as GenericConfig<D>>::F;

    let config = CircuitConfig::standard_recursion_config();

    // Start with a degree 2^14 proof
    let (proof, vd, cd) = dummy_proof::<F, C, D>(&config, 16_000)?;
    assert_eq!(cd.degree_bits, 14);

    // Shrink it to 2^13.
    let (proof, vd, cd) =
        recursive_proof::<F, C, C, D>(proof, vd, cd, &config, Some(13), false, false)?;
    assert_eq!(cd.degree_bits, 13);

    // Shrink it to 2^12.
    let (proof, _vd, cd) = recursive_proof::<F, C, C, D>(proof, vd, cd, &config, None, true, true)?;
    assert_eq!(cd.degree_bits, 12);

    test_serialization(&proof, &cd)?;

    Ok(())
}
