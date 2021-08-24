use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use plonky2::field::crandall_field::CrandallField;
use plonky2::field::extension_field::quartic::QuarticCrandallField;
use plonky2::field::field_types::Field;
use tynm::type_name;

pub(crate) fn bench_field<F: Field>(c: &mut Criterion) {
    c.bench_function(&format!("mul<{}>", type_name::<F>()), |b| {
        b.iter_batched(
            || (F::rand(), F::rand()),
            |(x, y)| {
                let yy = y * y;
                let xx = x * x;
                let xy = x * y;

                let xx_x = xx * x;
                let yy_y = yy * y;
                let xy_x = xy * x;
                let xy_y = xy * y;

                (xx_x * xy_x) * (yy_y * xy_y)
            },
            BatchSize::SmallInput,
        )
    });

    c.bench_function(&format!("add<{}>", type_name::<F>()), |b| {
        b.iter_batched(
            || (F::rand(), F::rand()),
            |(x, y)| {
                let yy = y + y;
                let xx = x + x;
                let xy = x + y;

                let xx_x = xx + x;
                let yy_y = yy + y;
                let xy_x = xy + x;
                let xy_y = xy + y;

                (xx_x + xy_x) + (yy_y + xy_y)
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
