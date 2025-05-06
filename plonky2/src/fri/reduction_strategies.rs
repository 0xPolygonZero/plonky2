#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};

use log::debug;
use serde::Serialize;
#[cfg(feature = "timing")]
use web_time::Instant;

use crate::hash::hash_types::RichField;

/// A method for deciding what arity to use at each reduction layer.
#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub enum FriReductionStrategy {
    /// Specifies the exact sequence of arities (expressed in bits) to use.
    Fixed(Vec<usize>),

    /// `ConstantArityBits(arity_bits, final_poly_bits)` applies reductions of arity `2^arity_bits`
    /// until the polynomial degree is less than or equal to `2^final_poly_bits` or until any further
    /// `arity_bits`-reduction makes the last FRI tree have height less than `cap_height`.
    /// This tends to work well in the recursive setting, as it avoids needing multiple configurations
    /// of gates used in FRI verification, such as `InterpolationGate`.
    ConstantArityBits(usize, usize),

    /// `MinSize(opt_max_arity_bits)` searches for an optimal sequence of reduction arities, with an
    /// optional max `arity_bits`. If this proof will have recursive proofs on top of it, a max
    /// `arity_bits` of 3 is recommended.
    MinSize(Option<usize>),
}

impl FriReductionStrategy {
    /// The arity of each FRI reduction step, expressed as the log2 of the actual arity.
    pub fn reduction_arity_bits(
        &self,
        mut degree_bits: usize,
        rate_bits: usize,
        cap_height: usize,
        num_queries: usize,
    ) -> Vec<usize> {
        match self {
            FriReductionStrategy::Fixed(reduction_arity_bits) => reduction_arity_bits.to_vec(),
            &FriReductionStrategy::ConstantArityBits(arity_bits, final_poly_bits) => {
                let mut result = Vec::new();
                while degree_bits > final_poly_bits
                    && degree_bits + rate_bits - arity_bits >= cap_height
                {
                    result.push(arity_bits);
                    assert!(degree_bits >= arity_bits);
                    degree_bits -= arity_bits;
                }
                result.shrink_to_fit();
                result
            }
            FriReductionStrategy::MinSize(opt_max_arity_bits) => {
                min_size_arity_bits(degree_bits, rate_bits, num_queries, *opt_max_arity_bits)
            }
        }
    }

    pub fn serialize<F: RichField>(&self) -> Vec<F> {
        match self {
            FriReductionStrategy::Fixed(reduction_arity_bits) => core::iter::once(F::ZERO)
                .chain(
                    reduction_arity_bits
                        .iter()
                        .map(|&x| F::from_canonical_usize(x)),
                )
                .collect(),
            FriReductionStrategy::ConstantArityBits(arity_bits, final_poly_bits) => {
                vec![
                    F::ONE,
                    F::from_canonical_usize(*arity_bits),
                    F::from_canonical_usize(*final_poly_bits),
                ]
            }
            FriReductionStrategy::MinSize(opt_max_arity_bits) => {
                let max_arity = opt_max_arity_bits.unwrap_or(0);
                vec![F::TWO, F::from_canonical_usize(max_arity)]
            }
        }
    }
}

fn min_size_arity_bits(
    degree_bits: usize,
    rate_bits: usize,
    num_queries: usize,
    opt_max_arity_bits: Option<usize>,
) -> Vec<usize> {
    // 2^4 is the largest arity we see in optimal reduction sequences in practice. For 2^5 to occur
    // in an optimal sequence, we would need a really massive polynomial.
    let max_arity_bits = opt_max_arity_bits.unwrap_or(4);

    #[cfg(feature = "timing")]
    let start = Instant::now();
    let (mut arity_bits, fri_proof_size) =
        min_size_arity_bits_helper(degree_bits, rate_bits, num_queries, max_arity_bits, vec![]);
    arity_bits.shrink_to_fit();

    #[cfg(feature = "timing")]
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
    global_max_arity_bits: usize,
    prefix: Vec<usize>,
) -> (Vec<usize>, usize) {
    let sum_of_arities: usize = prefix.iter().sum();
    let current_layer_bits = degree_bits + rate_bits - sum_of_arities;
    assert!(current_layer_bits >= rate_bits);

    let mut best_arity_bits = prefix.clone();
    let mut best_size = relative_proof_size(degree_bits, rate_bits, num_queries, &prefix);

    // The largest next_arity_bits to search. Note that any optimal arity sequence will be
    // monotonically non-increasing, as a larger arity will shrink more Merkle proofs if it occurs
    // earlier in the sequence.
    let max_arity_bits = prefix
        .last()
        .copied()
        .unwrap_or(global_max_arity_bits)
        .min(current_layer_bits - rate_bits);

    for next_arity_bits in 1..=max_arity_bits {
        let mut extended_prefix = prefix.clone();
        extended_prefix.push(next_arity_bits);

        let (arity_bits, size) = min_size_arity_bits_helper(
            degree_bits,
            rate_bits,
            num_queries,
            max_arity_bits,
            extended_prefix,
        );
        if size < best_size {
            best_arity_bits = arity_bits;
            best_size = size;
        }
    }

    (best_arity_bits, best_size)
}

/// Compute the approximate size of a FRI proof with the given reduction arities. Note that this
/// ignores initial evaluations, which aren't affected by arities, and some other minor
/// contributions. The result is measured in field elements.
fn relative_proof_size(
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
