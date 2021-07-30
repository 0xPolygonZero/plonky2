use anyhow::{ensure, Result};

use crate::field::extension_field::{flatten, Extendable, FieldExtension};
use crate::field::field_types::Field;
use crate::field::interpolation::{barycentric_weights, interpolate, interpolate2};
use crate::fri::proof::{FriInitialTreeProof, FriProof, FriQueryRound};
use crate::fri::FriConfig;
use crate::hash::hash_types::HashOut;
use crate::hash::hashing::hash_n_to_1;
use crate::hash::merkle_proofs::verify_merkle_proof;
use crate::iop::challenger::Challenger;
use crate::plonk::circuit_data::CommonCircuitData;
use crate::plonk::plonk_common::PlonkPolynomials;
use crate::plonk::proof::OpeningSet;
use crate::util::reducing::ReducingFactor;
use crate::util::{log2_strict, reverse_bits, reverse_index_bits_in_place};

/// Computes P'(x^arity) from {P(x*g^i)}_(i=0..arity), where g is a `arity`-th root of unity
/// and P' is the FRI reduced polynomial.
fn compute_evaluation<F: Field + Extendable<D>, const D: usize>(
    x: F,
    old_x_index: usize,
    arity_bits: usize,
    last_evals: &[F::Extension],
    beta: F::Extension,
) -> F::Extension {
    let arity = 1 << arity_bits;
    debug_assert_eq!(last_evals.len(), arity);

    let g = F::primitive_root_of_unity(arity_bits);

    // The evaluation vector needs to be reordered first.
    let mut evals = last_evals.to_vec();
    reverse_index_bits_in_place(&mut evals);
    let rev_old_x_index = reverse_bits(old_x_index, arity_bits);
    let coset_start = x * g.exp((arity - rev_old_x_index) as u64);
    // The answer is gotten by interpolating {(x*g^i, P(x*g^i))} and evaluating at beta.
    let points = g
        .powers()
        .zip(evals)
        .map(|(y, e)| ((coset_start * y).into(), e))
        .collect::<Vec<_>>();
    let barycentric_weights = barycentric_weights(&points);
    interpolate(&points, beta, &barycentric_weights)
}

fn fri_verify_proof_of_work<F: Field + Extendable<D>, const D: usize>(
    proof: &FriProof<F, D>,
    challenger: &mut Challenger<F>,
    config: &FriConfig,
) -> Result<()> {
    let hash = hash_n_to_1(
        challenger
            .get_hash()
            .elements
            .iter()
            .copied()
            .chain(Some(proof.pow_witness))
            .collect(),
        false,
    );
    ensure!(
        hash.to_canonical_u64().leading_zeros()
            >= config.proof_of_work_bits + (64 - F::order().bits()) as u32,
        "Invalid proof of work witness."
    );

    Ok(())
}

pub fn verify_fri_proof<F: Field + Extendable<D>, const D: usize>(
    purported_degree_log: usize,
    // Openings of the PLONK polynomials.
    os: &OpeningSet<F, D>,
    // Point at which the PLONK polynomials are opened.
    zeta: F::Extension,
    // Scaling factor to combine polynomials.
    alpha: F::Extension,
    initial_merkle_roots: &[HashOut<F>],
    proof: &FriProof<F, D>,
    challenger: &mut Challenger<F>,
    common_data: &CommonCircuitData<F, D>,
) -> Result<()> {
    let config = &common_data.config;
    let total_arities = config.fri_config.reduction_arity_bits.iter().sum::<usize>();
    ensure!(
        purported_degree_log == log2_strict(proof.final_poly.len()) + total_arities,
        "Final polynomial has wrong degree."
    );

    // Size of the LDE domain.
    let n = proof.final_poly.len() << (total_arities + config.rate_bits);

    // Recover the random betas used in the FRI reductions.
    let betas = proof
        .commit_phase_merkle_roots
        .iter()
        .map(|root| {
            challenger.observe_hash(root);
            challenger.get_extension_challenge()
        })
        .collect::<Vec<_>>();
    challenger.observe_extension_elements(&proof.final_poly.coeffs);

    // Check PoW.
    fri_verify_proof_of_work(proof, challenger, &config.fri_config)?;

    // Check that parameters are coherent.
    ensure!(
        config.fri_config.num_query_rounds == proof.query_round_proofs.len(),
        "Number of query rounds does not match config."
    );
    ensure!(
        !config.fri_config.reduction_arity_bits.is_empty(),
        "Number of reductions should be non-zero."
    );

    let precomputed_reduced_evals = PrecomputedReducedEvals::from_os_and_alpha(os, alpha);
    for round_proof in &proof.query_round_proofs {
        fri_verifier_query_round(
            zeta,
            alpha,
            precomputed_reduced_evals,
            initial_merkle_roots,
            &proof,
            challenger,
            n,
            &betas,
            round_proof,
            common_data,
        )?;
    }

    Ok(())
}

fn fri_verify_initial_proof<F: Field>(
    x_index: usize,
    proof: &FriInitialTreeProof<F>,
    initial_merkle_roots: &[HashOut<F>],
) -> Result<()> {
    for ((evals, merkle_proof), &root) in proof.evals_proofs.iter().zip(initial_merkle_roots) {
        verify_merkle_proof(evals.clone(), x_index, root, merkle_proof, false)?;
    }

    Ok(())
}

/// Holds the reduced (by `alpha`) evaluations at `zeta` for the polynomial opened just at
/// zeta, for `Z` at zeta and for `Z` at `g*zeta`.
#[derive(Copy, Clone)]
struct PrecomputedReducedEvals<F: Extendable<D>, const D: usize> {
    pub single: F::Extension,
    pub zs: F::Extension,
    pub zs_right: F::Extension,
}

impl<F: Extendable<D>, const D: usize> PrecomputedReducedEvals<F, D> {
    fn from_os_and_alpha(os: &OpeningSet<F, D>, alpha: F::Extension) -> Self {
        let mut alpha = ReducingFactor::new(alpha);
        let single = alpha.reduce(
            os.constants
                .iter()
                .chain(&os.plonk_sigmas)
                .chain(&os.wires)
                .chain(&os.quotient_polys)
                .chain(&os.partial_products),
        );
        let zs = alpha.reduce(os.plonk_zs.iter());
        let zs_right = alpha.reduce(os.plonk_zs_right.iter());

        Self {
            single,
            zs,
            zs_right,
        }
    }
}

fn fri_combine_initial<F: Field + Extendable<D>, const D: usize>(
    proof: &FriInitialTreeProof<F>,
    alpha: F::Extension,
    zeta: F::Extension,
    subgroup_x: F,
    precomputed_reduced_evals: PrecomputedReducedEvals<F, D>,
    common_data: &CommonCircuitData<F, D>,
) -> F::Extension {
    let config = &common_data.config;
    assert!(D > 1, "Not implemented for D=1.");
    let degree_log = proof.evals_proofs[0].1.siblings.len() - config.rate_bits;
    let subgroup_x = F::Extension::from_basefield(subgroup_x);
    let mut alpha = ReducingFactor::new(alpha);
    let mut sum = F::Extension::ZERO;

    // We will add three terms to `sum`:
    // - one for various polynomials which are opened at a single point `x`
    // - one for Zs, which are opened at `x` and `g x`

    // Polynomials opened at `x`, i.e., the constants-sigmas, wires, quotient and partial products polynomials.
    let single_evals = [
        PlonkPolynomials::CONSTANTS_SIGMAS,
        PlonkPolynomials::WIRES,
        PlonkPolynomials::QUOTIENT,
    ]
    .iter()
    .flat_map(|&p| proof.unsalted_evals(p, config.zero_knowledge))
    .chain(
        &proof.unsalted_evals(PlonkPolynomials::ZS_PARTIAL_PRODUCTS, config.zero_knowledge)
            [common_data.partial_products_range()],
    )
    .map(|&e| F::Extension::from_basefield(e));
    let single_composition_eval = alpha.reduce(single_evals);
    let single_numerator = single_composition_eval - precomputed_reduced_evals.single;
    let single_denominator = subgroup_x - zeta;
    sum += single_numerator / single_denominator;
    alpha.reset();

    // Polynomials opened at `x` and `g x`, i.e., the Zs polynomials.
    let zs_evals = proof
        .unsalted_evals(PlonkPolynomials::ZS_PARTIAL_PRODUCTS, config.zero_knowledge)
        .iter()
        .map(|&e| F::Extension::from_basefield(e))
        .take(common_data.zs_range().end);
    let zs_composition_eval = alpha.reduce(zs_evals);
    let zeta_right = F::Extension::primitive_root_of_unity(degree_log) * zeta;
    let zs_interpol = interpolate2(
        [
            (zeta, precomputed_reduced_evals.zs),
            (zeta_right, precomputed_reduced_evals.zs_right),
        ],
        subgroup_x,
    );
    let zs_numerator = zs_composition_eval - zs_interpol;
    let zs_denominator = (subgroup_x - zeta) * (subgroup_x - zeta_right);
    sum = alpha.shift(sum);
    sum += zs_numerator / zs_denominator;

    sum
}

fn fri_verifier_query_round<F: Field + Extendable<D>, const D: usize>(
    zeta: F::Extension,
    alpha: F::Extension,
    precomputed_reduced_evals: PrecomputedReducedEvals<F, D>,
    initial_merkle_roots: &[HashOut<F>],
    proof: &FriProof<F, D>,
    challenger: &mut Challenger<F>,
    n: usize,
    betas: &[F::Extension],
    round_proof: &FriQueryRound<F, D>,
    common_data: &CommonCircuitData<F, D>,
) -> Result<()> {
    let config = &common_data.config.fri_config;
    let x = challenger.get_challenge();
    let mut domain_size = n;
    let mut x_index = x.to_canonical_u64() as usize % n;
    fri_verify_initial_proof(
        x_index,
        &round_proof.initial_trees_proof,
        initial_merkle_roots,
    )?;
    let mut old_x_index = 0;
    // `subgroup_x` is `subgroup[x_index]`, i.e., the actual field element in the domain.
    let log_n = log2_strict(n);
    let mut subgroup_x = F::MULTIPLICATIVE_GROUP_GENERATOR
        * F::primitive_root_of_unity(log_n).exp(reverse_bits(x_index, log_n) as u64);

    let mut evaluations: Vec<Vec<F::Extension>> = Vec::new();
    for (i, &arity_bits) in config.reduction_arity_bits.iter().enumerate() {
        let arity = 1 << arity_bits;
        let next_domain_size = domain_size >> arity_bits;
        let e_x = if i == 0 {
            fri_combine_initial(
                &round_proof.initial_trees_proof,
                alpha,
                zeta,
                subgroup_x,
                precomputed_reduced_evals,
                common_data,
            )
        } else {
            let last_evals = &evaluations[i - 1];
            // Infer P(y) from {P(x)}_{x^arity=y}.
            compute_evaluation(
                subgroup_x,
                old_x_index,
                config.reduction_arity_bits[i - 1],
                last_evals,
                betas[i - 1],
            )
        };
        let mut evals = round_proof.steps[i].evals.clone();
        // Insert P(y) into the evaluation vector, since it wasn't included by the prover.
        evals.insert(x_index & (arity - 1), e_x);
        verify_merkle_proof(
            flatten(&evals),
            x_index >> arity_bits,
            proof.commit_phase_merkle_roots[i],
            &round_proof.steps[i].merkle_proof,
            false,
        )?;
        evaluations.push(evals);

        if i > 0 {
            // Update the point x to x^arity.
            subgroup_x = subgroup_x.exp_power_of_2(config.reduction_arity_bits[i - 1]);
        }
        domain_size = next_domain_size;
        old_x_index = x_index & (arity - 1);
        x_index >>= arity_bits;
    }

    let last_evals = evaluations.last().unwrap();
    let final_arity_bits = *config.reduction_arity_bits.last().unwrap();
    let purported_eval = compute_evaluation(
        subgroup_x,
        old_x_index,
        final_arity_bits,
        last_evals,
        *betas.last().unwrap(),
    );
    subgroup_x = subgroup_x.exp_power_of_2(final_arity_bits);

    // Final check of FRI. After all the reductions, we check that the final polynomial is equal
    // to the one sent by the prover.
    ensure!(
        proof.final_poly.eval(subgroup_x.into()) == purported_eval,
        "Final polynomial evaluation is invalid."
    );

    Ok(())
}
