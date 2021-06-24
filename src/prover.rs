use std::time::Instant;

use log::info;
use rayon::prelude::*;

use crate::circuit_data::{CommonCircuitData, ProverOnlyCircuitData};
use crate::field::extension_field::Extendable;
use crate::field::fft::ifft;
use crate::generator::generate_partial_witness;
use crate::plonk_challenger::Challenger;
use crate::plonk_common::eval_vanishing_poly_base;
use crate::polynomial::commitment::ListPolynomialCommitment;
use crate::polynomial::polynomial::{PolynomialCoeffs, PolynomialValues};
use crate::proof::Proof;
use crate::timed;
use crate::util::transpose;
use crate::vars::EvaluationVarsBase;
use crate::witness::{PartialWitness, Witness};

pub(crate) fn prove<F: Extendable<D>, const D: usize>(
    prover_data: &ProverOnlyCircuitData<F, D>,
    common_data: &CommonCircuitData<F, D>,
    inputs: PartialWitness<F>,
) -> Proof<F, D> {
    let fri_config = &common_data.config.fri_config;
    let config = &common_data.config;
    let num_wires = config.num_wires;
    let num_challenges = config.num_challenges;
    let quotient_degree = common_data.quotient_degree();
    let degree = common_data.degree();

    let start_proof_gen = Instant::now();

    let mut partial_witness = inputs;
    info!("Running {} generators", prover_data.generators.len());
    timed!(
        generate_partial_witness(&mut partial_witness, &prover_data.generators),
        "to generate witness"
    );

    let witness = timed!(
        partial_witness.full_witness(degree, num_wires),
        "to compute full witness"
    );

    timed!(
        witness
            .check_copy_constraints(&prover_data.copy_constraints, &prover_data.gate_instances)
            .unwrap(), // TODO: Change return value to `Result` and use `?` here.
        "to check copy constraints"
    );

    let wires_values: Vec<PolynomialValues<F>> = timed!(
        witness
            .wire_values
            .iter()
            .map(|column| PolynomialValues::new(column.clone()))
            .collect(),
        "to compute wire polynomials"
    );

    // TODO: Could try parallelizing the transpose, or not doing it explicitly, instead having
    // merkle_root_bit_rev_order do it implicitly.
    let wires_commitment = timed!(
        ListPolynomialCommitment::new(wires_values, fri_config.rate_bits, true),
        "to compute wires commitment"
    );

    let mut challenger = Challenger::new();
    // Observe the instance.
    // TODO: Need to include public inputs as well.
    challenger.observe_hash(&common_data.circuit_digest);

    challenger.observe_hash(&wires_commitment.merkle_tree.root);
    let betas = challenger.get_n_challenges(num_challenges);
    let gammas = challenger.get_n_challenges(num_challenges);

    let plonk_z_vecs = timed!(
        compute_zs(&witness, &betas, &gammas, &prover_data, &common_data),
        "to compute Z's"
    );

    let plonk_zs_commitment = timed!(
        ListPolynomialCommitment::new(plonk_z_vecs, fri_config.rate_bits, true),
        "to commit to Z's"
    );

    challenger.observe_hash(&plonk_zs_commitment.merkle_tree.root);

    let alphas = challenger.get_n_challenges(num_challenges);

    let quotient_polys = timed!(
        compute_quotient_polys(
            common_data,
            prover_data,
            &wires_commitment,
            &plonk_zs_commitment,
            &betas,
            &gammas,
            &alphas,
        ),
        "to compute vanishing polys"
    );

    // Compute the quotient polynomials, aka `t` in the Plonk paper.
    let all_quotient_poly_chunks = timed!(
        quotient_polys
            .into_par_iter()
            .flat_map(|mut quotient_poly| {
                quotient_poly.trim();
                quotient_poly.pad(quotient_degree);
                // Split t into degree-n chunks.
                quotient_poly.chunks(degree)
            })
            .collect(),
        "to compute quotient polys"
    );

    let quotient_polys_commitment = timed!(
        ListPolynomialCommitment::new_from_polys(
            all_quotient_poly_chunks,
            fri_config.rate_bits,
            true
        ),
        "to commit to quotient polys"
    );

    challenger.observe_hash(&quotient_polys_commitment.merkle_tree.root);

    let zeta = challenger.get_extension_challenge();

    let (opening_proof, openings) = timed!(
        ListPolynomialCommitment::open_plonk(
            &[
                &prover_data.constants_commitment,
                &prover_data.sigmas_commitment,
                &wires_commitment,
                &plonk_zs_commitment,
                &quotient_polys_commitment,
            ],
            zeta,
            &mut challenger,
            &common_data.config.fri_config
        ),
        "to compute opening proofs"
    );

    info!(
        "{:.3}s for overall witness & proof generation",
        start_proof_gen.elapsed().as_secs_f32()
    );

    Proof {
        wires_root: wires_commitment.merkle_tree.root,
        plonk_zs_root: plonk_zs_commitment.merkle_tree.root,
        quotient_polys_root: quotient_polys_commitment.merkle_tree.root,
        openings,
        opening_proof,
    }
}

fn compute_zs<F: Extendable<D>, const D: usize>(
    witness: &Witness<F>,
    betas: &[F],
    gammas: &[F],
    prover_data: &ProverOnlyCircuitData<F, D>,
    common_data: &CommonCircuitData<F, D>,
) -> Vec<PolynomialValues<F>> {
    (0..common_data.config.num_challenges)
        .map(|i| compute_z(witness, betas[i], gammas[i], prover_data, common_data))
        .collect()
}

fn compute_z<F: Extendable<D>, const D: usize>(
    witness: &Witness<F>,
    beta: F,
    gamma: F,
    prover_data: &ProverOnlyCircuitData<F, D>,
    common_data: &CommonCircuitData<F, D>,
) -> PolynomialValues<F> {
    let subgroup = &prover_data.subgroup;
    let mut plonk_z_points = vec![F::ONE];
    let k_is = &common_data.k_is;
    for i in 1..common_data.degree() {
        let x = subgroup[i - 1];
        let mut numerator = F::ONE;
        let mut denominator = F::ONE;
        let s_sigmas = prover_data.sigmas_commitment.original_values(i - 1);
        for j in 0..common_data.config.num_routed_wires {
            let wire_value = witness.get_wire(i - 1, j);
            let k_i = k_is[j];
            let s_id = k_i * x;
            let s_sigma = s_sigmas[j];
            numerator *= wire_value + beta * s_id + gamma;
            denominator *= wire_value + beta * s_sigma + gamma;
        }
        let last = *plonk_z_points.last().unwrap();
        plonk_z_points.push(last * numerator / denominator);
    }
    plonk_z_points.into()
}

fn compute_quotient_polys<'a, F: Extendable<D>, const D: usize>(
    common_data: &CommonCircuitData<F, D>,
    prover_data: &'a ProverOnlyCircuitData<F, D>,
    wires_commitment: &'a ListPolynomialCommitment<F>,
    plonk_zs_commitment: &'a ListPolynomialCommitment<F>,
    betas: &[F],
    gammas: &[F],
    alphas: &[F],
) -> Vec<PolynomialCoeffs<F>> {
    let num_challenges = common_data.config.num_challenges;
    assert!(
        common_data.max_filtered_constraint_degree_bits <= common_data.config.rate_bits,
        "Having constraints of degree higher than the rate is not supported yet. \
        If we need this in the future, we can precompute the larger LDE before computing the `ListPolynomialCommitment`s."
    );

    // We reuse the LDE computed in `ListPolynomialCommitment` and extract every `step` points to get
    // an LDE matching `max_filtered_constraint_degree`.
    let step =
        1 << (common_data.config.rate_bits - common_data.max_filtered_constraint_degree_bits);
    // When opening the `Z`s polys at the "next" point in Plonk, need to look at the point `next_step`
    // steps away since we work on an LDE of degree `max_filtered_constraint_degree`.
    let next_step = 1 << common_data.max_filtered_constraint_degree_bits;

    let points = F::two_adic_subgroup(
        common_data.degree_bits + common_data.max_filtered_constraint_degree_bits,
    );
    let lde_size = points.len();

    // Retrieve the LDE values at index `i`.
    let get_at_index = |comm: &'a ListPolynomialCommitment<F>, i: usize| -> &'a [F] {
        comm.get_lde_values(i * step)
    };

    let quotient_values: Vec<Vec<F>> = points
        .into_par_iter()
        .enumerate()
        .map(|(i, x)| {
            let i_next = (i + next_step) % lde_size;
            let local_constants = get_at_index(&prover_data.constants_commitment, i);
            let s_sigmas = get_at_index(&prover_data.sigmas_commitment, i);
            let local_wires = get_at_index(&wires_commitment, i);
            let local_plonk_zs = get_at_index(&plonk_zs_commitment, i);
            let next_plonk_zs = get_at_index(&plonk_zs_commitment, i_next);

            debug_assert_eq!(local_wires.len(), common_data.config.num_wires);
            debug_assert_eq!(local_plonk_zs.len(), num_challenges);

            let vars = EvaluationVarsBase {
                local_constants,
                local_wires,
            };
            let mut quotient_values = eval_vanishing_poly_base(
                common_data,
                x,
                vars,
                local_plonk_zs,
                next_plonk_zs,
                s_sigmas,
                betas,
                gammas,
                alphas,
            );
            // TODO: We can avoid computing the exp.
            let denominator_inv = x.exp(common_data.degree() as u64).inverse();
            quotient_values
                .iter_mut()
                .for_each(|v| *v *= denominator_inv);
            quotient_values
        })
        .collect();

    transpose(&quotient_values)
        .into_iter()
        .map(PolynomialValues::new)
        .map(|values| values.coset_ifft(F::MULTIPLICATIVE_GROUP_GENERATOR))
        .collect()
}
