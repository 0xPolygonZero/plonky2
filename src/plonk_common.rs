use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::Extendable;
use crate::field::field::Field;
use crate::gates::gate::GateRef;
use crate::polynomial::polynomial::PolynomialCoeffs;
use crate::target::Target;
use crate::vars::{EvaluationTargets, EvaluationVars};

/// Evaluates all gate constraints.
///
/// `num_gate_constraints` is the largest number of constraints imposed by any gate. It is not
/// strictly necessary, but it helps performance by ensuring that we allocate a vector with exactly
/// the capacity that we need.
pub fn evaluate_gate_constraints<F: Field>(
    gates: &[GateRef<F>],
    num_gate_constraints: usize,
    vars: EvaluationVars<F>,
) -> Vec<F> {
    let mut constraints = vec![F::ZERO; num_gate_constraints];
    for gate in gates {
        let gate_constraints = gate.0.eval_filtered(vars);
        for (i, c) in gate_constraints.into_iter().enumerate() {
            debug_assert!(
                i < num_gate_constraints,
                "num_constraints() gave too low of a number"
            );
            constraints[i] += c;
        }
    }
    constraints
}

pub fn evaluate_gate_constraints_recursively<F: Field>(
    builder: &mut CircuitBuilder<F>,
    gates: &[GateRef<F>],
    num_gate_constraints: usize,
    vars: EvaluationTargets,
) -> Vec<Target> {
    let mut constraints = vec![builder.zero(); num_gate_constraints];
    for gate in gates {
        let gate_constraints = gate.0.eval_filtered_recursively(builder, vars);
        for (i, c) in gate_constraints.into_iter().enumerate() {
            constraints[i] = builder.add(constraints[i], c);
        }
    }
    constraints
}

/// Evaluate the polynomial which vanishes on any multiplicative subgroup of a given order `n`.
pub(crate) fn eval_zero_poly<F: Field>(n: usize, x: F) -> F {
    // Z(x) = x^n - 1
    x.exp(n as u64) - F::ONE
}

/// Evaluate the Lagrange basis `L_1` with `L_1(1) = 1`, and `L_1(x) = 0` for other members of an
/// order `n` multiplicative subgroup.
pub(crate) fn eval_l_1<F: Field>(n: usize, x: F) -> F {
    if x.is_one() {
        // The code below would divide by zero, since we have (x - 1) in both the numerator and
        // denominator.
        return F::ONE;
    }

    // L_1(x) = (x^n - 1) / (n * (x - 1))
    //        = Z(x) / (n * (x - 1))
    eval_zero_poly(n, x) / (F::from_canonical_usize(n) * (x - F::ONE))
}

/// For each alpha in alphas, compute a reduction of the given terms using powers of alpha.
pub(crate) fn reduce_with_powers_multi<F: Field>(terms: &[F], alphas: &[F]) -> Vec<F> {
    alphas
        .iter()
        .map(|&alpha| reduce_with_powers(terms, alpha))
        .collect()
}

pub(crate) fn reduce_with_powers<F: Field>(terms: &[F], alpha: F) -> F {
    let mut sum = F::ZERO;
    for &term in terms.iter().rev() {
        sum = sum * alpha + term;
    }
    sum
}

pub(crate) fn reduce_with_powers_recursive<F: Field>(
    builder: &mut CircuitBuilder<F>,
    terms: Vec<Target>,
    alpha: Target,
) -> Target {
    todo!()
}
