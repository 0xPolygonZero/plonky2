use std::time::Instant;

use rayon::prelude::*;

use plonky2::field::crandall_field::CrandallField;
use plonky2::field::field::Field;
use plonky2::polynomial::polynomial::PolynomialValues;

type F = CrandallField;

// 113 wire polys, 3 Z polys, 4 parts of quotient poly.
const PROVER_POLYS: usize = 113 + 3 + 4;

fn main() {
    const DEGREE: usize = 1 << 13;
    const RATE_BITS: usize = 3;

    let start = Instant::now();
    (0usize..PROVER_POLYS).into_par_iter().for_each(|i| {
        let mut values = vec![F::ZERO; DEGREE];
        for j in 0usize..DEGREE {
            values[j] = F::from_canonical_u64((i * j) as u64);
        }
        let poly_values = PolynomialValues::new(values);
        let start = Instant::now();
        let result = poly_values.lde(RATE_BITS);
        let duration = start.elapsed();
        println!("LDE took {:?}", duration);
        println!("LDE result: {:?}", result.values[0]);
    });
    println!("FFT overall took {:?}", start.elapsed());
}
