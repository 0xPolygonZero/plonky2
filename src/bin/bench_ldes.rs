use std::time::Instant;

use rayon::prelude::*;

use plonky2::field::crandall_field::CrandallField;
use plonky2::field::field::Field;
use plonky2::polynomial::polynomial::PolynomialValues;

type F = CrandallField;

// This is an estimate of how many LDEs the prover will compute. The biggest component, 86, comes
// from wire polynomials which "store" the outputs of S-boxes in our Poseidon gate.
const NUM_LDES: usize = 8 + 8 + 3 + 86 + 3 + 8;

const DEGREE: usize = 1 << 13;

const RATE_BITS: usize = 3;

fn main() {
    // We start with random polynomials.
    let all_poly_values = (0..NUM_LDES)
        .map(|_| PolynomialValues::new(F::rand_vec(DEGREE)))
        .collect::<Vec<_>>();

    let start = Instant::now();

    all_poly_values.into_par_iter().for_each(|poly_values| {
        let start = Instant::now();
        let lde = poly_values.lde(RATE_BITS);
        let duration = start.elapsed();
        println!("LDE took {:?}", duration);
        println!("LDE result: {:?}", lde.values[0]);
    });
    println!("All LDEs took {:?}", start.elapsed());
}
