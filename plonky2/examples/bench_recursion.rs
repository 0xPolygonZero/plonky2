// HACK: Ideally this would live in `benches/`, but `cargo bench` doesn't allow
// custom CLI argument parsing (even with harness disabled). We could also have
// put it in `src/bin/`, but then we wouldn't have access to
// `[dev-dependencies]`.

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::sync::Arc;
use core::num::ParseIntError;
use core::ops::RangeInclusive;
use core::str::FromStr;
#[cfg(feature = "std")]
use std::sync::Arc;

use anyhow::{anyhow, Context as _, Result};
use itertools::Itertools;
use log::{info, Level, LevelFilter};
use plonky2::gadgets::lookup::TIP5_TABLE;
use plonky2::gates::noop::NoopGate;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::witness::{PartialWitness, WitnessWrite};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::circuit_data::{CircuitConfig, CommonCircuitData, VerifierOnlyCircuitData};
use plonky2::plonk::config::{AlgebraicHasher, GenericConfig, PoseidonGoldilocksConfig};
use plonky2::plonk::proof::{CompressedProofWithPublicInputs, ProofWithPublicInputs};
use plonky2::plonk::prover::prove;
use plonky2::util::serialization::DefaultGateSerializer;
use plonky2::util::timing::TimingTree;
use plonky2_field::extension::Extendable;
use plonky2_maybe_rayon::rayon;
use rand::rngs::OsRng;
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;
use structopt::StructOpt;

type ProofTuple<F, C, const D: usize> = (
    ProofWithPublicInputs<F, C, D>,
    VerifierOnlyCircuitData<C, D>,
    CommonCircuitData<F, D>,
);

#[derive(Clone, StructOpt, Debug)]
#[structopt(name = "bench_recursion")]
struct Options {
    /// Verbose mode (-v, -vv, -vvv, etc.)
    #[structopt(short, long, parse(from_occurrences))]
    verbose: usize,

    /// Apply an env_filter compatible log filter
    #[structopt(long, env, default_value)]
    log_filter: String,

    /// Random seed for deterministic runs.
    /// If not specified a new seed is generated from OS entropy.
    #[structopt(long, parse(try_from_str = parse_hex_u64))]
    seed: Option<u64>,

    /// Number of compute threads to use. Defaults to number of cores. Can be a single
    /// value or a rust style range.
    #[structopt(long, parse(try_from_str = parse_range_usize))]
    threads: Option<RangeInclusive<usize>>,

    /// Log2 gate count of the inner proof. Can be a single value or a rust style
    /// range.
    #[structopt(long, default_value="14", parse(try_from_str = parse_range_usize))]
    size: RangeInclusive<usize>,

    /// Lookup type. If `lookup_type == 0` or `lookup_type > 2`, then a benchmark with NoopGates only is run.
    /// If `lookup_type == 1`, a benchmark with one lookup is run.
    /// If `lookup_type == 2`, a benchmark with 515 lookups is run.
    #[structopt(long, default_value="0", parse(try_from_str = parse_hex_u64))]
    lookup_type: u64,
}

/// Creates a dummy proof which should have `2 ** log2_size` rows.
fn dummy_proof<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>(
    config: &CircuitConfig,
    log2_size: usize,
) -> Result<ProofTuple<F, C, D>> {
    // 'size' is in degree, but we want number of noop gates. A non-zero amount of padding will be added and size will be rounded to the next power of two. To hit our target size, we go just under the previous power of two and hope padding is less than half the proof.
    let num_dummy_gates = match log2_size {
        0 => return Err(anyhow!("size must be at least 1")),
        1 => 0,
        2 => 1,
        n => (1 << (n - 1)) + 1,
    };
    info!("Constructing inner proof with {} gates", num_dummy_gates);
    let mut builder = CircuitBuilder::<F, D>::new(config.clone());
    for _ in 0..num_dummy_gates {
        builder.add_gate(NoopGate, vec![]);
    }
    builder.print_gate_counts(0);

    let data = builder.build::<C>();
    let inputs = PartialWitness::new();

    let mut timing = TimingTree::new("prove", Level::Debug);
    let proof = prove::<F, C, D>(&data.prover_only, &data.common, inputs, &mut timing)?;
    timing.print();
    data.verify(proof.clone())?;

    Ok((proof, data.verifier_only, data.common))
}

fn dummy_lookup_proof<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>(
    config: &CircuitConfig,
    log2_size: usize,
) -> Result<ProofTuple<F, C, D>> {
    let mut builder = CircuitBuilder::<F, D>::new(config.clone());
    let tip5_table = TIP5_TABLE.to_vec();
    let inps = 0..256;
    let table = Arc::new(inps.zip_eq(tip5_table).collect());
    let tip5_idx = builder.add_lookup_table_from_pairs(table);
    let initial_a = builder.add_virtual_target();
    builder.add_lookup_from_index(initial_a, tip5_idx);
    builder.register_public_input(initial_a);

    // 'size' is in degree, but we want the number of gates in the circuit.
    // A non-zero amount of padding will be added and size will be rounded to the next power of two.
    // To hit our target size, we go just under the previous power of two and hope padding is less than half the proof.
    let targeted_num_gates = match log2_size {
        0 => return Err(anyhow!("size must be at least 1")),
        1 => 0,
        2 => 1,
        n => (1 << (n - 1)) + 1,
    };
    assert!(
        targeted_num_gates >= builder.num_gates(),
        "size is too small to support lookups"
    );

    for _ in builder.num_gates()..targeted_num_gates {
        builder.add_gate(NoopGate, vec![]);
    }
    builder.print_gate_counts(0);

    let data = builder.build::<C>();
    let mut inputs = PartialWitness::<F>::new();
    inputs.set_target(initial_a, F::ONE)?;
    let mut timing = TimingTree::new("prove with one lookup", Level::Debug);
    let proof = prove(&data.prover_only, &data.common, inputs, &mut timing)?;
    timing.print();
    data.verify(proof.clone())?;

    Ok((proof, data.verifier_only, data.common))
}

/// Creates a dummy proof which has more than 256 lookups to one LUT
fn dummy_many_rows_proof<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(
    config: &CircuitConfig,
    log2_size: usize,
) -> Result<ProofTuple<F, C, D>> {
    let mut builder = CircuitBuilder::<F, D>::new(config.clone());
    let tip5_table = TIP5_TABLE.to_vec();
    let inps: Vec<u16> = (0..256).collect();
    let tip5_idx = builder.add_lookup_table_from_table(&inps, &tip5_table);
    let initial_a = builder.add_virtual_target();

    let output = builder.add_lookup_from_index(initial_a, tip5_idx);
    for _ in 0..514 {
        builder.add_lookup_from_index(output, 0);
    }

    // 'size' is in degree, but we want the number of gates in the circuit.
    // A non-zero amount of padding will be added and size will be rounded to the next power of two.
    // To hit our target size, we go just under the previous power of two and hope padding is less than half the proof.
    let targeted_num_gates = match log2_size {
        0 => return Err(anyhow!("size must be at least 1")),
        1 => 0,
        2 => 1,
        n => (1 << (n - 1)) + 1,
    };
    assert!(
        targeted_num_gates >= builder.num_gates(),
        "size is too small to support so many lookups"
    );

    for _ in 0..targeted_num_gates {
        builder.add_gate(NoopGate, vec![]);
    }

    builder.register_public_input(initial_a);
    builder.register_public_input(output);

    let mut pw = PartialWitness::new();
    pw.set_target(initial_a, F::ONE)?;
    let data = builder.build::<C>();
    let mut timing = TimingTree::new("prove with many lookups", Level::Debug);
    let proof = prove(&data.prover_only, &data.common, pw, &mut timing)?;
    timing.print();

    data.verify(proof.clone())?;
    Ok((proof, data.verifier_only, data.common))
}

fn recursive_proof<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    InnerC: GenericConfig<D, F = F>,
    const D: usize,
>(
    inner: &ProofTuple<F, InnerC, D>,
    config: &CircuitConfig,
    min_degree_bits: Option<usize>,
) -> Result<ProofTuple<F, C, D>>
where
    InnerC::Hasher: AlgebraicHasher<F>,
{
    let (inner_proof, inner_vd, inner_cd) = inner;
    let mut builder = CircuitBuilder::<F, D>::new(config.clone());
    let pt = builder.add_virtual_proof_with_pis(inner_cd);

    let inner_data = builder.add_virtual_verifier_data(inner_cd.config.fri_config.cap_height);

    builder.verify_proof::<InnerC>(&pt, &inner_data, inner_cd);
    builder.print_gate_counts(0);

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

    let mut pw = PartialWitness::new();
    pw.set_proof_with_pis_target(&pt, inner_proof)?;
    pw.set_verifier_data_target(&inner_data, inner_vd)?;

    let mut timing = TimingTree::new("prove", Level::Debug);
    let proof = prove::<F, C, D>(&data.prover_only, &data.common, pw, &mut timing)?;
    timing.print();

    data.verify(proof.clone())?;

    Ok((proof, data.verifier_only, data.common))
}

/// Test serialization and print some size info.
fn test_serialization<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>(
    proof: &ProofWithPublicInputs<F, C, D>,
    vd: &VerifierOnlyCircuitData<C, D>,
    common_data: &CommonCircuitData<F, D>,
) -> Result<()> {
    let proof_bytes = proof.to_bytes();
    info!("Proof length: {} bytes", proof_bytes.len());
    let proof_from_bytes = ProofWithPublicInputs::from_bytes(proof_bytes, common_data)?;
    assert_eq!(proof, &proof_from_bytes);

    let now = std::time::Instant::now();
    let compressed_proof = proof.clone().compress(&vd.circuit_digest, common_data)?;
    let decompressed_compressed_proof = compressed_proof
        .clone()
        .decompress(&vd.circuit_digest, common_data)?;
    info!("{:.4}s to compress proof", now.elapsed().as_secs_f64());
    assert_eq!(proof, &decompressed_compressed_proof);

    let compressed_proof_bytes = compressed_proof.to_bytes();
    info!(
        "Compressed proof length: {} bytes",
        compressed_proof_bytes.len()
    );
    let compressed_proof_from_bytes =
        CompressedProofWithPublicInputs::from_bytes(compressed_proof_bytes, common_data)?;
    assert_eq!(compressed_proof, compressed_proof_from_bytes);

    let gate_serializer = DefaultGateSerializer;
    let common_data_bytes = common_data
        .to_bytes(&gate_serializer)
        .map_err(|_| anyhow::Error::msg("CommonCircuitData serialization failed."))?;
    info!(
        "Common circuit data length: {} bytes",
        common_data_bytes.len()
    );
    let common_data_from_bytes =
        CommonCircuitData::<F, D>::from_bytes(common_data_bytes, &gate_serializer)
            .map_err(|_| anyhow::Error::msg("CommonCircuitData deserialization failed."))?;
    assert_eq!(common_data, &common_data_from_bytes);

    Ok(())
}

pub fn benchmark_function(
    config: &CircuitConfig,
    log2_inner_size: usize,
    lookup_type: u64,
) -> Result<()> {
    const D: usize = 2;
    type C = PoseidonGoldilocksConfig;
    type F = <C as GenericConfig<D>>::F;

    let dummy_proof_function = match lookup_type {
        0 => dummy_proof::<F, C, D>,
        1 => dummy_lookup_proof::<F, C, D>,
        2 => dummy_many_rows_proof::<F, C, D>,
        _ => dummy_proof::<F, C, D>,
    };

    let name = match lookup_type {
        0 => "proof",
        1 => "one lookup proof",
        2 => "multiple lookups proof",
        _ => "proof",
    };
    // Start with a dummy proof of specified size
    let inner = dummy_proof_function(config, log2_inner_size)?;
    let (_, _, common_data) = &inner;
    info!(
        "Initial {} degree {} = 2^{}",
        name,
        common_data.degree(),
        common_data.degree_bits()
    );

    // Recursively verify the proof
    let middle = recursive_proof::<F, C, C, D>(&inner, config, None)?;
    let (_, _, common_data) = &middle;
    info!(
        "Single recursion {} degree {} = 2^{}",
        name,
        common_data.degree(),
        common_data.degree_bits()
    );

    // Add a second layer of recursion to shrink the proof size further
    let outer = recursive_proof::<F, C, C, D>(&middle, config, None)?;
    let (proof, vd, common_data) = &outer;
    info!(
        "Double recursion {} degree {} = 2^{}",
        name,
        common_data.degree(),
        common_data.degree_bits()
    );

    test_serialization(proof, vd, common_data)?;

    Ok(())
}

fn main() -> Result<()> {
    // Parse command line arguments, see `--help` for details.
    let options = Options::from_args_safe()?;
    // Initialize logging
    let mut builder = env_logger::Builder::from_default_env();
    builder.parse_filters(&options.log_filter);
    builder.format_timestamp(None);
    match options.verbose {
        0 => &mut builder,
        1 => builder.filter_level(LevelFilter::Info),
        2 => builder.filter_level(LevelFilter::Debug),
        _ => builder.filter_level(LevelFilter::Trace),
    };
    builder.try_init()?;

    // Initialize randomness source
    let rng_seed = options.seed.unwrap_or_else(|| OsRng.next_u64());
    info!("Using random seed {rng_seed:16x}");
    let _rng = ChaCha8Rng::seed_from_u64(rng_seed);
    // TODO: Use `rng` to create deterministic runs

    let num_cpus = num_cpus::get();
    let threads = options.threads.unwrap_or(num_cpus..=num_cpus);

    let config = CircuitConfig::standard_recursion_config();

    for log2_inner_size in options.size {
        // Since the `size` is most likely to be an unbounded range we make that the outer iterator.
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
                    // Run the benchmark. `options.lookup_type` determines which benchmark to run.
                    benchmark_function(&config, log2_inner_size, options.lookup_type)
                })?;
        }
    }

    Ok(())
}

fn parse_hex_u64(src: &str) -> Result<u64, ParseIntError> {
    let src = src.strip_prefix("0x").unwrap_or(src);
    u64::from_str_radix(src, 16)
}

fn parse_range_usize(src: &str) -> Result<RangeInclusive<usize>, ParseIntError> {
    if let Some((left, right)) = src.split_once("..=") {
        Ok(RangeInclusive::new(
            usize::from_str(left)?,
            usize::from_str(right)?,
        ))
    } else if let Some((left, right)) = src.split_once("..") {
        Ok(RangeInclusive::new(
            usize::from_str(left)?,
            if right.is_empty() {
                usize::MAX
            } else {
                usize::from_str(right)?.saturating_sub(1)
            },
        ))
    } else {
        let value = usize::from_str(src)?;
        Ok(RangeInclusive::new(value, value))
    }
}
