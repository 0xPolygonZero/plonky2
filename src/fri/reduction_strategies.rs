use std::time::Instant;

use log::debug;

/// A method for deciding what arity to use at each reduction layer.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum FriReductionStrategy {
    /// `ConstantArityBits(arity_bits, final_poly_bits)` applies reductions of arity `2^arity_bits`
    /// until the polynomial degree is `2^final_poly_bits` or less. This tends to work well in the
    /// recursive setting, as it avoids needing multiple configurations of gates used in FRI
    /// verification, such as `InterpolationGate`.
    ConstantArityBits(usize, usize),

    /// Optimize for size.
    MinSize,
}

impl FriReductionStrategy {
    /// The arity of each FRI reduction step, expressed as the log2 of the actual arity.
    pub(crate) fn reduction_arity_bits(
        &self,
        mut degree_bits: usize,
        rate_bits: usize,
        num_queries: usize,
    ) -> Vec<usize> {
        match self {
            &FriReductionStrategy::ConstantArityBits(arity_bits, final_poly_bits) => {
                let mut result = Vec::new();
                while result.is_empty() || degree_bits > final_poly_bits {
                    result.push(arity_bits);
                    assert!(degree_bits >= arity_bits);
                    degree_bits -= arity_bits;
                }
                result.shrink_to_fit();
                result
            }

            &FriReductionStrategy::MinSize => {
                min_size_arity_bits(degree_bits, rate_bits, num_queries)
            }
        }
    }
}

fn min_size_arity_bits(degree_bits: usize, rate_bits: usize, num_queries: usize) -> Vec<usize> {
    let start = Instant::now();
    let (mut arity_bits, fri_proof_size) =
        min_size_arity_bits_helper(degree_bits, rate_bits, num_queries, vec![]);
    arity_bits.shrink_to_fit();

    debug!(
        "min_size_arity_bits took {:.3}s",
        start.elapsed().as_secs_f32()
    );
    debug!(
        "Smallest arity_bits {:?} results in estimated FRI proof size of {} elements",
        arity_bits, fri_proof_size
    );

    arity_bits
}

/// Return `(arity_bits, fri_proof_size)`.
fn min_size_arity_bits_helper(
    degree_bits: usize,
    rate_bits: usize,
    num_queries: usize,
    prefix: Vec<usize>,
) -> (Vec<usize>, usize) {
    const MAX_ARITY_BITS: usize = 4;

    let sum_of_arities: usize = prefix.iter().sum();
    let current_layer_bits = degree_bits + rate_bits - sum_of_arities;
    assert!(current_layer_bits >= rate_bits);

    let mut best_arity_bits = prefix.clone();
    let mut best_size = fri_proof_size(degree_bits, rate_bits, num_queries, &prefix);

    for next_arity_bits in 1..MAX_ARITY_BITS.min(current_layer_bits - rate_bits) {
        let mut extended_prefix = prefix.clone();
        extended_prefix.push(next_arity_bits);

        let (arity_bits, size) =
            min_size_arity_bits_helper(degree_bits, rate_bits, num_queries, extended_prefix);
        if size < best_size {
            best_arity_bits = arity_bits;
            best_size = size;
        }
    }

    (best_arity_bits, best_size)
}

/// Compute the approximate size of a FRI proof with the given reduction arities. The result is
/// measured in field elements.
fn fri_proof_size(
    degree_bits: usize,
    rate_bits: usize,
    num_queries: usize,
    arity_bits: &[usize],
) -> usize {
    const D: usize = 4;

    let mut current_layer_bits = degree_bits + rate_bits;

    let mut total_elems = 0;
    for arity_bits in arity_bits {
        let arity = 1 << arity_bits;

        // Add neighboring evaluations, which are extension field elements.
        total_elems += (arity - 1) * D * num_queries;
        // Add siblings in the Merkle path.
        total_elems += current_layer_bits * 4 * num_queries;

        current_layer_bits -= arity_bits;
    }

    // Add the final polynomial's coefficients.
    assert!(current_layer_bits >= rate_bits);
    let final_poly_len = 1 << (current_layer_bits - rate_bits);
    total_elems += D * final_poly_len;

    total_elems
}
