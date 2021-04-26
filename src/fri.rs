use crate::field::fft::fft;
use crate::field::field::Field;
use crate::field::lagrange::{barycentric_weights, interpolate};
use crate::hash::hash_n_to_1;
use crate::merkle_proofs::verify_merkle_proof_subtree;
use crate::merkle_tree::MerkleTree;
use crate::plonk_challenger::Challenger;
use crate::polynomial::polynomial::{PolynomialCoeffs, PolynomialValues};
use crate::proof::{FriEvaluations, FriMerkleProofs, FriProof, FriQueryRound, Hash};
use crate::util::{log2_strict, reverse_bits};
use anyhow::{ensure, Result};

/// Somewhat arbitrary. Smaller values will increase delta, but with diminishing returns,
/// while increasing L, potentially requiring more challenge points.
const EPSILON: f64 = 0.01;

struct FriConfig {
    proof_of_work_bits: u32,

    rate_bits: usize,

    /// The arity of each FRI reduction step, expressed (i.e. the log2 of the actual arity).
    /// For example, `[3, 2, 1]` would describe a FRI reduction tree with 8-to-1 reduction, then
    /// a 4-to-1 reduction, then a 2-to-1 reduction. After these reductions, the reduced polynomial
    /// is sent directly.
    reduction_arity_bits: Vec<usize>,

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

    let current_hash = challenger.get_hash();
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

    let num_reductions = config.reduction_arity_bits.len();
    for i in 0..num_reductions {
        let arity = 1 << config.reduction_arity_bits[i];
        let beta = challenger.get_challenge();
        // P(x) = sum_{i<r} x^i * P_i(x^r) becomes sum_{i<r} beta^i * P_i(x).
        coeffs = PolynomialCoeffs::new(
            coeffs
                .coeffs
                .chunks_exact(arity)
                .map(|chunk| chunk.iter().rev().fold(F::ZERO, |acc, &c| acc * beta + c))
                .collect::<Vec<_>>(),
        );
        if i == num_reductions - 1 {
            break;
        }
        values = fft(coeffs.clone());

        let tree = MerkleTree::new(values.values.iter().map(|&v| vec![v]).collect(), true);
        challenger.observe_hash(&tree.root);
        trees.push(tree);
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

fn fri_query_rounds<F: Field>(
    trees: &[MerkleTree<F>],
    challenger: &mut Challenger<F>,
    n: usize,
    config: &FriConfig,
) -> Vec<FriQueryRound<F>> {
    let mut query_round_proofs = Vec::new();
    for _ in 0..config.num_query_rounds {
        fri_query_round(trees, challenger, n, &mut query_round_proofs, config);
    }
    query_round_proofs
}

/// Returns the indices of all `y` in `F` with `y^arity=x^arity`, starting with `x` itself.
fn index_roots_coset(
    x_index: usize,
    next_domain_size: usize,
    domain_size: usize,
    arity: usize,
) -> Vec<usize> {
    (0..arity)
        .map(|i| (i * next_domain_size + x_index) % domain_size)
        .collect()
}

fn fri_query_round<F: Field>(
    trees: &[MerkleTree<F>],
    challenger: &mut Challenger<F>,
    n: usize,
    query_round_proofs: &mut Vec<FriQueryRound<F>>,
    config: &FriConfig,
) {
    let mut merkle_proofs = FriMerkleProofs { proofs: Vec::new() };
    let mut evals = FriEvaluations { evals: Vec::new() };
    // TODO: Challenger doesn't change between query rounds, so x is always the same.
    let x = challenger.get_challenge();
    let mut domain_size = n;
    let mut x_index = x.to_canonical_u64() as usize;
    for (i, tree) in trees.iter().enumerate() {
        let arity_bits = config.reduction_arity_bits[i];
        let arity = 1 << arity_bits;
        let next_domain_size = domain_size >> arity_bits;
        x_index %= domain_size;
        let roots_coset_indices = index_roots_coset(x_index, next_domain_size, domain_size, arity);
        if i == 0 {
            // For the first layer, we need to send the evaluation at `x` too.
            evals.evals.push(
                roots_coset_indices
                    .iter()
                    .map(|&index| tree.get(index)[0])
                    .collect(),
            );
        } else {
            // For the other layers, we don't need to send the evaluation at `x`, since it can
            // be inferred by the verifier. See the `compute_evaluation` function.
            evals.evals.push(
                roots_coset_indices[1..]
                    .iter()
                    .map(|&index| tree.get(index)[0])
                    .collect(),
            );
        }
        merkle_proofs.proofs.push(tree.prove_subtree(
            x_index & ((1 << log2_strict(next_domain_size)) - 1),
            arity_bits,
        ));

        domain_size = next_domain_size;
    }
    query_round_proofs.push(FriQueryRound {
        evals,
        merkle_proofs,
    });
}

/// Computes P'(x^arity) from {P(x*g^i)}_(i=0..arity), where g is a `arity`-th root of unity and P' is the FRI reduced polynomial.
fn compute_evaluation<F: Field>(x: F, arity_bits: usize, last_evals: &[F], beta: F) -> F {
    let g = F::primitive_root_of_unity(arity_bits);
    let points = g
        .powers()
        .zip(last_evals)
        .take(1 << arity_bits)
        .map(|(y, &e)| (x * y, e))
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
    // let betas = proof.commit_phase_merkle_roots[..proof.commit_phase_merkle_roots.len() - 1]
    let betas = proof
        .commit_phase_merkle_roots
        .iter()
        .map(|root| {
            challenger.observe_hash(root);
            challenger.get_challenge()
        })
        .collect::<Vec<_>>();
    // challenger.observe_hash(proof.commit_phase_merkle_roots.last().unwrap());
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

    for round in 0..config.num_query_rounds {
        let round_proof = &proof.query_round_proofs[round];
        let mut e_xs = Vec::new();
        let x = challenger.get_challenge();
        let mut domain_size = n;
        let mut x_index = x.to_canonical_u64() as usize;
        // `subgroup_x` is `subgroup[x_index]`, i.e., the actual field element in the domain.
        let mut subgroup_x = F::primitive_root_of_unity(log2_strict(n)).exp_usize(x_index % n);
        for (i, &arity_bits) in config.reduction_arity_bits.iter().enumerate() {
            let arity = 1 << arity_bits;
            x_index %= domain_size;
            let next_domain_size = domain_size >> arity_bits;
            let roots_coset_indices =
                index_roots_coset(x_index, next_domain_size, domain_size, arity);
            if i == 0 {
                let evals = round_proof.evals.evals[0].clone();
                e_xs.push(evals);
            } else {
                let last_evals = &e_xs[i - 1];
                let e_x = compute_evaluation(
                    subgroup_x,
                    config.reduction_arity_bits[i - 1],
                    last_evals,
                    betas[i - 1],
                );
                let mut evals = round_proof.evals.evals[i].clone();
                evals.insert(0, e_x);
                e_xs.push(evals);
            };
            let sorted_evals = {
                let mut sorted_evals_enumerate = e_xs[i].iter().enumerate().collect::<Vec<_>>();
                sorted_evals_enumerate.sort_by_key(|&(j, _)| {
                    reverse_bits(roots_coset_indices[j], log2_strict(domain_size))
                });
                sorted_evals_enumerate
                    .into_iter()
                    .map(|(_, &e)| vec![e])
                    .collect()
            };
            verify_merkle_proof_subtree(
                sorted_evals,
                x_index & ((1 << log2_strict(next_domain_size)) - 1),
                proof.commit_phase_merkle_roots[i],
                &round_proof.merkle_proofs.proofs[i],
                true,
            )?;
            if i > 0 {
                for _ in 0..config.reduction_arity_bits[i - 1] {
                    subgroup_x = subgroup_x.square();
                }
            }
            domain_size = next_domain_size;
        }
        let last_evals = e_xs.last().unwrap();
        let final_arity_bits = *config.reduction_arity_bits.last().unwrap();
        let purported_eval = compute_evaluation(
            subgroup_x,
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
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::crandall_field::CrandallField;
    use crate::field::fft::ifft;
    use anyhow::Result;
    use rand::Rng;

    fn test_fri(
        degree_log: usize,
        rate_bits: usize,
        reduction_arity_bits: Vec<usize>,
        num_query_rounds: usize,
    ) -> Result<()> {
        type F = CrandallField;

        let n = 1 << degree_log;
        let evals = PolynomialValues::new((0..n).map(|_| F::rand()).collect());
        let lde = evals.clone().lde(rate_bits);
        let config = FriConfig {
            num_query_rounds,
            rate_bits,
            proof_of_work_bits: 2,
            reduction_arity_bits,
        };
        let mut challenger = Challenger::new();
        let proof = fri_proof(&ifft(lde.clone()), &lde, &mut challenger, &config);

        let mut challenger = Challenger::new();
        verify_fri_proof(degree_log, &proof, &mut challenger, &config)?;

        Ok(())
    }

    fn gen_arities(degree_log: usize) -> Vec<usize> {
        let mut rng = rand::thread_rng();
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
        for degree_log in 1..6 {
            for rate_bits in 0..4 {
                for num_query_round in 0..4 {
                    test_fri(
                        degree_log,
                        rate_bits,
                        gen_arities(degree_log),
                        num_query_round,
                    )?;
                }
            }
        }
        Ok(())
    }
}
