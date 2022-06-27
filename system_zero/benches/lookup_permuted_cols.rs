mod allocator;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use itertools::Itertools;
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::field::types::Field;
use rand::{thread_rng, Rng};
use system_zero::lookup::permuted_cols;

type F = GoldilocksField;

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("lookup-permuted-cols");

    for size_log in [16, 17, 18] {
        let size = 1 << size_log;
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            // We could benchmark a table of random values with
            //     let table = F::rand_vec(size);
            // But in practice we currently use tables that are pre-sorted, which makes
            // permuted_cols cheaper since it will sort the table.
            let table = (0..size).map(F::from_canonical_usize).collect_vec();
            let input = (0..size)
                .map(|_| table[thread_rng().gen_range(0..size)])
                .collect_vec();
            b.iter(|| permuted_cols(&input, &table));
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
