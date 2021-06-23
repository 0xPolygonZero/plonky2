use std::time::Instant;

use log::info;
use rayon::prelude::*;

use crate::circuit_data::{CommonCircuitData, ProverOnlyCircuitData};
use crate::field::extension_field::Extendable;
use crate::field::fft::ifft;
use crate::field::field::Field;
use crate::generator::generate_partial_witness;
use crate::plonk_challenger::Challenger;
use crate::plonk_common::eval_vanishing_poly_base;
use crate::polynomial::commitment::ListPolynomialCommitment;
use crate::polynomial::polynomial::{PolynomialCoeffs, PolynomialValues};
use crate::proof::Proof;
use crate::timed;
use crate::util::transpose;
use crate::vars::EvaluationVarsBase;
use crate::wire::Wire;
use crate::witness::PartialWitness;

/// Corresponds to constants - sigmas - wires - zs - quotient — polynomial commitments.
pub const PLONK_BLINDING: [bool; 5] = [false, false, true, true, true];

pub(crate) fn prove<F: Extendable<D>, const D: usize>(
    prover_data: &ProverOnlyCircuitData<F>,
    common_data: &CommonCircuitData<F, D>,
    inputs: PartialWitness<F>,
) -> Proof<F, D> {
    let fri_config = &common_data.config.fri_config;

    let start_proof_gen = Instant::now();

    let mut witness = inputs;
    info!("Running {} generators", prover_data.generators.len());
    timed!(
        generate_partial_witness(&mut witness, &prover_data.generators,),
        "to generate witness"
    );

    let config = &common_data.config;
    let num_wires = config.num_wires;
    let num_challenges = config.num_challenges;
    let quotient_degree = common_data.quotient_degree();

    let degree = common_data.degree();
    let wires_polynomials: Vec<PolynomialCoeffs<F>> = timed!(
        (0..num_wires)
            .into_par_iter()
            .map(|i| compute_wire_polynomial(i, &witness, degree))
            .collect(),
        "to compute wire polynomials"
    );

    // TODO: Could try parallelizing the transpose, or not doing it explicitly, instead having
    // merkle_root_bit_rev_order do it implicitly.
    let wires_commitment = timed!(
        ListPolynomialCommitment::new(wires_polynomials, fri_config.rate_bits, true),
        "to compute wires commitment"
    );

    let mut challenger = Challenger::new();
    // Observe the instance.
    // TODO: Need to include public inputs as well.
    challenger.observe_hash(&common_data.circuit_digest);

    challenger.observe_hash(&wires_commitment.merkle_tree.root);
    let betas = challenger.get_n_challenges(num_challenges);
    let gammas = challenger.get_n_challenges(num_challenges);

    let plonk_z_vecs = timed!(compute_zs(&common_data), "to compute Z's");

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
                let quotient_poly_coeff = vanishing_poly_coeff.divide_by_z_h(degree);
                // Split t into degree-n chunks.
                quotient_poly_coeff.chunks(degree)
            })
            .collect(),
        "to compute quotient polys"
    );

    let quotient_polys_commitment = timed!(
        ListPolynomialCommitment::new(all_quotient_poly_chunks, fri_config.rate_bits, true),
        "to commit to quotient polys"
    );

    challenger.observe_hash(&quotient_polys_commitment.merkle_tree.root);

    let zeta = challenger.get_extension_challenge();

    let (opening_proof, mut openings) = timed!(
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
    common_data: &CommonCircuitData<F, D>,
) -> Vec<PolynomialCoeffs<F>> {
    (0..common_data.config.num_challenges)
        .map(|i| compute_z(common_data, i))
        .collect()
}

fn compute_z<F: Extendable<D>, const D: usize>(
    common_data: &CommonCircuitData<F, D>,
    _i: usize,
) -> PolynomialCoeffs<F> {
    PolynomialCoeffs::zero(common_data.degree()) // TODO
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
    let lde_size = common_data.lde_size();
    let lde_gen = common_data.lde_generator();
    let num_challenges = common_data.config.num_challenges;

    let points = F::cyclic_subgroup_known_order(lde_gen, lde_size);
    let values: Vec<Vec<F>> = points
        .into_par_iter()
        .enumerate()
        .map(|(i, x)| {
            let i_next = (i + 1) % lde_size;
            let local_wires = wires_commitment.leaf(i);
            let local_constants = prover_data.constants_commitment.leaf(i);
            let local_plonk_zs = plonk_zs_commitment.leaf(i);
            let next_plonk_zs = plonk_zs_commitment.leaf(i_next);
            let s_sigmas = prover_data.sigmas_commitment.leaf(i);

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

fn compute_wire_polynomial<F: Field>(
    input: usize,
    witness: &PartialWitness<F>,
    degree: usize,
) -> PolynomialCoeffs<F> {
    let wire_values = (0..degree)
        // Some gates do not use all wires, and we do not require that generators populate unused
        // wires, so some wire values will not be set. We can set these to any value; here we
        // arbitrary pick zero. Ideally we would verify that no constraints operate on these unset
        // wires, but that isn't trivial.
        .map(|gate| {
            witness
                .try_get_wire(Wire { gate, input })
                .unwrap_or(F::ZERO)
        })
        .collect();
    PolynomialValues::new(wire_values).ifft()
}
