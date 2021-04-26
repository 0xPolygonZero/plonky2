use crate::field::fft::fft;
use crate::field::field::Field;
use crate::hash::hash_n_to_1;
use crate::merkle_proofs::verify_merkle_proof;
use crate::merkle_tree::MerkleTree;
use crate::plonk_challenger::Challenger;
use crate::polynomial::polynomial::{PolynomialCoeffs, PolynomialValues};
use crate::proof::{FriEvaluations, FriMerkleProofs, FriProof, FriQueryRound, Hash};
use crate::util::log2_strict;
use anyhow::{ensure, Result};
use std::intrinsics::rotate_left;
use std::iter::FromIterator;

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

    for &arity_bits in &config.reduction_arity_bits {
        let arity = 1 << arity_bits;
        let beta = challenger.get_challenge();
        // P(x) = sum_{i<r} x^i * P_i(x^r) becomes sum_{i<r} beta^i * P_i(x).
        coeffs = PolynomialCoeffs::new(
            coeffs
                .coeffs
                .chunks_exact(arity)
                .map(|chunk| chunk.iter().rev().fold(F::ZERO, |acc, &c| acc * beta + c))
                .collect::<Vec<_>>(),
        );
        values = fft(coeffs.clone());

        let tree = MerkleTree::new(values.values.iter().map(|&v| vec![v]).collect(), true);
        challenger.observe_hash(&tree.root);
        trees.push(tree);
    }
    (trees, coeffs)
}

fn fri_proof_of_work<F: Field>(current_hash: Hash<F>, config: &FriConfig) -> F {
    (0u64..)
        .find(|&i| {
            hash_n_to_1(
                Vec::from_iter(
                    current_hash
                        .elements
                        .iter()
                        .copied()
                        .chain(Some(F::from_canonical_u64(i))),
                ),
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
        Vec::from_iter(
            challenger
                .get_hash()
                .elements
                .iter()
                .copied()
                .chain(Some(proof.pow_witness)),
        ),
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
                roots_coset_indices[1..]
                    .iter()
                    .map(|&index| tree.get(index)[0])
                    .collect(),
            );
        } else {
            // For the other layers, we don't need to send the evaluation at `x`, since it can
            // be inferred by the verifier. See the `compute_evaluation` function.
            evals.evals.push(
                roots_coset_indices
                    .iter()
                    .map(|&index| tree.get(index)[0])
                    .collect(),
            );
        }
        dbg!(roots_coset_indices
            .into_iter()
            .map(|i| i & ((1 << log2_strict(next_domain_size)) - 1))
            .collect::<Vec<_>>());
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

/// Computes P'(x^2) from {P(x*g^i)}_(i=0..arity), where g is a `arity`-th root of unity and P' is the FRI reduced polynomial.
fn compute_evaluation<F: Field>(x: F, arity_bits: usize, last_evals: Vec<F>, beta: F) -> F {
    let g = F::primitive_root_of_unity(arity_bits);
    let points = g
        .powers()
        .take(1 << arity_bits)
        .map(|y| x * y)
        .collect::<Vec<_>>();
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
    fri_verify_proof_of_work(proof, challenger, config)?;
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
