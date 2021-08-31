// use core::arch::x86_64::*;
// use rand::{thread_rng, Rng};
// use std::time::Instant;

// use plonky2::field::crandall_field_vec::CrandallFieldVec;
// use plonky2::field::goldilocks_field_vec::GoldilocksFieldVec;

// // type F = GoldilocksFieldVec;
// type F = CrandallFieldVec;

// /// The number of exponentiations to perform in parallel.
// const WIDTH: usize = 4;

// const EXPONENT: usize = 375_000_000;

// fn main() {

//     let mut bases;
//     let mut rng = rand::thread_rng();
//     unsafe {
//         bases = [_mm256_undefined_si256(); WIDTH];
//         for base_i in bases.iter_mut() {
//             *base_i = _mm256_setr_epi64x(
//                 rng.gen::<i64>(),
//                 rng.gen::<i64>(),
//                 rng.gen::<i64>(),
//                 rng.gen::<i64>());
//         }
//     }
//     let mut state;
//     unsafe {
//         state = [_mm256_set1_epi64x(1); WIDTH];
//     }

//     let start = Instant::now();
//     for _ in 0..EXPONENT {
//         for i in 0..WIDTH {
//             unsafe {
//                 state[i] = F::mul(state[i], bases[i]);
//             }
//         }
//     }
//     let duration = start.elapsed();

//     println!("Result: {:?}", state);
//     println!(
//         "Average field mul: {:?}ns",
//         duration.as_secs_f64() * 1e9 / (4 * WIDTH * EXPONENT) as f64
//     );
// }
