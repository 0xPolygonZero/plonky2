#![allow(incomplete_features)]
#![feature(generic_const_exprs)]

use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::field::ops::Rand;
use plonky2::hash::blake3::{Blake3Hash, Blake3Permutation};
use plonky2::hash::hash_types::RichField;
use plonky2::hash::hashing::{PlonkyPermutation, SPONGE_WIDTH};
use plonky2::hash::keccak::{KeccakHash, KeccakPermutation};
use plonky2::hash::poseidon::{PoseidonHash, PoseidonPermutation};
use plonky2::plonk::config::Hasher;
use tynm::type_name;

fn bench_hasher<F: RichField, H: Hasher<F>>(c: &mut Criterion)
where
    H::Hash: Rand,
{
    let mut group = c.benchmark_group(type_name::<H>());

    group.bench_function("two_to_one", |b| {
        b.iter_batched(
            || (H::Hash::rand(), H::Hash::rand()),
            |(left, right)| H::two_to_one(left, right),
            BatchSize::SmallInput,
        )
    });

    for size in [0_usize, 1, 2, 4, 8, 16, 32, 64, 128, 256] {
        group.bench_with_input(&format!("hash_no_pad/{}", size), &size, |b, &size| {
            b.iter_batched(
                || F::rand_vec(size),
                |state| H::hash_no_pad(&state),
                BatchSize::SmallInput,
            )
        });
    }
}

fn bench_permutation<F: RichField, H: PlonkyPermutation<F>>(c: &mut Criterion) {
    c.bench_function(
        &format!("{}::permute<{}>", type_name::<H>(), type_name::<F>()),
        |b| {
            b.iter_batched(
                || F::rand_arr::<SPONGE_WIDTH>(),
                |input| H::permute(input),
                BatchSize::SmallInput,
            )
        },
    );
}

fn bench_poseidon<F: RichField>(c: &mut Criterion) {
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
    bench_hasher::<GoldilocksField, PoseidonHash>(c);
    bench_hasher::<GoldilocksField, KeccakHash<32>>(c);
    bench_hasher::<GoldilocksField, Blake3Hash<32>>(c);
    bench_permutation::<GoldilocksField, PoseidonPermutation>(c);
    bench_permutation::<GoldilocksField, KeccakPermutation>(c);
    bench_permutation::<GoldilocksField, Blake3Permutation>(c);
    bench_poseidon::<GoldilocksField>(c);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
