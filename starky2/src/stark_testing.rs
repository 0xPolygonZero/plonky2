use anyhow::{ensure, Result};
use plonky2::field::extension_field::Extendable;
use plonky2::field::field_types::Field;
use plonky2::field::polynomial::{PolynomialCoeffs, PolynomialValues};
use plonky2::hash::hash_types::RichField;
use plonky2::util::transpose;
use plonky2_util::{log2_ceil, log2_strict};

use crate::constraint_consumer::ConstraintConsumer;
use crate::stark::Stark;
use crate::vars::StarkEvaluationVars;

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
    let public_inputs = F::rand_arr::<{ S::PUBLIC_INPUTS }>();

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
