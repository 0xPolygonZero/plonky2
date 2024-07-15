mod allocator;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::field::polynomial::PolynomialCoeffs;
use plonky2::field::types::Field;
use tynm::type_name;

pub(crate) fn bench_ffts<F: Field>(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!("fft<{}>", type_name::<F>()));

    for size_log in [13, 14, 15, 16] {
        let size = 1 << size_log;
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            let coeffs = PolynomialCoeffs::new(F::rand_vec(size));
            b.iter(|| coeffs.clone().fft_with_options(None, None));
        });
    }
}

pub(crate) fn bench_ldes<F: Field>(c: &mut Criterion) {
    const RATE_BITS: usize = 3;

    let mut group = c.benchmark_group(format!("lde<{}>", type_name::<F>()));

    for size_log in [13, 14, 15, 16] {
        let orig_size = 1 << (size_log - RATE_BITS);
        let lde_size = 1 << size_log;

        group.bench_with_input(BenchmarkId::from_parameter(lde_size), &lde_size, |b, _| {
            let coeffs = PolynomialCoeffs::new(F::rand_vec(orig_size));
            b.iter(|| {
                let padded_coeffs = coeffs.lde(RATE_BITS);
                padded_coeffs.fft_with_options(Some(RATE_BITS), None)
            });
        });
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    bench_ffts::<GoldilocksField>(c);
    bench_ldes::<GoldilocksField>(c);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
