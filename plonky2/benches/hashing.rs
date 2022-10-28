#![allow(incomplete_features)]
#![feature(generic_const_exprs)]

mod allocator;

use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::hash::hash_types::{BytesHash, RichField};
use plonky2::hash::hashing::SPONGE_WIDTH;
use plonky2::hash::keccak::KeccakHash;
use plonky2::hash::poseidon::Poseidon;
use plonky2::plonk::config::Hasher;
use tynm::type_name;

pub(crate) fn bench_keccak<F: RichField>(c: &mut Criterion) {
    c.bench_function("keccak256", |b| {
        b.iter_batched(
            || (BytesHash::<32>::rand(), BytesHash::<32>::rand()),
            |(left, right)| <KeccakHash<32> as Hasher<F>>::two_to_one(left, right),
            BatchSize::SmallInput,
        )
    });
}

pub(crate) fn bench_poseidon<F: Poseidon>(c: &mut Criterion) {
    c.bench_function(
        &format!("poseidon<{}, {SPONGE_WIDTH}>", type_name::<F>()),
        |b| {
            b.iter_batched(
                || F::rand_arr::<SPONGE_WIDTH>(),
                |state| F::poseidon(state),
                BatchSize::SmallInput,
            )
        },
    );
}

fn criterion_benchmark(c: &mut Criterion) {
    bench_poseidon::<GoldilocksField>(c);
    bench_keccak::<GoldilocksField>(c);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
