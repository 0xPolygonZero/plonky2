use anyhow::Result;
use env_logger::Env;
use log::info;
use plonky2::field::extension_field::Extendable;
use plonky2::field::field_types::RichField;
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::fri::reduction_strategies::FriReductionStrategy;
use plonky2::fri::FriConfig;
use plonky2::hash::hashing::SPONGE_WIDTH;
use plonky2::iop::witness::PartialWitness;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::circuit_data::CircuitConfig;
use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};

fn main() -> Result<()> {
    // Set the default log filter. This can be overridden using the `RUST_LOG` environment variable,
    // e.g. `RUST_LOG=debug`.
    // We default to debug for now, since there aren't many logs anyway, but we should probably
    // change this to info or warn later.
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    bench_prove::<PoseidonGoldilocksConfig, 2>()
}

fn bench_prove<C: GenericConfig<D>, const D: usize>() -> Result<()> {
    let config = CircuitConfig {
        num_wires: 126,
        num_routed_wires: 33,
        constant_gate_size: 6,
        security_bits: 128,
        rate_bits: 3,
        num_challenges: 3,
        zero_knowledge: false,
        cap_height: 1,
        fri_config: FriConfig {
            proof_of_work_bits: 15,
            reduction_strategy: FriReductionStrategy::ConstantArityBits(3, 5),
            num_query_rounds: 35,
        },
    };

    let inputs = PartialWitness::new();
    let mut builder = CircuitBuilder::<C::F, D>::new(config);

    let zero = builder.zero();
    let zero_ext = builder.zero_extension();

    let mut state = [zero; SPONGE_WIDTH];
    for _ in 0..10000 {
        state = builder.permute::<<C as GenericConfig<D>>::InnerHasher>(state);
    }

    // Random other gates.
    builder.add(zero, zero);
    builder.add_extension(zero_ext, zero_ext);

    let circuit = builder.build::<C>();
    let proof_with_pis = circuit.prove(inputs)?;
    let proof_bytes = serde_cbor::to_vec(&proof_with_pis).unwrap();
    info!("Proof length: {} bytes", proof_bytes.len());
    circuit.verify(proof_with_pis)
}
