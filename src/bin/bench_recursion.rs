use std::thread;
use std::time::Instant;

use env_logger::Env;
use rayon::prelude::*;

use plonky2::circuit_builder::CircuitBuilder;
use plonky2::circuit_data::CircuitConfig;
use plonky2::field::crandall_field::CrandallField;
use plonky2::field::fft;
use plonky2::field::field::Field;
use plonky2::gates::constant::ConstantGate;
use plonky2::gates::gmimc::GMiMCGate;
use plonky2::gmimc::gmimc_permute_array;
use plonky2::hash::{GMIMC_CONSTANTS, GMIMC_ROUNDS};
use plonky2::polynomial::polynomial::PolynomialCoeffs;
use plonky2::witness::PartialWitness;

// 113 wire polys, 3 Z polys, 4 parts of quotient poly.
const PROVER_POLYS: usize = 113 + 3 + 4;

fn main() {
    // Set the default log filter. This can be overridden using the `RUST_LOG` environment variable,
    // e.g. `RUST_LOG=debug`.
    // We default to debug for now, since there aren't many logs anyway, but we should probably
    // change this to info or warn later.
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    bench_prove::<CrandallField>();

    // bench_field_mul::<CrandallField>();

    // bench_fft();
    println!();
    // bench_gmimc::<CrandallField>();
}

fn bench_prove<F: Field>() {
    let gmimc_gate = GMiMCGate::<F, GMIMC_ROUNDS>::with_automatic_constants();

    let config = CircuitConfig {
        num_wires: 134,
        num_routed_wires: 12,
        security_bits: 128,
        rate_bits: 3,
        num_checks: 3,
    };

    let mut builder = CircuitBuilder::<F>::new(config);

    for _ in 0..5000 {
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
