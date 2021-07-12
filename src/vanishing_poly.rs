use crate::circuit_builder::CircuitBuilder;
use crate::circuit_data::CommonCircuitData;
use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::field::field::Field;
use crate::gates::gate::{Gate, GateRef, PrefixedGate};
use crate::plonk_common;
use crate::plonk_common::{eval_l_1_recursively, ZeroPolyOnCoset};
use crate::target::Target;
use crate::util::partial_products::{check_partial_products, check_partial_products_recursively};
use crate::util::scaling::ReducingFactorTarget;
use crate::vars::{EvaluationTargets, EvaluationVars, EvaluationVarsBase};

/// Evaluate the vanishing polynomial at `x`. In this context, the vanishing polynomial is a random
/// linear combination of gate constraints, plus some other terms relating to the permutation
/// argument. All such terms should vanish on `H`.
pub(crate) fn eval_vanishing_poly<F: Extendable<D>, const D: usize>(
    common_data: &CommonCircuitData<F, D>,
    x: F::Extension,
    vars: EvaluationVars<F, D>,
    local_zs: &[F::Extension],
    next_zs: &[F::Extension],
    partial_products: &[F::Extension],
    s_sigmas: &[F::Extension],
    betas: &[F],
    gammas: &[F],
    alphas: &[F],
) -> Vec<F::Extension> {
    let max_degree = common_data.quotient_degree_factor;
    let (num_prods, final_num_prod) = common_data.num_partial_products;

    let constraint_terms =
        evaluate_gate_constraints(&common_data.gates, common_data.num_gate_constraints, vars);

    // The L_1(x) (Z(x) - 1) vanishing terms.
    let mut vanishing_z_1_terms = Vec::new();
    // The terms checking the partial products.
    let mut vanishing_partial_products_terms = Vec::new();
    // The Z(x) f'(x) - g'(x) Z(g x) terms.
    let mut vanishing_v_shift_terms = Vec::new();

    for i in 0..common_data.config.num_challenges {
        let z_x = local_zs[i];
        let z_gz = next_zs[i];
        vanishing_z_1_terms
            .push(plonk_common::eval_l_1(common_data.degree(), x) * (z_x - F::Extension::ONE));

        let numerator_values = (0..common_data.config.num_routed_wires)
            .map(|j| {
                let wire_value = vars.local_wires[j];
                let k_i = common_data.k_is[j];
                let s_id = x * k_i.into();
                wire_value + s_id * betas[i].into() + gammas[i].into()
            })
            .collect::<Vec<_>>();
        let denominator_values = (0..common_data.config.num_routed_wires)
            .map(|j| {
                let wire_value = vars.local_wires[j];
                let s_sigma = s_sigmas[j];
                wire_value + s_sigma * betas[i].into() + gammas[i].into()
            })
            .collect::<Vec<_>>();
        let quotient_values = (0..common_data.config.num_routed_wires)
            .map(|j| numerator_values[j] / denominator_values[j])
            .collect::<Vec<_>>();

        // The partial products considered for this iteration of `i`.
        let current_partial_products = &partial_products[i * num_prods..(i + 1) * num_prods];
        // Check the quotient partial products.
        let mut partial_product_check =
            check_partial_products(&quotient_values, current_partial_products, max_degree);
        // The first checks are of the form `q - n/d` which is a rational function not a polynomial.
        // We multiply them by `d` to get checks of the form `q*d - n` which low-degree polynomials.
        denominator_values
            .chunks(max_degree)
            .zip(partial_product_check.iter_mut())
            .for_each(|(d, q)| {
                *q *= d.iter().copied().product();
            });
        vanishing_partial_products_terms.extend(partial_product_check);

        // The quotient final product is the product of the last `final_num_prod` elements.
        let quotient: F::Extension = current_partial_products[num_prods - final_num_prod..]
            .iter()
            .copied()
            .product();
        vanishing_v_shift_terms.push(quotient * z_x - z_gz);
    }

    let vanishing_terms = [
        vanishing_z_1_terms,
        vanishing_partial_products_terms,
        vanishing_v_shift_terms,
        constraint_terms,
    ]
    .concat();
    dbg!(&vanishing_terms);

    let alphas = &alphas.iter().map(|&a| a.into()).collect::<Vec<_>>();
    plonk_common::reduce_with_powers_multi(&vanishing_terms, alphas)
}

/// Like `eval_vanishing_poly`, but specialized for base field points.
pub(crate) fn eval_vanishing_poly_base<F: Extendable<D>, const D: usize>(
    common_data: &CommonCircuitData<F, D>,
    index: usize,
    x: F,
    vars: EvaluationVarsBase<F>,
    local_zs: &[F],
    next_zs: &[F],
    partial_products: &[F],
    s_sigmas: &[F],
    betas: &[F],
    gammas: &[F],
    alphas: &[F],
    z_h_on_coset: &ZeroPolyOnCoset<F>,
) -> Vec<F> {
    let max_degree = common_data.quotient_degree_factor;
    let (num_prods, final_num_prod) = common_data.num_partial_products;

    let constraint_terms =
        evaluate_gate_constraints_base(&common_data.gates, common_data.num_gate_constraints, vars);

    // The L_1(x) (Z(x) - 1) vanishing terms.
    let mut vanishing_z_1_terms = Vec::new();
    // The terms checking the partial products.
    let mut vanishing_partial_products_terms = Vec::new();
    // The Z(x) f'(x) - g'(x) Z(g x) terms.
    let mut vanishing_v_shift_terms = Vec::new();

    for i in 0..common_data.config.num_challenges {
        let z_x = local_zs[i];
        let z_gz = next_zs[i];
        vanishing_z_1_terms.push(z_h_on_coset.eval_l1(index, x) * (z_x - F::ONE));

        let numerator_values = (0..common_data.config.num_routed_wires)
            .map(|j| {
                let wire_value = vars.local_wires[j];
                let k_i = common_data.k_is[j];
                let s_id = k_i * x;
                wire_value + betas[i] * s_id + gammas[i]
            })
            .collect::<Vec<_>>();
        let denominator_values = (0..common_data.config.num_routed_wires)
            .map(|j| {
                let wire_value = vars.local_wires[j];
                let s_sigma = s_sigmas[j];
                wire_value + betas[i] * s_sigma + gammas[i]
            })
            .collect::<Vec<_>>();
        let quotient_values = (0..common_data.config.num_routed_wires)
            .map(|j| numerator_values[j] / denominator_values[j])
            .collect::<Vec<_>>();

        // The partial products considered for this iteration of `i`.
        let current_partial_products = &partial_products[i * num_prods..(i + 1) * num_prods];
        // Check the numerator partial products.
        let mut partial_product_check =
            check_partial_products(&quotient_values, current_partial_products, max_degree);
        // The first checks are of the form `q - n/d` which is a rational function not a polynomial.
        // We multiply them by `d` to get checks of the form `q*d - n` which low-degree polynomials.
        denominator_values
            .chunks(max_degree)
            .zip(partial_product_check.iter_mut())
            .for_each(|(d, q)| {
                *q *= d.iter().copied().product();
            });
        vanishing_partial_products_terms.extend(partial_product_check);

        // The quotient final product is the product of the last `final_num_prod` elements.
        let quotient: F = current_partial_products[num_prods - final_num_prod..]
            .iter()
            .copied()
            .product();
        vanishing_v_shift_terms.push(quotient * z_x - z_gz);
    }
    let vanishing_terms = [
        vanishing_z_1_terms,
        vanishing_partial_products_terms,
        vanishing_v_shift_terms,
        constraint_terms,
    ]
    .concat();

    plonk_common::reduce_with_powers_multi(&vanishing_terms, alphas)
}

/// Evaluates all gate constraints.
///
/// `num_gate_constraints` is the largest number of constraints imposed by any gate. It is not
/// strictly necessary, but it helps performance by ensuring that we allocate a vector with exactly
/// the capacity that we need.
pub fn evaluate_gate_constraints<F: Extendable<D>, const D: usize>(
    gates: &[PrefixedGate<F, D>],
    num_gate_constraints: usize,
    vars: EvaluationVars<F, D>,
) -> Vec<F::Extension> {
    let mut constraints = vec![F::Extension::ZERO; num_gate_constraints];
    for gate in gates {
        let gate_constraints = gate.gate.0.eval_filtered(vars, &gate.prefix);
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
    gates: &[PrefixedGate<F, D>],
    num_gate_constraints: usize,
    vars: EvaluationVarsBase<F>,
) -> Vec<F> {
    let mut constraints = vec![F::ZERO; num_gate_constraints];
    for gate in gates {
        let gate_constraints = gate.gate.0.eval_filtered_base(vars, &gate.prefix);
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
    gates: &[PrefixedGate<F, D>],
    num_gate_constraints: usize,
    vars: EvaluationTargets<D>,
) -> Vec<ExtensionTarget<D>> {
    let mut constraints = vec![builder.zero_extension(); num_gate_constraints];
    for gate in gates {
        let gate_constraints = gate
            .gate
            .0
            .eval_filtered_recursively(builder, vars, &gate.prefix);
        for (i, c) in gate_constraints.into_iter().enumerate() {
            constraints[i] = builder.add_extension(constraints[i], c);
        }
    }
    constraints
}

/// Evaluate the vanishing polynomial at `x`. In this context, the vanishing polynomial is a random
/// linear combination of gate constraints, plus some other terms relating to the permutation
/// argument. All such terms should vanish on `H`.
pub(crate) fn eval_vanishing_poly_recursively<F: Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    common_data: &CommonCircuitData<F, D>,
    x: ExtensionTarget<D>,
    x_pow_deg: ExtensionTarget<D>,
    vars: EvaluationTargets<D>,
    local_zs: &[ExtensionTarget<D>],
    next_zs: &[ExtensionTarget<D>],
    partial_products: &[ExtensionTarget<D>],
    s_sigmas: &[ExtensionTarget<D>],
    betas: &[Target],
    gammas: &[Target],
    alphas: &[Target],
) -> Vec<ExtensionTarget<D>> {
    let max_degree = common_data.quotient_degree_factor;
    let (num_prods, final_num_prod) = common_data.num_partial_products;

    let constraint_terms = evaluate_gate_constraints_recursively(
        builder,
        &common_data.gates,
        common_data.num_gate_constraints,
        vars,
    );

    // The L_1(x) (Z(x) - 1) vanishing terms.
    let mut vanishing_z_1_terms = Vec::new();
    // The terms checking the partial products.
    let mut vanishing_partial_products_terms = Vec::new();
    // The Z(x) f'(x) - g'(x) Z(g x) terms.
    let mut vanishing_v_shift_terms = Vec::new();

    for i in 0..common_data.config.num_challenges {
        let z_x = local_zs[i];
        let z_gz = next_zs[i];
        let l1 = eval_l_1_recursively(builder, common_data.degree(), x, x_pow_deg);
        vanishing_z_1_terms.push(builder.arithmetic_extension(F::ONE, F::NEG_ONE, l1, z_x, l1));

        let numerator_values = (0..common_data.config.num_routed_wires)
            .map(|j| {
                let wire_value = vars.local_wires[j];
                let k_i = builder.constant(common_data.k_is[j]);
                let s_id = builder.scalar_mul_ext(k_i, x);
                let gamma_ext = builder.convert_to_ext(gammas[i]);
                let tmp = builder.scalar_mul_add_extension(betas[i], s_id, wire_value);
                builder.add_extension(tmp, gamma_ext)
            })
            .collect::<Vec<_>>();
        let denominator_values = (0..common_data.config.num_routed_wires)
            .map(|j| {
                let wire_value = vars.local_wires[j];
                let s_sigma = s_sigmas[j];
                let gamma_ext = builder.convert_to_ext(gammas[i]);
                let tmp = builder.scalar_mul_add_extension(betas[i], s_sigma, wire_value);
                builder.add_extension(tmp, gamma_ext)
            })
            .collect::<Vec<_>>();
        let quotient_values = (0..common_data.config.num_routed_wires)
            .map(|j| builder.div_unsafe_extension(numerator_values[j], denominator_values[j]))
            .collect::<Vec<_>>();

        // The partial products considered for this iteration of `i`.
        let current_partial_products = &partial_products[i * num_prods..(i + 1) * num_prods];
        // Check the quotient partial products.
        let mut partial_product_check = check_partial_products_recursively(
            builder,
            &quotient_values,
            current_partial_products,
            max_degree,
        );
        // The first checks are of the form `q - n/d` which is a rational function not a polynomial.
        // We multiply them by `d` to get checks of the form `q*d - n` which low-degree polynomials.
        denominator_values
            .chunks(max_degree)
            .zip(partial_product_check.iter_mut())
            .for_each(|(d, q)| {
                let tmp = builder.mul_many_extension(d);
                *q = builder.mul_extension(*q, tmp);
            });
        vanishing_partial_products_terms.extend(partial_product_check);

        // The quotient final product is the product of the last `final_num_prod` elements.
        let quotient =
            builder.mul_many_extension(&current_partial_products[num_prods - final_num_prod..]);
        vanishing_v_shift_terms.push(builder.mul_sub_extension(quotient, z_x, z_gz));
    }

    let vanishing_terms = [
        vanishing_z_1_terms,
        vanishing_partial_products_terms,
        vanishing_v_shift_terms,
        constraint_terms,
    ]
    .concat();
    dbg!(&vanishing_terms);

    alphas
        .iter()
        .map(|&alpha| {
            let alpha = builder.convert_to_ext(alpha);
            let mut alpha = ReducingFactorTarget::new(alpha);
            alpha.reduce(&vanishing_terms, builder)
        })
        .collect()
}
