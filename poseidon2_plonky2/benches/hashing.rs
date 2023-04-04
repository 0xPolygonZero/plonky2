mod allocator;

use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::field::types::Sample;
use plonky2::hash::hash_types::{BytesHash, RichField};
use plonky2::hash::hashing::HashConfig;
use plonky2::hash::keccak::KeccakHash;
use plonky2::hash::poseidon::Poseidon;
use plonky2::plonk::config::{Hasher, KeccakHashConfig, PoseidonHashConfig};
use rand_chacha::ChaCha12Rng;
use rand_chacha::rand_core::SeedableRng;
use tynm::type_name;
use poseidon2_plonky2::poseidon2_hash::{Poseidon2, Poseidon2HashConfig};

pub(crate) fn bench_keccak<F: RichField>(c: &mut Criterion) {
    let mut rng = ChaCha12Rng::seed_from_u64(38u64);
    c.bench_function("keccak256", |b| {
        b.iter_batched(
            || (BytesHash::<32>::sample(&mut rng), BytesHash::<32>::sample(&mut rng)),
            |(left, right)| <KeccakHash<32> as Hasher<F, KeccakHashConfig>>::two_to_one(left, right),
            BatchSize::SmallInput,
        )
    });
}

pub(crate) fn bench_poseidon<F: Poseidon>(c: &mut Criterion) {
    let mut rng = ChaCha12Rng::seed_from_u64(42u64);
    c.bench_function(
        &format!("poseidon<{}, {}>", type_name::<F>(), PoseidonHashConfig::WIDTH),
        |b| {
            b.iter_batched(
                || (0..PoseidonHashConfig::WIDTH).map(|_| F::sample(&mut rng)).collect::<Vec<_>>().try_into().unwrap(),
                |state| F::poseidon(state),
                BatchSize::SmallInput,
            )
        },
    );
}

pub(crate) fn bench_poseidon_2<F: Poseidon2>(c: &mut Criterion) {
    let mut rng = ChaCha12Rng::seed_from_u64(42u64);
    c.bench_function(
        &format!("poseidon2<{}, {}>", type_name::<F>(), Poseidon2HashConfig::WIDTH),
        |b| {
            b.iter_batched(
                || (0..Poseidon2HashConfig::WIDTH).map(|_| F::sample(&mut rng)).collect::<Vec<_>>().try_into().unwrap(),
                |state| F::poseidon2(state),
                BatchSize::SmallInput,
            )
        },
    );
}

fn criterion_benchmark(c: &mut Criterion) {
    bench_poseidon::<GoldilocksField>(c);
    bench_poseidon_2::<GoldilocksField>(c);
    bench_keccak::<GoldilocksField>(c);

}

criterion_group!(name = benches;
    config = Criterion::default().sample_size(500);
    targets = criterion_benchmark);
criterion_main!(benches);
