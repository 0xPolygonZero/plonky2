//! Performs several exponentiations in an interleaved loop, to enable parallelism on the core.

use std::time::Instant;

use plonky2::field::crandall_field::CrandallField;
use plonky2::field::field::Field;

type F = CrandallField;

/// The number of exponentiations to perform in parallel.
const WIDTH: usize = 6;

const EXPONENT: usize = 1000000000;

fn main() {
    let mut bases = [F::ZERO; WIDTH];
    for i in 0..WIDTH {
        bases[i] = F::rand();
    }
    let mut state = [F::ONE; WIDTH];

    let start = Instant::now();
    for _ in 0..EXPONENT {
        for i in 0..WIDTH {
            state[i] *= bases[i];
        }
    }
    let duration = start.elapsed();

    println!("Result: {:?}", state);
    println!(
        "Average field mul: {:?}ns",
        duration.as_secs_f64() * 1e9 / (WIDTH * EXPONENT) as f64
    );
}
