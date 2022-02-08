use std::marker::PhantomData;

use plonky2::field::extension_field::{Extendable, FieldExtension};
use plonky2::field::packed_field::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use starky::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use starky::stark::Stark;
use starky::vars::StarkEvaluationTargets;
use starky::vars::StarkEvaluationVars;

use crate::arithmetic::{
    eval_arithmetic_unit, eval_arithmetic_unit_recursively, generate_arithmetic_unit,
};
use crate::column_layout::NUM_COLUMNS;
use crate::memory::TransactionMemory;
use crate::public_input_layout::NUM_PUBLIC_INPUTS;

/// We require at least 2^16 rows as it helps support efficient 16-bit range checks.
const MIN_TRACE_ROWS: usize = 1 << 16;

#[derive(Copy, Clone)]
pub struct SystemZero<F: RichField + Extendable<D>, const D: usize> {
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> SystemZero<F, D> {
    fn generate_trace(&self) -> Vec<[F; NUM_COLUMNS]> {
        let memory = TransactionMemory::default();

        let mut row = [F::ZERO; NUM_COLUMNS];
        self.generate_first_row_core_registers(&mut row);
        Self::generate_permutation_unit(&mut row);

        let mut trace = Vec::with_capacity(MIN_TRACE_ROWS);

        loop {
            let mut next_row = [F::ZERO; NUM_COLUMNS];
            self.generate_next_row_core_registers(&row, &mut next_row);
            generate_arithmetic_unit(&mut next_row);
            Self::generate_permutation_unit(&mut next_row);

            trace.push(row);
            row = next_row;
        }

        trace.push(row);
        trace
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Default for SystemZero<F, D> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Stark<F, D> for SystemZero<F, D> {
    const COLUMNS: usize = NUM_COLUMNS;
    const PUBLIC_INPUTS: usize = NUM_PUBLIC_INPUTS;

    fn eval_packed_generic<FE, P, const D2: usize>(
        &self,
        vars: StarkEvaluationVars<FE, P, NUM_COLUMNS, NUM_PUBLIC_INPUTS>,
        yield_constr: &mut ConstraintConsumer<P>,
    ) where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>,
    {
        self.eval_core_registers(vars, yield_constr);
        eval_arithmetic_unit(vars, yield_constr);
        Self::eval_permutation_unit(vars, yield_constr);
        todo!()
    }

    fn eval_ext_recursively(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: StarkEvaluationTargets<D, NUM_COLUMNS, NUM_PUBLIC_INPUTS>,
        yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    ) {
        self.eval_core_registers_recursively(builder, vars, yield_constr);
        eval_arithmetic_unit_recursively(builder, vars, yield_constr);
        Self::eval_permutation_unit_recursively(builder, vars, yield_constr);
        todo!()
    }

    fn constraint_degree(&self) -> usize {
        3
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use log::Level;
    use plonky2::field::field_types::Field;
    use plonky2::field::goldilocks_field::GoldilocksField;
    use plonky2::plonk::config::PoseidonGoldilocksConfig;
    use plonky2::util::timing::TimingTree;
    use starky::config::StarkConfig;
    use starky::prover::prove;
    use starky::stark::Stark;
    use starky::stark_testing::test_stark_low_degree;
    use starky::verifier::verify;

    use crate::system_zero::SystemZero;

    #[test]
    #[ignore] // TODO
    fn run() -> Result<()> {
        type F = GoldilocksField;
        type C = PoseidonGoldilocksConfig;
        const D: usize = 2;

        type S = SystemZero<F, D>;
        let system = S::default();
        let public_inputs = [F::ZERO; S::PUBLIC_INPUTS];
        let config = StarkConfig::standard_fast_config();
        let mut timing = TimingTree::new("prove", Level::Debug);
        let trace = system.generate_trace();
        let proof = prove::<F, C, S, D>(system, &config, trace, public_inputs, &mut timing)?;

        verify(system, proof, &config)
    }

    #[test]
    #[ignore] // TODO
    fn degree() -> Result<()> {
        type F = GoldilocksField;
        type C = PoseidonGoldilocksConfig;
        const D: usize = 2;

        type S = SystemZero<F, D>;
        let system = S::default();
        test_stark_low_degree(system)
    }
}
