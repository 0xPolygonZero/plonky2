use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use itertools::Itertools;
use plonky2::field::field_types::Field;
use plonky2::field::goldilocks_field::GoldilocksField;
use rand::{thread_rng, Rng};
use system_zero::lookup::{permuted_cols, permuted_cols_v2};

type F = GoldilocksField;

pub(crate) fn bench_hash_bag_method(c: &mut Criterion) {
    let mut group = c.benchmark_group("lookup-with-hash-bag");

    for size_log in [16, 17, 18] {
        let size = 1 << size_log;
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            let table = F::rand_vec(size);
            let input = (0..size)
                .map(|_| table[thread_rng().gen_range(0..size)])
                .collect_vec();
            b.iter(|| permuted_cols(&input, &table));
        });
    }
}

pub(crate) fn bench_sort_method(c: &mut Criterion) {
    let mut group = c.benchmark_group("lookup-with-sorting");

    for size_log in [16, 17, 18] {
        let size = 1 << size_log;
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            let table = F::rand_vec(size);
            let input = (0..size)
                .map(|_| table[thread_rng().gen_range(0..size)])
                .collect_vec();
            b.iter(|| permuted_cols_v2(&input, &table));
        });
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    bench_hash_bag_method(c);
    bench_sort_method(c);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
