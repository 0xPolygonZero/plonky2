use std::time::Instant;

use plonky2::field::crandall_field::CrandallField as F;
use plonky2::field::field_types::Field;

use plonky2::hash::gmimc::gmimc_permute_array;
use plonky2::hash::hashing::{GMIMC_CONSTANTS, GMIMC_ROUNDS};
use plonky2::hash::rescue::rescue;
use plonky2::hash::poseidon::{poseidon8, poseidon8_naive, poseidon12, poseidon12_naive};


#[inline]
fn gmimc12_hash(x: [F; 12]) -> [F; 12] {
    gmimc_permute_array::<_, 12, GMIMC_ROUNDS>(x, GMIMC_CONSTANTS)
}

#[inline]
fn rescue12_hash(x: [F; 12]) -> [F; 12] {
    rescue(x)
}

#[inline]
fn poseidon12_hash(x: [F; 12]) -> [F; 12] {
    poseidon12(x)
}

#[inline]
fn poseidon12_naive_hash(x: [F; 12]) -> [F; 12] {
    poseidon12_naive(x)
}

#[inline]
fn gmimc8_hash(x: [F; 8]) -> [F; 8] {
    gmimc_permute_array::<_, 8, GMIMC_ROUNDS>(x, GMIMC_CONSTANTS)
}

#[inline]
fn poseidon8_hash(x: [F; 8]) -> [F; 8] {
    poseidon8(x)
}

#[inline]
fn poseidon8_naive_hash(x: [F; 8]) -> [F; 8] {
    poseidon8_naive(x)
}


fn bench_hash<const W: usize>(name: &str, hash: fn([F; W])-> [F; W], gmimc_tm: &mut f64) {
    // 113 wire polys, 3 Z polys, 4 parts of quotient poly.
    const PROVER_POLYS: usize = 113 + 3 + 4;
    const LDE_BITS: i32 = 3;
    const HASHES_PER_POLY: usize = 1 << (13 + LDE_BITS) / 6;
    const N_HASHES: usize = HASHES_PER_POLY * PROVER_POLYS;

    let mut input = [F::ZERO; W];
    for i in 0..W {
        input[i] = F::from_canonical_u64((i as u64) * 123456 + 789);
    }

    print!("{:16}", name);

    let mut x = input;
    let start = Instant::now();
    for _ in 0..N_HASHES {
        x = hash(x);
    }
    let duration = start.elapsed();

    let tm = duration.as_micros() as f64 / N_HASHES as f64;

    if *gmimc_tm == 0.0 {
        *gmimc_tm = tm;
    }

    println!(" {:5.2}  {:5.2}", tm, tm / *gmimc_tm);
}

fn main() {
    println!(" -- Width 8 (time μs, slowdown wrt GMiMC)--");
    let mut tm: f64 = 0.0;
    bench_hash("GMiMC", gmimc8_hash, &mut tm);
    bench_hash("Poseidon", poseidon8_hash, &mut tm);
    bench_hash("Poseidon naive", poseidon8_naive_hash, &mut tm);

    println!("\n -- Width 12 (time μs, slowdown wrt GMiMC) --");
    let mut tm: f64 = 0.0;
    bench_hash("GMiMC", gmimc12_hash, &mut tm);
    bench_hash("Poseidon", poseidon12_hash, &mut tm);
    bench_hash("Poseidon naive", poseidon12_naive_hash, &mut tm);
    bench_hash("Rescue", rescue12_hash, &mut tm);
}
