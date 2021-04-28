//! Benchmark field modular multiplication in various scenarios

use std::time::Instant;

use plonky2::field::crandall_field::CrandallField;
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::field::proth_field::ProthField;
use plonky2::field::field::Field;

/// The number of exponentiations to perform in parallel.
const WIDTH: usize = 6;

const EXPONENT: usize = 1000000000;

/// Performs a single exponentiation.
fn run_bench_serial<F: Field>() {
    let base = F::rand();
    let mut state = F::ONE;

    let start = Instant::now();
    for _ in 0..EXPONENT {
        state *= base;
    }
    let duration = start.elapsed();

    println!("  result: {:?}", state);
    println!(
        "  average field mul: {:?}ns",
        duration.as_secs_f64() * 1e9 / EXPONENT as f64
    );
}


/// Performs several exponentiations in an interleaved loop, to enable parallelism on the core.
fn run_bench_interleaved<F: Field>() {
    let mut bases = [F::ZERO; WIDTH];
    for base_i in bases.iter_mut() {
        *base_i = F::rand();
    }
    let mut state = [F::ONE; WIDTH];

    let start = Instant::now();
    for _ in 0..EXPONENT {
        for i in 0..WIDTH {
            state[i] *= bases[i];
        }
    }
    let duration = start.elapsed();

    println!("  result: {:?}", state);
    println!(
        "  average field mul: {:?}ns",
        duration.as_secs_f64() * 1e9 / (WIDTH * EXPONENT) as f64
    );
}


fn run_bench<F: Field>(name: &str) {
    println!("Field: {}", name);
    println!("Serial:");
    run_bench_serial::<F>();
    println!("Interleaved:");
    run_bench_interleaved::<F>();
}

fn main() {
    run_bench::<CrandallField>("Crandall");
    run_bench::<GoldilocksField>("Goldilocks");
    run_bench::<ProthField>("Proth");
}
