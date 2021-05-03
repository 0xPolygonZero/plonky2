use crate::field::fft::fft;
use crate::field::field::Field;
use crate::field::lagrange::{barycentric_weights, interpolate};
use crate::hash::hash_n_to_1;
use crate::merkle_proofs::verify_merkle_proof;
use crate::merkle_tree::MerkleTree;
use crate::plonk_challenger::Challenger;
use crate::plonk_common::reduce_with_powers;
use crate::polynomial::polynomial::{PolynomialCoeffs, PolynomialValues};
use crate::proof::{FriProof, FriQueryRound, FriQueryStep, Hash};
use crate::util::{log2_strict, reverse_bits, reverse_index_bits_in_place};
use anyhow::{ensure, Result};

/// Somewhat arbitrary. Smaller values will increase delta, but with diminishing returns,
/// while increasing L, potentially requiring more challenge points.
const EPSILON: f64 = 0.01;

#[derive(Debug, Clone)]
pub struct FriConfig {
    pub proof_of_work_bits: u32,

    pub rate_bits: usize,

    /// The arity of each FRI reduction step, expressed (i.e. the log2 of the actual arity).
    /// For example, `[3, 2, 1]` would describe a FRI reduction tree with 8-to-1 reduction, then
    /// a 4-to-1 reduction, then a 2-to-1 reduction. After these reductions, the reduced polynomial
    /// is sent directly.
    pub reduction_arity_bits: Vec<usize>,

    /// Number of query rounds to perform.
    pub num_query_rounds: usize,
}

fn fri_delta(rate_log: usize, conjecture: bool) -> f64 {
    let rate = (1 << rate_log) as f64;
    if conjecture {
        // See Conjecture 2.3 in DEEP-FRI.
        1.0 - rate - EPSILON
    } else {
        // See the Johnson radius.
        1.0 - rate.sqrt() - EPSILON
    }
}

fn fri_l(codeword_len: usize, rate_log: usize, conjecture: bool) -> f64 {
    let rate = (1 << rate_log) as f64;
    if conjecture {
        // See Conjecture 2.3 in DEEP-FRI.
        // We assume the conjecture holds with a constant of 1 (as do other STARK implementations).
        (codeword_len as f64) / EPSILON
    } else {
        // See the Johnson bound.
        1.0 / (2.0 * EPSILON * rate.sqrt())
    }
}

/// Builds a FRI proof.
pub fn fri_proof<F: Field>(
    // Coefficients of the polynomial on which the LDT is performed.
    // Only the first `1/rate` coefficients are non-zero.
    polynomial_coeffs: &PolynomialCoeffs<F>,
    // Evaluation of the polynomial on the large domain.
    polynomial_values: &PolynomialValues<F>,
    challenger: &mut Challenger<F>,
    config: &FriConfig,
) -> FriProof<F> {
    let n = polynomial_values.values.len();
    assert_eq!(polynomial_coeffs.coeffs.len(), n);

    // Commit phase
    let (trees, final_coeffs) =
        fri_committed_trees(polynomial_coeffs, polynomial_values, challenger, config);

    // PoW phase
    let current_hash = challenger.get_hash();
    let pow_witness = fri_proof_of_work(current_hash, config);

    // Query phase
    let query_round_proofs = fri_prover_query_rounds(&trees, challenger, n, config);

    FriProof {
        commit_phase_merkle_roots: trees.iter().map(|t| t.root).collect(),
        // TODO: Fix this
        initial_merkle_proofs: vec![],
        query_round_proofs,
        final_poly: final_coeffs,
        pow_witness,
    }
}

fn fri_committed_trees<F: Field>(
    polynomial_coeffs: &PolynomialCoeffs<F>,
    polynomial_values: &PolynomialValues<F>,
    challenger: &mut Challenger<F>,
    config: &FriConfig,
) -> (Vec<MerkleTree<F>>, PolynomialCoeffs<F>) {
    let mut values = polynomial_values.clone();
    let mut coeffs = polynomial_coeffs.clone();

    let mut trees = Vec::new();

    let mut shift = F::MULTIPLICATIVE_GROUP_GENERATOR;
    let num_reductions = config.reduction_arity_bits.len();
    for i in 0..num_reductions {
        let arity = 1 << config.reduction_arity_bits[i];

        reverse_index_bits_in_place(&mut values.values);
        let tree = MerkleTree::new(
            values
                .values
                .chunks(arity)
                .map(|chunk| chunk.to_vec())
                .collect(),
            false,
        );

        challenger.observe_hash(&tree.root);
        trees.push(tree);

        let beta = challenger.get_challenge();
        // P(x) = sum_{i<r} x^i * P_i(x^r) becomes sum_{i<r} beta^i * P_i(x).
        coeffs = PolynomialCoeffs::new(
            coeffs
                .coeffs
                .chunks_exact(arity)
                .map(|chunk| reduce_with_powers(chunk, beta))
                .collect::<Vec<_>>(),
        );
        shift = shift.exp_u32(arity as u32);
        // TODO: Is it faster to interpolate?
        values = coeffs.clone().coset_fft(shift);
    }

    challenger.observe_elements(&coeffs.coeffs);
    (trees, coeffs)
}

fn fri_proof_of_work<F: Field>(current_hash: Hash<F>, config: &FriConfig) -> F {
    (0u64..)
        .find(|&i| {
            hash_n_to_1(
                current_hash
                    .elements
                    .iter()
                    .copied()
                    .chain(Some(F::from_canonical_u64(i)))
                    .collect(),
                false,
            )
            .to_canonical_u64()
            .leading_zeros()
                >= config.proof_of_work_bits
        })
        .map(F::from_canonical_u64)
        .expect("Proof of work failed.")
}

fn fri_verify_proof_of_work<F: Field>(
    proof: &FriProof<F>,
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
            >= config.proof_of_work_bits + F::ORDER.leading_zeros(),
        "Invalid proof of work witness."
    );

    Ok(())
}

fn fri_prover_query_rounds<F: Field>(
    trees: &[MerkleTree<F>],
    challenger: &mut Challenger<F>,
    n: usize,
    config: &FriConfig,
) -> Vec<FriQueryRound<F>> {
    (0..config.num_query_rounds)
        .map(|_| fri_prover_query_round(trees, challenger, n, config))
        .collect()
}

fn fri_prover_query_round<F: Field>(
    trees: &[MerkleTree<F>],
    challenger: &mut Challenger<F>,
    n: usize,
    config: &FriConfig,
) -> FriQueryRound<F> {
    let mut query_steps = Vec::new();
    // TODO: Challenger doesn't change between query rounds, so x is always the same.
    let x = challenger.get_challenge();
    let mut domain_size = n;
    let mut x_index = x.to_canonical_u64() as usize % n;
    for (i, tree) in trees.iter().enumerate() {
        let arity_bits = config.reduction_arity_bits[i];
        let arity = 1 << arity_bits;
        let next_domain_size = domain_size >> arity_bits;
        let evals = if i == 0 {
            // For the first layer, we need to send the evaluation at `x` too.
            tree.get(x_index >> arity_bits).to_vec()
        } else {
            // For the other layers, we don't need to send the evaluation at `x`, since it can
            // be inferred by the verifier. See the `compute_evaluation` function.
            let mut evals = tree.get(x_index >> arity_bits).to_vec();
            evals.remove(x_index & (arity - 1));
            evals
        };
        let merkle_proof = tree.prove(x_index >> arity_bits);

        query_steps.push(FriQueryStep {
            evals,
            merkle_proof,
        });

        domain_size = next_domain_size;
        x_index >>= arity_bits;
    }
    FriQueryRound { steps: query_steps }
}

/// Computes P'(x^arity) from {P(x*g^i)}_(i=0..arity), where g is a `arity`-th root of unity
/// and P' is the FRI reduced polynomial.
fn compute_evaluation<F: Field>(
    x: F,
    old_x_index: usize,
    arity_bits: usize,
    last_evals: &[F],
    beta: F,
) -> F {
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
        .map(|(y, e)| (x * y, e))
        .collect::<Vec<_>>();
    let barycentric_weights = barycentric_weights(&points);
    interpolate(&points, beta, &barycentric_weights)
}

fn verify_fri_proof<F: Field>(
    purported_degree_log: usize,
    proof: &FriProof<F>,
    challenger: &mut Challenger<F>,
    config: &FriConfig,
) -> Result<()> {
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
            challenger.get_challenge()
        })
        .collect::<Vec<_>>();
    challenger.observe_elements(&proof.final_poly.coeffs);

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

    for round_proof in &proof.query_round_proofs {
        fri_verifier_query_round(&proof, challenger, n, &betas, round_proof, config)?;
    }

    Ok(())
}

fn fri_verifier_query_round<F: Field>(
    proof: &FriProof<F>,
    challenger: &mut Challenger<F>,
    n: usize,
    betas: &[F],
    round_proof: &FriQueryRound<F>,
    config: &FriConfig,
) -> Result<()> {
    let mut evaluations = Vec::new();
    let x = challenger.get_challenge();
    let mut domain_size = n;
    let mut x_index = x.to_canonical_u64() as usize % n;
    let mut old_x_index = 0;
    // `subgroup_x` is `subgroup[x_index]`, i.e., the actual field element in the domain.
    let log_n = log2_strict(n);
    let mut subgroup_x = F::MULTIPLICATIVE_GROUP_GENERATOR
        * F::primitive_root_of_unity(log_n).exp_usize(reverse_bits(x_index, log_n));
    for (i, &arity_bits) in config.reduction_arity_bits.iter().enumerate() {
        let arity = 1 << arity_bits;
        let next_domain_size = domain_size >> arity_bits;
        if i == 0 {
            let evals = round_proof.steps[0].evals.clone();
            evaluations.push(evals);
        } else {
            let last_evals = &evaluations[i - 1];
            // Infer P(y) from {P(x)}_{x^arity=y}.
            let e_x = compute_evaluation(
                subgroup_x,
                old_x_index,
                config.reduction_arity_bits[i - 1],
                last_evals,
                betas[i - 1],
            );
            let mut evals = round_proof.steps[i].evals.clone();
            // Insert P(y) into the evaluation vector, since it wasn't included by the prover.
            evals.insert(x_index & (arity - 1), e_x);
            evaluations.push(evals);
        };
        verify_merkle_proof(
            evaluations[i].clone(),
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
        proof.final_poly.eval(subgroup_x) == purported_eval,
        "Final polynomial evaluation is invalid."
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::crandall_field::CrandallField;
    use crate::field::fft::ifft;
    use anyhow::Result;
    use rand::rngs::ThreadRng;
    use rand::Rng;

    fn test_fri(
        degree_log: usize,
        rate_bits: usize,
        reduction_arity_bits: Vec<usize>,
        num_query_rounds: usize,
    ) -> Result<()> {
        type F = CrandallField;

        let n = 1 << degree_log;
        let coeffs = PolynomialCoeffs::new((0..n).map(|_| F::rand()).collect()).lde(rate_bits);
        let coset_lde = coeffs.clone().coset_fft(F::MULTIPLICATIVE_GROUP_GENERATOR);
        let config = FriConfig {
            num_query_rounds,
            rate_bits,
            proof_of_work_bits: 2,
            reduction_arity_bits,
        };
        let mut challenger = Challenger::new();
        let proof = fri_proof(&coeffs, &coset_lde, &mut challenger, &config);

        let mut challenger = Challenger::new();
        verify_fri_proof(degree_log, &proof, &mut challenger, &config)?;

        Ok(())
    }

    fn gen_arities(degree_log: usize, rng: &mut ThreadRng) -> Vec<usize> {
        let mut arities = Vec::new();
        let mut remaining = degree_log;
        while remaining > 0 {
            let arity = rng.gen_range(0, remaining + 1);
            arities.push(arity);
            remaining -= arity;
        }
        arities
    }

    #[test]
    fn test_fri_multi_params() -> Result<()> {
        let mut rng = rand::thread_rng();
        for degree_log in 1..6 {
            for rate_bits in 0..3 {
                for num_query_round in 0..4 {
                    for _ in 0..3 {
                        test_fri(
                            degree_log,
                            rate_bits,
                            gen_arities(degree_log, &mut rng),
                            num_query_round,
                        )?;
                    }
                }
            }
        }
        Ok(())
    }
}
