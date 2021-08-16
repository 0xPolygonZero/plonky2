pub mod commitment;
pub mod proof;
pub mod prover;
pub mod recursive_verifier;
pub mod verifier;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FriConfig {
    pub proof_of_work_bits: u32,

    /// The arity of each FRI reduction step, expressed (i.e. the log2 of the actual arity).
    /// For example, `[3, 2, 1]` would describe a FRI reduction tree with 8-to-1 reduction, then
    /// a 4-to-1 reduction, then a 2-to-1 reduction. After these reductions, the reduced polynomial
    /// is sent directly.
    pub reduction_arity_bits: Vec<usize>,

    /// Number of query rounds to perform.
    pub num_query_rounds: usize,
}
