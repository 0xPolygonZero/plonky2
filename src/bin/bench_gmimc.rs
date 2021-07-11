use std::time::Instant;

use plonky2::field::crandall_field::CrandallField as F;
use plonky2::field::field::Field;

use plonky2::gmimc::gmimc_permute_array;
use plonky2::hash::{GMIMC_CONSTANTS, GMIMC_ROUNDS};
use plonky2::rescue::rescue;

// TODO: I was getting 560us for this in the old version

fn bench_hash<const W: usize>(name: &str, input: &[F; W]) {
    // 113 wire polys, 3 Z polys, 4 parts of quotient poly.
    const PROVER_POLYS: usize = 113 + 3 + 4;
    const LDE_BITS: i32 = 3;
    const HASHES_PER_POLY: usize = 1 << (13 + LDE_BITS) / 6;
    const N_HASHES: usize = HASHES_PER_POLY * PROVER_POLYS;

    println!("Bench for {}:", name);

    let mut x = *input;
    let start = Instant::now();
    for _ in 0..N_HASHES {
        x = gmimc_permute_array::<_, W, GMIMC_ROUNDS>(x, GMIMC_CONSTANTS);
    }
    let duration = start.elapsed();

    println!("--- result {:?}", x);
    println!("--- took {:?}", duration);
    println!("--- avg {:?}μs", (duration.as_micros() as f64 / N_HASHES as f64));
}

fn main() {
    const W: usize = 12;

    let mut x = [F::ZERO; W];
    for i in 0..W {
        x[i] = F::from_canonical_u64((i as u64) * 123456 + 789);
    }

    bench_hash("GMiMC", &x);
}
