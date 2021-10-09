#![feature(destructuring_assignment)]
#![feature(generic_const_exprs)]

use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::hash::gmimc::GMiMC;
use plonky2::hash::poseidon::Poseidon;
use tynm::type_name;

pub(crate) fn bench_gmimc<F: GMiMC<WIDTH>, const WIDTH: usize>(c: &mut Criterion) {
    c.bench_function(&format!("gmimc<{}, {}>", type_name::<F>(), WIDTH), |b| {
        b.iter_batched(
            || F::rand_arr::<WIDTH>(),
            |mut state| F::gmimc_permute(state),
            BatchSize::SmallInput,
        )
    });
}

pub(crate) fn bench_poseidon<F: Poseidon<WIDTH>, const WIDTH: usize>(c: &mut Criterion)
where
    [(); WIDTH - 1]: ,
{
    c.bench_function(&format!("poseidon<{}, {}>", type_name::<F>(), WIDTH), |b| {
        b.iter_batched(
            || F::rand_arr::<WIDTH>(),
            |mut state| F::poseidon(state),
            BatchSize::SmallInput,
        )
    });
}

fn criterion_benchmark(c: &mut Criterion) {
    bench_gmimc::<GoldilocksField, 12>(c);
    bench_poseidon::<GoldilocksField, 12>(c);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
