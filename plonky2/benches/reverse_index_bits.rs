mod allocator;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::field::types::Sample;
use plonky2_util::{reverse_index_bits, reverse_index_bits_in_place};

type F = GoldilocksField;

fn benchmark_in_place(c: &mut Criterion) {
    let mut group = c.benchmark_group("reverse-index-bits-in-place");
    for width in [1 << 8, 1 << 16, 1 << 24] {
        group.bench_with_input(BenchmarkId::from_parameter(width), &width, |b, _| {
            let mut values = F::rand_vec(width);
            b.iter(|| reverse_index_bits_in_place(&mut values));
        });
    }
}

fn benchmark_out_of_place(c: &mut Criterion) {
    let mut group = c.benchmark_group("reverse-index-bits");
    for width in [1 << 8, 1 << 16, 1 << 24] {
        group.bench_with_input(BenchmarkId::from_parameter(width), &width, |b, _| {
            let values = F::rand_vec(width);
            b.iter(|| reverse_index_bits(&values));
        });
    }
}

criterion_group!(benches_in_place, benchmark_in_place);
criterion_group!(benches_out_of_place, benchmark_out_of_place);
criterion_main!(benches_in_place, benches_out_of_place);
