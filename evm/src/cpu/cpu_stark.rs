use std::borrow::{Borrow, BorrowMut};
use std::marker::PhantomData;

use itertools::Itertools;
use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::{CpuColumnsView, COL_MAP, NUM_CPU_COLUMNS};
use crate::cpu::{bootstrap_kernel, control_flow, decode, simple_logic};
use crate::cross_table_lookup::Column;
use crate::memory::NUM_CHANNELS;
use crate::stark::Stark;
use crate::vars::{StarkEvaluationTargets, StarkEvaluationVars};

pub fn ctl_data_keccak<F: Field>() -> Vec<Column<F>> {
    let keccak = COL_MAP.general.keccak();
    let mut res: Vec<_> = Column::singles(keccak.input_limbs).collect();
    res.extend(Column::singles(keccak.output_limbs));
    res
}

pub fn ctl_filter_keccak<F: Field>() -> Column<F> {
    Column::single(COL_MAP.is_keccak)
}

pub fn ctl_data_logic<F: Field>() -> Vec<Column<F>> {
    let mut res = Column::singles([COL_MAP.is_and, COL_MAP.is_or, COL_MAP.is_xor]).collect_vec();
    let logic = COL_MAP.general.logic();
    res.extend(Column::singles(logic.input0));
    res.extend(Column::singles(logic.input1));
    res.extend(Column::singles(logic.output));
    res
}

pub fn ctl_filter_logic<F: Field>() -> Column<F> {
    Column::sum([COL_MAP.is_and, COL_MAP.is_or, COL_MAP.is_xor])
}

pub fn ctl_data_memory<F: Field>(channel: usize) -> Vec<Column<F>> {
    debug_assert!(channel < NUM_CHANNELS);
    let mut cols: Vec<Column<F>> = Column::singles([
        COL_MAP.mem_is_read[channel],
        COL_MAP.mem_addr_context[channel],
        COL_MAP.mem_addr_segment[channel],
        COL_MAP.mem_addr_virtual[channel],
    ])
    .collect_vec();
    cols.extend(Column::singles(COL_MAP.mem_value[channel]));

    let scalar = F::from_canonical_usize(NUM_CHANNELS);
    let addend = F::from_canonical_usize(channel);
    cols.push(Column::linear_combination_with_constant(
        vec![(COL_MAP.clock, scalar)],
        addend,
    ));

    cols
}

pub fn ctl_filter_memory<F: Field>(channel: usize) -> Column<F> {
    Column::single(COL_MAP.mem_channel_used[channel])
}

#[derive(Copy, Clone, Default)]
pub struct CpuStark<F, const D: usize> {
    pub f: PhantomData<F>,
}

impl<F: RichField, const D: usize> CpuStark<F, D> {
    pub fn generate(&self, local_values: &mut [F; NUM_CPU_COLUMNS]) {
        let local_values: &mut CpuColumnsView<_> = local_values.borrow_mut();
        decode::generate(local_values);
        simple_logic::generate(local_values);
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Stark<F, D> for CpuStark<F, D> {
    const COLUMNS: usize = NUM_CPU_COLUMNS;
    const PUBLIC_INPUTS: usize = 0;

    fn eval_packed_generic<FE, P, const D2: usize>(
        &self,
        vars: StarkEvaluationVars<FE, P, { Self::COLUMNS }, { Self::PUBLIC_INPUTS }>,
        yield_constr: &mut ConstraintConsumer<P>,
    ) where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>,
    {
        let local_values = vars.local_values.borrow();
        let next_values = vars.next_values.borrow();
        bootstrap_kernel::eval_bootstrap_kernel(vars, yield_constr);
        control_flow::eval_packed_generic(local_values, next_values, yield_constr);
        decode::eval_packed_generic(local_values, yield_constr);
        simple_logic::eval_packed(local_values, yield_constr);
    }

    fn eval_ext_circuit(
        &self,
        builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
        vars: StarkEvaluationTargets<D, { Self::COLUMNS }, { Self::PUBLIC_INPUTS }>,
        yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    ) {
        let local_values = vars.local_values.borrow();
        let next_values = vars.next_values.borrow();
        bootstrap_kernel::eval_bootstrap_kernel_circuit(builder, vars, yield_constr);
        control_flow::eval_ext_circuit(builder, local_values, next_values, yield_constr);
        decode::eval_ext_circuit(builder, local_values, yield_constr);
        simple_logic::eval_ext_circuit(builder, local_values, yield_constr);
    }

    fn constraint_degree(&self) -> usize {
        3
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};

    use crate::cpu::cpu_stark::CpuStark;
    use crate::stark_testing::{test_stark_circuit_constraints, test_stark_low_degree};

    #[test]
    fn test_stark_degree() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type S = CpuStark<F, D>;

        let stark = S {
            f: Default::default(),
        };
        test_stark_low_degree(stark)
    }

    #[test]
    fn test_stark_circuit() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type S = CpuStark<F, D>;

        let stark = S {
            f: Default::default(),
        };
        test_stark_circuit_constraints::<F, C, S, D>(stark)
    }
}
