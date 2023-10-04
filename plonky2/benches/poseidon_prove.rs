#![allow(incomplete_features)]
#![feature(generic_const_exprs)]

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use plonky2::gates::gate::Gate;
use plonky2::gates::poseidon::PoseidonGate;
use plonky2::hash::hash_types::{HashOut, RichField};
use plonky2::iop::witness::{PartialWitness, WitnessWrite};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::circuit_data::CircuitConfig;
use plonky2::plonk::config::{GenericConfig, Hasher, PoseidonGoldilocksConfig};
use plonky2::plonk::vars::{EvaluationTargets, EvaluationVars};
use plonky2_field::extension::Extendable;
use plonky2_field::types::Sample;

pub fn bench_poseidon<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>(
    c: &mut Criterion,
) where
    [(); C::Hasher::HASH_SIZE]:,
{
    let gate = PoseidonGate::<F, D>::new();
    let mut group = c.benchmark_group("poseidon prove");
    group.sample_size(10);

    for i in 0..1 {
        group.bench_with_input(BenchmarkId::from_parameter(i), &i, |b, _| {
            b.iter(|| {
                // Test that `eval_unfiltered` and `eval_unfiltered_recursively` are coherent.
                let wires = F::Extension::rand_vec(gate.num_wires());
                let constants = F::Extension::rand_vec(gate.num_constants());
                let public_inputs_hash = HashOut::rand();

                let config = CircuitConfig::standard_recursion_config();
                let mut pw = PartialWitness::new();
                let mut builder = CircuitBuilder::<F, D>::new(config);

                let wires_t = builder.add_virtual_extension_targets(wires.len());
                let constants_t = builder.add_virtual_extension_targets(constants.len());
                pw.set_extension_targets(&wires_t, &wires);
                pw.set_extension_targets(&constants_t, &constants);
                let public_inputs_hash_t = builder.add_virtual_hash();
                pw.set_hash_target(public_inputs_hash_t, public_inputs_hash);

                let vars = EvaluationVars {
                    local_constants: &constants,
                    local_wires: &wires,
                    public_inputs_hash: &public_inputs_hash,
                };
                let evals = gate.eval_unfiltered(vars);

                let vars_t = EvaluationTargets {
                    local_constants: &constants_t,
                    local_wires: &wires_t,
                    public_inputs_hash: &public_inputs_hash_t,
                };
                let evals_t = gate.eval_unfiltered_circuit(&mut builder, vars_t);
                pw.set_extension_targets(&evals_t, &evals);

                let data = builder.build::<C>();

                let _proof = data.prove(pw);
            });
        });
    }
}

pub fn bench_poseidon_remove_prove<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(
    c: &mut Criterion,
) where
    [(); C::Hasher::HASH_SIZE]:,
{
    let gate = PoseidonGate::<F, D>::new();
    let mut group = c.benchmark_group("poseidon prove");
    group.sample_size(10);

    for i in 0..1 {
        group.bench_with_input(BenchmarkId::from_parameter(i), &i, |b, _| {
            b.iter(|| {
                // Test that `eval_unfiltered` and `eval_unfiltered_recursively` are coherent.
                let wires = F::Extension::rand_vec(gate.num_wires());
                let constants = F::Extension::rand_vec(gate.num_constants());
                let public_inputs_hash = HashOut::rand();

                let config = CircuitConfig::standard_recursion_config();
                let mut pw = PartialWitness::new();
                let mut builder = CircuitBuilder::<F, D>::new(config);

                let wires_t = builder.add_virtual_extension_targets(wires.len());
                let constants_t = builder.add_virtual_extension_targets(constants.len());
                pw.set_extension_targets(&wires_t, &wires);
                pw.set_extension_targets(&constants_t, &constants);
                let public_inputs_hash_t = builder.add_virtual_hash();
                pw.set_hash_target(public_inputs_hash_t, public_inputs_hash);

                let vars = EvaluationVars {
                    local_constants: &constants,
                    local_wires: &wires,
                    public_inputs_hash: &public_inputs_hash,
                };
                let evals = gate.eval_unfiltered(vars);

                let vars_t = EvaluationTargets {
                    local_constants: &constants_t,
                    local_wires: &wires_t,
                    public_inputs_hash: &public_inputs_hash_t,
                };
                let evals_t = gate.eval_unfiltered_circuit(&mut builder, vars_t);
                pw.set_extension_targets(&evals_t, &evals);

                let _data = builder.build::<C>();
            });
        });
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    const D: usize = 2;
    type C = PoseidonGoldilocksConfig;
    type F = <C as GenericConfig<D>>::F;
    bench_poseidon::<F, C, D>(c);
    bench_poseidon_remove_prove::<F, C, D>(c);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
