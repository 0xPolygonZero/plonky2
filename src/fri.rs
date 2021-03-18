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
