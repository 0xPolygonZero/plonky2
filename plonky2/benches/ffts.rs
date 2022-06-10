mod allocator;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use plonky2::field::field_types::Field;
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::field::polynomial::PolynomialCoeffs;
use plonky2_field::polynomial::PolynomialValues;
use rayon::prelude::*;
use tynm::type_name;

const SIZE_LOG: usize = 23;
const SIZE: usize = 1 << SIZE_LOG;

pub(crate) fn bench_ffts<F: Field>(c: &mut Criterion) {
    let mut group = c.benchmark_group(&format!("fft<{}>", type_name::<F>()));
    group.sample_size(10);

    for num_polys in [64, 100, 128, 255] {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_polys),
            &num_polys,
            |b, _| {
                let coeffs = PolynomialCoeffs::new(F::rand_vec(SIZE));
                b.iter(|| {
                    (0..num_polys)
                        .into_par_iter()
                        .map(|_| coeffs.clone().fft_with_options(None, None))
                        .collect::<Vec<_>>()
                });
            },
        );
    }
}

pub(crate) fn bench_ldes<F: Field>(c: &mut Criterion) {
    const RATE_BITS: usize = 2;

    let mut group = c.benchmark_group(&format!("lde<{}>", type_name::<F>()));
    group.sample_size(10);

    for num_polys in [64, 100, 128, 255] {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_polys),
            &num_polys,
            |b, _| {
                let values = PolynomialValues::new(F::rand_vec(SIZE));
                b.iter(|| {
                    (0..num_polys)
                        .into_par_iter()
                        .map(|_| values.clone().lde_onto_coset(RATE_BITS))
                        .collect::<Vec<_>>()
                });
            },
        );
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    bench_ffts::<GoldilocksField>(c);
    bench_ldes::<GoldilocksField>(c);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
