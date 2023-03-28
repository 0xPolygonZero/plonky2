use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use plonky2::field::extension::Extendable;
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::hash::hash_types::RichField;
use plonky2::hash::poseidon::PoseidonHash;
use plonky2::plonk::circuit_data::CircuitConfig;
use plonky2::plonk::config::{AlgebraicHasher, GenericConfig, Hasher, PoseidonGoldilocksConfig};
use tynm::type_name;
use poseidon2_plonky2::poseidon2_goldilock::Poseidon2GoldilocksConfig;
use poseidon2_plonky2::poseidon2_hash::Poseidon2Hash;
use crate::circuits::BaseCircuit;

mod circuits;

fn bench_base_proof<
    F: RichField + Extendable<D>,
    const D: usize,
    C: GenericConfig<D, F = F>,
    H: Hasher<F> + AlgebraicHasher<F>
>(c: &mut Criterion) {
    let mut group = c.benchmark_group(&format!(
        "base-proof<{}, {}>",
        type_name::<C>(),
        type_name::<H>()
    ));

    let config = CircuitConfig::standard_recursion_config();
    for degree in [12,14,16] {
        group.bench_function(
            format!("build circuit for degree {}", degree).as_str(), |b| b.iter_with_large_drop(
                || {
                    BaseCircuit::<F, C, D, H>::build_base_circuit(config.clone(), degree);
                }
            )
        );

        let base_circuit = BaseCircuit::<F, C, D, H>::build_base_circuit(config.clone(), degree);

        group.bench_function(
            format!("prove for degree {}", degree).as_str(),
            |b| b.iter_batched(
                || F::rand(),
                |init| base_circuit.generate_base_proof(init).unwrap(),
                BatchSize::PerIteration,
            ),
        );

        let proof = base_circuit.generate_base_proof(F::rand()).unwrap();

        group.bench_function(
            format!("verify for degree {}", degree).as_str(),
            |b| b.iter_batched(
                || (base_circuit.get_circuit_data(), proof.clone()),
                |(data, proof)| data.verify(proof).unwrap(),
                BatchSize::PerIteration,
            )
        );
    }

    group.finish();
}

fn benchmark(c: &mut Criterion) {
    const D: usize = 2;
    type F = GoldilocksField;
    bench_base_proof::<F, D, PoseidonGoldilocksConfig, PoseidonHash>(c);
    bench_base_proof::<F, D, Poseidon2GoldilocksConfig, PoseidonHash>(c);
    bench_base_proof::<F, D, PoseidonGoldilocksConfig, Poseidon2Hash>(c);
    bench_base_proof::<F, D, Poseidon2GoldilocksConfig, Poseidon2Hash>(c);
}


criterion_group!(name = benches;
    config = Criterion::default().sample_size(10);
    targets = benchmark);
criterion_main!(benches);