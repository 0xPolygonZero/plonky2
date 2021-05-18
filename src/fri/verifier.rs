use crate::field::extension_field::{flatten, Extendable, FieldExtension};
use crate::field::field::Field;
use crate::field::lagrange::{barycentric_weights, interpolant, interpolate};
use crate::fri::FriConfig;
use crate::hash::hash_n_to_1;
use crate::merkle_proofs::verify_merkle_proof;
use crate::plonk_challenger::Challenger;
use crate::polynomial::commitment::{EXTENSION_DEGREE, SALT_SIZE};
use crate::polynomial::polynomial::PolynomialCoeffs;
use crate::proof::{FriInitialTreeProof, FriProof, FriQueryRound, Hash};
use crate::util::{log2_strict, reverse_bits, reverse_index_bits_in_place};
use anyhow::{ensure, Result};

/// Computes P'(x^arity) from {P(x*g^i)}_(i=0..arity), where g is a `arity`-th root of unity
/// and P' is the FRI reduced polynomial.
fn compute_evaluation<F: Field>(
    x: F,
    old_x_index: usize,
    arity_bits: usize,
    last_evals: &[F::Extension],
    beta: F::Extension,
) -> F::Extension
where
    F: Extendable<EXTENSION_DEGREE>,
{
    debug_assert_eq!(last_evals.len(), 1 << arity_bits);

    let g = F::primitive_root_of_unity(arity_bits);

    // The evaluation vector needs to be reordered first.
    let mut evals = last_evals.to_vec();
    reverse_index_bits_in_place(&mut evals);
    evals.rotate_left(reverse_bits(old_x_index, arity_bits));

    // The answer is gotten by interpolating {(x*g^i, P(x*g^i))} and evaluating at beta.
    let points = g
        .powers()
        .zip(evals)
        .map(|(y, e)| ((x * y).into(), e))
        .collect::<Vec<_>>();
    dbg!(&points);
    let barycentric_weights = barycentric_weights(&points);
    interpolate(&points, beta, &barycentric_weights)
}

fn fri_verify_proof_of_work<F: Field>(
    proof: &FriProof<F>,
    challenger: &mut Challenger<F>,
    config: &FriConfig,
) -> Result<()>
where
    F: Extendable<EXTENSION_DEGREE>,
{
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
            >= config.proof_of_work_bits + F::ORDER.leading_zeros(),
        "Invalid proof of work witness."
    );

    Ok(())
}

pub fn verify_fri_proof<F: Field>(
    purported_degree_log: usize,
    // Point-evaluation pairs for polynomial commitments.
    points: &[(F::Extension, F::Extension)],
    // Scaling factor to combine polynomials.
    alpha: F::Extension,
    initial_merkle_roots: &[Hash<F>],
    proof: &FriProof<F>,
    challenger: &mut Challenger<F>,
    config: &FriConfig,
) -> Result<()>
where
    F: Extendable<EXTENSION_DEGREE>,
{
    let total_arities = config.reduction_arity_bits.iter().sum::<usize>();
    ensure!(
        purported_degree_log
            == log2_strict(proof.final_poly.len()) + total_arities - config.rate_bits,
        "Final polynomial has wrong degree."
    );

    // Size of the LDE domain.
    let n = proof.final_poly.len() << total_arities;

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
    fri_verify_proof_of_work(proof, challenger, config)?;

    // Check that parameters are coherent.
    ensure!(
        config.num_query_rounds == proof.query_round_proofs.len(),
        "Number of query rounds does not match config."
    );
    ensure!(
        !config.reduction_arity_bits.is_empty(),
        "Number of reductions should be non-zero."
    );

    let interpolant = interpolant(points);
    for round_proof in &proof.query_round_proofs {
        fri_verifier_query_round(
            &interpolant,
            points,
            alpha,
            initial_merkle_roots,
            &proof,
            challenger,
            n,
            &betas,
            round_proof,
            config,
        )?;
    }

    Ok(())
}

fn fri_verify_initial_proof<F: Field>(
    x_index: usize,
    proof: &FriInitialTreeProof<F>,
    initial_merkle_roots: &[Hash<F>],
) -> Result<()> {
    for ((evals, merkle_proof), &root) in proof.evals_proofs.iter().zip(initial_merkle_roots) {
        verify_merkle_proof(evals.clone(), x_index, root, merkle_proof, false)?;
    }

    Ok(())
}

fn fri_combine_initial<F: Field>(
    proof: &FriInitialTreeProof<F>,
    alpha: F::Extension,
    interpolant: &PolynomialCoeffs<F::Extension>,
    points: &[(F::Extension, F::Extension)],
    subgroup_x: F,
    config: &FriConfig,
) -> F::Extension
where
    F: Extendable<EXTENSION_DEGREE>,
{
    let e = proof
        .evals_proofs
        .iter()
        .enumerate()
        .flat_map(|(i, (v, _))| &v[..v.len() - if config.blinding[i] { SALT_SIZE } else { 0 }])
        .rev()
        .fold(F::Extension::ZERO, |acc, &e| alpha * acc + e.into());
    let numerator = e - interpolant.eval(subgroup_x.into());
    let denominator = points
        .iter()
        .map(|&(x, _)| F::Extension::from_basefield(subgroup_x) - x)
        .product();
    numerator / denominator
}

fn fri_verifier_query_round<F: Field>(
    interpolant: &PolynomialCoeffs<F::Extension>,
    points: &[(F::Extension, F::Extension)],
    alpha: F::Extension,
    initial_merkle_roots: &[Hash<F>],
    proof: &FriProof<F>,
    challenger: &mut Challenger<F>,
    n: usize,
    betas: &[F::Extension],
    round_proof: &FriQueryRound<F>,
    config: &FriConfig,
) -> Result<()>
where
    F: Extendable<EXTENSION_DEGREE>,
{
    let mut evaluations: Vec<Vec<F::Extension>> = Vec::new();
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
        * F::primitive_root_of_unity(log_n).exp_usize(reverse_bits(x_index, log_n));
    for (i, &arity_bits) in config.reduction_arity_bits.iter().enumerate() {
        let arity = 1 << arity_bits;
        let next_domain_size = domain_size >> arity_bits;
        let e_x = if i == 0 {
            fri_combine_initial(
                &round_proof.initial_trees_proof,
                alpha,
                interpolant,
                points,
                subgroup_x,
                config,
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
        evaluations.push(evals);
        verify_merkle_proof(
            flatten(&evaluations[i]),
            x_index >> arity_bits,
            proof.commit_phase_merkle_roots[i],
            &round_proof.steps[i].merkle_proof,
            false,
        )?;

        if i > 0 {
            // Update the point x to x^arity.
            for _ in 0..config.reduction_arity_bits[i - 1] {
                subgroup_x = subgroup_x.square();
            }
        }
        domain_size = next_domain_size;
        old_x_index = x_index;
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
    for _ in 0..final_arity_bits {
        subgroup_x = subgroup_x.square();
    }

    // Final check of FRI. After all the reductions, we check that the final polynomial is equal
    // to the one sent by the prover.
    ensure!(
        proof.final_poly.eval(subgroup_x.into()) == purported_eval,
        "Final polynomial evaluation is invalid."
    );

    Ok(())
}
