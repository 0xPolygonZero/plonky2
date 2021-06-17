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
use crate::polynomial::polynomial::PolynomialValues;
use crate::proof::Proof;
use crate::timed;
use crate::util::transpose;
use crate::vars::EvaluationVarsBase;
use crate::witness::{PartialWitness, Witness};

/// Corresponds to constants - sigmas - wires - zs - quotient — polynomial commitments.
pub const PLONK_BLINDING: [bool; 5] = [false, false, true, true, true];

pub(crate) fn prove<F: Extendable<D>, const D: usize>(
    prover_data: &ProverOnlyCircuitData<F>,
    common_data: &CommonCircuitData<F, D>,
    inputs: PartialWitness<F>,
) -> Proof<F, D> {
    let fri_config = &common_data.config.fri_config;

    let start_proof_gen = Instant::now();

    let mut partial_witness = inputs;
    info!("Running {} generators", prover_data.generators.len());
    timed!(
        generate_partial_witness(&mut partial_witness, &prover_data.generators),
        "to generate witness"
    );

    let config = &common_data.config;
    let num_wires = config.num_wires;
    let num_challenges = config.num_challenges;
    let quotient_degree = common_data.quotient_degree();
    let degree = common_data.degree();

    let witness = partial_witness.full_witness(degree, num_wires);

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

    let vanishing_polys = timed!(
        compute_vanishing_polys(
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
        vanishing_polys
            .into_par_iter()
            .flat_map(|vanishing_poly| {
                let vanishing_poly_coeff = ifft(vanishing_poly);
                // TODO: run `padded` when the division works.
                let quotient_poly_coeff = vanishing_poly_coeff.divide_by_z_h(degree);
                let x = F::rand();
                assert!(
                    quotient_poly_coeff.eval(x) * (x.exp(degree as u64) - F::ONE)
                        != vanishing_poly_coeff.eval(x),
                    "That's good news, this should fail! The division by z_h doesn't work yet,\
                    most likely because compute_vanishing_polys isn't complete (doesn't use filters for example)."
                );
                // Split t into degree-n chunks.
                quotient_poly_coeff.chunks(degree)
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
    prover_data: &ProverOnlyCircuitData<F>,
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
    prover_data: &ProverOnlyCircuitData<F>,
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

fn compute_vanishing_polys<F: Extendable<D>, const D: usize>(
    common_data: &CommonCircuitData<F, D>,
    prover_data: &ProverOnlyCircuitData<F>,
    wires_commitment: &ListPolynomialCommitment<F>,
    plonk_zs_commitment: &ListPolynomialCommitment<F>,
    betas: &[F],
    gammas: &[F],
    alphas: &[F],
) -> Vec<PolynomialValues<F>> {
    let num_challenges = common_data.config.num_challenges;
    let points = F::two_adic_subgroup(
        common_data.degree_bits + common_data.max_filtered_constraint_degree_bits,
    );
    let lde_size = points.len();

    // Low-degree extend the polynomials commited in `comm` to the subgroup of size `lde_size`.
    let commitment_to_lde = |comm: &ListPolynomialCommitment<F>| -> Vec<PolynomialValues<F>> {
        comm.polynomials
            .iter()
            .map(|p| p.lde(common_data.max_filtered_constraint_degree_bits).fft())
            .collect()
    };

    let constants_lde = commitment_to_lde(&prover_data.constants_commitment);
    let sigmas_lde = commitment_to_lde(&prover_data.sigmas_commitment);
    let wires_lde = commitment_to_lde(wires_commitment);
    let zs_lde = commitment_to_lde(plonk_zs_commitment);

    // Retrieve the polynomial values at index `i`.
    let get_at_index = |ldes: &[PolynomialValues<F>], i: usize| {
        ldes.iter().map(|l| l.values[i]).collect::<Vec<_>>()
    };

    let values: Vec<Vec<F>> = points
        .into_par_iter()
        .enumerate()
        .map(|(i, x)| {
            let i_next = (i + 1) % lde_size;
            let local_constants = &get_at_index(&constants_lde, i);
            let s_sigmas = &get_at_index(&sigmas_lde, i);
            let local_wires = &get_at_index(&wires_lde, i);
            let local_plonk_zs = &get_at_index(&zs_lde, i);
            let next_plonk_zs = &get_at_index(&zs_lde, i_next);

            debug_assert_eq!(local_wires.len(), common_data.config.num_wires);
            debug_assert_eq!(local_plonk_zs.len(), num_challenges);

            let vars = EvaluationVarsBase {
                local_constants,
                local_wires,
            };
            eval_vanishing_poly_base(
                common_data,
                x,
                vars,
                local_plonk_zs,
                next_plonk_zs,
                s_sigmas,
                betas,
                gammas,
                alphas,
            )
        })
        .collect();

    transpose(&values)
        .into_iter()
        .map(PolynomialValues::new)
        .collect()
}
