use std::time::Instant;

use plonky2::field::crandall_field::CrandallField as F;
use plonky2::field::field_types::Field;
use plonky2::hash::gmimc::GMiMC;
use plonky2::hash::poseidon::Poseidon;
use plonky2::hash::rescue::rescue;

#[inline]
fn gmimc_hash<const W: usize>(x: [F; W]) -> [F; W]
where
    F: GMiMC<W>,
{
    F::gmimc_permute(x)
}

#[inline]
fn rescue_hash(x: [F; 12]) -> [F; 12] {
    rescue(x)
}

#[inline]
fn poseidon8_hash(x: [F; 8]) -> [F; 8] {
    F::poseidon(x)
}

#[inline]
fn poseidon8_naive_hash(x: [F; 8]) -> [F; 8] {
    F::poseidon_naive(x)
}

#[inline]
fn poseidon12_hash(x: [F; 12]) -> [F; 12] {
    F::poseidon(x)
}

#[inline]
fn poseidon12_naive_hash(x: [F; 12]) -> [F; 12] {
    F::poseidon_naive(x)
}

fn bench_hash<const W: usize>(name: &str, hash: fn([F; W]) -> [F; W], gmimc_tm: &mut f64) {
    // 113 wire polys, 3 Z polys, 4 parts of quotient poly.
    const PROVER_POLYS: usize = 113 + 3 + 4;
    const LDE_BITS: i32 = 3;
    const HASHES_PER_POLY: usize = 1 << (13 + LDE_BITS) / 6;
    const N_HASHES: usize = HASHES_PER_POLY * PROVER_POLYS * 2000;

    let mut input = [F::ZERO; W];
    for i in 0..W {
        input[i] = F::from_canonical_u64((i as u64) * 123456 + 789);
    }

    print!("{}", name);

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

    println!(" {:5.2}", tm);
}

fn main() {
    println!(" -- Width 8 (time μs, slowdown wrt GMiMC)--");
    let mut tm: f64 = 0.0;
    // bench_hash("GMiMC", gmimc_hash::<8>, &mut tm); // Not implemented yet.
    // bench_hash("08F", poseidon8_hash, &mut tm);
    // bench_hash("08N", poseidon8_naive_hash, &mut tm);

    // let mut tm: f64 = 0.0;
    // bench_hash("GMiMC", gmimc_hash::<12>, &mut tm);
    bench_hash("12F", poseidon12_hash, &mut tm);
    // bench_hash("12N", poseidon12_naive_hash, &mut tm);
    // bench_hash("Rescue", rescue_hash, &mut tm);
}
