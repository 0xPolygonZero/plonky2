#![feature(destructuring_assignment)]

use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use plonky2::field::crandall_field::CrandallField;
use plonky2::field::extension_field::quartic::QuarticCrandallField;
use plonky2::field::field_types::Field;
use tynm::type_name;

pub(crate) fn bench_field<F: Field>(c: &mut Criterion) {
    c.bench_function(&format!("mul-throughput<{}>", type_name::<F>()), |b| {
        b.iter_batched(
            || (F::rand(), F::rand(), F::rand(), F::rand()),
            |(mut x, mut y, mut z, mut w)| {
                for _ in 0..25 {
                    (x, y, z, w) = (x * y, y * z, z * w, w * x);
                }
                (x, y, z, w)
            },
            BatchSize::SmallInput,
        )
    });

    c.bench_function(&format!("mul-latency<{}>", type_name::<F>()), |b| {
        b.iter_batched(
            || F::rand(),
            |(mut x)| {
                for _ in 0..100 {
                    x = x * x;
                }
                x
            },
            BatchSize::SmallInput,
        )
    });

    c.bench_function(&format!("add-throughput<{}>", type_name::<F>()), |b| {
        b.iter_batched(
            || (F::rand(), F::rand(), F::rand(), F::rand()),
            |(mut x, mut y, mut z, mut w)| {
                for _ in 0..25 {
                    (x, y, z, w) = (x + y, y + z, z + w, w + x);
                }
                (x, y, z, w)
            },
            BatchSize::SmallInput,
        )
    });

    c.bench_function(&format!("add-latency<{}>", type_name::<F>()), |b| {
        b.iter_batched(
            || F::rand(),
            |mut x| {
                for _ in 0..100 {
                    x = x + x;
                }
                x
            },
            BatchSize::SmallInput,
        )
    });

    c.bench_function(&format!("try_inverse<{}>", type_name::<F>()), |b| {
        b.iter_batched(|| F::rand(), |x| x.try_inverse(), BatchSize::SmallInput)
    });
}

fn criterion_benchmark(c: &mut Criterion) {
    bench_field::<CrandallField>(c);
    bench_field::<QuarticCrandallField>(c);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
