use plonky2_field::batch_util::batch_add_inplace;
use plonky2_field::extension::{Extendable, FieldExtension};
use plonky2_field::types::Field;
use plonky2_field::zero_poly_coset::ZeroPolyOnCoset;

use crate::hash::hash_types::RichField;
use crate::iop::ext_target::ExtensionTarget;
use crate::iop::target::Target;
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::circuit_data::CommonCircuitData;
use crate::plonk::config::GenericConfig;
use crate::plonk::plonk_common;
use crate::plonk::plonk_common::eval_l_0_circuit;
use crate::plonk::vars::{EvaluationTargets, EvaluationVars, EvaluationVarsBaseBatch};
use crate::util::partial_products::{check_partial_products, check_partial_products_circuit};
use crate::util::reducing::ReducingFactorTarget;
use crate::util::strided_view::PackedStridedView;
use crate::with_context;

/// Evaluate the vanishing polynomial at `x`. In this context, the vanishing polynomial is a random
/// linear combination of gate constraints, plus some other terms relating to the permutation
/// argument. All such terms should vanish on `H`.
pub(crate) fn eval_vanishing_poly<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(
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
    let num_prods = common_data.num_partial_products;

    let constraint_terms = evaluate_gate_constraints::<F, C, D>(common_data, vars);

    // The L_0(x) (Z(x) - 1) vanishing terms.
    let mut vanishing_z_1_terms = Vec::new();
    // The terms checking the partial products.
    let mut vanishing_partial_products_terms = Vec::new();

    let l_0_x = plonk_common::eval_l_0(common_data.degree(), x);

    for i in 0..common_data.config.num_challenges {
        let z_x = local_zs[i];
        let z_gx = next_zs[i];
        vanishing_z_1_terms.push(l_0_x * (z_x - F::Extension::ONE));

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
            z_gx,
            max_degree,
        );
        vanishing_partial_products_terms.extend(partial_product_checks);
    }

    let vanishing_terms = [
        vanishing_z_1_terms,
        vanishing_partial_products_terms,
        constraint_terms,
    ]
    .concat();

    let alphas = &alphas.iter().map(|&a| a.into()).collect::<Vec<_>>();
    plonk_common::reduce_with_powers_multi(&vanishing_terms, alphas)
}

/// Like `eval_vanishing_poly`, but specialized for base field points. Batched.
pub(crate) fn eval_vanishing_poly_base_batch<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(
    common_data: &CommonCircuitData<F, D>,
    indices_batch: &[usize],
    xs_batch: &[F],
    vars_batch: EvaluationVarsBaseBatch<F>,
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
    let num_prods = common_data.num_partial_products;

    let num_gate_constraints = common_data.num_gate_constraints;

    let constraint_terms_batch =
        evaluate_gate_constraints_base_batch::<F, C, D>(common_data, vars_batch);
    debug_assert!(constraint_terms_batch.len() == n * num_gate_constraints);

    let num_challenges = common_data.config.num_challenges;
    let num_routed_wires = common_data.config.num_routed_wires;

    let mut numerator_values = Vec::with_capacity(num_routed_wires);
    let mut denominator_values = Vec::with_capacity(num_routed_wires);

    // The L_0(x) (Z(x) - 1) vanishing terms.
    let mut vanishing_z_1_terms = Vec::with_capacity(num_challenges);
    // The terms checking the partial products.
    let mut vanishing_partial_products_terms = Vec::new();

    let mut res_batch: Vec<Vec<F>> = Vec::with_capacity(n);
    for k in 0..n {
        let index = indices_batch[k];
        let x = xs_batch[k];
        let vars = vars_batch.view(k);
        let local_zs = local_zs_batch[k];
        let next_zs = next_zs_batch[k];
        let partial_products = partial_products_batch[k];
        let s_sigmas = s_sigmas_batch[k];

        let constraint_terms = PackedStridedView::new(&constraint_terms_batch, n, k);

        let l_0_x = z_h_on_coset.eval_l_0(index, x);
        for i in 0..num_challenges {
            let z_x = local_zs[i];
            let z_gx = next_zs[i];
            vanishing_z_1_terms.push(l_0_x * z_x.sub_one());

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
                z_gx,
                max_degree,
            );
            vanishing_partial_products_terms.extend(partial_product_checks);

            numerator_values.clear();
            denominator_values.clear();
        }

        let vanishing_terms = vanishing_z_1_terms
            .iter()
            .chain(vanishing_partial_products_terms.iter())
            .chain(constraint_terms);
        let res = plonk_common::reduce_with_powers_multi(vanishing_terms, alphas);
        res_batch.push(res);

        vanishing_z_1_terms.clear();
        vanishing_partial_products_terms.clear();
    }
    res_batch
}

/// Evaluates all gate constraints.
///
/// `num_gate_constraints` is the largest number of constraints imposed by any gate. It is not
/// strictly necessary, but it helps performance by ensuring that we allocate a vector with exactly
/// the capacity that we need.
pub fn evaluate_gate_constraints<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(
    common_data: &CommonCircuitData<F, D>,
    vars: EvaluationVars<F, D>,
) -> Vec<F::Extension> {
    let mut constraints = vec![F::Extension::ZERO; common_data.num_gate_constraints];
    for (i, gate) in common_data.gates.iter().enumerate() {
        let selector_index = common_data.selectors_info.selector_indices[i];
        let gate_constraints = gate.0.eval_filtered(
            vars,
            i,
            selector_index,
            common_data.selectors_info.groups[selector_index].clone(),
            common_data.selectors_info.num_selectors(),
        );
        for (i, c) in gate_constraints.into_iter().enumerate() {
            debug_assert!(
                i < common_data.num_gate_constraints,
                "num_constraints() gave too low of a number"
            );
            constraints[i] += c;
        }
    }
    constraints
}

/// Evaluate all gate constraints in the base field.
///
/// Returns a vector of `num_gate_constraints * vars_batch.len()` field elements. The constraints
/// corresponding to `vars_batch[i]` are found in `result[i], result[vars_batch.len() + i],
/// result[2 * vars_batch.len() + i], ...`.
pub fn evaluate_gate_constraints_base_batch<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(
    common_data: &CommonCircuitData<F, D>,
    vars_batch: EvaluationVarsBaseBatch<F>,
) -> Vec<F> {
    let mut constraints_batch = vec![F::ZERO; common_data.num_gate_constraints * vars_batch.len()];
    for (i, gate) in common_data.gates.iter().enumerate() {
        let selector_index = common_data.selectors_info.selector_indices[i];
        let gate_constraints_batch = gate.0.eval_filtered_base_batch(
            vars_batch,
            i,
            selector_index,
            common_data.selectors_info.groups[selector_index].clone(),
            common_data.selectors_info.num_selectors(),
        );
        debug_assert!(
            gate_constraints_batch.len() <= constraints_batch.len(),
            "num_constraints() gave too low of a number"
        );
        // below adds all constraints for all points
        batch_add_inplace(
            &mut constraints_batch[..gate_constraints_batch.len()],
            &gate_constraints_batch,
        );
    }
    constraints_batch
}

pub fn evaluate_gate_constraints_circuit<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(
    builder: &mut CircuitBuilder<F, D>,
    common_data: &CommonCircuitData<F, D>,
    vars: EvaluationTargets<D>,
) -> Vec<ExtensionTarget<D>> {
    let mut all_gate_constraints = vec![builder.zero_extension(); common_data.num_gate_constraints];
    for (i, gate) in common_data.gates.iter().enumerate() {
        let selector_index = common_data.selectors_info.selector_indices[i];
        with_context!(
            builder,
            &format!("evaluate {} constraints", gate.0.id()),
            gate.0.eval_filtered_circuit(
                builder,
                vars,
                i,
                selector_index,
                common_data.selectors_info.groups[selector_index].clone(),
                common_data.selectors_info.num_selectors(),
                &mut all_gate_constraints,
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
pub(crate) fn eval_vanishing_poly_circuit<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(
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
    let num_prods = common_data.num_partial_products;

    let constraint_terms = with_context!(
        builder,
        "evaluate gate constraints",
        evaluate_gate_constraints_circuit::<F, C, D>(builder, common_data, vars,)
    );

    // The L_0(x) (Z(x) - 1) vanishing terms.
    let mut vanishing_z_1_terms = Vec::new();
    // The terms checking the partial products.
    let mut vanishing_partial_products_terms = Vec::new();

    let l_0_x = eval_l_0_circuit(builder, common_data.degree(), x, x_pow_deg);

    // Holds `k[i] * x`.
    let mut s_ids = Vec::new();
    for j in 0..common_data.config.num_routed_wires {
        let k = builder.constant(common_data.k_is[j]);
        s_ids.push(builder.scalar_mul_ext(k, x));
    }

    for i in 0..common_data.config.num_challenges {
        let z_x = local_zs[i];
        let z_gx = next_zs[i];

        // L_0(x) (Z(x) - 1) = 0.
        vanishing_z_1_terms.push(builder.mul_sub_extension(l_0_x, z_x, l_0_x));

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
        let partial_product_checks = check_partial_products_circuit(
            builder,
            &numerator_values,
            &denominator_values,
            current_partial_products,
            z_x,
            z_gx,
            max_degree,
        );
        vanishing_partial_products_terms.extend(partial_product_checks);
    }

    let vanishing_terms = [
        vanishing_z_1_terms,
        vanishing_partial_products_terms,
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
