use std::time::Instant;

use plonky2::field::crandall_field::CrandallField as F;
use plonky2::field::field_types::Field;

use plonky2::hash::gmimc::gmimc_permute_array;
use plonky2::hash::hashing::{GMIMC_CONSTANTS, GMIMC_ROUNDS};
use plonky2::hash::rescue::rescue;
use plonky2::hash::poseidon::{poseidon, poseidon_naive};

/// Number of elements in the hash input/state/result.
const W: usize = 12;

#[inline]
fn gmimc_hash(x: [F; W]) -> [F; W] {
    gmimc_permute_array::<_, W, GMIMC_ROUNDS>(x, GMIMC_CONSTANTS)
}

#[inline]
fn rescue_hash(x: [F; W]) -> [F; W] {
    rescue(x)
}

#[inline]
fn poseidon_hash(x: [F; W]) -> [F; W] {
    poseidon(x)
}

#[inline]
fn poseidon_naive_hash(x: [F; W]) -> [F; W] {
    poseidon_naive(x)
}


fn bench_hash(name: &str, hash: fn([F; W])-> [F; W], input: &[F; W]) {
    // 113 wire polys, 3 Z polys, 4 parts of quotient poly.
    const PROVER_POLYS: usize = 113 + 3 + 4;
    const LDE_BITS: i32 = 3;
    const HASHES_PER_POLY: usize = 1 << (13 + LDE_BITS) / 6;
    const N_HASHES: usize = HASHES_PER_POLY * PROVER_POLYS;

    println!("Bench for {}:", name);

    let mut x = *input;
    let start = Instant::now();
    for _ in 0..N_HASHES {
        x = hash(x);
    }
    let duration = start.elapsed();

    println!("--- result sum {:?}", x.iter().copied().sum::<F>());
    println!("--- took {:?}μs", duration.as_micros());
    println!("--- avg {:?}μs", (duration.as_micros() as f64 / N_HASHES as f64));
}

fn main() {
    let mut x = [F::ZERO; W];
    for i in 0..W {
        x[i] = F::from_canonical_u64((i as u64) * 123456 + 789);
    }

    bench_hash("GMiMC", gmimc_hash, &x);
    bench_hash("Rescue", rescue_hash, &x);
    bench_hash("Poseidon naive", poseidon_naive_hash, &x);
    bench_hash("Poseidon", poseidon_hash, &x);
}
