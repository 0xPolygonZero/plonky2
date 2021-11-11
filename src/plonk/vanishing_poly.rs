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

        // The partial products considered for this iteration of `i`.
        let current_partial_products = &partial_products[i * num_prods..(i + 1) * num_prods];
        // Check the quotient partial products.
        let partial_product_checks = check_partial_products(
            &numerator_values,
            &denominator_values,
            current_partial_products,
            z_x,
            max_degree,
        );
        vanishing_partial_products_terms.extend(partial_product_checks);

        let final_nume_product = numerator_values[final_num_prod..].iter().copied().product();
        let final_deno_product = denominator_values[final_num_prod..].iter().copied().product();
        let last_partial = *current_partial_products.last().unwrap();
        let v_shift_term = last_partial * final_nume_product - z_gz * final_deno_product;
        vanishing_v_shift_terms.push(v_shift_term);
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

/// Like `eval_vanishing_poly`, but specialized for base field points. Batched.
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
    assert_eq!(xs_batch.len(), n);
    assert_eq!(vars_batch.len(), n);
    assert_eq!(local_zs_batch.len(), n);
    assert_eq!(next_zs_batch.len(), n);
    assert_eq!(partial_products_batch.len(), n);
    assert_eq!(s_sigmas_batch.len(), n);

    let max_degree = common_data.quotient_degree_factor;
    let (num_prods, final_num_prod) = common_data.num_partial_products;

    let num_gate_constraints = common_data.num_gate_constraints;

    let constraint_terms_batch =
        evaluate_gate_constraints_base_batch(&common_data.gates, num_gate_constraints, vars_batch);
    debug_assert!(constraint_terms_batch.len() == n * num_gate_constraints);

    let num_challenges = common_data.config.num_challenges;
    let num_routed_wires = common_data.config.num_routed_wires;

    let mut numerator_values = Vec::with_capacity(num_routed_wires);
    let mut denominator_values = Vec::with_capacity(num_routed_wires);

    // The L_1(x) (Z(x) - 1) vanishing terms.
    let mut vanishing_z_1_terms = Vec::with_capacity(num_challenges);
    // The terms checking the partial products.
    let mut vanishing_partial_products_terms = Vec::new();
    // The Z(x) f'(x) - g'(x) Z(g x) terms.
    let mut vanishing_v_shift_terms = Vec::with_capacity(num_challenges);

    let mut res_batch: Vec<Vec<F>> = Vec::with_capacity(n);
    for k in 0..n {
        let index = indices_batch[k];
        let x = xs_batch[k];
        let vars = vars_batch[k];
        let local_zs = local_zs_batch[k];
        let next_zs = next_zs_batch[k];
        let partial_products = partial_products_batch[k];
        let s_sigmas = s_sigmas_batch[k];

        let constraint_terms =
            &constraint_terms_batch[k * num_gate_constraints..(k + 1) * num_gate_constraints];

        let l1_x = z_h_on_coset.eval_l1(index, x);
        for i in 0..num_challenges {
            let z_x = local_zs[i];
            let z_gz = next_zs[i];
            vanishing_z_1_terms.push(l1_x * z_x.sub_one());

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

            // The partial products considered for this iteration of `i`.
            let current_partial_products = &partial_products[i * num_prods..(i + 1) * num_prods];
            // Check the numerator partial products.
            let partial_product_checks = check_partial_products(
                &numerator_values,
                &denominator_values,
                current_partial_products,
                z_x,
                max_degree,
            );
            vanishing_partial_products_terms.extend(partial_product_checks);

            let final_nume_product = numerator_values[final_num_prod..].iter().copied().product();
            let final_deno_product = denominator_values[final_num_prod..].iter().copied().product();
            let last_partial = *current_partial_products.last().unwrap();
            let v_shift_term = last_partial * final_nume_product - z_gz * final_deno_product;
            vanishing_v_shift_terms.push(v_shift_term);

            numerator_values.clear();
            denominator_values.clear();
        }

        let vanishing_terms = vanishing_z_1_terms
            .iter()
            .chain(vanishing_partial_products_terms.iter())
            .chain(vanishing_v_shift_terms.iter())
            .chain(constraint_terms);
        let res = plonk_common::reduce_with_powers_multi(vanishing_terms, alphas);
        res_batch.push(res);

        vanishing_z_1_terms.clear();
        vanishing_partial_products_terms.clear();
        vanishing_v_shift_terms.clear();
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

/// Evaluate all gate constraints in the base field.
///
/// Returns a vector of num_gate_constraints * vars_batch.len() field elements. The constraints
/// corresponding to vars_batch[i] are found in
/// result[num_gate_constraints * i..num_gate_constraints * (i + 1)].
pub fn evaluate_gate_constraints_base_batch<F: RichField + Extendable<D>, const D: usize>(
    gates: &[PrefixedGate<F, D>],
    num_gate_constraints: usize,
    vars_batch: &[EvaluationVarsBase<F>],
) -> Vec<F> {
    let mut constraints_batch = vec![F::ZERO; num_gate_constraints * vars_batch.len()];
    for gate in gates {
        let gate_constraints_batch = gate
            .gate
            .0
            .eval_filtered_base_batch(vars_batch, &gate.prefix);
        for (constraints, gate_constraints) in constraints_batch
            .chunks_exact_mut(num_gate_constraints)
            .zip(gate_constraints_batch.iter())
        {
            debug_assert!(
                gate_constraints.len() <= constraints.len(),
                "num_constraints() gave too low of a number"
            );
            for (constraint, &gate_constraint) in
                constraints.iter_mut().zip(gate_constraints.iter())
            {
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
    let mut all_gate_constraints = vec![builder.zero_extension(); num_gate_constraints];
    for gate in gates {
        with_context!(
            builder,
            &format!("evaluate {} constraints", gate.gate.0.id()),
            gate.gate.0.eval_filtered_recursively(
                builder,
                vars,
                &gate.prefix,
                &mut all_gate_constraints
            )
        );
    }
    all_gate_constraints
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
        s_ids.push(builder.scalar_mul_ext(k, x));
    }

    for i in 0..common_data.config.num_challenges {
        let z_x = local_zs[i];
        let z_gz = next_zs[i];

        // L_1(x) Z(x) = 0.
        vanishing_z_1_terms.push(builder.mul_sub_extension(l1_x, z_x, l1_x));

        let mut numerator_values = Vec::new();
        let mut denominator_values = Vec::new();

        for j in 0..common_data.config.num_routed_wires {
            let wire_value = vars.local_wires[j];
            let beta_ext = builder.convert_to_ext(betas[i]);
            let gamma_ext = builder.convert_to_ext(gammas[i]);

            // The numerator is `beta * s_id + wire_value + gamma`, and the denominator is
            // `beta * s_sigma + wire_value + gamma`.
            let wire_value_plus_gamma = builder.add_extension(wire_value, gamma_ext);
            let numerator = builder.mul_add_extension(beta_ext, s_ids[j], wire_value_plus_gamma);
            let denominator =
                builder.mul_add_extension(beta_ext, s_sigmas[j], wire_value_plus_gamma);
            numerator_values.push(numerator);
            denominator_values.push(denominator);
        }

        // The partial products considered for this iteration of `i`.
        let current_partial_products = &partial_products[i * num_prods..(i + 1) * num_prods];
        // Check the quotient partial products.
        let partial_product_checks = check_partial_products_recursively(
            builder,
            &numerator_values,
            &denominator_values,
            current_partial_products,
            z_x,
            max_degree,
        );
        vanishing_partial_products_terms.extend(partial_product_checks);

        let final_nume_product = builder.mul_many_extension(&numerator_values[final_num_prod..]);
        let final_deno_product = builder.mul_many_extension(&denominator_values[final_num_prod..]);
        let z_gz_denominators = builder.mul_extension(z_gz, final_deno_product);
        let last_partial = *current_partial_products.last().unwrap();
        let v_shift_term = builder.mul_sub_extension(last_partial, final_nume_product, z_gz_denominators);
        vanishing_v_shift_terms.push(v_shift_term);
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
