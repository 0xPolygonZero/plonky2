#![allow(incomplete_features)]
#![feature(generic_const_exprs)]

use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::hash::hash_types::{BytesHash, RichField};
use plonky2::hash::hashing::SPONGE_WIDTH;
use plonky2::hash::keccak::KeccakHash;
use plonky2::hash::poseidon::{Poseidon, PoseidonHash};
use plonky2::hash::blake3::Blake3Hash;
use plonky2::plonk::config::Hasher;
use tynm::type_name;

// TODO: We could create a generic benchmark on the [`Hasher`] trait. This works,
// except that `rand()`, which we need on [`Hasher::Hash`] is not a trait method.

pub(crate) fn bench_keccak<F: RichField>(c: &mut Criterion) {
    c.bench_function("keccak256", |b| {
        b.iter_batched(
            || (BytesHash::<32>::rand(), BytesHash::<32>::rand()),
            |(left, right)| <KeccakHash<32> as Hasher<F>>::two_to_one(left, right),
            BatchSize::SmallInput,
        )
    });
}

pub(crate) fn bench_blake3<F: RichField>(c: &mut Criterion) {
    c.bench_function("blake3", |b| {
        b.iter_batched(
            || (BytesHash::<32>::rand(), BytesHash::<32>::rand()),
            |(left, right)| <Blake3Hash<32> as Hasher<F>>::two_to_one(left, right),
            BatchSize::SmallInput,
            )
        },
    );
}

pub(crate) fn bench_poseidon<F: RichField>(c: &mut Criterion) {
    c.bench_function("poseidon", |b| {
        b.iter_batched(
            || (<PoseidonHash as Hasher<F>>::Hash::rand(), <PoseidonHash as Hasher<F>>::Hash::rand()),
            |(left, right)| <PoseidonHash as Hasher<F>>::two_to_one(left, right),
            BatchSize::SmallInput,
            )
        },
    );
    c.bench_function(
        &format!("poseidon<{}, {}>", type_name::<F>(), SPONGE_WIDTH),
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
    bench_keccak::<GoldilocksField>(c);
    bench_blake3::<GoldilocksField>(c);
    bench_poseidon::<GoldilocksField>(c);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
