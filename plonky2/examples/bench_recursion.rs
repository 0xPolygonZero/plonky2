// HACK: Ideally this would live in `benches/`, but `cargo bench` doesn't allow
// custom CLI argument parsing (even with harness disabled). We could also have
// put it in `src/bin/`, but then we wouldn't have access to
// `[dev-dependencies]`.

#![feature(generic_const_exprs)]

use std::{ops::RangeInclusive, str::FromStr};

use anyhow::{Context as _, Result};
use log::{info, Level};
use plonky2::{
    fri::{reduction_strategies::FriReductionStrategy, FriConfig},
    gates::noop::NoopGate,
    hash::hash_types::RichField,
    iop::witness::{PartialWitness, Witness},
    plonk::{
        circuit_builder::CircuitBuilder,
        circuit_data::{
            CircuitConfig, CommonCircuitData, VerifierCircuitTarget, VerifierOnlyCircuitData,
        },
        config::{
            AlgebraicHasher, GenericConfig, Hasher, KeccakGoldilocksConfig,
            PoseidonGoldilocksConfig,
        },
        proof::{CompressedProofWithPublicInputs, ProofWithPublicInputs},
        prover::prove,
    },
    util::timing::TimingTree,
};
use plonky2_field::extension_field::Extendable;
use rand::{rngs::OsRng, RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;
use structopt::StructOpt;

#[derive(Clone, StructOpt, Debug)]
#[structopt(name = "bench_recursion")]
struct Options {
    /// Random seed for deterministic random number generation.
    /// If not specified a seed is generated from OS entropy.
    #[structopt(long, parse(try_from_str = parse_hex_u64))]
    seed: Option<u64>,

    /// Number of compute threads to use (defaults to number of cores)
    #[structopt(long, parse(try_from_str = parse_range_usize))]
    threads: Option<RangeInclusive<usize>>,

    /// Gate count of the inner proof. Can be a single value or rust style
    /// ranges.
    #[structopt(long, default_value="16000", parse(try_from_str = parse_range_usize))]
    inner_size: RangeInclusive<usize>,
}

/// Creates a dummy proof which should have roughly `num_dummy_gates` gates.
fn dummy_proof<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>(
    config: &CircuitConfig,
    num_dummy_gates: usize,
) -> Result<(
    ProofWithPublicInputs<F, C, D>,
    VerifierOnlyCircuitData<C, D>,
    CommonCircuitData<F, C, D>,
)>
where
    [(); C::Hasher::HASH_SIZE]:,
{
    info!("Constructing inner proof with {} gates", num_dummy_gates);
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
        // We don't want to pad all the way up to 2^min_degree_bits, as the builder will
        // add a few special gates afterward. So just pad to 2^(min_degree_bits
        // - 1) + 1. Then the builder will pad to the next power of two,
        // 2^min_degree_bits.
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

fn benchmark(config: &CircuitConfig, inner_size: usize) -> Result<()> {
    const D: usize = 2;
    type C = PoseidonGoldilocksConfig;
    type F = <C as GenericConfig<D>>::F;

    // Start with a degree 2^14 proof
    let (proof, vd, cd) = dummy_proof::<F, C, D>(&config, inner_size)?;
    info!(
        "Initial proof degree {} = 2^{}",
        cd.degree(),
        cd.degree_bits
    );

    // Recursively verify the proof
    let (proof, vd, cd) =
        recursive_proof::<F, C, C, D>(proof, vd, cd, &config, Some(13), false, false)?;
    info!(
        "Single recursion proof degree {} = 2^{}",
        cd.degree(),
        cd.degree_bits
    );

    // Shrink it to 2^12.
    let (proof, _vd, cd) = recursive_proof::<F, C, C, D>(proof, vd, cd, &config, None, true, true)?;
    info!(
        "Double recursion proof degree {} = 2^{}",
        cd.degree(),
        cd.degree_bits
    );

    test_serialization(&proof, &cd)?;

    Ok(())
}

fn main() -> Result<()> {
    // Parse command line arguments, see `--help` for details.
    let options = Options::from_args_safe()?;

    // Initialize logging
    let _ = env_logger::builder().format_timestamp(None).try_init();

    // Initialize randomness source
    let rng_seed = options.seed.unwrap_or_else(|| OsRng::default().next_u64());
    info!("Using random seed {rng_seed:16x}");
    let rng = ChaCha8Rng::seed_from_u64(rng_seed);
    // TODO: Use `rng` to create deterministic runs


    let num_cpus = num_cpus::get();
    let threads = options.threads.unwrap_or(num_cpus..=num_cpus);

    let config = CircuitConfig::standard_recursion_config();
    for inner_size in options.inner_size {
        // Since the `inner_size` is most likely to be and unbounded range, we
        // make that the outer iterator.
        // TODO: log 2 inner size.

        for threads in threads.clone() {
            rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .context("Failed to build thread pool.")?
                .install(|| {
                    info!(
                        "Using {} compute threads on {} cores",
                        rayon::current_num_threads(),
                        num_cpus
                    );
                    // Run the benchmark
                    benchmark(&config, inner_size)
                });
        }
    }

    Ok(())
}

#[must_use]
fn parse_hex_u64(src: &str) -> Result<u64, std::num::ParseIntError> {
    let src = src.strip_prefix("0x").unwrap_or(src);
    u64::from_str_radix(src, 16)
}

#[must_use]
fn parse_range_usize(src: &str) -> Result<RangeInclusive<usize>, std::num::ParseIntError> {
    if let Some(index) = src.find("..=") {
        let left = usize::from_str(&src[..index])?;
        let right = usize::from_str(&src[(index + 3)..])?;
        Ok(RangeInclusive::new(left, right))
    } else if let Some(index) = src.find("..") {
        let left = usize::from_str(&src[..index])?;
        let right = &src[(index + 2)..];
        let right = if right.is_empty() {
            usize::MAX
        } else {
            usize::from_str(right)?
        };
        Ok(RangeInclusive::new(left, right - 1))
    } else {
        let value = usize::from_str(src)?;
        Ok(RangeInclusive::new(value, value))
    }
}
