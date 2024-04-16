mod allocator;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::hash::field_merkle_tree::FieldMerkleTree;
use plonky2::hash::hash_types::RichField;
use plonky2::hash::keccak::KeccakHash;
use plonky2::hash::poseidon::PoseidonHash;
use plonky2::plonk::config::Hasher;
use tynm::type_name;

const ELEMS_PER_LEAF_1: usize = 70;
const ELEMS_PER_LEAF_2: usize = 5;
const ELEMS_PER_LEAF_3: usize = 100;

pub(crate) fn bench_field_merkle_tree<F: RichField, H: Hasher<F>>(c: &mut Criterion) {
    let mut group = c.benchmark_group(&format!(
        "field-merkle-tree<{}, {}>",
        type_name::<F>(),
        type_name::<H>()
    ));
    group.sample_size(10);

    for size_log in [13, 14, 15] {
        let size = 1 << size_log;
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            let leaves = vec![
                vec![F::rand_vec(ELEMS_PER_LEAF_1); size],
                vec![F::rand_vec(ELEMS_PER_LEAF_2); size >> 1],
                vec![F::rand_vec(ELEMS_PER_LEAF_3); size >> 2],
            ];
            b.iter(|| FieldMerkleTree::<F, H>::new(black_box(leaves.clone()), black_box(5)));
        });
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    bench_field_merkle_tree::<GoldilocksField, PoseidonHash>(c);
    bench_field_merkle_tree::<GoldilocksField, KeccakHash<25>>(c);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
