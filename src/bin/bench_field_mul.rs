//! Performs a single exponentiation.

use std::time::Instant;

use plonky2::field::crandall_field::CrandallField;
use plonky2::field::field::Field;

type F = CrandallField;

const EXPONENT: usize = 1000000000;

fn main() {
    let base = F::rand();
    let mut state = F::ONE;

    let start = Instant::now();
    for _ in 0..EXPONENT {
        state *= base;
    }
    let duration = start.elapsed();

    println!("Result: {:?}", state);
    println!(
        "Average field mul: {:?}ns",
        duration.as_secs_f64() * 1e9 / EXPONENT as f64
    );
}
