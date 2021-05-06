pub mod prover;
pub mod verifier;

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

    /// True if the last element of the Merkle trees' leaf vectors is a blinding element.
    pub blinding: bool,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::crandall_field::CrandallField;
    use crate::field::fft::ifft;
    use crate::field::field::Field;
    use crate::fri::prover::fri_proof;
    use crate::fri::verifier::verify_fri_proof;
    use crate::merkle_tree::MerkleTree;
    use crate::plonk_challenger::Challenger;
    use crate::polynomial::polynomial::PolynomialCoeffs;
    use crate::util::reverse_index_bits_in_place;
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
        let coeffs = PolynomialCoeffs::new(F::rand_vec(n)).lde(rate_bits);
        let coset_lde = coeffs.clone().coset_fft(F::MULTIPLICATIVE_GROUP_GENERATOR);
        let config = FriConfig {
            num_query_rounds,
            rate_bits,
            proof_of_work_bits: 2,
            reduction_arity_bits,
            blinding: false,
        };
        let tree = {
            let mut leaves = coset_lde
                .values
                .iter()
                .map(|&x| vec![x])
                .collect::<Vec<_>>();
            reverse_index_bits_in_place(&mut leaves);
            MerkleTree::new(leaves, false)
        };
        let root = tree.root;
        let mut challenger = Challenger::new();
        let proof = fri_proof(&[tree], &coeffs, &coset_lde, &mut challenger, &config);

        let mut challenger = Challenger::new();
        verify_fri_proof(
            degree_log,
            &[],
            F::ONE,
            &[root],
            &proof,
            &mut challenger,
            &config,
        )?;

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
