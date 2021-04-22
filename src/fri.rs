use crate::field::fft::fft;
use crate::field::field::Field;
use crate::hash::hash_n_to_1;
use crate::merkle_proofs::verify_merkle_proof;
use crate::merkle_tree::MerkleTree;
use crate::plonk_challenger::Challenger;
use crate::polynomial::polynomial::{PolynomialCoeffs, PolynomialValues};
use crate::proof::{FriEvaluations, FriMerkleProofs, FriProof, FriQueryRound};
use crate::util::log2_strict;
use anyhow::{ensure, Result};

/// Somewhat arbitrary. Smaller values will increase delta, but with diminishing returns,
/// while increasing L, potentially requiring more challenge points.
const EPSILON: f64 = 0.01;

struct FriConfig {
    proof_of_work_bits: u32,

    /// The arity of each FRI reduction step, expressed (i.e. the log2 of the actual arity).
    /// For example, `[3, 2, 1]` would describe a FRI reduction tree with 8-to-1 reduction, then
    /// a 4-to-1 reduction, then a 2-to-1 reduction. After these reductions, the reduced polynomial
    /// is sent directly.
    reduction_arity_bits: Vec<usize>,

    /// Number of reductions in the FRI protocol. So if the original domain has size `2^n`,
    /// then the final domain will have size `2^(n-reduction_count)`.
    reduction_count: usize,

    /// Number of query rounds to perform.
    num_query_rounds: usize,
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

// TODO: Different arity  + PoW.
/// Builds a FRI proof.
fn fri_proof<F: Field>(
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

    let current_hash = challenger.get_challenge();
    let pow_witness = fri_proof_of_work(current_hash, config);

    // Query phase
    let query_round_proofs = fri_query_rounds(&trees, challenger, n, config);

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
    let mut trees = vec![MerkleTree::new(
        polynomial_values.values.iter().map(|&v| vec![v]).collect(),
        true,
    )];
    let mut coeffs = polynomial_coeffs.clone();
    let mut values;

    challenger.observe_hash(&trees[0].root);

    for _ in 0..config.reduction_count {
        let beta = challenger.get_challenge();
        // P(x) = P_0(x^2) + xP_1(x^2) becomes P_0(x) + beta*P_1(x)
        coeffs = PolynomialCoeffs::new(
            coeffs
                .coeffs
                .chunks_exact(2)
                .map(|chunk| chunk[0] + beta * chunk[1])
                .collect::<Vec<_>>(),
        );
        values = fft(coeffs.clone());

        let tree = MerkleTree::new(values.values.iter().map(|&v| vec![v]).collect(), true);
        challenger.observe_hash(&tree.root);
        trees.push(tree);
    }
    (trees, coeffs)
}

fn fri_proof_of_work<F: Field>(current_hash: F, config: &FriConfig) -> F {
    (0u64..)
        .find(|&i| {
            hash_n_to_1(vec![current_hash, F::from_canonical_u64(i)], false)
                .to_canonical_u64()
                .leading_zeros()
                >= config.proof_of_work_bits
        })
        .map(F::from_canonical_u64)
        .expect("Proof of work failed.")
}

fn fri_query_rounds<F: Field>(
    trees: &[MerkleTree<F>],
    challenger: &mut Challenger<F>,
    n: usize,
    config: &FriConfig,
) -> Vec<FriQueryRound<F>> {
    let mut query_round_proofs = Vec::new();
    for _ in 0..config.num_query_rounds {
        let mut merkle_proofs = FriMerkleProofs { proofs: Vec::new() };
        let mut evals = FriEvaluations {
            first_layer: (F::ZERO, F::ZERO),
            rest: Vec::new(),
        };
        // TODO: Challenger doesn't change between query rounds, so x is always the same.
        // Once PoW is added, this should be fixed.
        let x = challenger.get_challenge();
        let mut domain_size = n;
        let mut x_index = x.to_canonical_u64() as usize;
        for (i, tree) in trees.iter().enumerate() {
            let next_domain_size = domain_size >> 1;
            x_index %= domain_size;
            let minus_x_index = (next_domain_size + x_index) % domain_size;
            if i == 0 {
                // For the first layer, we need to send the evaluation at `x` and `-x`.
                evals.first_layer = (tree.get(x_index)[0], tree.get(minus_x_index)[0]);
            } else {
                // For the other layers, we only need to send the `-x`, the one at `x` can be inferred
                // by the verifier. See the `compute_evaluation` function.
                evals.rest.push(tree.get(minus_x_index)[0]);
            }
            merkle_proofs
                .proofs
                .push((tree.prove(x_index), tree.prove(minus_x_index)));

            domain_size = next_domain_size;
        }
        query_round_proofs.push(FriQueryRound {
            evals,
            merkle_proofs,
        });
    }
    query_round_proofs
}

/// Computes P'(x^2) from P_even(x) and P_odd(x), where P' is the FRI reduced polynomial,
/// P_even is the even coefficients polynomial and P_odd is the odd coefficients polynomial.
fn compute_evaluation<F: Field>(x: F, last_e_x: F, last_e_x_minus: F, beta: F) -> F {
    // P(x) = P_0(x^2) + xP_1(x^2)
    // P'(x^2) = P_0(x^2) + beta*P_1(x^2)
    // P'(x^2) = ((P(x)+P(-x))/2) + beta*((P(x)-P(-x))/(2x)
    (last_e_x + last_e_x_minus) / F::TWO + beta * (last_e_x - last_e_x_minus) / (F::TWO * x)
}

fn verify_fri_proof<F: Field>(
    proof: &FriProof<F>,
    challenger: &mut Challenger<F>,
    config: &FriConfig,
) -> Result<()> {
    // Size of the LDE domain.
    let n = proof.final_poly.len() << config.reduction_count;

    // Recover the random betas used in the FRI reductions.
    let betas = proof.commit_phase_merkle_roots[..proof.commit_phase_merkle_roots.len() - 1]
        .iter()
        .map(|root| {
            challenger.observe_hash(root);
            challenger.get_challenge()
        })
        .collect::<Vec<_>>();
    challenger.observe_hash(proof.commit_phase_merkle_roots.last().unwrap());

    // Check PoW.
    ensure!(
        hash_n_to_1(vec![challenger.get_challenge(), proof.pow_witness], false)
            .to_canonical_u64()
            .leading_zeros()
            >= config.proof_of_work_bits,
        "Invalid proof of work witness."
    );

    // Check that parameters are coherent.
    ensure!(
        config.num_query_rounds == proof.query_round_proofs.len(),
        "Number of query rounds does not match config."
    );
    ensure!(
        config.reduction_count > 0,
        "Number of reductions should be non-zero."
    );

    for round in 0..config.num_query_rounds {
        let round_proof = &proof.query_round_proofs[round];
        let mut e_xs = Vec::new();
        let x = challenger.get_challenge();
        let mut domain_size = n;
        let mut x_index = x.to_canonical_u64() as usize;
        // `subgroup_x` is `subgroup[x_index]`, i.e., the actual field element in the domain.
        let mut subgroup_x = F::primitive_root_of_unity(log2_strict(n)).exp_usize(x_index % n);
        for i in 0..config.reduction_count {
            x_index %= domain_size;
            let next_domain_size = domain_size >> 1;
            let minus_x_index = (next_domain_size + x_index) % domain_size;
            let (e_x, e_x_minus, merkle_proof, merkle_proof_minus) = if i == 0 {
                let (e_x, e_x_minus) = round_proof.evals.first_layer;
                let (merkle_proof, merkle_proof_minus) = &round_proof.merkle_proofs.proofs[i];
                e_xs.push((e_x, e_x_minus));
                (e_x, e_x_minus, merkle_proof, merkle_proof_minus)
            } else {
                let (last_e_x, last_e_x_minus) = e_xs[i - 1];
                let e_x = compute_evaluation(subgroup_x, last_e_x, last_e_x_minus, betas[i - 1]);
                let e_x_minus = round_proof.evals.rest[i - 1];
                let (merkle_proof, merkle_proof_minus) = &round_proof.merkle_proofs.proofs[i];
                e_xs.push((e_x, e_x_minus));
                (e_x, e_x_minus, merkle_proof, merkle_proof_minus)
            };
            verify_merkle_proof(
                vec![e_x],
                x_index,
                proof.commit_phase_merkle_roots[i],
                merkle_proof,
                true,
            )?;
            verify_merkle_proof(
                vec![e_x_minus],
                minus_x_index,
                proof.commit_phase_merkle_roots[i],
                merkle_proof_minus,
                true,
            )?;
            if i > 0 {
                subgroup_x = subgroup_x.square();
            }
            domain_size = next_domain_size;
        }
        let (last_e_x, last_e_x_minus) = e_xs[config.reduction_count - 1];
        let purported_eval = compute_evaluation(
            subgroup_x,
            last_e_x,
            last_e_x_minus,
            betas[config.reduction_count - 1],
        );
        // Final check of FRI. After all the reductions, we check that the final polynomial is equal
        // to the one sent by the prover.
        ensure!(
            proof.final_poly.eval(subgroup_x.square()) == purported_eval,
            "Final polynomial evaluation is invalid."
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::crandall_field::CrandallField;
    use crate::field::fft::ifft;
    use anyhow::Result;

    fn test_fri(
        degree: usize,
        rate_bits: usize,
        reduction_count: usize,
        num_query_rounds: usize,
    ) -> Result<()> {
        type F = CrandallField;

        let n = degree;
        let evals = PolynomialValues::new((0..n).map(|_| F::rand()).collect());
        let lde = evals.clone().lde(rate_bits);
        let config = FriConfig {
            reduction_count,
            num_query_rounds,
            proof_of_work_bits: 2,
            reduction_arity_bits: Vec::new(),
        };
        let mut challenger = Challenger::new();
        let proof = fri_proof(&ifft(lde.clone()), &lde, &mut challenger, &config);

        let mut challenger = Challenger::new();
        verify_fri_proof(&proof, &mut challenger, &config)?;

        Ok(())
    }

    #[test]
    fn test_fri_multi_params() -> Result<()> {
        for degree_log in 1..6 {
            for rate_bits in 0..4 {
                for reduction_count in 1..=(degree_log + rate_bits) {
                    for num_query_round in 0..4 {
                        test_fri(1 << degree_log, rate_bits, reduction_count, num_query_round)?;
                    }
                }
            }
        }
        Ok(())
    }
}
