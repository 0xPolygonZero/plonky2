use std::time::Instant;

use plonky2::field::crandall_field::CrandallField;
use plonky2::field::field::Field;

type F = CrandallField;

fn main() {
    let m = F::from_canonical_u64(12345678901234567890);
    let mut x = F::ONE;
    let start = Instant::now();
    let num_muls = 2000000000;
    for _ in 0..num_muls {
        x *= m;
    }
    let duration = start.elapsed();

    println!("result {:?}", x);
    println!("took {:?}", duration);
    println!(
        "avg {:?}ns",
        duration.as_secs_f64() * 1e9 / (num_muls as f64)
    );
}
