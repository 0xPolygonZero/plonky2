use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::{anyhow, ensure, Result};
use itertools::Itertools;
use once_cell::sync::Lazy;
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
use plonky2::util::transpose;
use plonky2_maybe_rayon::*;
use plonky2_util::{log2_ceil, log2_strict};

use crate::all_stark::{AllStark, Table, NUM_TABLES};
use crate::config::StarkConfig;
use crate::constraint_consumer::ConstraintConsumer;
use crate::cpu::kernel::aggregator::KERNEL;
use crate::cross_table_lookup::{
    cross_table_lookup_data, get_grand_product_challenge_set, CtlCheckVars, CtlData,
    GrandProductChallengeSet,
};
use crate::evaluation_frame::StarkEvaluationFrame;
use crate::generation::{generate_traces, GenerationInputs};
use crate::get_challenges::observe_public_values;
use crate::lookup::{lookup_helper_columns, Lookup, LookupCheckVars};
use crate::proof::{AllProof, PublicValues, StarkOpeningSet, StarkProof, StarkProofWithMetadata};
use crate::stark::Stark;
use crate::vanishing_poly::eval_vanishing_poly;
use crate::witness::errors::ProgramError;
#[cfg(test)]
use crate::{
    cross_table_lookup::testutils::check_ctls, verifier::testutils::get_memory_extra_looking_values,
};

/// Generate traces, then create all STARK proofs.
pub fn prove<F, C, const D: usize>(
    all_stark: &AllStark<F, D>,
    config: &StarkConfig,
    inputs: GenerationInputs,
    timing: &mut TimingTree,
    abort_signal: Option<Arc<AtomicBool>>,
) -> Result<AllProof<F, C, D>>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
{
    timed!(timing, "build kernel", Lazy::force(&KERNEL));
    let (traces, public_values) = timed!(
        timing,
        "generate all traces",
        generate_traces(all_stark, inputs, config, timing)?
    );
    check_abort_signal(abort_signal.clone())?;

    let proof = prove_with_traces(
        all_stark,
        config,
        traces,
        public_values,
        timing,
        abort_signal,
    )?;
    Ok(proof)
}

/// Compute all STARK proofs.
pub(crate) fn prove_with_traces<F, C, const D: usize>(
    all_stark: &AllStark<F, D>,
    config: &StarkConfig,
    trace_poly_values: [Vec<PolynomialValues<F>>; NUM_TABLES],
    public_values: PublicValues,
    timing: &mut TimingTree,
    abort_signal: Option<Arc<AtomicBool>>,
) -> Result<AllProof<F, C, D>>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
{
    let rate_bits = config.fri_config.rate_bits;
    let cap_height = config.fri_config.cap_height;

    // For each STARK, we compute the polynomial commitments for the polynomials interpolating its trace.
    let trace_commitments = timed!(
        timing,
        "compute all trace commitments",
        trace_poly_values
            .iter()
            .zip_eq(Table::all())
            .map(|(trace, table)| {
                timed!(
                    timing,
                    &format!("compute trace commitment for {:?}", table),
                    PolynomialBatch::<F, C, D>::from_values(
                        trace.clone(),
                        rate_bits,
                        false,
                        cap_height,
                        timing,
                        None,
                    )
                )
            })
            .collect::<Vec<_>>()
    );

    // Get the Merkle caps for all trace commitments and observe them.
    let trace_caps = trace_commitments
        .iter()
        .map(|c| c.merkle_tree.cap.clone())
        .collect::<Vec<_>>();
    let mut challenger = Challenger::<F, C::InnerHasher>::new();
    for cap in &trace_caps {
        challenger.observe_cap(cap);
    }

    observe_public_values::<F, C, D>(&mut challenger, &public_values)
        .map_err(|_| anyhow::Error::msg("Invalid conversion of public values."))?;

    // Get challenges for the cross-table lookups.
    let ctl_challenges = get_grand_product_challenge_set(&mut challenger, config.num_challenges);
    // For each STARK, compute its cross-table lookup Z polynomials and get the associated `CtlData`.
    let ctl_data_per_table = timed!(
        timing,
        "compute CTL data",
        cross_table_lookup_data::<F, D>(
            &trace_poly_values,
            &all_stark.cross_table_lookups,
            &ctl_challenges,
            all_stark.arithmetic_stark.constraint_degree()
        )
    );

    let stark_proofs = timed!(
        timing,
        "compute all proofs given commitments",
        prove_with_commitments(
            all_stark,
            config,
            &trace_poly_values,
            trace_commitments,
            ctl_data_per_table,
            &mut challenger,
            &ctl_challenges,
            timing,
            abort_signal,
        )?
    );

    #[cfg(test)]
    {
        check_ctls(
            &trace_poly_values,
            &all_stark.cross_table_lookups,
            &get_memory_extra_looking_values(&public_values),
        );
    }

    Ok(AllProof {
        stark_proofs,
        ctl_challenges,
        public_values,
    })
}

/// Generates a proof for each STARK.
/// At this stage, we have computed the trace polynomials commitments for the various STARKs,
/// and we have the cross-table lookup data for each table, including the associated challenges.
/// - `trace_poly_values` are the trace values for each STARK.
/// - `trace_commitments` are the trace polynomials commitments for each STARK.
/// - `ctl_data_per_table` group all the cross-table lookup data for each STARK.
/// Each STARK uses its associated data to generate a proof.
fn prove_with_commitments<F, C, const D: usize>(
    all_stark: &AllStark<F, D>,
    config: &StarkConfig,
    trace_poly_values: &[Vec<PolynomialValues<F>>; NUM_TABLES],
    trace_commitments: Vec<PolynomialBatch<F, C, D>>,
    ctl_data_per_table: [CtlData<F>; NUM_TABLES],
    challenger: &mut Challenger<F, C::InnerHasher>,
    ctl_challenges: &GrandProductChallengeSet<F>,
    timing: &mut TimingTree,
    abort_signal: Option<Arc<AtomicBool>>,
) -> Result<[StarkProofWithMetadata<F, C, D>; NUM_TABLES]>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
{
    let arithmetic_proof = timed!(
        timing,
        "prove Arithmetic STARK",
        prove_single_table(
            &all_stark.arithmetic_stark,
            config,
            &trace_poly_values[Table::Arithmetic as usize],
            &trace_commitments[Table::Arithmetic as usize],
            &ctl_data_per_table[Table::Arithmetic as usize],
            ctl_challenges,
            challenger,
            timing,
            abort_signal.clone(),
        )?
    );
    let byte_packing_proof = timed!(
        timing,
        "prove byte packing STARK",
        prove_single_table(
            &all_stark.byte_packing_stark,
            config,
            &trace_poly_values[Table::BytePacking as usize],
            &trace_commitments[Table::BytePacking as usize],
            &ctl_data_per_table[Table::BytePacking as usize],
            ctl_challenges,
            challenger,
            timing,
            abort_signal.clone(),
        )?
    );
    let cpu_proof = timed!(
        timing,
        "prove CPU STARK",
        prove_single_table(
            &all_stark.cpu_stark,
            config,
            &trace_poly_values[Table::Cpu as usize],
            &trace_commitments[Table::Cpu as usize],
            &ctl_data_per_table[Table::Cpu as usize],
            ctl_challenges,
            challenger,
            timing,
            abort_signal.clone(),
        )?
    );
    let keccak_proof = timed!(
        timing,
        "prove Keccak STARK",
        prove_single_table(
            &all_stark.keccak_stark,
            config,
            &trace_poly_values[Table::Keccak as usize],
            &trace_commitments[Table::Keccak as usize],
            &ctl_data_per_table[Table::Keccak as usize],
            ctl_challenges,
            challenger,
            timing,
            abort_signal.clone(),
        )?
    );
    let keccak_sponge_proof = timed!(
        timing,
        "prove Keccak sponge STARK",
        prove_single_table(
            &all_stark.keccak_sponge_stark,
            config,
            &trace_poly_values[Table::KeccakSponge as usize],
            &trace_commitments[Table::KeccakSponge as usize],
            &ctl_data_per_table[Table::KeccakSponge as usize],
            ctl_challenges,
            challenger,
            timing,
            abort_signal.clone(),
        )?
    );
    let logic_proof = timed!(
        timing,
        "prove logic STARK",
        prove_single_table(
            &all_stark.logic_stark,
            config,
            &trace_poly_values[Table::Logic as usize],
            &trace_commitments[Table::Logic as usize],
            &ctl_data_per_table[Table::Logic as usize],
            ctl_challenges,
            challenger,
            timing,
            abort_signal.clone(),
        )?
    );
    let memory_proof = timed!(
        timing,
        "prove memory STARK",
        prove_single_table(
            &all_stark.memory_stark,
            config,
            &trace_poly_values[Table::Memory as usize],
            &trace_commitments[Table::Memory as usize],
            &ctl_data_per_table[Table::Memory as usize],
            ctl_challenges,
            challenger,
            timing,
            abort_signal,
        )?
    );

    Ok([
        arithmetic_proof,
        byte_packing_proof,
        cpu_proof,
        keccak_proof,
        keccak_sponge_proof,
        logic_proof,
        memory_proof,
    ])
}

/// Computes a proof for a single STARK table, including:
/// - the initial state of the challenger,
/// - all the requires Merkle caps,
/// - all the required polynomial and FRI argument openings.
pub(crate) fn prove_single_table<F, C, S, const D: usize>(
    stark: &S,
    config: &StarkConfig,
    trace_poly_values: &[PolynomialValues<F>],
    trace_commitment: &PolynomialBatch<F, C, D>,
    ctl_data: &CtlData<F>,
    ctl_challenges: &GrandProductChallengeSet<F>,
    challenger: &mut Challenger<F, C::InnerHasher>,
    timing: &mut TimingTree,
    abort_signal: Option<Arc<AtomicBool>>,
) -> Result<StarkProofWithMetadata<F, C, D>>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
{
    check_abort_signal(abort_signal.clone())?;

    let degree = trace_poly_values[0].len();
    let degree_bits = log2_strict(degree);
    let fri_params = config.fri_params(degree_bits);
    let rate_bits = config.fri_config.rate_bits;
    let cap_height = config.fri_config.cap_height;
    assert!(
        fri_params.total_arities() <= degree_bits + rate_bits - cap_height,
        "FRI total reduction arity is too large.",
    );

    let init_challenger_state = challenger.compact();

    let constraint_degree = stark.constraint_degree();
    let lookup_challenges = stark.uses_lookups().then(|| {
        ctl_challenges
            .challenges
            .iter()
            .map(|ch| ch.beta)
            .collect::<Vec<_>>()
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
    let num_lookup_columns = lookup_helper_columns.as_ref().map(|v| v.len()).unwrap_or(0);

    // We add CTLs to the permutation arguments so that we can batch commit to
    // all auxiliary polynomials.
    let auxiliary_polys = match lookup_helper_columns {
        None => {
            let mut ctl_polys = ctl_data.ctl_helper_polys();
            ctl_polys.extend(ctl_data.ctl_z_polys());
            ctl_polys
        }
        Some(mut lookup_columns) => {
            lookup_columns.extend(ctl_data.ctl_helper_polys());
            lookup_columns.extend(ctl_data.ctl_z_polys());
            lookup_columns
        }
    };
    assert!(!auxiliary_polys.is_empty(), "No CTL?");

    // Get the polynomial commitments for all auxiliary polynomials.
    let auxiliary_polys_commitment = timed!(
        timing,
        "compute auxiliary polynomials commitment",
        PolynomialBatch::from_values(
            auxiliary_polys,
            rate_bits,
            false,
            config.fri_config.cap_height,
            timing,
            None,
        )
    );

    let auxiliary_polys_cap = auxiliary_polys_commitment.merkle_tree.cap.clone();
    challenger.observe_cap(&auxiliary_polys_cap);

    let alphas = challenger.get_n_challenges(config.num_challenges);

    let num_ctl_polys = ctl_data.num_ctl_helper_polys();

    #[cfg(test)]
    {
        check_constraints(
            stark,
            trace_commitment,
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

    check_abort_signal(abort_signal.clone())?;

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
            alphas,
            degree_bits,
            num_lookup_columns,
            &num_ctl_polys,
            config,
        )
    );
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
        &auxiliary_polys_commitment,
        &quotient_commitment,
        stark.num_lookup_helper_columns(config),
        &num_ctl_polys,
    );
    // Get the FRI openings and observe them.
    challenger.observe_openings(&openings.to_fri_openings());

    let initial_merkle_trees = vec![
        trace_commitment,
        &auxiliary_polys_commitment,
        &quotient_commitment,
    ];

    check_abort_signal(abort_signal.clone())?;

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
    Ok(StarkProofWithMetadata {
        init_challenger_state,
        proof,
    })
}

/// Computes the quotient polynomials `(sum alpha^i C_i(x)) / Z_H(x)` for `alpha` in `alphas`,
/// where the `C_i`s are the Stark constraints.
fn compute_quotient_polys<'a, F, P, C, S, const D: usize>(
    stark: &S,
    trace_commitment: &'a PolynomialBatch<F, C, D>,
    auxiliary_polys_commitment: &'a PolynomialBatch<F, C, D>,
    lookup_challenges: Option<&'a Vec<F>>,
    lookups: &[Lookup<F>],
    ctl_data: &CtlData<F>,
    alphas: Vec<F>,
    degree_bits: usize,
    num_lookup_columns: usize,
    num_ctl_columns: &[usize],
    config: &StarkConfig,
) -> Vec<PolynomialCoeffs<F>>
where
    F: RichField + Extendable<D>,
    P: PackedField<Scalar = F>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
{
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
            // Get the local and next row evaluations for the current STARK.
            let vars = S::EvaluationFrame::from_values(
                &get_trace_values_packed(i_start),
                &get_trace_values_packed(i_next_start),
            );
            // Get the local and next row evaluations for the permutation argument, as well as the associated challenges.
            let lookup_vars = lookup_challenges.map(|challenges| LookupCheckVars {
                local_values: auxiliary_polys_commitment.get_lde_values_packed(i_start, step)
                    [..num_lookup_columns]
                    .to_vec(),
                next_values: auxiliary_polys_commitment.get_lde_values_packed(i_next_start, step),
                challenges: challenges.to_vec(),
            });

            // Get all the data for this STARK's CTLs:
            // - the local and next row evaluations for the CTL Z polynomials
            // - the associated challenges.
            // - for each CTL:
            //     - the filter `Column`
            //     - the `Column`s that form the looking/looked table.

            let mut start_index = 0;
            let ctl_vars = ctl_data
                .zs_columns
                .iter()
                .enumerate()
                .map(|(i, zs_columns)| {
                    let num_ctl_helper_cols = num_ctl_columns[i];
                    let helper_columns = auxiliary_polys_commitment
                        .get_lde_values_packed(i_start, step)[num_lookup_columns
                        + start_index
                        ..num_lookup_columns + start_index + num_ctl_helper_cols]
                        .to_vec();

                    let ctl_vars = CtlCheckVars::<F, F, P, 1> {
                        helper_columns,
                        local_z: auxiliary_polys_commitment.get_lde_values_packed(i_start, step)
                            [num_lookup_columns + total_num_helper_cols + i],
                        next_z: auxiliary_polys_commitment
                            .get_lde_values_packed(i_next_start, step)
                            [num_lookup_columns + total_num_helper_cols + i],
                        challenges: zs_columns.challenge,
                        columns: zs_columns.columns.clone(),
                        filter: zs_columns.filter.clone(),
                    };

                    start_index += num_ctl_helper_cols;

                    ctl_vars
                })
                .collect::<Vec<_>>();

            // Evaluate the polynomial combining all constraints, including those associated
            // to the permutation and CTL arguments.
            eval_vanishing_poly::<F, F, P, S, D, 1>(
                stark,
                &vars,
                lookups,
                lookup_vars,
                &ctl_vars,
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

    transpose(&quotient_values)
        .into_par_iter()
        .map(PolynomialValues::new)
        .map(|values| values.coset_ifft(F::coset_shift()))
        .collect()
}

/// Utility method that checks whether a kill signal has been emitted by one of the workers,
/// which will result in an early abort for all the other processes involved in the same set
/// of transactions.
pub fn check_abort_signal(abort_signal: Option<Arc<AtomicBool>>) -> Result<()> {
    if let Some(signal) = abort_signal {
        if signal.load(Ordering::Relaxed) {
            return Err(anyhow!("Stopping job from abort signal."));
        }
    }

    Ok(())
}

#[cfg(test)]
/// Check that all constraints evaluate to zero on `H`.
/// Can also be used to check the degree of the constraints by evaluating on a larger subgroup.
fn check_constraints<'a, F, C, S, const D: usize>(
    stark: &S,
    trace_commitment: &'a PolynomialBatch<F, C, D>,
    auxiliary_commitment: &'a PolynomialBatch<F, C, D>,
    lookup_challenges: Option<&'a Vec<F>>,
    lookups: &[Lookup<F>],
    ctl_data: &CtlData<F>,
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

    // Get batch evaluations of the trace, permutation and CTL polynomials over our subgroup.
    let trace_subgroup_evals = get_subgroup_evals(trace_commitment);
    let auxiliary_subgroup_evals = get_subgroup_evals(auxiliary_commitment);

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
            );
            // Get the local and next row evaluations for the current STARK's permutation argument.
            let lookup_vars = lookup_challenges.map(|challenges| LookupCheckVars {
                local_values: auxiliary_subgroup_evals[i][..num_lookup_columns].to_vec(),
                next_values: auxiliary_subgroup_evals[i_next][..num_lookup_columns].to_vec(),
                challenges: challenges.to_vec(),
            });

            // Get the local and next row evaluations for the current STARK's CTL Z polynomials.
            let mut start_index = 0;
            let ctl_vars = ctl_data
                .zs_columns
                .iter()
                .enumerate()
                .map(|(iii, zs_columns)| {
                    let num_helper_cols = num_ctl_helper_cols[iii];
                    let helper_columns = auxiliary_subgroup_evals[i][num_lookup_columns
                        + start_index
                        ..num_lookup_columns + start_index + num_helper_cols]
                        .to_vec();
                    let ctl_vars = CtlCheckVars::<F, F, F, 1> {
                        helper_columns,
                        local_z: auxiliary_subgroup_evals[i]
                            [num_lookup_columns + total_num_helper_cols + iii],
                        next_z: auxiliary_subgroup_evals[i_next]
                            [num_lookup_columns + total_num_helper_cols + iii],
                        challenges: zs_columns.challenge,
                        columns: zs_columns.columns.clone(),
                        filter: zs_columns.filter.clone(),
                    };

                    start_index += num_helper_cols;

                    ctl_vars
                })
                .collect::<Vec<_>>();

            // Evaluate the polynomial combining all constraints, including those associated
            // to the permutation and CTL arguments.
            eval_vanishing_poly::<F, F, F, S, D, 1>(
                stark,
                &vars,
                lookups,
                lookup_vars,
                &ctl_vars,
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
            std::any::type_name::<S>()
        );
    }
}
