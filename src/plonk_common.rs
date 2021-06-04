use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::field::field::Field;
use crate::gates::gate::GateRef;
use crate::target::Target;
use crate::vars::{EvaluationTargets, EvaluationVars, EvaluationVarsBase};

/// Evaluates all gate constraints.
///
/// `num_gate_constraints` is the largest number of constraints imposed by any gate. It is not
/// strictly necessary, but it helps performance by ensuring that we allocate a vector with exactly
/// the capacity that we need.
pub fn evaluate_gate_constraints<F: Extendable<D>, const D: usize>(
    gates: &[GateRef<F, D>],
    num_gate_constraints: usize,
    vars: EvaluationVars<F, D>,
) -> Vec<F::Extension> {
    let mut constraints = vec![F::Extension::ZERO; num_gate_constraints];
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

pub fn evaluate_gate_constraints_base<F: Extendable<D>, const D: usize>(
    gates: &[GateRef<F, D>],
    num_gate_constraints: usize,
    vars: EvaluationVarsBase<F>,
) -> Vec<F> {
    let mut constraints = vec![F::ZERO; num_gate_constraints];
    for gate in gates {
        let gate_constraints = gate.0.eval_filtered_base(vars);
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

pub fn evaluate_gate_constraints_recursively<F: Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    gates: &[GateRef<F, D>],
    num_gate_constraints: usize,
    vars: EvaluationTargets<D>,
) -> Vec<ExtensionTarget<D>> {
    let mut constraints = vec![builder.zero_extension(); num_gate_constraints];
    for gate in gates {
        let gate_constraints = gate.0.eval_filtered_recursively(builder, vars);
        for (i, c) in gate_constraints.into_iter().enumerate() {
            constraints[i] = builder.add_extension(constraints[i], c);
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

pub(crate) fn reduce_with_powers_recursive<F: Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    terms: &[ExtensionTarget<D>],
    alpha: Target,
) -> ExtensionTarget<D> {
    let mut sum = builder.zero_extension();
    for &term in terms.iter().rev() {
        sum = builder.scalar_mul_ext(alpha, sum);
        sum = builder.add_extension(sum, term);
    }
    sum
}

pub(crate) fn reduce_with_iter<F: Field, I>(terms: &[F], coeffs: I) -> F
where
    I: IntoIterator<Item = F>,
{
    let mut sum = F::ZERO;
    for (&term, coeff) in terms.iter().zip(coeffs) {
        sum += coeff * term;
    }
    sum
}
