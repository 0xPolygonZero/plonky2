#[cfg(not(feature = "std"))]
use alloc::{format, vec, vec::Vec};
use core::cmp::min;

use plonky2_field::polynomial::PolynomialCoeffs;

use super::circuit_builder::{LookupChallenges, NUM_COINS_LOOKUP};
use super::vars::EvaluationVarsBase;
use crate::field::batch_util::batch_add_inplace;
use crate::field::extension::{Extendable, FieldExtension};
use crate::field::types::Field;
use crate::field::zero_poly_coset::ZeroPolyOnCoset;
use crate::gates::lookup::LookupGate;
use crate::gates::lookup_table::LookupTableGate;
use crate::gates::selectors::LookupSelectors;
use crate::hash::hash_types::RichField;
use crate::iop::ext_target::ExtensionTarget;
use crate::iop::target::Target;
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::circuit_data::CommonCircuitData;
use crate::plonk::plonk_common;
use crate::plonk::plonk_common::eval_l_0_circuit;
use crate::plonk::vars::{EvaluationTargets, EvaluationVars, EvaluationVarsBaseBatch};
use crate::util::partial_products::{check_partial_products, check_partial_products_circuit};
use crate::util::reducing::ReducingFactorTarget;
use crate::util::strided_view::PackedStridedView;
use crate::with_context;

/// Get the polynomial associated to a lookup table with current challenges.
pub(crate) fn get_lut_poly<F: RichField + Extendable<D>, const D: usize>(
    common_data: &CommonCircuitData<F, D>,
    lut_index: usize,
    deltas: &[F],
    degree: usize,
) -> PolynomialCoeffs<F> {
    let b = deltas[LookupChallenges::ChallengeB as usize];
    let mut coeffs = Vec::with_capacity(common_data.luts[lut_index].len());
    let n = common_data.luts[lut_index].len();
    let nb_padded_elts = LookupTableGate::num_slots(&common_data.config)
        - n % LookupTableGate::num_slots(&common_data.config);
    let (padding_inp, padding_out) = common_data.luts[lut_index][0];
    for (input, output) in common_data.luts[lut_index].iter() {
        coeffs.push(F::from_canonical_u16(*input) + b * F::from_canonical_u16(*output));
    }
    // Padding with the first element of the LUT.
    for _ in 0..nb_padded_elts {
        coeffs.push(F::from_canonical_u16(padding_inp) + b * F::from_canonical_u16(padding_out));
    }
    coeffs.append(&mut vec![F::ZERO; degree - (n + nb_padded_elts)]);
    coeffs.reverse();
    PolynomialCoeffs::new(coeffs)
}

/// Evaluate the vanishing polynomial at `x`. In this context, the vanishing polynomial is a random
/// linear combination of gate constraints, plus some other terms relating to the permutation
/// argument. All such terms should vanish on `H`.
pub(crate) fn eval_vanishing_poly<F: RichField + Extendable<D>, const D: usize>(
    common_data: &CommonCircuitData<F, D>,
    x: F::Extension,
    vars: EvaluationVars<F, D>,
    local_zs: &[F::Extension],
    next_zs: &[F::Extension],
    local_lookup_zs: &[F::Extension],
    next_lookup_zs: &[F::Extension],
    partial_products: &[F::Extension],
    s_sigmas: &[F::Extension],
    betas: &[F],
    gammas: &[F],
    alphas: &[F],
    deltas: &[F],
) -> Vec<F::Extension> {
    let has_lookup = common_data.num_lookup_polys != 0;
    let max_degree = common_data.quotient_degree_factor;
    let num_prods = common_data.num_partial_products;

    let constraint_terms = evaluate_gate_constraints::<F, D>(common_data, vars);

    let lookup_selectors = &vars.local_constants[common_data.selectors_info.num_selectors()
        ..common_data.selectors_info.num_selectors() + common_data.num_lookup_selectors];

    // The L_0(x) (Z(x) - 1) vanishing terms.
    let mut vanishing_z_1_terms = Vec::new();

    // The terms checking the lookup constraints, if any.
    let mut vanishing_all_lookup_terms = if has_lookup {
        let num_sldc_polys = common_data.num_lookup_polys - 1;
        Vec::with_capacity(
            common_data.config.num_challenges * (4 + common_data.luts.len() + 2 * num_sldc_polys),
        )
    } else {
        Vec::new()
    };

    // The terms checking the partial products.
    let mut vanishing_partial_products_terms = Vec::new();

    let l_0_x = plonk_common::eval_l_0(common_data.degree(), x);

    for i in 0..common_data.config.num_challenges {
        let z_x = local_zs[i];
        let z_gx = next_zs[i];
        vanishing_z_1_terms.push(l_0_x * (z_x - F::Extension::ONE));

        if has_lookup {
            let cur_local_lookup_zs = &local_lookup_zs
                [common_data.num_lookup_polys * i..common_data.num_lookup_polys * (i + 1)];
            let cur_next_lookup_zs = &next_lookup_zs
                [common_data.num_lookup_polys * i..common_data.num_lookup_polys * (i + 1)];

            let cur_deltas = &deltas[NUM_COINS_LOOKUP * i..NUM_COINS_LOOKUP * (i + 1)];

            let lookup_constraints = check_lookup_constraints(
                common_data,
                vars,
                cur_local_lookup_zs,
                cur_next_lookup_zs,
                lookup_selectors,
                cur_deltas.try_into().unwrap(),
            );

            vanishing_all_lookup_terms.extend(lookup_constraints);
        }

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
        vanishing_all_lookup_terms,
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
    vars_batch: EvaluationVarsBaseBatch<F>,
    local_zs_batch: &[&[F]],
    next_zs_batch: &[&[F]],
    local_lookup_zs_batch: &[&[F]],
    next_lookup_zs_batch: &[&[F]],
    partial_products_batch: &[&[F]],
    s_sigmas_batch: &[&[F]],
    betas: &[F],
    gammas: &[F],
    deltas: &[F],
    alphas: &[F],
    z_h_on_coset: &ZeroPolyOnCoset<F>,
    lut_re_poly_evals: &[&[F]],
) -> Vec<Vec<F>> {
    let has_lookup = common_data.num_lookup_polys != 0;

    let n = indices_batch.len();
    assert_eq!(xs_batch.len(), n);
    assert_eq!(vars_batch.len(), n);
    assert_eq!(local_zs_batch.len(), n);
    assert_eq!(next_zs_batch.len(), n);
    if has_lookup {
        assert_eq!(local_lookup_zs_batch.len(), n);
        assert_eq!(next_lookup_zs_batch.len(), n);
    } else {
        assert_eq!(local_lookup_zs_batch.len(), 0);
        assert_eq!(next_lookup_zs_batch.len(), 0);
    }
    assert_eq!(partial_products_batch.len(), n);
    assert_eq!(s_sigmas_batch.len(), n);

    let max_degree = common_data.quotient_degree_factor;
    let num_prods = common_data.num_partial_products;

    let num_gate_constraints = common_data.num_gate_constraints;

    let constraint_terms_batch =
        evaluate_gate_constraints_base_batch::<F, D>(common_data, vars_batch);
    debug_assert!(constraint_terms_batch.len() == n * num_gate_constraints);

    let num_challenges = common_data.config.num_challenges;
    let num_routed_wires = common_data.config.num_routed_wires;

    let mut numerator_values = Vec::with_capacity(num_routed_wires);
    let mut denominator_values = Vec::with_capacity(num_routed_wires);

    // The L_0(x) (Z(x) - 1) vanishing terms.
    let mut vanishing_z_1_terms = Vec::with_capacity(num_challenges);
    // The terms checking the partial products.
    let mut vanishing_partial_products_terms = Vec::new();

    // The terms checking the lookup constraints.
    let mut vanishing_all_lookup_terms = if has_lookup {
        let num_sldc_polys = common_data.num_lookup_polys - 1;
        Vec::with_capacity(
            common_data.config.num_challenges * (4 + common_data.luts.len() + 2 * num_sldc_polys),
        )
    } else {
        Vec::new()
    };

    let mut res_batch: Vec<Vec<F>> = Vec::with_capacity(n);
    for k in 0..n {
        let index = indices_batch[k];
        let x = xs_batch[k];
        let vars = vars_batch.view(k);

        let lookup_selectors: Vec<F> = (0..common_data.num_lookup_selectors)
            .map(|i| vars.local_constants[common_data.selectors_info.num_selectors() + i])
            .collect();

        let local_zs = local_zs_batch[k];
        let next_zs = next_zs_batch[k];
        let local_lookup_zs = if has_lookup {
            local_lookup_zs_batch[k]
        } else {
            &[]
        };

        let next_lookup_zs = if has_lookup {
            next_lookup_zs_batch[k]
        } else {
            &[]
        };

        let partial_products = partial_products_batch[k];
        let s_sigmas = s_sigmas_batch[k];

        let constraint_terms = PackedStridedView::new(&constraint_terms_batch, n, k);

        let l_0_x = z_h_on_coset.eval_l_0(index, x);
        for i in 0..num_challenges {
            let z_x = local_zs[i];
            let z_gx = next_zs[i];
            vanishing_z_1_terms.push(l_0_x * z_x.sub_one());

            // If there are lookups in the circuit, then we add the lookup constraints.
            if has_lookup {
                let cur_deltas = &deltas[NUM_COINS_LOOKUP * i..NUM_COINS_LOOKUP * (i + 1)];

                let cur_local_lookup_zs = &local_lookup_zs
                    [common_data.num_lookup_polys * i..common_data.num_lookup_polys * (i + 1)];
                let cur_next_lookup_zs = &next_lookup_zs
                    [common_data.num_lookup_polys * i..common_data.num_lookup_polys * (i + 1)];

                let lookup_constraints = check_lookup_constraints_batch(
                    common_data,
                    vars,
                    cur_local_lookup_zs,
                    cur_next_lookup_zs,
                    &lookup_selectors,
                    cur_deltas.try_into().unwrap(),
                    lut_re_poly_evals[i],
                );
                vanishing_all_lookup_terms.extend(lookup_constraints);
            }

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
            .chain(vanishing_all_lookup_terms.iter())
            .chain(constraint_terms);
        let res = plonk_common::reduce_with_powers_multi(vanishing_terms, alphas);
        res_batch.push(res);

        vanishing_z_1_terms.clear();
        vanishing_partial_products_terms.clear();
        vanishing_all_lookup_terms.clear();
    }
    res_batch
}

/// Evaluates all lookup constraints, based on the logarithmic derivatives paper (<https://eprint.iacr.org/2022/1530.pdf>),
/// following the Tip5 paper's implementation (<https://eprint.iacr.org/2023/107.pdf>).
///
/// There are three polynomials to check:
/// - RE ensures the well formation of lookup tables;
/// - Sum is a running sum of m_i/(X - (input_i + a * output_i)) where (input_i, output_i) are input pairs in the lookup table (LUT);
/// - LDC is a running sum of 1/(X - (input_i + a * output_i)) where (input_i, output_i) are input pairs that look in the LUT.
///
/// Sum and LDC are broken down in partial polynomials to lower the constraint degree, similarly to the permutation argument.
/// They also share the same partial SLDC polynomials, so that the last SLDC value is Sum(end) - LDC(end). The final constraint
/// Sum(end) = LDC(end) becomes simply SLDC(end) = 0, and we can remove the LDC initial constraint.
pub fn check_lookup_constraints<F: RichField + Extendable<D>, const D: usize>(
    common_data: &CommonCircuitData<F, D>,
    vars: EvaluationVars<F, D>,
    local_lookup_zs: &[F::Extension],
    next_lookup_zs: &[F::Extension],
    lookup_selectors: &[F::Extension],
    deltas: &[F; 4],
) -> Vec<F::Extension> {
    let num_lu_slots = LookupGate::num_slots(&common_data.config);
    let num_lut_slots = LookupTableGate::num_slots(&common_data.config);
    let lu_degree = common_data.quotient_degree_factor - 1;
    let num_sldc_polys = local_lookup_zs.len() - 1;
    let lut_degree = num_lut_slots.div_ceil(num_sldc_polys);

    let mut constraints = Vec::with_capacity(4 + common_data.luts.len() + 2 * num_sldc_polys);

    // RE is the first polynomial stored.
    let z_re = local_lookup_zs[0];
    let next_z_re = next_lookup_zs[0];

    // Partial Sums and LDCs are both stored in the remaining SLDC polynomials.
    let z_x_lookup_sldcs = &local_lookup_zs[1..num_sldc_polys + 1];
    let z_gx_lookup_sldcs = &next_lookup_zs[1..num_sldc_polys + 1];

    let delta_challenge_a = F::Extension::from(deltas[LookupChallenges::ChallengeA as usize]);
    let delta_challenge_b = F::Extension::from(deltas[LookupChallenges::ChallengeB as usize]);

    // Compute all current looked and looking combos, i.e. the combos we need for the SLDC polynomials.
    let current_looked_combos: Vec<F::Extension> = (0..num_lut_slots)
        .map(|s| {
            let input_wire = vars.local_wires[LookupTableGate::wire_ith_looked_inp(s)];
            let output_wire = vars.local_wires[LookupTableGate::wire_ith_looked_out(s)];
            input_wire + delta_challenge_a * output_wire
        })
        .collect();

    let current_looking_combos: Vec<F::Extension> = (0..num_lu_slots)
        .map(|s| {
            let input_wire = vars.local_wires[LookupGate::wire_ith_looking_inp(s)];
            let output_wire = vars.local_wires[LookupGate::wire_ith_looking_out(s)];
            input_wire + delta_challenge_a * output_wire
        })
        .collect();

    // Compute all current lookup combos, i.e. the combos used to check that the LUT is correct.
    let current_lookup_combos: Vec<F::Extension> = (0..num_lut_slots)
        .map(|s| {
            let input_wire = vars.local_wires[LookupTableGate::wire_ith_looked_inp(s)];
            let output_wire = vars.local_wires[LookupTableGate::wire_ith_looked_out(s)];
            input_wire + delta_challenge_b * output_wire
        })
        .collect();

    // Check last LDC constraint.
    constraints.push(
        lookup_selectors[LookupSelectors::LastLdc as usize] * z_x_lookup_sldcs[num_sldc_polys - 1],
    );

    // Check initial Sum constraint.
    constraints.push(lookup_selectors[LookupSelectors::InitSre as usize] * z_x_lookup_sldcs[0]);

    // Check initial RE constraint.
    constraints.push(lookup_selectors[LookupSelectors::InitSre as usize] * z_re);

    let current_delta = deltas[LookupChallenges::ChallengeDelta as usize];

    // Check final RE constraints for each different LUT.
    for r in LookupSelectors::StartEnd as usize..common_data.num_lookup_selectors {
        let cur_ends_selector = lookup_selectors[r];
        let lut_row_number = common_data.luts[r - LookupSelectors::StartEnd as usize]
            .len()
            .div_ceil(num_lut_slots);
        let cur_function_eval = get_lut_poly(
            common_data,
            r - LookupSelectors::StartEnd as usize,
            deltas,
            num_lut_slots * lut_row_number,
        )
        .eval(current_delta);

        constraints.push(cur_ends_selector * (z_re - cur_function_eval.into()))
    }

    // Check RE row transition constraint.
    let mut cur_sum = next_z_re;
    for elt in &current_lookup_combos {
        cur_sum =
            cur_sum * F::Extension::from(deltas[LookupChallenges::ChallengeDelta as usize]) + *elt;
    }
    let unfiltered_re_line = z_re - cur_sum;

    constraints.push(lookup_selectors[LookupSelectors::TransSre as usize] * unfiltered_re_line);

    for poly in 0..num_sldc_polys {
        // Compute prod(alpha - combo) for the current slot for Sum.
        let lut_prod: F::Extension = (poly * lut_degree
            ..min((poly + 1) * lut_degree, num_lut_slots))
            .map(|i| {
                F::Extension::from(deltas[LookupChallenges::ChallengeAlpha as usize])
                    - current_looked_combos[i]
            })
            .product();

        // Compute prod(alpha - combo) for the current slot for LDC.
        let lu_prod: F::Extension = (poly * lu_degree..min((poly + 1) * lu_degree, num_lu_slots))
            .map(|i| {
                F::Extension::from(deltas[LookupChallenges::ChallengeAlpha as usize])
                    - current_looking_combos[i]
            })
            .product();

        // Function which computes, given index i: prod_{j!=i}(alpha - combo_j) for Sum.
        let lut_prod_i = |i| {
            (poly * lut_degree..min((poly + 1) * lut_degree, num_lut_slots))
                .map(|j| {
                    if j != i {
                        F::Extension::from(deltas[LookupChallenges::ChallengeAlpha as usize])
                            - current_looked_combos[j]
                    } else {
                        F::Extension::ONE
                    }
                })
                .product()
        };

        // Function which computes, given index i: prod_{j!=i}(alpha - combo_j) for LDC.
        let lu_prod_i = |i| {
            (poly * lu_degree..min((poly + 1) * lu_degree, num_lu_slots))
                .map(|j| {
                    if j != i {
                        F::Extension::from(deltas[LookupChallenges::ChallengeAlpha as usize])
                            - current_looking_combos[j]
                    } else {
                        F::Extension::ONE
                    }
                })
                .product()
        };
        // Compute sum_i(prod_{j!=i}(alpha - combo_j)) for LDC.
        let lu_sum_prods = (poly * lu_degree..min((poly + 1) * lu_degree, num_lu_slots))
            .fold(F::Extension::ZERO, |acc, i| acc + lu_prod_i(i));

        // Compute sum_i(mul_i.prod_{j!=i}(alpha - combo_j)) for Sum.
        let lut_sum_prods_with_mul = (poly * lut_degree
            ..min((poly + 1) * lut_degree, num_lut_slots))
            .fold(F::Extension::ZERO, |acc, i| {
                acc + vars.local_wires[LookupTableGate::wire_ith_multiplicity(i)] * lut_prod_i(i)
            });

        // The previous element is the previous poly of the current row or the last poly of the next row.
        let prev = if poly == 0 {
            z_gx_lookup_sldcs[num_sldc_polys - 1]
        } else {
            z_x_lookup_sldcs[poly - 1]
        };

        // Check Sum row and col transitions. It's the same constraint, with a row transition happening for slot == 0.
        let unfiltered_sum_transition =
            lut_prod * (z_x_lookup_sldcs[poly] - prev) - lut_sum_prods_with_mul;
        constraints
            .push(lookup_selectors[LookupSelectors::TransSre as usize] * unfiltered_sum_transition);

        // Check LDC row and col transitions. It's the same constraint, with a row transition happening for slot == 0.
        let unfiltered_ldc_transition = lu_prod * (z_x_lookup_sldcs[poly] - prev) + lu_sum_prods;
        constraints
            .push(lookup_selectors[LookupSelectors::TransLdc as usize] * unfiltered_ldc_transition);
    }

    constraints
}

/// Same as `check_lookup_constraints`, but for the base field case.
pub fn check_lookup_constraints_batch<F: RichField + Extendable<D>, const D: usize>(
    common_data: &CommonCircuitData<F, D>,
    vars: EvaluationVarsBase<F>,
    local_lookup_zs: &[F],
    next_lookup_zs: &[F],
    lookup_selectors: &[F],
    deltas: &[F; 4],
    lut_re_poly_evals: &[F],
) -> Vec<F> {
    let num_lu_slots = LookupGate::num_slots(&common_data.config);
    let num_lut_slots = LookupTableGate::num_slots(&common_data.config);
    let lu_degree = common_data.quotient_degree_factor - 1;
    let num_sldc_polys = local_lookup_zs.len() - 1;
    let lut_degree = num_lut_slots.div_ceil(num_sldc_polys);

    let mut constraints = Vec::with_capacity(4 + common_data.luts.len() + 2 * num_sldc_polys);

    // RE is the first polynomial stored.
    let z_re = local_lookup_zs[0];
    let next_z_re = next_lookup_zs[0];

    // Partial Sums and LDCs are both stored in the remaining polynomials.
    let z_x_lookup_sldcs = &local_lookup_zs[1..num_sldc_polys + 1];
    let z_gx_lookup_sldcs = &next_lookup_zs[1..num_sldc_polys + 1];

    // Compute all current looked and looking combos, i.e. the combos we need for the SLDC polynomials.
    let current_looked_combos: Vec<F> = (0..num_lut_slots)
        .map(|s| {
            let input_wire = vars.local_wires[LookupTableGate::wire_ith_looked_inp(s)];
            let output_wire = vars.local_wires[LookupTableGate::wire_ith_looked_out(s)];
            input_wire + deltas[LookupChallenges::ChallengeA as usize] * output_wire
        })
        .collect();

    let current_looking_combos: Vec<F> = (0..num_lu_slots)
        .map(|s| {
            let input_wire = vars.local_wires[LookupGate::wire_ith_looking_inp(s)];
            let output_wire = vars.local_wires[LookupGate::wire_ith_looking_out(s)];
            input_wire + deltas[LookupChallenges::ChallengeA as usize] * output_wire
        })
        .collect();

    // Compute all current lookup combos, i.e. the combos used to check that the LUT is correct.
    let current_lookup_combos: Vec<F> = (0..num_lut_slots)
        .map(|s| {
            let input_wire = vars.local_wires[LookupTableGate::wire_ith_looked_inp(s)];
            let output_wire = vars.local_wires[LookupTableGate::wire_ith_looked_out(s)];
            input_wire + deltas[LookupChallenges::ChallengeB as usize] * output_wire
        })
        .collect();

    // Check last LDC constraint.
    constraints.push(
        lookup_selectors[LookupSelectors::LastLdc as usize] * z_x_lookup_sldcs[num_sldc_polys - 1],
    );

    // Check initial Sum constraint.
    constraints.push(lookup_selectors[LookupSelectors::InitSre as usize] * z_x_lookup_sldcs[0]);

    // Check initial RE constraint.
    constraints.push(lookup_selectors[LookupSelectors::InitSre as usize] * z_re);

    // Check final RE constraints for each different LUT.
    for r in LookupSelectors::StartEnd as usize..common_data.num_lookup_selectors {
        let cur_ends_selector = lookup_selectors[r];

        // Use the precomputed value for the lut poly evaluation
        let re_poly_eval = lut_re_poly_evals[r - LookupSelectors::StartEnd as usize];

        constraints.push(cur_ends_selector * (z_re - re_poly_eval))
    }

    // Check RE row transition constraint.
    let mut cur_sum = next_z_re;
    for elt in &current_lookup_combos {
        cur_sum = cur_sum * deltas[LookupChallenges::ChallengeDelta as usize] + *elt;
    }
    let unfiltered_re_line = z_re - cur_sum;

    constraints.push(lookup_selectors[LookupSelectors::TransSre as usize] * unfiltered_re_line);

    for poly in 0..num_sldc_polys {
        // Compute prod(alpha - combo) for the current slot for Sum.
        let lut_prod: F = (poly * lut_degree..min((poly + 1) * lut_degree, num_lut_slots))
            .map(|i| deltas[LookupChallenges::ChallengeAlpha as usize] - current_looked_combos[i])
            .product();

        // Compute prod(alpha - combo) for the current slot for LDC.
        let lu_prod: F = (poly * lu_degree..min((poly + 1) * lu_degree, num_lu_slots))
            .map(|i| deltas[LookupChallenges::ChallengeAlpha as usize] - current_looking_combos[i])
            .product();

        // Function which computes, given index i: prod_{j!=i}(alpha - combo_j) for Sum.
        let lut_prod_i = |i| {
            (poly * lut_degree..min((poly + 1) * lut_degree, num_lut_slots))
                .map(|j| {
                    if j != i {
                        deltas[LookupChallenges::ChallengeAlpha as usize] - current_looked_combos[j]
                    } else {
                        F::ONE
                    }
                })
                .product()
        };

        // Function which computes, given index i: prod_{j!=i}(alpha - combo_j) for LDC.
        let lu_prod_i = |i| {
            (poly * lu_degree..min((poly + 1) * lu_degree, num_lu_slots))
                .map(|j| {
                    if j != i {
                        deltas[LookupChallenges::ChallengeAlpha as usize]
                            - current_looking_combos[j]
                    } else {
                        F::ONE
                    }
                })
                .product()
        };

        // Compute sum_i(prod_{j!=i}(alpha - combo_j)) for LDC.
        let lu_sum_prods = (poly * lu_degree..min((poly + 1) * lu_degree, num_lu_slots))
            .fold(F::ZERO, |acc, i| acc + lu_prod_i(i));

        // Compute sum_i(mul_i.prod_{j!=i}(alpha - combo_j)) for Sum.
        let lut_sum_prods_with_mul = (poly * lut_degree
            ..min((poly + 1) * lut_degree, num_lut_slots))
            .fold(F::ZERO, |acc, i| {
                acc + vars.local_wires[LookupTableGate::wire_ith_multiplicity(i)] * lut_prod_i(i)
            });

        // The previous element is the previous poly of the current row or the last poly of the next row.
        let prev = if poly == 0 {
            z_gx_lookup_sldcs[num_sldc_polys - 1]
        } else {
            z_x_lookup_sldcs[poly - 1]
        };

        // Check Sum row and col transitions. It's the same constraint, with a row transition happening for slot == 0.
        let unfiltered_sum_transition =
            lut_prod * (z_x_lookup_sldcs[poly] - prev) - lut_sum_prods_with_mul;
        constraints
            .push(lookup_selectors[LookupSelectors::TransSre as usize] * unfiltered_sum_transition);

        // Check LDC row and col transitions. It's the same constraint, with a row transition happening for slot == 0.
        let unfiltered_ldc_transition = lu_prod * (z_x_lookup_sldcs[poly] - prev) + lu_sum_prods;
        constraints
            .push(lookup_selectors[LookupSelectors::TransLdc as usize] * unfiltered_ldc_transition);
    }
    constraints
}

/// Evaluates all gate constraints.
///
/// `num_gate_constraints` is the largest number of constraints imposed by any gate. It is not
/// strictly necessary, but it helps performance by ensuring that we allocate a vector with exactly
/// the capacity that we need.
pub fn evaluate_gate_constraints<F: RichField + Extendable<D>, const D: usize>(
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
            common_data.num_lookup_selectors,
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
pub fn evaluate_gate_constraints_base_batch<F: RichField + Extendable<D>, const D: usize>(
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
            common_data.num_lookup_selectors,
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

pub fn evaluate_gate_constraints_circuit<F: RichField + Extendable<D>, const D: usize>(
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
                common_data.num_lookup_selectors,
                &mut all_gate_constraints,
            )
        );
    }
    all_gate_constraints
}

pub(crate) fn get_lut_poly_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    common_data: &CommonCircuitData<F, D>,
    lut_index: usize,
    deltas: &[Target],
    degree: usize,
) -> Target {
    let b = deltas[LookupChallenges::ChallengeB as usize];
    let delta = deltas[LookupChallenges::ChallengeDelta as usize];
    let n = common_data.luts[lut_index].len();
    let nb_padded_elts = LookupTableGate::num_slots(&common_data.config)
        - n % LookupTableGate::num_slots(&common_data.config);
    let (padding_inp, padding_out) = common_data.luts[lut_index][0];
    let mut coeffs: Vec<Target> = common_data.luts[lut_index]
        .iter()
        .map(|(input, output)| {
            let temp = builder.mul_const(F::from_canonical_u16(*output), b);
            builder.add_const(temp, F::from_canonical_u16(*input))
        })
        .collect();

    // Padding with the first element of the LUT.
    for _ in 0..nb_padded_elts {
        let temp = builder.mul_const(F::from_canonical_u16(padding_out), b);
        let temp = builder.add_const(temp, F::from_canonical_u16(padding_inp));
        coeffs.push(temp);
    }
    for _ in (n + nb_padded_elts)..degree {
        coeffs.push(builder.zero());
    }
    coeffs.reverse();
    coeffs
        .iter()
        .rev()
        .fold(builder.constant(F::ZERO), |acc, &c| {
            let temp = builder.mul(acc, delta);
            builder.add(temp, c)
        })
}

/// Evaluate the vanishing polynomial at `x`. In this context, the vanishing polynomial is a random
/// linear combination of gate constraints, plus some other terms relating to the permutation
/// argument. All such terms should vanish on `H`.
///
/// Assumes `x != 1`; if `x` could be 1 then this is unsound. This is fine if `x` is a random
/// variable drawn from a sufficiently large domain.
pub(crate) fn eval_vanishing_poly_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    common_data: &CommonCircuitData<F, D>,
    x: ExtensionTarget<D>,
    x_pow_deg: ExtensionTarget<D>,
    vars: EvaluationTargets<D>,
    local_zs: &[ExtensionTarget<D>],
    next_zs: &[ExtensionTarget<D>],
    local_lookup_zs: &[ExtensionTarget<D>],
    next_lookup_zs: &[ExtensionTarget<D>],
    partial_products: &[ExtensionTarget<D>],
    s_sigmas: &[ExtensionTarget<D>],
    betas: &[Target],
    gammas: &[Target],
    alphas: &[Target],
    deltas: &[Target],
) -> Vec<ExtensionTarget<D>> {
    let has_lookup = common_data.num_lookup_polys != 0;
    let max_degree = common_data.quotient_degree_factor;
    let num_prods = common_data.num_partial_products;

    let constraint_terms = with_context!(
        builder,
        "evaluate gate constraints",
        evaluate_gate_constraints_circuit::<F, D>(builder, common_data, vars,)
    );

    let lookup_selectors = &vars.local_constants[common_data.selectors_info.num_selectors()
        ..common_data.selectors_info.num_selectors() + common_data.num_lookup_selectors];

    // The L_0(x) (Z(x) - 1) vanishing terms.
    let mut vanishing_z_1_terms = Vec::new();

    // The terms checking lookup constraints.
    let mut vanishing_all_lookup_terms = if has_lookup {
        let num_sldc_polys = common_data.num_lookup_polys - 1;
        Vec::with_capacity(
            common_data.config.num_challenges * (4 + common_data.luts.len() + 2 * num_sldc_polys),
        )
    } else {
        Vec::new()
    };

    // The terms checking the partial products.
    let mut vanishing_partial_products_terms = Vec::new();

    let l_0_x = eval_l_0_circuit(builder, common_data.degree(), x, x_pow_deg);

    // Holds `k[i] * x`.
    let mut s_ids = Vec::with_capacity(common_data.config.num_routed_wires);
    for j in 0..common_data.config.num_routed_wires {
        let k = builder.constant(common_data.k_is[j]);
        s_ids.push(builder.scalar_mul_ext(k, x));
    }

    for i in 0..common_data.config.num_challenges {
        let z_x = local_zs[i];
        let z_gx = next_zs[i];

        // L_0(x) (Z(x) - 1) = 0.
        vanishing_z_1_terms.push(builder.mul_sub_extension(l_0_x, z_x, l_0_x));

        // If there are lookups in the circuit, then we add the lookup constraints
        if has_lookup {
            let cur_local_lookup_zs = &local_lookup_zs
                [common_data.num_lookup_polys * i..common_data.num_lookup_polys * (i + 1)];
            let cur_next_lookup_zs = &next_lookup_zs
                [common_data.num_lookup_polys * i..common_data.num_lookup_polys * (i + 1)];

            let cur_deltas = &deltas[NUM_COINS_LOOKUP * i..NUM_COINS_LOOKUP * (i + 1)];

            let lookup_constraints = check_lookup_constraints_circuit(
                builder,
                common_data,
                vars,
                cur_local_lookup_zs,
                cur_next_lookup_zs,
                lookup_selectors,
                cur_deltas,
            );
            vanishing_all_lookup_terms.extend(lookup_constraints);
        }

        let mut numerator_values = Vec::with_capacity(common_data.config.num_routed_wires);
        let mut denominator_values = Vec::with_capacity(common_data.config.num_routed_wires);

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
        vanishing_all_lookup_terms,
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

/// Same as `check_lookup_constraints`, but for the recursive case.
pub fn check_lookup_constraints_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    common_data: &CommonCircuitData<F, D>,
    vars: EvaluationTargets<D>,
    local_lookup_zs: &[ExtensionTarget<D>],
    next_lookup_zs: &[ExtensionTarget<D>],
    lookup_selectors: &[ExtensionTarget<D>],
    deltas: &[Target],
) -> Vec<ExtensionTarget<D>> {
    let num_lu_slots = LookupGate::num_slots(&common_data.config);
    let num_lut_slots = LookupTableGate::num_slots(&common_data.config);
    let lu_degree = common_data.quotient_degree_factor - 1;
    let num_sldc_polys = local_lookup_zs.len() - 1;
    let lut_degree = num_lut_slots.div_ceil(num_sldc_polys);

    let mut constraints = Vec::with_capacity(4 + common_data.luts.len() + 2 * num_sldc_polys);

    // RE is the first polynomial stored.
    let z_re = local_lookup_zs[0];
    let next_z_re = next_lookup_zs[0];

    // Partial Sums and LDCs (i.e. the SLDC polynomials) are stored in the remaining polynomials.
    let z_x_lookup_sldcs = &local_lookup_zs[1..num_sldc_polys + 1];
    let z_gx_lookup_sldcs = &next_lookup_zs[1..num_sldc_polys + 1];

    // Convert deltas to ExtensionTargets.
    let ext_deltas = deltas
        .iter()
        .map(|d| builder.convert_to_ext(*d))
        .collect::<Vec<_>>();

    // Computing all current looked and looking combos, i.e. the combos we need for the SLDC polynomials.
    let current_looked_combos = (0..num_lut_slots)
        .map(|s| {
            let input_wire = vars.local_wires[LookupTableGate::wire_ith_looked_inp(s)];
            let output_wire = vars.local_wires[LookupTableGate::wire_ith_looked_out(s)];
            builder.mul_add_extension(
                ext_deltas[LookupChallenges::ChallengeA as usize],
                output_wire,
                input_wire,
            )
        })
        .collect::<Vec<_>>();
    let current_looking_combos = (0..num_lu_slots)
        .map(|s| {
            let input_wire = vars.local_wires[LookupGate::wire_ith_looking_inp(s)];
            let output_wire = vars.local_wires[LookupGate::wire_ith_looking_out(s)];
            builder.mul_add_extension(
                ext_deltas[LookupChallenges::ChallengeA as usize],
                output_wire,
                input_wire,
            )
        })
        .collect::<Vec<_>>();

    let current_lut_subs = (0..num_lut_slots)
        .map(|s| {
            builder.sub_extension(
                ext_deltas[LookupChallenges::ChallengeAlpha as usize],
                current_looked_combos[s],
            )
        })
        .collect::<Vec<_>>();

    let current_lu_subs = (0..num_lu_slots)
        .map(|s| {
            builder.sub_extension(
                ext_deltas[LookupChallenges::ChallengeAlpha as usize],
                current_looking_combos[s],
            )
        })
        .collect::<Vec<_>>();

    // Computing all current lookup combos, i.e. the combos used to check that the LUT is correct.
    let current_lookup_combos = (0..num_lut_slots)
        .map(|s| {
            let input_wire = vars.local_wires[LookupTableGate::wire_ith_looked_inp(s)];
            let output_wire = vars.local_wires[LookupTableGate::wire_ith_looked_out(s)];
            builder.mul_add_extension(
                ext_deltas[LookupChallenges::ChallengeB as usize],
                output_wire,
                input_wire,
            )
        })
        .collect::<Vec<_>>();

    // Check last LDC constraint.
    constraints.push(builder.mul_extension(
        lookup_selectors[LookupSelectors::LastLdc as usize],
        z_x_lookup_sldcs[num_sldc_polys - 1],
    ));

    // Check initial Sum constraint.
    constraints.push(builder.mul_extension(
        lookup_selectors[LookupSelectors::InitSre as usize],
        z_x_lookup_sldcs[0],
    ));

    // Check initial RE constraint.
    constraints
        .push(builder.mul_extension(lookup_selectors[LookupSelectors::InitSre as usize], z_re));

    // Check final RE constraints for each different LUT.
    for r in LookupSelectors::StartEnd as usize..common_data.num_lookup_selectors {
        let cur_ends_selectors = lookup_selectors[r];
        let lut_row_number = common_data.luts[r - LookupSelectors::StartEnd as usize]
            .len()
            .div_ceil(num_lut_slots);
        let cur_function_eval = get_lut_poly_circuit(
            builder,
            common_data,
            r - LookupSelectors::StartEnd as usize,
            deltas,
            num_lut_slots * lut_row_number,
        );
        let cur_function_eval_ext = builder.convert_to_ext(cur_function_eval);

        let cur_re = builder.sub_extension(z_re, cur_function_eval_ext);
        constraints.push(builder.mul_extension(cur_ends_selectors, cur_re));
    }

    // Check RE row transition constraint.
    let mut cur_sum = next_z_re;
    for elt in &current_lookup_combos {
        cur_sum = builder.mul_add_extension(
            cur_sum,
            ext_deltas[LookupChallenges::ChallengeDelta as usize],
            *elt,
        );
    }
    let unfiltered_re_line = builder.sub_extension(z_re, cur_sum);

    constraints.push(builder.mul_extension(
        lookup_selectors[LookupSelectors::TransSre as usize],
        unfiltered_re_line,
    ));

    for poly in 0..num_sldc_polys {
        // Compute prod(alpha - combo) for the current slot for Sum.
        let mut lut_prod = builder.one_extension();
        for i in poly * lut_degree..min((poly + 1) * lut_degree, num_lut_slots) {
            lut_prod = builder.mul_extension(lut_prod, current_lut_subs[i]);
        }

        // Compute prod(alpha - combo) for the current slot for LDC.
        let mut lu_prod = builder.one_extension();
        for i in poly * lu_degree..min((poly + 1) * lu_degree, num_lu_slots) {
            lu_prod = builder.mul_extension(lu_prod, current_lu_subs[i]);
        }

        let one = builder.one_extension();
        let zero = builder.zero_extension();

        // Compute sum_i(prod_{j!=i}(alpha - combo_j)) for LDC.
        let lu_sum_prods =
            (poly * lu_degree..min((poly + 1) * lu_degree, num_lu_slots)).fold(zero, |acc, i| {
                let mut prod_i = one;

                for j in poly * lu_degree..min((poly + 1) * lu_degree, num_lu_slots) {
                    if j != i {
                        prod_i = builder.mul_extension(prod_i, current_lu_subs[j]);
                    }
                }
                builder.add_extension(acc, prod_i)
            });

        // Compute sum_i(mul_i.prod_{j!=i}(alpha - combo_j)) for Sum.
        let lut_sum_prods_mul = (poly * lut_degree..min((poly + 1) * lut_degree, num_lut_slots))
            .fold(zero, |acc, i| {
                let mut prod_i = one;

                for j in poly * lut_degree..min((poly + 1) * lut_degree, num_lut_slots) {
                    if j != i {
                        prod_i = builder.mul_extension(prod_i, current_lut_subs[j]);
                    }
                }
                builder.mul_add_extension(
                    prod_i,
                    vars.local_wires[LookupTableGate::wire_ith_multiplicity(i)],
                    acc,
                )
            });

        // The previous element is the previous poly of the current row or the last poly of the next row.
        let prev = if poly == 0 {
            z_gx_lookup_sldcs[num_sldc_polys - 1]
        } else {
            z_x_lookup_sldcs[poly - 1]
        };

        let cur_sub = builder.sub_extension(z_x_lookup_sldcs[poly], prev);

        // Check sum row and col transitions. It's the same constraint, with a row transition happening for slot == 0.
        let unfiltered_sum_transition =
            builder.mul_sub_extension(lut_prod, cur_sub, lut_sum_prods_mul);
        constraints.push(builder.mul_extension(
            lookup_selectors[LookupSelectors::TransSre as usize],
            unfiltered_sum_transition,
        ));

        // Check ldc row and col transitions. It's the same constraint, with a row transition happening for slot == 0.
        let unfiltered_ldc_transition = builder.mul_add_extension(lu_prod, cur_sub, lu_sum_prods);
        constraints.push(builder.mul_extension(
            lookup_selectors[LookupSelectors::TransLdc as usize],
            unfiltered_ldc_transition,
        ));
    }
    constraints
}
