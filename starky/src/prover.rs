use std::iter::once;

use anyhow::{ensure, Result};
use itertools::Itertools;
use plonky2::field::extension_field::Extendable;
use plonky2::field::field_types::Field;
use plonky2::field::polynomial::{PolynomialCoeffs, PolynomialValues};
use plonky2::field::zero_poly_coset::ZeroPolyOnCoset;
use plonky2::fri::oracle::PolynomialBatch;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::challenger::Challenger;
use plonky2::plonk::config::{GenericConfig, Hasher};
use plonky2::timed;
use plonky2::util::timing::TimingTree;
use plonky2::util::transpose;
use plonky2_util::{log2_ceil, log2_strict};
use rayon::prelude::*;

use crate::config::StarkConfig;
use crate::constraint_consumer::ConstraintConsumer;
use crate::permutation::PermutationCheckData;
use crate::permutation::{
    compute_permutation_z_polys, get_n_permutation_challenge_sets, PermutationChallengeSet,
};
use crate::proof::{StarkOpeningSet, StarkProof, StarkProofWithPublicInputs};
use crate::stark::Stark;
use crate::vanishing_poly::eval_vanishing_poly;
use crate::vars::StarkEvaluationVars;

pub fn prove<F, C, S, const D: usize>(
    stark: S,
    config: &StarkConfig,
    trace: Vec<[F; S::COLUMNS]>,
    public_inputs: [F; S::PUBLIC_INPUTS],
    timing: &mut TimingTree,
) -> Result<StarkProofWithPublicInputs<F, C, D>>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
    [(); S::COLUMNS]:,
    [(); S::PUBLIC_INPUTS]:,
    [(); C::Hasher::HASH_SIZE]:,
{
    let degree = trace.len();
    let degree_bits = log2_strict(degree);
    let fri_params = config.fri_params(degree_bits);
    let rate_bits = config.fri_config.rate_bits;
    let cap_height = config.fri_config.cap_height;
    assert!(
        fri_params.total_arities() <= degree_bits + rate_bits - cap_height,
        "FRI total reduction arity is too large.",
    );

    let trace_vecs = trace.iter().map(|row| row.to_vec()).collect_vec();
    let trace_col_major: Vec<Vec<F>> = transpose(&trace_vecs);

    let trace_poly_values: Vec<PolynomialValues<F>> = timed!(
        timing,
        "compute trace polynomials",
        trace_col_major
            .par_iter()
            .map(|column| PolynomialValues::new(column.clone()))
            .collect()
    );

    let trace_commitment = timed!(
        timing,
        "compute trace commitment",
        PolynomialBatch::<F, C, D>::from_values(
            // TODO: Cloning this isn't great; consider having `from_values` accept a reference,
            // or having `compute_permutation_z_polys` read trace values from the `PolynomialBatch`.
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

    // Permutation arguments.
    let permutation_zs_commitment_challenges = if stark.uses_permutation_args() {
        let permutation_challenge_sets = get_n_permutation_challenge_sets(
            &mut challenger,
            config.num_challenges,
            stark.permutation_batch_size(),
        );
        let permutation_z_polys = compute_permutation_z_polys::<F, C, S, D>(
            &stark,
            config,
            &mut challenger,
            &trace_poly_values,
            &permutation_challenge_sets,
        );

        timed!(
            timing,
            "compute permutation Z commitments",
            Some((
                PolynomialBatch::from_values(
                    permutation_z_polys,
                    rate_bits,
                    false,
                    config.fri_config.cap_height,
                    timing,
                    None,
                ),
                permutation_challenge_sets
            ))
        )
    } else {
        None
    };
    let permutation_zs_commitment = permutation_zs_commitment_challenges
        .as_ref()
        .map(|(comm, _)| comm);
    let permutation_zs_cap = permutation_zs_commitment
        .as_ref()
        .map(|commit| commit.merkle_tree.cap.clone());
    for cap in &permutation_zs_cap {
        challenger.observe_cap(cap);
    }

    let alphas = challenger.get_n_challenges(config.num_challenges);
    let quotient_polys = compute_quotient_polys::<F, C, S, D>(
        &stark,
        &trace_commitment,
        &permutation_zs_commitment_challenges,
        public_inputs,
        alphas,
        degree_bits,
        config,
    );
    let all_quotient_chunks = quotient_polys
        .into_par_iter()
        .flat_map(|mut quotient_poly| {
            quotient_poly
                .trim_to_len(degree * stark.quotient_degree_factor())
                .expect("Quotient has failed, the vanishing polynomial is not divisible by Z_H");
            // Split quotient into degree-n chunks.
            quotient_poly.chunks(degree)
        })
        .collect();
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
    let openings = StarkOpeningSet::new(
        zeta,
        g,
        &trace_commitment,
        permutation_zs_commitment,
        &quotient_commitment,
    );
    challenger.observe_openings(&openings.to_fri_openings());

    let initial_merkle_trees = once(&trace_commitment)
        .chain(permutation_zs_commitment)
        .chain(once(&quotient_commitment))
        .collect_vec();

    let opening_proof = timed!(
        timing,
        "compute openings proof",
        PolynomialBatch::prove_openings(
            &stark.fri_instance(zeta, g, config),
            &initial_merkle_trees,
            &mut challenger,
            &fri_params,
            timing,
        )
    );
    let proof = StarkProof {
        trace_cap,
        permutation_zs_cap,
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
/// where the `C_i`s are the Stark constraints.
fn compute_quotient_polys<'a, F, C, S, const D: usize>(
    stark: &S,
    trace_commitment: &'a PolynomialBatch<F, C, D>,
    permutation_zs_commitment_challenges: &'a Option<(
        PolynomialBatch<F, C, D>,
        Vec<PermutationChallengeSet<F>>,
    )>,
    public_inputs: [F; S::PUBLIC_INPUTS],
    alphas: Vec<F>,
    degree_bits: usize,
    config: &StarkConfig,
) -> Vec<PolynomialCoeffs<F>>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
    [(); S::COLUMNS]:,
    [(); S::PUBLIC_INPUTS]:,
{
    let degree = 1 << degree_bits;
    let rate_bits = config.fri_config.rate_bits;

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
    let get_at_index =
        |comm: &'a PolynomialBatch<F, C, D>, i: usize| -> &'a [F] { comm.get_lde_values(i * step) };
    // Last element of the subgroup.
    let last = F::primitive_root_of_unity(degree_bits).inverse();
    let size = degree << quotient_degree_bits;
    let coset = F::cyclic_subgroup_coset_known_order(
        F::primitive_root_of_unity(degree_bits + quotient_degree_bits),
        F::coset_shift(),
        size,
    );

    let quotient_values = (0..size)
        .into_par_iter()
        .map(|i| {
            // TODO: Set `P` to a genuine `PackedField` here.
            let mut consumer = ConstraintConsumer::<F>::new(
                alphas.clone(),
                coset[i] - last,
                lagrange_first.values[i],
                lagrange_last.values[i],
            );
            let vars = StarkEvaluationVars::<F, F, { S::COLUMNS }, { S::PUBLIC_INPUTS }> {
                local_values: &get_at_index(trace_commitment, i).try_into().unwrap(),
                next_values: &get_at_index(trace_commitment, (i + next_step) % size)
                    .try_into()
                    .unwrap(),
                public_inputs: &public_inputs,
            };
            let permutation_check_data = permutation_zs_commitment_challenges.as_ref().map(
                |(permutation_zs_commitment, permutation_challenge_sets)| PermutationCheckData {
                    local_zs: get_at_index(&permutation_zs_commitment, i).to_vec(),
                    next_zs: get_at_index(&permutation_zs_commitment, (i + next_step) % size)
                        .to_vec(),
                    permutation_challenge_sets: permutation_challenge_sets.to_vec(),
                },
            );
            eval_vanishing_poly::<F, F, C, S, D, 1>(
                stark,
                config,
                vars,
                permutation_check_data,
                &mut consumer,
            );
            // stark.eval_packed_base(vars, &mut consumer);
            // TODO: Add in constraints for permutation arguments.
            // TODO: Fix this once we use a genuine `PackedField`.
            let mut constraints_evals = consumer.accumulators();
            // We divide the constraints evaluations by `Z_H(x)`.
            let denominator_inv = z_h_on_coset.eval_inverse(i);
            for eval in &mut constraints_evals {
                *eval *= denominator_inv;
            }
            constraints_evals
        })
        .collect::<Vec<_>>();

    transpose(&quotient_values)
        .into_par_iter()
        .map(PolynomialValues::new)
        .map(|values| values.coset_ifft(F::coset_shift()))
        .collect()
}
