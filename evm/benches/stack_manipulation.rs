use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use plonky2_evm::cpu::kernel::assemble_to_bytes;

fn criterion_benchmark(c: &mut Criterion) {
    rotl_group(c);
    rotr_group(c);
    insert_group(c);
    delete_group(c);
    replace_group(c);
    shuffle_group(c);
    misc_group(c);
}

fn rotl_group(c: &mut Criterion) {
    let mut group = c.benchmark_group("rotl");
    group.sample_size(10);
    group.bench_function(BenchmarkId::from_parameter(8), |b| {
        b.iter(|| assemble("%stack (a, b, c, d, e, f, g, h) -> (b, c, d, e, f, g, h, a)"))
    });
}

fn rotr_group(c: &mut Criterion) {
    let mut group = c.benchmark_group("rotr");
    group.sample_size(10);
    group.bench_function(BenchmarkId::from_parameter(8), |b| {
        b.iter(|| assemble("%stack (a, b, c, d, e, f, g, h) -> (h, a, b, c, d, e, f, g)"))
    });
}

fn insert_group(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert");
    group.sample_size(10);
    group.bench_function(BenchmarkId::from_parameter(8), |b| {
        b.iter(|| assemble("%stack (a, b, c, d, e, f, g, h) -> (a, b, c, d, 123, e, f, g, h)"))
    });
}

fn delete_group(c: &mut Criterion) {
    let mut group = c.benchmark_group("delete");
    group.sample_size(10);
    group.bench_function(BenchmarkId::from_parameter(8), |b| {
        b.iter(|| assemble("%stack (a, b, c, d, e, f, g, h) -> (a, b, c, e, f, g, h)"))
    });
}

fn replace_group(c: &mut Criterion) {
    let mut group = c.benchmark_group("replace");
    group.sample_size(10);
    group.bench_function(BenchmarkId::from_parameter(8), |b| {
        b.iter(|| assemble("%stack (a, b, c, d, e, f, g, h) -> (a, b, c, 5, e, f, g, h)"))
    });
}

fn shuffle_group(c: &mut Criterion) {
    let mut group = c.benchmark_group("shuffle");
    group.sample_size(10);
    group.bench_function(BenchmarkId::from_parameter(8), |b| {
        b.iter(|| assemble("%stack (a, b, c, d, e, f, g, h) -> (g, d, h, a, f, e, b, c)"))
    });
}

fn misc_group(c: &mut Criterion) {
    let mut group = c.benchmark_group("misc");
    group.sample_size(10);
    group.bench_function(BenchmarkId::from_parameter(8), |b| {
        b.iter(|| assemble("%stack (a, b, c, a, e, f, g, h) -> (g, 1, h, g, f, 3, b, b)"))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

fn assemble(code: &str) {
    assemble_to_bytes(&[code.into()]);
}
