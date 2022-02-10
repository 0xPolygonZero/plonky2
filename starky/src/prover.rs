use anyhow::{ensure, Result};
use itertools::Itertools;
use plonky2::field::extension_field::Extendable;
use plonky2::field::field_types::Field;
use plonky2::field::polynomial::{PolynomialCoeffs, PolynomialValues};
use plonky2::field::zero_poly_coset::ZeroPolyOnCoset;
use plonky2::fri::oracle::PolynomialBatch;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::challenger::Challenger;
use plonky2::plonk::config::GenericConfig;
use plonky2::timed;
use plonky2::util::timing::TimingTree;
use plonky2::util::transpose;
use plonky2_util::{log2_ceil, log2_strict};
use rayon::prelude::*;

use crate::config::StarkConfig;
use crate::constraint_consumer::ConstraintConsumer;
use crate::proof::{StarkOpeningSet, StarkProof, StarkProofWithPublicInputs};
use crate::stark::{PermutationPair, Stark};
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
{
    let degree = trace.len();
    let degree_bits = log2_strict(degree);

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

    let rate_bits = config.fri_config.rate_bits;
    let cap_height = config.fri_config.cap_height;
    let trace_commitment = timed!(
        timing,
        "compute trace commitment",
        PolynomialBatch::<F, C, D>::from_values(
            trace_poly_values,
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
    let betas = challenger.get_n_challenges(config.num_challenges);
    let gammas = challenger.get_n_challenges(config.num_challenges);
    let z_polys = compute_z_polys(&stark, &trace, &betas, &gammas);

    let alphas = challenger.get_n_challenges(config.num_challenges);
    let quotient_polys = compute_quotient_polys::<F, C, S, D>(
        &stark,
        &trace_commitment,
        public_inputs,
        alphas,
        degree_bits,
        rate_bits,
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
    let g = F::Extension::primitive_root_of_unity(degree_bits);
    ensure!(
        zeta.exp_power_of_2(degree_bits) != F::Extension::ONE,
        "Opening point is in the subgroup."
    );
    let openings = StarkOpeningSet::new(zeta, g, &trace_commitment, &quotient_commitment);
    challenger.observe_openings(&openings.to_fri_openings());

    // TODO: Add permuation checks
    let initial_merkle_trees = &[&trace_commitment, &quotient_commitment];
    let fri_params = config.fri_params(degree_bits);

    let opening_proof = timed!(
        timing,
        "compute openings proof",
        PolynomialBatch::prove_openings(
            &stark.fri_instance(zeta, g, rate_bits, config.num_challenges),
            initial_merkle_trees,
            &mut challenger,
            &fri_params,
            timing,
        )
    );
    let proof = StarkProof {
        trace_cap,
        quotient_polys_cap,
        openings,
        opening_proof,
    };

    Ok(StarkProofWithPublicInputs {
        proof,
        public_inputs: public_inputs.to_vec(),
    })
}

/// Compute all Z polynomials (for permutation arguments).
fn compute_z_polys<F, S, const D: usize>(
    stark: &S,
    trace: &[[F; S::COLUMNS]],
    betas: &[F],
    gammas: &[F],
) -> Vec<PolynomialValues<F>>
where
    F: RichField + Extendable<D>,
    S: Stark<F, D>,
{
    let degree = trace.len();
    let pairs = stark.permutation_pairs();
    let zs_row_major: Vec<_> = trace
        .into_par_iter()
        .map(|trace_row| compute_z_polys_row::<F, S, D>(&pairs, trace_row, betas, gammas))
        .collect();
    let zs_col_major = transpose(&zs_row_major);
    zs_col_major
        .into_iter()
        .map(|col| PolynomialValues::new(col))
        .collect()
}

/// Compute all Z polynomials (for permutation arguments) at a single point.
fn compute_z_polys_row<F, S, const D: usize>(
    pairs: &[PermutationPair],
    trace_row: &[F; S::COLUMNS],
    betas: &[F],
    gammas: &[F],
) -> Vec<F>
where
    F: RichField + Extendable<D>,
    S: Stark<F, D>,
{
    assert_eq!(betas.len(), gammas.len());
    let num_challenges = betas.len();
    assert_ne!(num_challenges, 0);

    let num_zs = pairs.len() * num_challenges;
    let mut numerators = Vec::with_capacity(num_zs);
    let mut denominators = Vec::with_capacity(num_zs);
    let denominator_invs = F::batch_multiplicative_inverse(&denominators);

    for pp in pairs {
        for (&beta, &gamma) in betas.iter().zip(gammas) {
            let cols_to_reduced_values = |cols: &[usize]| {
                cols.iter()
                    .map(|&c| trace_row[c])
                    .rev()
                    .reduce(|acc, x| acc * beta + x)
                    .unwrap()
            };
            numerators.push(cols_to_reduced_values(&pp.lhs_columns) + gamma);
            denominators.push(cols_to_reduced_values(&pp.rhs_columns) + gamma);
        }
    }

    numerators
        .into_iter()
        .zip(denominator_invs)
        .map(|(n, d_inv)| n * d_inv)
        .collect()
}

/// Computes the quotient polynomials `(sum alpha^i C_i(x)) / Z_H(x)` for `alpha` in `alphas`,
/// where the `C_i`s are the Stark constraints.
fn compute_quotient_polys<F, C, S, const D: usize>(
    stark: &S,
    trace_commitment: &PolynomialBatch<F, C, D>,
    public_inputs: [F; S::PUBLIC_INPUTS],
    alphas: Vec<F>,
    degree_bits: usize,
    rate_bits: usize,
) -> Vec<PolynomialCoeffs<F>>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
    [(); S::COLUMNS]:,
    [(); S::PUBLIC_INPUTS]:,
{
    let degree = 1 << degree_bits;

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
    let get_at_index = |comm: &PolynomialBatch<F, C, D>, i: usize| -> [F; S::COLUMNS] {
        comm.get_lde_values(i * step).try_into().unwrap()
    };
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
                local_values: &get_at_index(trace_commitment, i),
                next_values: &get_at_index(trace_commitment, (i + next_step) % size),
                public_inputs: &public_inputs,
            };
            stark.eval_packed_base(vars, &mut consumer);
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
