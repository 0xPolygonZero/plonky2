use std::marker::PhantomData;

use plonky2::field::extension_field::{Extendable, FieldExtension};
use plonky2::field::packed_field::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::circuit_builder::CircuitBuilder;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::stark::Stark;
use crate::vars::{StarkEvaluationTargets, StarkEvaluationVars};

/// Toy STARK system used for testing.
/// Computes a Fibonacci sequence with inital values `x0, x1` using the transition
/// `x0 <- x1, x1 <- x0 + x1`.
pub struct FibonacciStark<F: RichField + Extendable<D>, const D: usize> {
    x0: F,
    x1: F,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> FibonacciStark<F, D> {
    const NUM_COLUMNS: usize = 2;
    const NUM_ROWS: usize = 1 << 5;

    fn new(x0: F, x1: F) -> Self {
        Self {
            x0,
            x1,
            _phantom: PhantomData,
        }
    }

    fn generate_trace(&self) -> Vec<[F; Self::NUM_COLUMNS]> {
        (0..Self::NUM_ROWS)
            .scan([self.x0, self.x1], |acc, _| {
                let tmp = *acc;
                acc[0] = tmp[1];
                acc[1] = tmp[0] + tmp[1];
                Some(tmp)
            })
            .collect()
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Stark<F, D> for FibonacciStark<F, D> {
    const COLUMNS: usize = Self::NUM_COLUMNS;
    const PUBLIC_INPUTS: usize = 0;

    fn eval_packed_generic<FE, P, const D2: usize>(
        &self,
        vars: StarkEvaluationVars<FE, P, { Self::COLUMNS }, { Self::PUBLIC_INPUTS }>,
        yield_constr: &mut ConstraintConsumer<P>,
    ) where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>,
    {
        // x0 <- x1
        yield_constr.one(vars.next_values[0] - vars.local_values[1]);
        // x1 <- x0 + x1
        yield_constr.one(vars.next_values[1] - vars.local_values[0] - vars.local_values[1]);
    }

    fn eval_ext_recursively(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: StarkEvaluationTargets<D, { Self::COLUMNS }, { Self::PUBLIC_INPUTS }>,
        yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    ) {
        todo!()
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

    #[test]
    fn test_fibonacci_stark() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type S = FibonacciStark<F, D>;

        let config = StarkConfig::standard_fast_config();
        let stark = S::new(F::ZERO, F::ONE);
        let trace = stark.generate_trace();
        prove::<F, C, S, D>(stark, config, trace, &mut TimingTree::default())?;

        Ok(())
    }
}
