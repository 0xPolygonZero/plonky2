//! Implementation of the STARK prover.

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
use core::iter::once;

use anyhow::{ensure, Result};
use itertools::Itertools;
use plonky2::field::extension::Extendable;
use plonky2::field::packable::Packable;
use plonky2::field::packed::PackedField;
use plonky2::field::polynomial::{PolynomialCoeffs, PolynomialValues};
use plonky2::field::types::Field;
use plonky2::field::zero_poly_coset::ZeroPolyOnCoset;
use plonky2::fri::oracle::PolynomialBatch;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::challenger::Challenger;
use plonky2::plonk::config::GenericConfig;
use plonky2::timed;
use plonky2::util::timing::TimingTree;
use plonky2::util::{log2_ceil, log2_strict, transpose};
use plonky2_maybe_rayon::*;

use crate::config::StarkConfig;
use crate::constraint_consumer::ConstraintConsumer;
use crate::cross_table_lookup::{get_ctl_auxiliary_polys, CtlCheckVars, CtlData};
use crate::evaluation_frame::StarkEvaluationFrame;
use crate::lookup::{
    get_grand_product_challenge_set, lookup_helper_columns, GrandProductChallengeSet, Lookup,
    LookupCheckVars,
};
use crate::proof::{StarkOpeningSet, StarkProof, StarkProofWithPublicInputs};
use crate::stark::Stark;
use crate::vanishing_poly::eval_vanishing_poly;

/// From a STARK trace, computes a STARK proof to attest its correctness.
pub fn prove<F, C, S, const D: usize>(
    stark: S,
    config: &StarkConfig,
    trace_poly_values: Vec<PolynomialValues<F>>,
    public_inputs: &[F],
    timing: &mut TimingTree,
) -> Result<StarkProofWithPublicInputs<F, C, D>>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
{
    let degree = trace_poly_values[0].len();
    let degree_bits = log2_strict(degree);
    let fri_params = config.fri_params(degree_bits);
    let rate_bits = config.fri_config.rate_bits;
    let cap_height = config.fri_config.cap_height;
    assert!(
        fri_params.total_arities() <= degree_bits + rate_bits - cap_height,
        "FRI total reduction arity is too large.",
    );

    let trace_commitment = timed!(
        timing,
        "compute trace commitment",
        PolynomialBatch::<F, C, D>::from_values(
            trace_poly_values.clone(),
            rate_bits,
            false,
            cap_height,
            timing,
            None,
        )
    );

    let trace_cap = trace_commitment.merkle_tree.cap.clone();
    let mut challenger = Challenger::new();
    challenger.observe_cap(&trace_cap);

    prove_with_commitment(
        &stark,
        config,
        &trace_poly_values,
        &trace_commitment,
        None,
        None,
        &mut challenger,
        public_inputs,
        timing,
    )
}

/// Generates a proof for a single STARK table, including:
///
/// - the initial state of the challenger,
/// - all the required Merkle caps,
/// - all the required polynomial and FRI argument openings.
/// - individual `ctl_data` and common `ctl_challenges` if the STARK is part
/// of a multi-STARK system.
pub fn prove_with_commitment<F, C, S, const D: usize>(
    stark: &S,
    config: &StarkConfig,
    trace_poly_values: &[PolynomialValues<F>],
    trace_commitment: &PolynomialBatch<F, C, D>,
    ctl_data: Option<&CtlData<F>>,
    ctl_challenges: Option<&GrandProductChallengeSet<F>>,
    challenger: &mut Challenger<F, C::Hasher>,
    public_inputs: &[F],
    timing: &mut TimingTree,
) -> Result<StarkProofWithPublicInputs<F, C, D>>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
{
    let degree = trace_poly_values[0].len();
    let degree_bits = log2_strict(degree);
    let fri_params = config.fri_params(degree_bits);
    let rate_bits = config.fri_config.rate_bits;
    let cap_height = config.fri_config.cap_height;
    assert!(
        fri_params.total_arities() <= degree_bits + rate_bits - cap_height,
        "FRI total reduction arity is too large.",
    );

    let constraint_degree = stark.constraint_degree();
    assert!(
        constraint_degree <= (1 << rate_bits) + 1,
        "The degree of the Stark constraints must be <= blowup_factor + 1"
    );

    // Permutation arguments.
    let lookup_challenges = stark.uses_lookups().then(|| {
        if let Some(c) = ctl_challenges {
            c.challenges.iter().map(|ch| ch.beta).collect::<Vec<_>>()
        } else {
            get_grand_product_challenge_set(challenger, config.num_challenges)
                .challenges
                .iter()
                .map(|ch| ch.beta)
                .collect::<Vec<_>>()
        }
    });

    let lookups = stark.lookups();
    let lookup_helper_columns = timed!(
        timing,
        "compute lookup helper columns",
        lookup_challenges.as_ref().map(|challenges| {
            let mut columns = Vec::new();
            for lookup in &lookups {
                for &challenge in challenges {
                    columns.extend(lookup_helper_columns(
                        lookup,
                        trace_poly_values,
                        challenge,
                        constraint_degree,
                    ));
                }
            }
            columns
        })
    );
    let num_lookup_columns = lookup_helper_columns.as_ref().map_or(0, |v| v.len());

    // We add CTLs, if there are any, to the permutation arguments so that
    // we can batch commit to all auxiliary polynomials.
    let auxiliary_polys = match lookup_helper_columns {
        None => get_ctl_auxiliary_polys(ctl_data),
        Some(mut lookup_columns) => {
            if let Some(p) = get_ctl_auxiliary_polys(ctl_data) {
                lookup_columns.extend(p)
            };

            Some(lookup_columns)
        }
    };

    debug_assert!(
        (stark.uses_lookups() || stark.requires_ctls()) || auxiliary_polys.is_none(),
        "There should be auxiliary polynomials if and only if we have either lookups or require cross-table lookups."
    );

    // Get the polynomial commitments for all auxiliary polynomials.
    let auxiliary_polys_commitment = auxiliary_polys.map(|aux_polys| {
        timed!(
            timing,
            "compute auxiliary polynomials commitment",
            PolynomialBatch::from_values(
                aux_polys,
                rate_bits,
                false,
                config.fri_config.cap_height,
                timing,
                None,
            )
        )
    });

    let auxiliary_polys_cap = auxiliary_polys_commitment
        .as_ref()
        .map(|commit| commit.merkle_tree.cap.clone());
    if let Some(cap) = &auxiliary_polys_cap {
        challenger.observe_cap(cap);
    }

    let alphas = challenger.get_n_challenges(config.num_challenges);

    let num_ctl_polys = ctl_data
        .map(|data| data.num_ctl_helper_polys())
        .unwrap_or_default();

    // This is an expensive check, hence is only run when `debug_assertions` are enabled.
    #[cfg(debug_assertions)]
    {
        check_constraints(
            stark,
            trace_commitment,
            public_inputs,
            &auxiliary_polys_commitment,
            lookup_challenges.as_ref(),
            &lookups,
            ctl_data,
            alphas.clone(),
            degree_bits,
            num_lookup_columns,
            &num_ctl_polys,
        );
    }

    let quotient_polys = timed!(
        timing,
        "compute quotient polys",
        compute_quotient_polys::<F, <F as Packable>::Packing, C, S, D>(
            stark,
            trace_commitment,
            &auxiliary_polys_commitment,
            lookup_challenges.as_ref(),
            &lookups,
            ctl_data,
            public_inputs,
            alphas.clone(),
            degree_bits,
            num_lookup_columns,
            &num_ctl_polys,
            config,
        )
    );
    let (quotient_commitment, quotient_polys_cap) = if let Some(quotient_polys) = quotient_polys {
        let all_quotient_chunks = timed!(
            timing,
            "split quotient polys",
            quotient_polys
                .into_par_iter()
                .flat_map(|mut quotient_poly| {
                    quotient_poly
                        .trim_to_len(degree * stark.quotient_degree_factor())
                        .expect(
                            "Quotient has failed, the vanishing polynomial is not divisible by Z_H",
                        );
                    // Split quotient into degree-n chunks.
                    quotient_poly.chunks(degree)
                })
                .collect()
        );
        // Commit to the quotient polynomials.
        let quotient_commitment = timed!(
            timing,
            "compute quotient commitment",
            PolynomialBatch::from_coeffs(
                all_quotient_chunks,
                rate_bits,
                false,
                config.fri_config.cap_height,
                timing,
                None,
            )
        );
        // Observe the quotient polynomials Merkle cap.
        let quotient_polys_cap = quotient_commitment.merkle_tree.cap.clone();
        challenger.observe_cap(&quotient_polys_cap);
        (Some(quotient_commitment), Some(quotient_polys_cap))
    } else {
        (None, None)
    };

    let zeta = challenger.get_extension_challenge::<D>();

    // To avoid leaking witness data, we want to ensure that our opening locations, `zeta` and
    // `g * zeta`, are not in our subgroup `H`. It suffices to check `zeta` only, since
    // `(g * zeta)^n = zeta^n`, where `n` is the order of `g`.
    let g = F::primitive_root_of_unity(degree_bits);
    ensure!(
        zeta.exp_power_of_2(degree_bits) != F::Extension::ONE,
        "Opening point is in the subgroup."
    );

    // Compute all openings: evaluate all committed polynomials at `zeta` and, when necessary, at `g * zeta`.
    let openings = StarkOpeningSet::new(
        zeta,
        g,
        trace_commitment,
        auxiliary_polys_commitment.as_ref(),
        quotient_commitment.as_ref(),
        stark.num_lookup_helper_columns(config),
        stark.requires_ctls(),
        &num_ctl_polys,
    );
    // Get the FRI openings and observe them.
    challenger.observe_openings(&openings.to_fri_openings());

    let initial_merkle_trees = once(trace_commitment)
        .chain(&auxiliary_polys_commitment)
        .chain(&quotient_commitment)
        .collect_vec();

    let opening_proof = timed!(
        timing,
        "compute openings proof",
        PolynomialBatch::prove_openings(
            &stark.fri_instance(zeta, g, num_ctl_polys.iter().sum(), num_ctl_polys, config),
            &initial_merkle_trees,
            challenger,
            &fri_params,
            timing,
        )
    );

    let proof = StarkProof {
        trace_cap: trace_commitment.merkle_tree.cap.clone(),
        auxiliary_polys_cap,
        quotient_polys_cap,
        openings,
        opening_proof,
    };

    Ok(StarkProofWithPublicInputs {
        proof,
        public_inputs: public_inputs.to_vec(),
    })
}

/// Computes the quotient polynomials `(sum alpha^i C_i(x)) / Z_H(x)` for `alpha` in `alphas`,
/// where the `C_i`s are the STARK constraints.
fn compute_quotient_polys<'a, F, P, C, S, const D: usize>(
    stark: &S,
    trace_commitment: &'a PolynomialBatch<F, C, D>,
    auxiliary_polys_commitment: &'a Option<PolynomialBatch<F, C, D>>,
    lookup_challenges: Option<&'a Vec<F>>,
    lookups: &[Lookup<F>],
    ctl_data: Option<&CtlData<F>>,
    public_inputs: &[F],
    alphas: Vec<F>,
    degree_bits: usize,
    num_lookup_columns: usize,
    num_ctl_columns: &[usize],
    config: &StarkConfig,
) -> Option<Vec<PolynomialCoeffs<F>>>
where
    F: RichField + Extendable<D>,
    P: PackedField<Scalar = F>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
{
    if stark.quotient_degree_factor() == 0 {
        return None;
    }

    let degree = 1 << degree_bits;
    let rate_bits = config.fri_config.rate_bits;
    let total_num_helper_cols: usize = num_ctl_columns.iter().sum();

    let quotient_degree_bits = log2_ceil(stark.quotient_degree_factor());
    assert!(
        quotient_degree_bits <= rate_bits,
        "Having constraints of degree higher than the rate is not supported yet."
    );
    let step = 1 << (rate_bits - quotient_degree_bits);
    // When opening the `Z`s polys at the "next" point, need to look at the point `next_step` steps away.
    let next_step = 1 << quotient_degree_bits;

    // Evaluation of the first Lagrange polynomial on the LDE domain.
    let lagrange_first = PolynomialValues::selector(degree, 0).lde_onto_coset(quotient_degree_bits);
    // Evaluation of the last Lagrange polynomial on the LDE domain.
    let lagrange_last =
        PolynomialValues::selector(degree, degree - 1).lde_onto_coset(quotient_degree_bits);

    let z_h_on_coset = ZeroPolyOnCoset::<F>::new(degree_bits, quotient_degree_bits);

    // Retrieve the LDE values at index `i`.
    let get_trace_values_packed =
        |i_start| -> Vec<P> { trace_commitment.get_lde_values_packed(i_start, step) };

    // Last element of the subgroup.
    let last = F::primitive_root_of_unity(degree_bits).inverse();
    let size = degree << quotient_degree_bits;
    let coset = F::cyclic_subgroup_coset_known_order(
        F::primitive_root_of_unity(degree_bits + quotient_degree_bits),
        F::coset_shift(),
        size,
    );

    // We will step by `P::WIDTH`, and in each iteration, evaluate the quotient polynomial at
    // a batch of `P::WIDTH` points.
    let quotient_values = (0..size)
        .into_par_iter()
        .step_by(P::WIDTH)
        .flat_map_iter(|i_start| {
            let i_next_start = (i_start + next_step) % size;
            let i_range = i_start..i_start + P::WIDTH;

            let x = *P::from_slice(&coset[i_range.clone()]);
            let z_last = x - last;
            let lagrange_basis_first = *P::from_slice(&lagrange_first.values[i_range.clone()]);
            let lagrange_basis_last = *P::from_slice(&lagrange_last.values[i_range]);

            let mut consumer = ConstraintConsumer::new(
                alphas.clone(),
                z_last,
                lagrange_basis_first,
                lagrange_basis_last,
            );
            // Get the local and next row evaluations for the current STARK,
            // as well as the public inputs.
            let vars = S::EvaluationFrame::from_values(
                &get_trace_values_packed(i_start),
                &get_trace_values_packed(i_next_start),
                public_inputs,
            );
            // Get the local and next row evaluations for the permutation argument,
            // as well as the associated challenges.
            let lookup_vars = lookup_challenges.map(|challenges| LookupCheckVars {
                local_values: auxiliary_polys_commitment
                    .as_ref()
                    .unwrap()
                    .get_lde_values_packed(i_start, step)[..num_lookup_columns]
                    .to_vec(),
                next_values: auxiliary_polys_commitment
                    .as_ref()
                    .unwrap()
                    .get_lde_values_packed(i_next_start, step)[..num_lookup_columns]
                    .to_vec(),
                challenges: challenges.to_vec(),
            });

            // Get all the data for this STARK's CTLs, if any:
            // - the local and next row evaluations for the CTL Z polynomials
            // - the associated challenges.
            // - for each CTL:
            //     - the filter `Column`
            //     - the `Column`s that form the looking/looked table.

            let ctl_vars = ctl_data.map(|data| {
                let mut start_index = 0;
                data.zs_columns
                    .iter()
                    .enumerate()
                    .map(|(i, zs_columns)| {
                        let num_ctl_helper_cols = num_ctl_columns[i];
                        let helper_columns = auxiliary_polys_commitment
                            .as_ref()
                            .unwrap()
                            .get_lde_values_packed(i_start, step)
                            [num_lookup_columns + start_index
                                ..num_lookup_columns + start_index + num_ctl_helper_cols]
                            .to_vec();

                        let ctl_vars = CtlCheckVars::<F, F, P, 1> {
                            helper_columns,
                            local_z: auxiliary_polys_commitment
                                .as_ref()
                                .unwrap()
                                .get_lde_values_packed(i_start, step)
                                [num_lookup_columns + total_num_helper_cols + i],
                            next_z: auxiliary_polys_commitment
                                .as_ref()
                                .unwrap()
                                .get_lde_values_packed(i_next_start, step)
                                [num_lookup_columns + total_num_helper_cols + i],
                            challenges: zs_columns.challenge,
                            columns: zs_columns.columns.clone(),
                            filter: zs_columns.filter.clone(),
                        };

                        start_index += num_ctl_helper_cols;

                        ctl_vars
                    })
                    .collect::<Vec<_>>()
            });

            // Evaluate the polynomial combining all constraints, including
            // those associated to the permutation arguments.
            eval_vanishing_poly::<F, F, P, S, D, 1>(
                stark,
                &vars,
                lookups,
                lookup_vars,
                ctl_vars.as_deref(),
                &mut consumer,
            );

            let mut constraints_evals = consumer.accumulators();
            // We divide the constraints evaluations by `Z_H(x)`.
            let denominator_inv: P = z_h_on_coset.eval_inverse_packed(i_start);

            for eval in &mut constraints_evals {
                *eval *= denominator_inv;
            }

            let num_challenges = alphas.len();

            (0..P::WIDTH).map(move |i| {
                (0..num_challenges)
                    .map(|j| constraints_evals[j].as_slice()[i])
                    .collect()
            })
        })
        .collect::<Vec<_>>();

    Some(
        transpose(&quotient_values)
            .into_par_iter()
            .map(PolynomialValues::new)
            .map(|values| values.coset_ifft(F::coset_shift()))
            .collect(),
    )
}

/// Check that all constraints evaluate to zero on `H`.
/// Can also be used to check the degree of the constraints by evaluating on a larger subgroup.
///
/// Debugging module, to assert that all constraints evaluate to zero on `H`.
/// It can also be used to check the degree of the constraints by evaluating on a larger subgroup.
///
/// **Note**: this is an expensive check, hence is only available when the `debug_assertions`
/// flag is activated, to not hinder performances with regular `release` build.
#[cfg(debug_assertions)]
fn check_constraints<'a, F, C, S, const D: usize>(
    stark: &S,
    trace_commitment: &'a PolynomialBatch<F, C, D>,
    public_inputs: &[F],
    auxiliary_commitment: &'a Option<PolynomialBatch<F, C, D>>,
    lookup_challenges: Option<&'a Vec<F>>,
    lookups: &[Lookup<F>],
    ctl_data: Option<&CtlData<F>>,
    alphas: Vec<F>,
    degree_bits: usize,
    num_lookup_columns: usize,
    num_ctl_helper_cols: &[usize],
) where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
{
    let degree = 1 << degree_bits;
    let rate_bits = 0; // Set this to higher value to check constraint degree.
    let total_num_helper_cols: usize = num_ctl_helper_cols.iter().sum();

    let size = degree << rate_bits;
    let step = 1 << rate_bits;

    // Evaluation of the first Lagrange polynomial.
    let lagrange_first = PolynomialValues::selector(degree, 0).lde(rate_bits);
    // Evaluation of the last Lagrange polynomial.
    let lagrange_last = PolynomialValues::selector(degree, degree - 1).lde(rate_bits);

    let subgroup = F::two_adic_subgroup(degree_bits + rate_bits);

    // Get the evaluations of a batch of polynomials over our subgroup.
    let get_subgroup_evals = |comm: &PolynomialBatch<F, C, D>| -> Vec<Vec<F>> {
        let values = comm
            .polynomials
            .par_iter()
            .map(|coeffs| coeffs.clone().fft().values)
            .collect::<Vec<_>>();
        transpose(&values)
    };

    // Get batch evaluations of the trace and permutation polynomials over our subgroup.
    let trace_subgroup_evals = get_subgroup_evals(trace_commitment);
    let auxiliary_subgroup_evals = auxiliary_commitment.as_ref().map(get_subgroup_evals);

    // Last element of the subgroup.
    let last = F::primitive_root_of_unity(degree_bits).inverse();

    let constraint_values = (0..size)
        .map(|i| {
            let i_next = (i + step) % size;

            let x = subgroup[i];
            let z_last = x - last;
            let lagrange_basis_first = lagrange_first.values[i];
            let lagrange_basis_last = lagrange_last.values[i];

            let mut consumer = ConstraintConsumer::new(
                alphas.clone(),
                z_last,
                lagrange_basis_first,
                lagrange_basis_last,
            );
            // Get the local and next row evaluations for the current STARK's trace.
            let vars = S::EvaluationFrame::from_values(
                &trace_subgroup_evals[i],
                &trace_subgroup_evals[i_next],
                public_inputs,
            );
            // Get the local and next row evaluations for the current STARK's permutation argument.
            let lookup_vars = lookup_challenges.map(|challenges| LookupCheckVars {
                local_values: auxiliary_subgroup_evals.as_ref().unwrap()[i][..num_lookup_columns]
                    .to_vec(),
                next_values: auxiliary_subgroup_evals.as_ref().unwrap()[i_next]
                    [..num_lookup_columns]
                    .to_vec(),
                challenges: challenges.to_vec(),
            });

            // Get the local and next row evaluations for the current STARK's CTL Z polynomials.
            let mut start_index = 0;
            let ctl_vars = ctl_data.map(|data| {
                data.zs_columns
                    .iter()
                    .enumerate()
                    .map(|(iii, zs_columns)| {
                        let num_helper_cols = num_ctl_helper_cols[iii];
                        let helper_columns = auxiliary_subgroup_evals.as_ref().unwrap()[i]
                            [num_lookup_columns + start_index
                                ..num_lookup_columns + start_index + num_helper_cols]
                            .to_vec();
                        let ctl_vars = CtlCheckVars::<F, F, F, 1> {
                            helper_columns,
                            local_z: auxiliary_subgroup_evals.as_ref().unwrap()[i]
                                [num_lookup_columns + total_num_helper_cols + iii],
                            next_z: auxiliary_subgroup_evals.as_ref().unwrap()[i_next]
                                [num_lookup_columns + total_num_helper_cols + iii],
                            challenges: zs_columns.challenge,
                            columns: zs_columns.columns.clone(),
                            filter: zs_columns.filter.clone(),
                        };

                        start_index += num_helper_cols;

                        ctl_vars
                    })
                    .collect::<Vec<_>>()
            });

            // Evaluate the polynomial combining all constraints, including those associated
            // to the permutation arguments.
            eval_vanishing_poly::<F, F, F, S, D, 1>(
                stark,
                &vars,
                lookups,
                lookup_vars,
                ctl_vars.as_deref(),
                &mut consumer,
            );
            consumer.accumulators()
        })
        .collect::<Vec<_>>();

    // Assert that all constraints evaluate to 0 over our subgroup.
    for v in constraint_values {
        assert!(
            v.iter().all(|x| x.is_zero()),
            "Constraint failed in {}",
            core::any::type_name::<S>()
        );
    }
}
