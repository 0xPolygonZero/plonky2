use std::thread;
use std::time::Instant;

use env_logger::Env;
use rayon::prelude::*;

use field::crandall_field::CrandallField;
use field::fft;

use crate::circuit_builder::CircuitBuilder;
use crate::circuit_data::CircuitConfig;
use crate::field::field::Field;
use crate::gates::constant::ConstantGate;
use crate::gates::gmimc::GMiMCGate;
use crate::hash::{GMIMC_CONSTANTS, GMIMC_ROUNDS};
use crate::polynomial::polynomial::PolynomialCoeffs;
use crate::witness::PartialWitness;

mod circuit_builder;
mod circuit_data;
mod vars;
mod field;
mod fri;
mod gadgets;
mod gates;
mod generator;
mod gmimc;
mod hash;
mod plonk_challenger;
mod plonk_common;
mod polynomial;
mod proof;
mod prover;
mod recursive_verifier;
mod rescue;
mod target;
mod util;
mod verifier;
mod wire;
mod witness;

// 112 wire polys, 3 Z polys, 4 parts of quotient poly.
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

    // field_search()
}

fn bench_field_mul<F: Field>() {
    let m = F::from_canonical_u64(12345678901234567890);
    let mut x = F::ONE;
    let start = Instant::now();
    let num_muls = 2000000000;
    for _ in 0..num_muls {
        x *= m;
    }
    let duration = start.elapsed();
    println!("result {:?}", x);
    println!("took {:?}", duration);
    println!("avg {:?}ns", duration.as_secs_f64() * 1e9 / (num_muls as f64));
}

fn bench_prove<F: Field>() {
    let gmimc_gate = GMiMCGate::<F, GMIMC_ROUNDS>::with_automatic_constants();

    let config = CircuitConfig {
        num_wires: 120,
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

fn bench_gmimc<F: Field>() {
    const THREADS: usize = 12;
    const LDE_BITS: i32 = 3;
    const W: usize = 13;
    let hashes_per_poly = 1 << (13 + LDE_BITS);
    let threads = (0..THREADS).map(|_i| {
        thread::spawn(move || {
            let mut x = [F::ZERO; W];
            for i in 0..W {
                x[i] = F::from_canonical_u64((i as u64) * 123456 + 789);
            }

            let hashes_per_thread = hashes_per_poly * PROVER_POLYS / THREADS;
            let start = Instant::now();
            for _ in 0..hashes_per_thread {
                x = gmimc::gmimc_permute_array::<_, W, GMIMC_ROUNDS>(x, GMIMC_CONSTANTS);
            }
            let duration = start.elapsed();
            println!("took {:?}", duration);
            println!("avg {:?}us", duration.as_secs_f64() * 1e6 / (hashes_per_thread as f64));
            println!("result {:?}", x);
        })
    }).collect::<Vec<_>>();

    for t in threads {
        t.join().expect("oops");
    }
}

fn bench_fft() {
    let degree = 1 << 13;
    let lde_bits = 3;
    let lde_size = degree << lde_bits;
    println!("{} << {} = {}", degree, lde_bits, lde_size);

    let start = Instant::now();
    (0usize..PROVER_POLYS).into_par_iter().for_each(|i| {
        let mut coeffs = vec![CrandallField::ZERO; lde_size];
        for j in 0usize..lde_size {
            coeffs[j] = CrandallField((i * j) as u64);
        }

        let start = Instant::now();
        let result = fft::fft(PolynomialCoeffs { coeffs });
        let duration = start.elapsed();
        println!("FFT took {:?}", duration);
        println!("FFT result: {:?}", result.values[0]);
    });
    println!("FFT overall took {:?}", start.elapsed());
}
