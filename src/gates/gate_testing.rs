use crate::field::extension_field::Extendable;
use crate::field::field::Field;
use crate::gates::gate::{Gate, GateRef};
use crate::polynomial::polynomial::{PolynomialCoeffs, PolynomialValues};
use crate::proof::Hash;
use crate::util::{log2_ceil, transpose};
use crate::vars::EvaluationVars;

const WITNESS_SIZE: usize = 1 << 5;
const WITNESS_DEGREE: usize = WITNESS_SIZE - 1;

/// Tests that the constraints imposed by the given gate are low-degree by applying them to random
/// low-degree witness polynomials.
pub(crate) fn test_low_degree<F: Extendable<D>, G: Gate<F, D>, const D: usize>(gate: G) {
    let rate_bits = log2_ceil(gate.degree() + 1);

    let wire_ldes = random_low_degree_matrix::<F::Extension>(gate.num_wires(), rate_bits);
    let constant_ldes = random_low_degree_matrix::<F::Extension>(gate.num_constants(), rate_bits);
    assert_eq!(wire_ldes.len(), constant_ldes.len());
    let public_inputs_hash = &Hash::rand();

    let constraint_evals = wire_ldes
        .iter()
        .zip(constant_ldes.iter())
        .map(|(local_wires, local_constants)| EvaluationVars {
            local_constants,
            local_wires,
            public_inputs_hash,
        })
        .map(|vars| gate.eval_unfiltered(vars))
        .collect::<Vec<_>>();

    let constraint_eval_degrees = transpose(&constraint_evals)
        .into_iter()
        .map(PolynomialValues::new)
        .map(|p| p.degree())
        .collect::<Vec<_>>();

    assert_eq!(
        constraint_eval_degrees.len(),
        gate.num_constraints(),
        "eval should return num_constraints() constraints"
    );

    let expected_eval_degree = WITNESS_DEGREE * gate.degree();

    assert!(
        constraint_eval_degrees
            .iter()
            .all(|&deg| deg <= expected_eval_degree),
        "Expected degrees at most {} * {} = {}, actual {:?}",
        WITNESS_SIZE,
        gate.degree(),
        expected_eval_degree,
        constraint_eval_degrees
    );
}

fn random_low_degree_matrix<F: Field>(num_polys: usize, rate_bits: usize) -> Vec<Vec<F>> {
    let polys = (0..num_polys)
        .map(|_| random_low_degree_values(rate_bits))
        .collect::<Vec<_>>();

    if polys.is_empty() {
        // We want a Vec of many empty Vecs, whereas transpose would just give an empty Vec.
        vec![Vec::new(); WITNESS_SIZE << rate_bits]
    } else {
        transpose(&polys)
    }
}

fn random_low_degree_values<F: Field>(rate_bits: usize) -> Vec<F> {
    PolynomialCoeffs::new(F::rand_vec(WITNESS_SIZE))
        .lde(rate_bits)
        .fft()
        .values
}
