use std::time::Instant;

use plonky2::field::field_types::Field;
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::polynomial::polynomial::PolynomialValues;
use rayon::prelude::*;

type F = GoldilocksField;

// This is an estimate of how many LDEs the prover will compute. The biggest component, 86, comes
// from wire polynomials which "store" the outputs of S-boxes in our Poseidon gate.
const NUM_LDES: usize = 8 + 8 + 3 + 86 + 3 + 8;

const DEGREE: usize = 1 << 14;

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
