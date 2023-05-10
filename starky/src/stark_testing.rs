use alloc::vec;
use alloc::vec::Vec;

use anyhow::{ensure, Result};
use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::polynomial::{PolynomialCoeffs, PolynomialValues};
use plonky2::field::types::{Field, Sample};
use plonky2::hash::hash_types::RichField;
use plonky2::iop::witness::{PartialWitness, WitnessWrite};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::circuit_data::CircuitConfig;
use plonky2::plonk::config::GenericConfig;
use plonky2::util::{log2_ceil, log2_strict, transpose};

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::stark::Stark;
use crate::vars::{StarkEvaluationTargets, StarkEvaluationVars};

const WITNESS_SIZE: usize = 1 << 5;

/// Tests that the constraints imposed by the given STARK are low-degree by applying them to random
/// low-degree witness polynomials.
pub fn test_stark_low_degree<F: RichField + Extendable<D>, S: Stark<F, D>, const D: usize>(
    stark: S,
) -> Result<()>
where
    [(); S::COLUMNS]:,
    [(); S::PUBLIC_INPUTS]:,
{
    let rate_bits = log2_ceil(stark.constraint_degree() + 1);

    let trace_ldes = random_low_degree_matrix::<F>(S::COLUMNS, rate_bits);
    let size = trace_ldes.len();
    let public_inputs = F::rand_array::<{ S::PUBLIC_INPUTS }>();

    let lagrange_first = PolynomialValues::selector(WITNESS_SIZE, 0).lde(rate_bits);
    let lagrange_last = PolynomialValues::selector(WITNESS_SIZE, WITNESS_SIZE - 1).lde(rate_bits);

    let last = F::primitive_root_of_unity(log2_strict(WITNESS_SIZE)).inverse();
    let subgroup =
        F::cyclic_subgroup_known_order(F::primitive_root_of_unity(log2_strict(size)), size);
    let alpha = F::rand();
    let constraint_evals = (0..size)
        .map(|i| {
            let vars = StarkEvaluationVars {
                local_values: &trace_ldes[i].clone().try_into().unwrap(),
                next_values: &trace_ldes[(i + (1 << rate_bits)) % size]
                    .clone()
                    .try_into()
                    .unwrap(),
                public_inputs: &public_inputs,
            };

            let mut consumer = ConstraintConsumer::<F>::new(
                vec![alpha],
                subgroup[i] - last,
                lagrange_first.values[i],
                lagrange_last.values[i],
            );
            stark.eval_packed_base(vars, &mut consumer);
            consumer.accumulators()[0]
        })
        .collect::<Vec<_>>();

    let constraint_eval_degree = PolynomialValues::new(constraint_evals).degree();
    let maximum_degree = WITNESS_SIZE * stark.constraint_degree() - 1;

    ensure!(
        constraint_eval_degree <= maximum_degree,
        "Expected degrees at most {} * {} - 1 = {}, actual {:?}",
        WITNESS_SIZE,
        stark.constraint_degree(),
        maximum_degree,
        constraint_eval_degree
    );

    Ok(())
}

/// Tests that the circuit constraints imposed by the given STARK are coherent with the native constraints.
pub fn test_stark_circuit_constraints<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
    const D: usize,
>(
    stark: S,
) -> Result<()>
where
    [(); S::COLUMNS]:,
    [(); S::PUBLIC_INPUTS]:,
{
    // Compute native constraint evaluation on random values.
    let vars = StarkEvaluationVars {
        local_values: &F::Extension::rand_array::<{ S::COLUMNS }>(),
        next_values: &F::Extension::rand_array::<{ S::COLUMNS }>(),
        public_inputs: &F::Extension::rand_array::<{ S::PUBLIC_INPUTS }>(),
    };
    let alphas = F::rand_vec(1);
    let z_last = F::Extension::rand();
    let lagrange_first = F::Extension::rand();
    let lagrange_last = F::Extension::rand();
    let mut consumer = ConstraintConsumer::<F::Extension>::new(
        alphas
            .iter()
            .copied()
            .map(F::Extension::from_basefield)
            .collect(),
        z_last,
        lagrange_first,
        lagrange_last,
    );
    stark.eval_ext(vars, &mut consumer);
    let native_eval = consumer.accumulators()[0];

    // Compute circuit constraint evaluation on same random values.
    let circuit_config = CircuitConfig::standard_recursion_config();
    let mut builder = CircuitBuilder::<F, D>::new(circuit_config);
    let mut pw = PartialWitness::<F>::new();

    let locals_t = builder.add_virtual_extension_targets(S::COLUMNS);
    pw.set_extension_targets(&locals_t, vars.local_values);
    let nexts_t = builder.add_virtual_extension_targets(S::COLUMNS);
    pw.set_extension_targets(&nexts_t, vars.next_values);
    let pis_t = builder.add_virtual_extension_targets(S::PUBLIC_INPUTS);
    pw.set_extension_targets(&pis_t, vars.public_inputs);
    let alphas_t = builder.add_virtual_targets(1);
    pw.set_target(alphas_t[0], alphas[0]);
    let z_last_t = builder.add_virtual_extension_target();
    pw.set_extension_target(z_last_t, z_last);
    let lagrange_first_t = builder.add_virtual_extension_target();
    pw.set_extension_target(lagrange_first_t, lagrange_first);
    let lagrange_last_t = builder.add_virtual_extension_target();
    pw.set_extension_target(lagrange_last_t, lagrange_last);

    let vars = StarkEvaluationTargets::<D, { S::COLUMNS }, { S::PUBLIC_INPUTS }> {
        local_values: &locals_t.try_into().unwrap(),
        next_values: &nexts_t.try_into().unwrap(),
        public_inputs: &pis_t.try_into().unwrap(),
    };
    let mut consumer = RecursiveConstraintConsumer::<F, D>::new(
        builder.zero_extension(),
        alphas_t,
        z_last_t,
        lagrange_first_t,
        lagrange_last_t,
    );
    stark.eval_ext_circuit(&mut builder, vars, &mut consumer);
    let circuit_eval = consumer.accumulators()[0];
    let native_eval_t = builder.constant_extension(native_eval);
    builder.connect_extension(circuit_eval, native_eval_t);

    let data = builder.build::<C>();
    let proof = data.prove(pw)?;
    data.verify(proof)
}

fn random_low_degree_matrix<F: Field>(num_polys: usize, rate_bits: usize) -> Vec<Vec<F>> {
    let polys = (0..num_polys)
        .map(|_| random_low_degree_values(rate_bits))
        .collect::<Vec<_>>();

    transpose(&polys)
}

fn random_low_degree_values<F: Field>(rate_bits: usize) -> Vec<F> {
    PolynomialCoeffs::new(F::rand_vec(WITNESS_SIZE))
        .lde(rate_bits)
        .fft()
        .values
}
