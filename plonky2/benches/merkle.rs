#![feature(generic_const_exprs)]

mod allocator;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::hash::hash_types::RichField;
use plonky2::hash::keccak::KeccakHash;
use plonky2::hash::merkle_tree::MerkleTree;
use plonky2::hash::poseidon::PoseidonHash;
use plonky2::plonk::config::Hasher;
use tynm::type_name;

const SIZE_LOG: usize = 23;
const SIZE: usize = 1 << SIZE_LOG;

pub(crate) fn bench_merkle_tree<F: RichField, H: Hasher<F>>(c: &mut Criterion)
where
    [(); H::HASH_SIZE]:,
{
    let mut group = c.benchmark_group(&format!(
        "merkle-tree<{}, {}>",
        type_name::<F>(),
        type_name::<H>()
    ));
    group.sample_size(10);

    for num_polys in [64, 100, 128, 255] {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_polys),
            &num_polys,
            |b, _| {
                let leaves = vec![F::rand_vec(num_polys); SIZE];
                b.iter(|| MerkleTree::<F, H>::new(leaves.clone(), 0));
            },
        );
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    bench_merkle_tree::<GoldilocksField, PoseidonHash>(c);
    bench_merkle_tree::<GoldilocksField, KeccakHash<25>>(c);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
