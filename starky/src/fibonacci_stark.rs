use std::marker::PhantomData;

use plonky2::field::extension_field::{Extendable, FieldExtension};
use plonky2::field::packed_field::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::circuit_builder::CircuitBuilder;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::stark::Stark;
use crate::vars::{StarkEvaluationTargets, StarkEvaluationVars};

/// Toy STARK system used for testing.
/// Computes a Fibonacci sequence with state `[x0, x1]` using the state transition
/// `x0 <- x1, x1 <- x0 + x1`.
#[derive(Copy, Clone)]
struct FibonacciStark<F: RichField + Extendable<D>, const D: usize> {
    num_rows: usize,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> FibonacciStark<F, D> {
    // The first public input is `x0`.
    const PI_INDEX_X0: usize = 0;
    // The second public input is `x1`.
    const PI_INDEX_X1: usize = 1;
    // The third public input is the second element of the last row, which should be equal to the
    // `num_rows`-th Fibonacci number.
    const PI_INDEX_RES: usize = 2;

    fn new(num_rows: usize) -> Self {
        Self {
            num_rows,
            _phantom: PhantomData,
        }
    }

    /// Generate the trace using `x0, x1` as inital state values.
    fn generate_trace(&self, x0: F, x1: F) -> Vec<[F; Self::COLUMNS]> {
        (0..self.num_rows)
            .scan([x0, x1], |acc, _| {
                let tmp = *acc;
                acc[0] = tmp[1];
                acc[1] = tmp[0] + tmp[1];
                Some(tmp)
            })
            .collect()
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Stark<F, D> for FibonacciStark<F, D> {
    const COLUMNS: usize = 2;
    const PUBLIC_INPUTS: usize = 3;

    fn eval_packed_generic<FE, P, const D2: usize>(
        &self,
        vars: StarkEvaluationVars<FE, P, { Self::COLUMNS }, { Self::PUBLIC_INPUTS }>,
        yield_constr: &mut ConstraintConsumer<P>,
    ) where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>,
    {
        // Check public inputs.
        yield_constr
            .constraint_first_row(vars.local_values[0] - vars.public_inputs[Self::PI_INDEX_X0]);
        yield_constr
            .constraint_first_row(vars.local_values[1] - vars.public_inputs[Self::PI_INDEX_X1]);
        yield_constr
            .constraint_last_row(vars.local_values[1] - vars.public_inputs[Self::PI_INDEX_RES]);

        // x0 <- x1
        yield_constr.constraint(vars.next_values[0] - vars.local_values[1]);
        // x1 <- x0 + x1
        yield_constr.constraint(vars.next_values[1] - vars.local_values[0] - vars.local_values[1]);
    }

    fn eval_ext_recursively(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: StarkEvaluationTargets<D, { Self::COLUMNS }, { Self::PUBLIC_INPUTS }>,
        yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    ) {
        todo!()
    }

    fn degree(&self) -> usize {
        2
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use plonky2::field::field_types::Field;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use plonky2::util::timing::TimingTree;

    use crate::config::StarkConfig;
    use crate::fibonacci_stark::FibonacciStark;
    use crate::prover::prove;
    use crate::stark_testing::test_stark_low_degree;
    use crate::verifier::verify;

    fn fibonacci(n: usize, x0: usize, x1: usize) -> usize {
        (0..n).fold((0, 1), |x, _| (x.1, x.0 + x.1)).1
    }

    #[test]
    fn test_fibonacci_stark() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type S = FibonacciStark<F, D>;

        let config = StarkConfig::standard_fast_config();
        let num_rows = 1 << 5;
        let public_inputs = [
            F::ZERO,
            F::ONE,
            F::from_canonical_usize(fibonacci(num_rows - 1, 0, 1)),
        ];
        let stark = S::new(num_rows);
        let trace = stark.generate_trace(public_inputs[0], public_inputs[1]);
        let proof = prove::<F, C, S, D>(
            stark,
            &config,
            trace,
            public_inputs,
            &mut TimingTree::default(),
        )?;

        verify(stark, proof, &config)
    }

    #[test]
    fn test_fibonacci_stark_degree() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type S = FibonacciStark<F, D>;

        let config = StarkConfig::standard_fast_config();
        let num_rows = 1 << 5;
        let stark = S::new(num_rows);
        test_stark_low_degree(stark)
    }
}
