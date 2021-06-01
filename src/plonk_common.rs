use crate::circuit_builder::CircuitBuilder;
use crate::circuit_data::CommonCircuitData;
use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::field::field::Field;
use crate::gates::gate::GateRef;
use crate::target::Target;
use crate::vars::{EvaluationTargets, EvaluationVars, EvaluationVarsBase};

/// Evaluate the vanishing polynomial at `x`. In this context, the vanishing polynomial is a random
/// linear combination of gate constraints, plus some other terms relating to the permutation
/// argument. All such terms should vanish on `H`.
pub(crate) fn eval_vanishing_poly<F: Extendable<D>, const D: usize>(
    common_data: &CommonCircuitData<F, D>,
    x: F::Extension,
    vars: EvaluationVars<F, D>,
    local_plonk_zs: &[F::Extension],
    next_plonk_zs: &[F::Extension],
    s_sigmas: &[F::Extension],
    betas: &[F],
    gammas: &[F],
    alphas: &[F],
) -> Vec<F::Extension> {
    let constraint_terms =
        evaluate_gate_constraints(&common_data.gates, common_data.num_gate_constraints, vars);

    // The L_1(x) (Z(x) - 1) vanishing terms.
    let mut vanishing_z_1_terms = Vec::new();
    // The Z(x) f'(x) - g'(x) Z(g x) terms.
    let mut vanishing_v_shift_terms = Vec::new();

    for i in 0..common_data.config.num_challenges {
        let z_x = local_plonk_zs[i];
        let z_gz = next_plonk_zs[i];
        vanishing_z_1_terms.push(eval_l_1(common_data.degree(), x) * (z_x - F::Extension::ONE));

        let mut f_prime = F::Extension::ONE;
        let mut g_prime = F::Extension::ONE;
        for j in 0..common_data.config.num_routed_wires {
            let wire_value = vars.local_wires[j];
            let k_i = common_data.k_is[j];
            let s_id = x * k_i.into();
            let s_sigma = s_sigmas[j];
            f_prime *= wire_value + s_id * betas[i].into() + gammas[i].into();
            g_prime *= wire_value + s_sigma * betas[i].into() + gammas[i].into();
        }
        vanishing_v_shift_terms.push(f_prime * z_x - g_prime * z_gz);
    }

    let vanishing_terms = [
        vanishing_z_1_terms,
        vanishing_v_shift_terms,
        constraint_terms,
    ]
    .concat();

    let alphas = &alphas.iter().map(|&a| a.into()).collect::<Vec<_>>();
    reduce_with_powers_multi(&vanishing_terms, alphas)
}

/// Like `eval_vanishing_poly`, but specialized for base field points.
pub(crate) fn eval_vanishing_poly_base<F: Extendable<D>, const D: usize>(
    common_data: &CommonCircuitData<F, D>,
    x: F,
    vars: EvaluationVarsBase<F>,
    local_plonk_zs: &[F],
    next_plonk_zs: &[F],
    s_sigmas: &[F],
    betas: &[F],
    gammas: &[F],
    alphas: &[F],
) -> Vec<F> {
    let constraint_terms =
        evaluate_gate_constraints_base(&common_data.gates, common_data.num_gate_constraints, vars);

    // The L_1(x) (Z(x) - 1) vanishing terms.
    let mut vanishing_z_1_terms = Vec::new();
    // The Z(x) f'(x) - g'(x) Z(g x) terms.
    let mut vanishing_v_shift_terms = Vec::new();

    for i in 0..common_data.config.num_challenges {
        let z_x = local_plonk_zs[i];
        let z_gz = next_plonk_zs[i];
        vanishing_z_1_terms.push(eval_l_1(common_data.degree(), x) * (z_x - F::ONE));

        let mut f_prime = F::ONE;
        let mut g_prime = F::ONE;
        for j in 0..common_data.config.num_routed_wires {
            let wire_value = vars.local_wires[j];
            let k_i = common_data.k_is[j];
            let s_id = k_i * x;
            let s_sigma = s_sigmas[j];
            f_prime *= wire_value + betas[i] * s_id + gammas[i];
            g_prime *= wire_value + betas[i] * s_sigma + gammas[i];
        }
        vanishing_v_shift_terms.push(f_prime * z_x - g_prime * z_gz);
    }

    let vanishing_terms = [
        vanishing_z_1_terms,
        vanishing_v_shift_terms,
        constraint_terms,
    ]
    .concat();

    reduce_with_powers_multi(&vanishing_terms, alphas)
}

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
    terms: Vec<Target>,
    alpha: Target,
) -> Target {
    todo!()
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
