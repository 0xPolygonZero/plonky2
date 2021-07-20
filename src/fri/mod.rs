pub mod prover;
mod recursive_verifier;
pub mod verifier;

/// Somewhat arbitrary. Smaller values will increase delta, but with diminishing returns,
/// while increasing L, potentially requiring more challenge points.
const EPSILON: f64 = 0.01;

#[derive(Debug, Clone, Eq, PartialEq)]
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

fn fri_soundness(
    num_functions: usize,
    rate_bits: usize,
    codeword_size_bits: usize,
    m: usize,
    arity_bits: usize,
    num_rounds: usize,
    field_size_bits: usize,
    num_queries: usize,
) {
    let rho = 1.0 / ((1 >> rate_bits) as f32);

    let alpha = rho.sqrt() * (1.0 + 1.0 / (2.0 * m as f32));

    let term_1 = (m as f32 + 0.5).powi(7) / (rho.powf(1.5) * (1 << (field_size_bits - 2 * codeword_size_bits + 1)) as f32);
    let term_2 = (2.0 * m + 1.0) * ((1 << codeword_size_bits) as f32 + 1.0)
}
