use crate::field::field::Field;
use crate::hash::{compress, hash_n_to_hash};
use crate::plonk_challenger::Challenger;
use crate::polynomial::polynomial::{PolynomialCoeffs, PolynomialValues};
use crate::proof::{Hash, FriProof};
use crate::field::fft::fft;
use crate::gadgets::merkle_proofs::MerkleTree;

/// Somewhat arbitrary. Smaller values will increase delta, but with diminishing returns,
/// while increasing L, potentially requiring more challenge points.
const EPSILON: f64 = 0.01;

struct FriConfig {
    proof_of_work_bits: usize,

    /// The arity of each FRI reduction step, expressed (i.e. the log2 of the actual arity).
    /// For example, `[3, 2, 1]` would describe a FRI reduction tree with 8-to-1 reduction, then
    /// a 4-to-1 reduction, then a 2-to-1 reduction. After these reductions, the reduced polynomial
    /// is sent directly.
    reduction_arity_bits: Vec<usize>,

    /// Number of reductions in the FRI protocol. So if the original domain has size `2^n`,
    /// then the final domain will have size `2^(n-reduction_count)`.
    reduction_count: usize,

    rate_bits: usize,
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
/// Performs a FRI round.
fn fri_round<F: Field>(
    polynomial_coeffs: &PolynomialCoeffs<F>,
    polynomial_values: &PolynomialValues<F>,
    challenger: &mut Challenger<F>,
    config: &FriConfig,
) -> FriProof<F> {
    let n = polynomial_values.values.len();
    assert_eq!(
        polynomial_coeffs.coeffs.len(),
        n
    );
    let mut trees = vec![MerkleTree::new(polynomial_values.values.iter().map(|&v| vec![v]).collect())];
    let mut root = trees.last().unwrap().root;
    let mut coeffs = polynomial_coeffs.clone();
    let mut values;

    challenger.observe_hash(&root);

    // Commit phase
    for _ in 0..config.reduction_count {
        let beta = challenger.get_challenge();
        coeffs = PolynomialCoeffs::new(
            coeffs
                .coeffs
                .chunks_exact(2)
                .map(|chunk| chunk[0] + beta * chunk[1])
                .collect::<Vec<_>>(),
        );
        values = fft(coeffs.clone().lde(config.rate_bits));

        let tree = MerkleTree::new(values.values.iter().map(|&v| vec![v]).collect());
        challenger.observe_hash(&tree.root);
        trees.push(tree);
    }

    // Query phase
    let mut merkle_proofs = Vec::new();
    let mut evals = Vec::new();
    for i in 0..config.reduction_count {
        let x = challenger.get_challenge();
        let x_index = (x.to_canonical_u64() as usize) % n;
        let n2 = n>>1;
        evals.extend(std::array::IntoIter::new([polynomial_values.values[x_index], polynomial_values.values[n2 + x_index]]));
        merkle_proofs.extend(std::array::IntoIter::new([trees[i].prove(x_index), trees[i].prove(n2 + x_index)]));
    }

    FriProof {
        commit_phase_merkle_roots: trees.iter().map(|t| t.root).collect(),
        initial_merkle_proofs: vec![],
        intermediate_merkle_proofs: merkle_proofs,
        final_poly: coeffs
    }

}

