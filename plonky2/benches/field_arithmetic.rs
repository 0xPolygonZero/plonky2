use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use plonky2::field::extension_field::quartic::QuarticExtension;
use plonky2::field::extension_field::quintic::QuinticExtension;
use plonky2::field::field_types::Field;
use plonky2::field::goldilocks_field::GoldilocksField;
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
            |mut x| {
                for _ in 0..100 {
                    x = x * x;
                }
                x
            },
            BatchSize::SmallInput,
        )
    });

    c.bench_function(&format!("sqr-throughput<{}>", type_name::<F>()), |b| {
        b.iter_batched(
            || (F::rand(), F::rand(), F::rand(), F::rand()),
            |(mut x, mut y, mut z, mut w)| {
                for _ in 0..25 {
                    (x, y, z, w) = (x.square(), y.square(), z.square(), w.square());
                }
                (x, y, z, w)
            },
            BatchSize::SmallInput,
        )
    });

    c.bench_function(&format!("sqr-latency<{}>", type_name::<F>()), |b| {
        b.iter_batched(
            || F::rand(),
            |mut x| {
                for _ in 0..100 {
                    x = x.square();
                }
                x
            },
            BatchSize::SmallInput,
        )
    });

    c.bench_function(&format!("add-throughput<{}>", type_name::<F>()), |b| {
        b.iter_batched(
            || {
                (
                    F::rand(),
                    F::rand(),
                    F::rand(),
                    F::rand(),
                    F::rand(),
                    F::rand(),
                    F::rand(),
                    F::rand(),
                    F::rand(),
                    F::rand(),
                )
            },
            |(mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h, mut i, mut j)| {
                for _ in 0..10 {
                    (a, b, c, d, e, f, g, h, i, j) = (
                        a + b,
                        b + c,
                        c + d,
                        d + e,
                        e + f,
                        f + g,
                        g + h,
                        h + i,
                        i + j,
                        j + a,
                    );
                }
                (a, b, c, d, e, f, g, h, i, j)
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

    c.bench_function(
        &format!("batch_multiplicative_inverse-tiny<{}>", type_name::<F>()),
        |b| {
            b.iter_batched(
                || (0..2).into_iter().map(|_| F::rand()).collect::<Vec<_>>(),
                |x| F::batch_multiplicative_inverse(&x),
                BatchSize::SmallInput,
            )
        },
    );

    c.bench_function(
        &format!("batch_multiplicative_inverse-small<{}>", type_name::<F>()),
        |b| {
            b.iter_batched(
                || (0..4).into_iter().map(|_| F::rand()).collect::<Vec<_>>(),
                |x| F::batch_multiplicative_inverse(&x),
                BatchSize::SmallInput,
            )
        },
    );

    c.bench_function(
        &format!("batch_multiplicative_inverse-medium<{}>", type_name::<F>()),
        |b| {
            b.iter_batched(
                || (0..16).into_iter().map(|_| F::rand()).collect::<Vec<_>>(),
                |x| F::batch_multiplicative_inverse(&x),
                BatchSize::SmallInput,
            )
        },
    );

    c.bench_function(
        &format!("batch_multiplicative_inverse-large<{}>", type_name::<F>()),
        |b| {
            b.iter_batched(
                || (0..256).into_iter().map(|_| F::rand()).collect::<Vec<_>>(),
                |x| F::batch_multiplicative_inverse(&x),
                BatchSize::LargeInput,
            )
        },
    );

    c.bench_function(
        &format!("batch_multiplicative_inverse-huge<{}>", type_name::<F>()),
        |b| {
            b.iter_batched(
                || {
                    (0..65536)
                        .into_iter()
                        .map(|_| F::rand())
                        .collect::<Vec<_>>()
                },
                |x| F::batch_multiplicative_inverse(&x),
                BatchSize::LargeInput,
            )
        },
    );
}

use rand::{thread_rng, Rng};
use plonky2::field::goldilocks_field::{ext5_mul, ext5_sqr};

fn rand_u64<R: Rng>(rng: &mut R) -> u64 {
    rng.gen_range(0 .. 0xFFFFFFFFFFFFFFFF)
}

fn rand_u64_5<R: Rng>(rng: &mut R) -> [u64; 5] {
    [rand_u64(rng), rand_u64(rng), rand_u64(rng), rand_u64(rng), rand_u64(rng)]
}

fn from_goldi(a: &[GoldilocksField; 5]) -> [u64; 5] {
    [a[0].0, a[1].0, a[2].0, a[3].0, a[4].0]
}

fn criterion_benchmark(c: &mut Criterion) {
    bench_field::<GoldilocksField>(c);
    bench_field::<QuarticExtension<GoldilocksField>>(c);
    bench_field::<QuinticExtension<GoldilocksField>>(c);

    let mut rng = thread_rng();

    c.bench_function("ext5_mul-throughput", |b| {
        b.iter_batched(
            || (rand_u64_5(&mut rng), rand_u64_5(&mut rng), rand_u64_5(&mut rng), rand_u64_5(&mut rng)),
            |(mut x, mut y, mut z, mut w)| {
                for _ in 0..25 {
                    let (xx, yy, zz, ww) = (ext5_mul(x, y), ext5_mul(y, z), ext5_mul(z, w), ext5_mul(w, x));
                    (x, y, z, w) = (from_goldi(&xx), from_goldi(&yy), from_goldi(&zz), from_goldi(&ww));
                }
                (x, y, z, w)
            },
            BatchSize::SmallInput,
        )
    });

    c.bench_function("ext5_mul-latency", |b| {
        b.iter_batched(
            || rand_u64_5(&mut rng),
            |mut x| {
                for _ in 0..100 {
                    let y = ext5_mul(x, x);
                    x = from_goldi(&y);
                }
                x
            },
            BatchSize::SmallInput,
        )
    });

    c.bench_function("ext5_sqr-throughput", |b| {
        b.iter_batched(
            || (rand_u64_5(&mut rng), rand_u64_5(&mut rng), rand_u64_5(&mut rng), rand_u64_5(&mut rng)),
            |(mut x, mut y, mut z, mut w)| {
                for _ in 0..25 {
                    let (xx, yy, zz, ww) = (ext5_sqr(x), ext5_sqr(y), ext5_sqr(z), ext5_sqr(w));
                    (x, y, z, w) = (from_goldi(&xx), from_goldi(&yy), from_goldi(&zz), from_goldi(&ww));
                }
                (x, y, z, w)
            },
            BatchSize::SmallInput,
        )
    });

    c.bench_function("ext5_sqr-latency", |b| {
        b.iter_batched(
            || rand_u64_5(&mut rng),
            |mut x| {
                for _ in 0..100 {
                    let y = ext5_sqr(x);
                    x = from_goldi(&y);
                }
                x
            },
            BatchSize::SmallInput,
        )
    });

}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
