use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use plonky2::field::crandall_field::CrandallField;
use plonky2::field::fft::FftStrategy;
use plonky2::field::field_types::Field;
use plonky2::polynomial::polynomial::PolynomialCoeffs;
use tynm::type_name;

pub(crate) fn bench_ffts<F: Field>(c: &mut Criterion, strategy: FftStrategy) {
    let mut group = c.benchmark_group(&format!("fft-{:?}<{}>", strategy, type_name::<F>()));

    for size_log in [13, 14, 15, 16] {
        let size = 1 << size_log;
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            let coeffs = PolynomialCoeffs::new(F::rand_vec(size));
            b.iter(|| coeffs.fft_with_options(strategy, None, None));
        });
    }
}

pub(crate) fn bench_ldes<F: Field>(c: &mut Criterion, strategy: FftStrategy) {
    const RATE_BITS: usize = 3;

    let mut group = c.benchmark_group(&format!("lde-{:?}<{}>", strategy, type_name::<F>()));

    for size_log in [16] {
        let orig_size = 1 << (size_log - RATE_BITS);
        let lde_size = 1 << size_log;

        group.bench_with_input(BenchmarkId::from_parameter(lde_size), &lde_size, |b, _| {
            let coeffs = PolynomialCoeffs::new(F::rand_vec(orig_size));
            b.iter(|| {
                let padded_coeffs = coeffs.lde(RATE_BITS);
                padded_coeffs.fft_with_options(strategy, Some(RATE_BITS), None)
            });
        });
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    bench_ffts::<CrandallField>(c, FftStrategy::Classic);
    bench_ffts::<CrandallField>(c, FftStrategy::Unrolled);
    bench_ldes::<CrandallField>(c, FftStrategy::Classic);
    bench_ldes::<CrandallField>(c, FftStrategy::Unrolled);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
