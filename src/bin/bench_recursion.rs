use env_logger::Env;
use plonky2::circuit_builder::CircuitBuilder;
use plonky2::circuit_data::CircuitConfig;
use plonky2::field::crandall_field::CrandallField;
use plonky2::field::extension_field::Extendable;
use plonky2::field::field::Field;
use plonky2::fri::FriConfig;
use plonky2::gates::constant::ConstantGate;
use plonky2::gates::gmimc::GMiMCGate;
use plonky2::hash::GMIMC_ROUNDS;
use plonky2::prover::PLONK_BLINDING;
use plonky2::witness::PartialWitness;

fn main() {
    // Set the default log filter. This can be overridden using the `RUST_LOG` environment variable,
    // e.g. `RUST_LOG=debug`.
    // We default to debug for now, since there aren't many logs anyway, but we should probably
    // change this to info or warn later.
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    bench_prove::<CrandallField, 4>();

    // bench_field_mul::<CrandallField>();

    // bench_fft();
    println!();
    // bench_gmimc::<CrandallField>();
}

fn bench_prove<F: Field + Extendable<D>, const D: usize>() {
    let gmimc_gate = GMiMCGate::<F, D, GMIMC_ROUNDS>::with_automatic_constants();

    let config = CircuitConfig {
        num_wires: 134,
        num_routed_wires: 12,
        security_bits: 128,
        rate_bits: 3,
        num_challenges: 3,
        fri_config: FriConfig {
            proof_of_work_bits: 1,
            rate_bits: 3,
            reduction_arity_bits: vec![1],
            num_query_rounds: 1,
            blinding: PLONK_BLINDING.to_vec(),
        },
    };

    let mut builder = CircuitBuilder::<F, D>::new(config);

    for _ in 0..10000 {
        builder.add_gate_no_constants(gmimc_gate.clone());
    }

    builder.add_gate(ConstantGate::get(), vec![F::NEG_ONE]);

    // for _ in 0..(40 * 5) {
    //     builder.add_gate(
    //         FriConsistencyGate::new(2, 3, 13),
    //         vec![F::primitive_root_of_unity(13)]);
    // }

    let prover = builder.build_prover();
    let inputs = PartialWitness::new();
    prover.prove(inputs);
}
