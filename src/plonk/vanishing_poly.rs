use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::{Extendable, FieldExtension};
use crate::field::field_types::{Field, RichField};
use crate::gates::gate::PrefixedGate;
use crate::iop::target::Target;
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::circuit_data::CommonCircuitData;
use crate::plonk::plonk_common;
use crate::plonk::plonk_common::{eval_l_1_recursively, ZeroPolyOnCoset};
use crate::plonk::vars::{EvaluationTargets, EvaluationVars, EvaluationVarsBase};
use crate::util::partial_products::{check_partial_products, check_partial_products_recursively};
use crate::util::reducing::ReducingFactorTarget;
use crate::with_context;

/// Evaluate the vanishing polynomial at `x`. In this context, the vanishing polynomial is a random
/// linear combination of gate constraints, plus some other terms relating to the permutation
/// argument. All such terms should vanish on `H`.
pub(crate) fn eval_vanishing_poly<F: RichField + Extendable<D>, const D: usize>(
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

    let l1_x = plonk_common::eval_l_1(common_data.degree(), x);

    for i in 0..common_data.config.num_challenges {
        let z_x = local_zs[i];
        let z_gz = next_zs[i];
        vanishing_z_1_terms.push(l1_x * (z_x - F::Extension::ONE));

        let numerator_values = (0..common_data.config.num_routed_wires)
            .map(|j| {
                let wire_value = vars.local_wires[j];
                let k_i = common_data.k_is[j];
                let s_id = x.scalar_mul(k_i);
                wire_value + s_id.scalar_mul(betas[i]) + gammas[i].into()
            })
            .collect::<Vec<_>>();
        let denominator_values = (0..common_data.config.num_routed_wires)
            .map(|j| {
                let wire_value = vars.local_wires[j];
                let s_sigma = s_sigmas[j];
                wire_value + s_sigma.scalar_mul(betas[i]) + gammas[i].into()
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

    let alphas = &alphas.iter().map(|&a| a.into()).collect::<Vec<_>>();
    plonk_common::reduce_with_powers_multi(&vanishing_terms, alphas)
}

/// Like `eval_vanishing_poly`, but specialized for base field points.
pub(crate) fn eval_vanishing_poly_base<F: RichField + Extendable<D>, const D: usize>(
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

    let num_challenges = common_data.config.num_challenges;
    let num_routed_wires = common_data.config.num_routed_wires;

    // The L_1(x) (Z(x) - 1) vanishing terms.
    let mut vanishing_z_1_terms = Vec::with_capacity(num_challenges);
    // The terms checking the partial products.
    let mut vanishing_partial_products_terms = Vec::new();
    // The Z(x) f'(x) - g'(x) Z(g x) terms.
    let mut vanishing_v_shift_terms = Vec::with_capacity(num_challenges);

    let l1_x = z_h_on_coset.eval_l1(index, x);

    let mut numerator_values = Vec::with_capacity(num_routed_wires);
    let mut denominator_values = Vec::with_capacity(num_routed_wires);
    let mut quotient_values = Vec::with_capacity(num_routed_wires);
    for i in 0..num_challenges {
        let z_x = local_zs[i];
        let z_gz = next_zs[i];
        vanishing_z_1_terms.push(l1_x * (z_x - F::ONE));

        numerator_values.extend((0..num_routed_wires).map(|j| {
            let wire_value = vars.local_wires[j];
            let k_i = common_data.k_is[j];
            let s_id = k_i * x;
            wire_value + betas[i] * s_id + gammas[i]
        }));
        denominator_values.extend((0..num_routed_wires).map(|j| {
            let wire_value = vars.local_wires[j];
            let s_sigma = s_sigmas[j];
            wire_value + betas[i] * s_sigma + gammas[i]
        }));
        let denominator_inverses = F::batch_multiplicative_inverse(&denominator_values);
        quotient_values
            .extend((0..num_routed_wires).map(|j| numerator_values[j] * denominator_inverses[j]));

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

        numerator_values.clear();
        denominator_values.clear();
        quotient_values.clear();
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

pub(crate) fn eval_vanishing_poly_base_batch<F: RichField + Extendable<D>, const D: usize>(
    common_data: &CommonCircuitData<F, D>,
    indices_batch: &[usize],
    xs_batch: &[F],
    vars_batch: &[EvaluationVarsBase<F>],
    local_zs_batch: &[&[F]],
    next_zs_batch: &[&[F]],
    partial_products_batch: &[&[F]],
    s_sigmas_batch: &[&[F]],
    betas: &[F],
    gammas: &[F],
    alphas: &[F],
    z_h_on_coset: &ZeroPolyOnCoset<F>,
) -> Vec<Vec<F>> {
    let n = indices_batch.len();
    assert!(xs_batch.len() == n);
    assert!(vars_batch.len() == n);
    assert!(local_zs_batch.len() == n);
    assert!(next_zs_batch.len() == n);
    assert!(partial_products_batch.len() == n);
    assert!(s_sigmas_batch.len() == n);

    let max_degree = common_data.quotient_degree_factor;
    let (num_prods, final_num_prod) = common_data.num_partial_products;

    let constraint_terms_batch = evaluate_gate_constraints_base_batch(
        &common_data.gates, common_data.num_gate_constraints, vars_batch);
    debug_assert!(constraint_terms_batch.len() == n);

    let num_challenges = common_data.config.num_challenges;
    let num_routed_wires = common_data.config.num_routed_wires;


    let mut res_batch: Vec<Vec<F>> = Vec::with_capacity(n);
    for i in 0..n {
        // The L_1(x) (Z(x) - 1) vanishing terms.
        let mut vanishing_z_1_terms = Vec::with_capacity(num_challenges);
        // The terms checking the partial products.
        let mut vanishing_partial_products_terms = Vec::new();
        // The Z(x) f'(x) - g'(x) Z(g x) terms.
        let mut vanishing_v_shift_terms = Vec::with_capacity(num_challenges);

        let mut numerator_values = Vec::with_capacity(num_routed_wires);
        let mut denominator_values = Vec::with_capacity(num_routed_wires);
        let mut quotient_values = Vec::with_capacity(num_routed_wires);

        let index = indices_batch[i];
        let x = xs_batch[i];
        let vars = vars_batch[i];
        let local_zs = local_zs_batch[i];
        let next_zs = next_zs_batch[i];
        let partial_products = partial_products_batch[i];
        let s_sigmas = s_sigmas_batch[i];

        let constraint_terms = &constraint_terms_batch[i];

        let l1_x = z_h_on_coset.eval_l1(index, x);
        for i in 0..num_challenges {
            let z_x = local_zs[i];
            let z_gz = next_zs[i];
            vanishing_z_1_terms.push(l1_x * (z_x - F::ONE));

            numerator_values.extend((0..num_routed_wires).map(|j| {
                let wire_value = vars.local_wires[j];
                let k_i = common_data.k_is[j];
                let s_id = k_i * x;
                wire_value + betas[i] * s_id + gammas[i]
            }));
            denominator_values.extend((0..num_routed_wires).map(|j| {
                let wire_value = vars.local_wires[j];
                let s_sigma = s_sigmas[j];
                wire_value + betas[i] * s_sigma + gammas[i]
            }));
            let denominator_inverses = F::batch_multiplicative_inverse(&denominator_values);
            quotient_values
                .extend((0..num_routed_wires).map(|j| numerator_values[j] * denominator_inverses[j]));

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

            numerator_values.clear();
            denominator_values.clear();
            quotient_values.clear();
        }

        let vanishing_terms = [
            vanishing_z_1_terms,
            vanishing_partial_products_terms,
            vanishing_v_shift_terms,
            constraint_terms.clone(),
        ]
        .concat();

        let res = plonk_common::reduce_with_powers_multi(&vanishing_terms, alphas);
        res_batch.push(res);
    }
    res_batch
}

/// Evaluates all gate constraints.
///
/// `num_gate_constraints` is the largest number of constraints imposed by any gate. It is not
/// strictly necessary, but it helps performance by ensuring that we allocate a vector with exactly
/// the capacity that we need.
pub fn evaluate_gate_constraints<F: RichField + Extendable<D>, const D: usize>(
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

pub fn evaluate_gate_constraints_base<F: RichField + Extendable<D>, const D: usize>(
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

pub fn evaluate_gate_constraints_base_batch<F: RichField + Extendable<D>, const D: usize>(
    gates: &[PrefixedGate<F, D>],
    num_gate_constraints: usize,
    vars_batch: &[EvaluationVarsBase<F>],
) -> Vec<Vec<F>> {
    let mut constraints_batch = vec![vec![F::ZERO; num_gate_constraints]; vars_batch.len()];
    for gate in gates {
        let gate_constraints_batch = gate.gate.0.eval_filtered_base_batch(vars_batch, &gate.prefix);
        for (constraints, gate_constraints) in constraints_batch.iter_mut().zip(gate_constraints_batch.iter()) {
            debug_assert!(gate_constraints.len() <= constraints.len(), "num_constraints() gave too low of a number");
            for (constraint, &gate_constraint) in constraints.iter_mut().zip(gate_constraints.iter()) {
                *constraint += gate_constraint;
            }
        }
    }
    constraints_batch
}

pub fn evaluate_gate_constraints_recursively<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    gates: &[PrefixedGate<F, D>],
    num_gate_constraints: usize,
    vars: EvaluationTargets<D>,
) -> Vec<ExtensionTarget<D>> {
    let mut all_gate_constraints = vec![vec![]; num_gate_constraints];
    for gate in gates {
        let gate_constraints = with_context!(
            builder,
            &format!("evaluate {} constraints", gate.gate.0.id()),
            gate.gate
                .0
                .eval_filtered_recursively(builder, vars, &gate.prefix)
        );
        for (i, c) in gate_constraints.into_iter().enumerate() {
            all_gate_constraints[i].push(c);
        }
    }
    let mut constraints = vec![builder.zero_extension(); num_gate_constraints];
    for (i, v) in all_gate_constraints.into_iter().enumerate() {
        constraints[i] = builder.add_many_extension(&v);
    }
    constraints
}

/// Evaluate the vanishing polynomial at `x`. In this context, the vanishing polynomial is a random
/// linear combination of gate constraints, plus some other terms relating to the permutation
/// argument. All such terms should vanish on `H`.
///
/// Assumes `x != 1`; if `x` could be 1 then this is unsound. This is fine if `x` is a random
/// variable drawn from a sufficiently large domain.
pub(crate) fn eval_vanishing_poly_recursively<F: RichField + Extendable<D>, const D: usize>(
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
    let one = builder.one_extension();
    let max_degree = common_data.quotient_degree_factor;
    let (num_prods, final_num_prod) = common_data.num_partial_products;

    let constraint_terms = with_context!(
        builder,
        "evaluate gate constraints",
        evaluate_gate_constraints_recursively(
            builder,
            &common_data.gates,
            common_data.num_gate_constraints,
            vars,
        )
    );

    // The L_1(x) (Z(x) - 1) vanishing terms.
    let mut vanishing_z_1_terms = Vec::new();
    // The terms checking the partial products.
    let mut vanishing_partial_products_terms = Vec::new();
    // The Z(x) f'(x) - g'(x) Z(g x) terms.
    let mut vanishing_v_shift_terms = Vec::new();

    let l1_x = eval_l_1_recursively(builder, common_data.degree(), x, x_pow_deg);

    // Holds `k[i] * x`.
    let mut s_ids = Vec::new();
    for j in 0..common_data.config.num_routed_wires {
        let k = builder.constant(common_data.k_is[j]);
        let k_ext = builder.convert_to_ext(k);
        s_ids.push(builder.mul_extension(k_ext, x));
    }

    for i in 0..common_data.config.num_challenges {
        let z_x = local_zs[i];
        let z_gz = next_zs[i];
        vanishing_z_1_terms.push(builder.arithmetic_extension(F::ONE, F::NEG_ONE, l1_x, z_x, l1_x));

        let numerator_values = (0..common_data.config.num_routed_wires)
            .map(|j| {
                let wire_value = vars.local_wires[j];
                let beta_ext = builder.convert_to_ext(betas[i]);
                let gamma_ext = builder.convert_to_ext(gammas[i]);
                // `beta * s_id + wire_value + gamma`
                builder.wide_arithmetic_extension(beta_ext, s_ids[j], one, wire_value, gamma_ext)
            })
            .collect::<Vec<_>>();
        let denominator_values = (0..common_data.config.num_routed_wires)
            .map(|j| {
                let wire_value = vars.local_wires[j];
                let beta_ext = builder.convert_to_ext(betas[i]);
                let gamma_ext = builder.convert_to_ext(gammas[i]);
                // `beta * s_sigma + wire_value + gamma`
                builder.wide_arithmetic_extension(beta_ext, s_sigmas[j], one, wire_value, gamma_ext)
            })
            .collect::<Vec<_>>();
        let quotient_values = (0..common_data.config.num_routed_wires)
            .map(|j| builder.div_extension(numerator_values[j], denominator_values[j]))
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
                let mut v = d.to_vec();
                v.push(*q);
                *q = builder.mul_many_extension(&v);
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

    alphas
        .iter()
        .map(|&alpha| {
            let alpha = builder.convert_to_ext(alpha);
            let mut alpha = ReducingFactorTarget::new(alpha);
            alpha.reduce(&vanishing_terms, builder)
        })
        .collect()
}
