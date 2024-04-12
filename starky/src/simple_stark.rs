//! An example of generating and verifying STARK proofs for the Fibonacci sequence.
//! The toy STARK system also includes two columns that are a permutation of the other,
//! to highlight the use of the permutation argument with logUp.

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
use core::marker::PhantomData;

use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::packed::PackedField;
use plonky2::field::polynomial::PolynomialValues;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::plonk::circuit_builder::CircuitBuilder;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::evaluation_frame::StarkFrame;
use crate::stark::Stark;
use crate::util::trace_rows_to_poly_values;

/// Toy STARK system used for testing.
/// Computes a Fibonacci sequence with state `[x0, x1]` using the state transition
/// `x0' <- x1, x1' <- x0 + x1.
#[derive(Copy, Clone)]
struct SimpleStark<F: RichField + Extendable<D>, const D: usize> {
    num_rows: usize,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> SimpleStark<F, D> {
    const fn new(num_rows: usize) -> Self {
        Self {
            num_rows,
            _phantom: PhantomData,
        }
    }

    /// Generate the trace using `x0, x1` as initial state values.
    fn generate_trace(&self, x: F) -> Vec<PolynomialValues<F>> {
        let trace_rows = (0..self.num_rows)
            .scan([x], |acc, _| {
                acc[0] += F::ONE;
                Some(*acc)
            })
            .collect::<Vec<_>>();
        trace_rows_to_poly_values(trace_rows)
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Stark<F, D> for SimpleStark<F, D> {
    type EvaluationFrame<FE, P, const D2: usize> = StarkFrame<P, P::Scalar, 1, 0>
    where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>;

    type EvaluationFrameTarget = StarkFrame<ExtensionTarget<D>, ExtensionTarget<D>, 1, 0>;

    fn eval_packed_generic<FE, P, const D2: usize>(
        &self,
        _vars: &Self::EvaluationFrame<FE, P, D2>,
        _yield_constr: &mut ConstraintConsumer<P>,
    ) where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>,
    {
    }

    fn eval_ext_circuit(
        &self,
        _builder: &mut CircuitBuilder<F, D>,
        _vars: &Self::EvaluationFrameTarget,
        _yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    ) {
    }

    fn constraint_degree(&self) -> usize {
        0
    }
}

#[cfg(test)]
mod tests {
    #[cfg(not(feature = "std"))]
    use alloc::vec;

    use anyhow::Result;
    use plonky2::field::types::Field;
    use plonky2::fri::reduction_strategies::FriReductionStrategy;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use plonky2::util::timing::TimingTree;

    use crate::config::StarkConfig;
    use crate::prover::prove;
    use crate::simple_stark::SimpleStark;
    use crate::verifier::verify_stark_proof;

    #[test]
    fn test_fibonacci_stark() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type S = SimpleStark<F, D>;

        let mut config = StarkConfig::standard_fast_config();
        config.fri_config.cap_height = 5;
        let reduction_arity_bits = vec![1, 2, 1];
        config.fri_config.reduction_strategy =
            FriReductionStrategy::Fixed(reduction_arity_bits.clone());
        config.fri_config.num_query_rounds = 10;
        config.fri_config.proof_of_work_bits = 0;

        let num_rows = 1 << 9;

        let stark = S::new(num_rows);
        let trace = stark.generate_trace(F::ZERO);
        let proof = prove::<F, C, S, D>(stark, &config, trace, &[], &mut TimingTree::default())?;

        verify_stark_proof(stark, proof, &config)
    }
}
