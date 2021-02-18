use std::thread;
use std::time::Instant;

use rayon::prelude::*;

use field::crandall_field::CrandallField;
use field::fft;
use field::fft::fft_precompute;

use crate::field::field::Field;
use crate::util::log2_ceil;

mod circuit_data;
mod constraint_polynomial;
mod field;
mod fri;
mod gates;
mod generator;
mod gmimc;
mod proof;
mod prover;
mod rescue;
mod target;
mod util;
mod verifier;
mod wire;
mod witness;

// 12 wire polys, 3 Z polys, 4 parts of quotient poly.
const PROVER_POLYS: usize = 101 + 3 + 4; // TODO: Check

fn main() {
    let overall_start = Instant::now();

    // bench_fft();
    println!();
    bench_gmimc::<CrandallField>();

    let overall_duration = overall_start.elapsed();
    println!("Overall time: {:?}", overall_duration);

    // field_search()
}

fn bench_gmimc<F: Field>() {
    let threads = 12;
    // let hashes_per_poly = 623328;
    // let hashes_per_poly = 1 << log2_ceil(hashes_per_poly);
    let hashes_per_poly = 1 << (13 + 3);
    let threads = (0..threads).map(|_i| thread::spawn(move || {
        let mut x = [F::ZERO; 12];
        for i in 0..12 {
            x[i] = F::from_canonical_u64((i as u64) * 123456 + 789);
        }

        let hashes_per_thread = hashes_per_poly * PROVER_POLYS / threads;
        let start = Instant::now();
        for _ in 0..hashes_per_thread {
            x = gmimc::gmimc_permute(x);
        }
        let duration = start.elapsed();
        println!("took {:?}", duration);
        println!("avg {:?}us", duration.as_secs_f64() * 1e6 / (hashes_per_thread as f64));
        println!("result {:?}", x);
    })).collect::<Vec<_>>();

    for t in threads {
        t.join().expect("oops");
    }
}

fn bench_fft() {
    let degree = 1 << log2_ceil(77916);
    let lde_bits = 4;
    let lde_size = degree << lde_bits;
    let precomputation = fft_precompute(lde_size);
    println!("{} << {} = {}", degree, lde_bits, lde_size);

    let start = Instant::now();
    (0usize..PROVER_POLYS).into_par_iter().for_each(|i| {
        let mut coeffs = vec![CrandallField::ZERO; lde_size];
        for j in 0usize..lde_size {
            coeffs[j] = CrandallField((i * j) as u64);
        }

        let start = Instant::now();
        let result = fft::fft_with_precomputation_power_of_2(coeffs, &precomputation);
        let duration = start.elapsed();
        println!("FFT took {:?}", duration);
        println!("FFT result: {:?}", result[0]);
    });
    println!("FFT overall took {:?}", start.elapsed());
}
