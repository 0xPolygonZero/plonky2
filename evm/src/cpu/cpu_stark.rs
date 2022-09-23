use std::borrow::{Borrow, BorrowMut};
use std::iter::repeat;
use std::marker::PhantomData;

use itertools::Itertools;
use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::{CpuColumnsView, COL_MAP, NUM_CPU_COLUMNS};
use crate::cpu::{
    bootstrap_kernel, control_flow, decode, jumps, membus, simple_logic, stack_bounds, syscalls,
};
use crate::cross_table_lookup::Column;
use crate::memory::segments::Segment;
use crate::memory::{NUM_CHANNELS, VALUE_LIMBS};
use crate::stark::Stark;
use crate::vars::{StarkEvaluationTargets, StarkEvaluationVars};

pub fn ctl_data_keccak<F: Field>() -> Vec<Column<F>> {
    let keccak = COL_MAP.general.keccak();
    let mut res: Vec<_> = Column::singles(keccak.input_limbs).collect();
    res.extend(Column::singles(keccak.output_limbs));
    res
}

pub fn ctl_data_keccak_memory<F: Field>() -> Vec<Column<F>> {
    // When executing KECCAK_GENERAL, the GP memory channels are used as follows:
    // GP channel 0: stack[-1] = context
    // GP channel 1: stack[-2] = segment
    // GP channel 2: stack[-3] = virtual
    let context = Column::single(COL_MAP.mem_channels[0].value[0]);
    let segment = Column::single(COL_MAP.mem_channels[1].value[0]);
    let virt = Column::single(COL_MAP.mem_channels[2].value[0]);

    let num_channels = F::from_canonical_usize(NUM_CHANNELS);
    let clock = Column::linear_combination([(COL_MAP.clock, num_channels)]);

    vec![context, segment, virt, clock]
}

pub fn ctl_filter_keccak<F: Field>() -> Column<F> {
    Column::single(COL_MAP.is_keccak)
}

pub fn ctl_filter_keccak_memory<F: Field>() -> Column<F> {
    Column::single(COL_MAP.is_keccak_memory)
}

pub fn ctl_data_logic<F: Field>() -> Vec<Column<F>> {
    let mut res = Column::singles([COL_MAP.is_and, COL_MAP.is_or, COL_MAP.is_xor]).collect_vec();
    res.extend(Column::singles(COL_MAP.mem_channels[0].value));
    res.extend(Column::singles(COL_MAP.mem_channels[1].value));
    res.extend(Column::singles(COL_MAP.mem_channels[2].value));
    res
}

pub fn ctl_filter_logic<F: Field>() -> Column<F> {
    Column::sum([COL_MAP.is_and, COL_MAP.is_or, COL_MAP.is_xor])
}

pub const MEM_CODE_CHANNEL_IDX: usize = 0;
pub const MEM_GP_CHANNELS_IDX_START: usize = MEM_CODE_CHANNEL_IDX + 1;

/// Make the time/channel column for memory lookups.
fn mem_time_and_channel<F: Field>(channel: usize) -> Column<F> {
    let scalar = F::from_canonical_usize(NUM_CHANNELS);
    let addend = F::from_canonical_usize(channel);
    Column::linear_combination_with_constant([(COL_MAP.clock, scalar)], addend)
}

pub fn ctl_data_code_memory<F: Field>() -> Vec<Column<F>> {
    let mut cols = vec![
        Column::constant(F::ONE),                                      // is_read
        Column::single(COL_MAP.code_context),                          // addr_context
        Column::constant(F::from_canonical_u64(Segment::Code as u64)), // addr_segment
        Column::single(COL_MAP.program_counter),                       // addr_virtual
    ];

    // Low limb of the value matches the opcode bits
    cols.push(Column::le_bits(COL_MAP.opcode_bits));

    // High limbs of the value are all zero.
    cols.extend(repeat(Column::constant(F::ZERO)).take(VALUE_LIMBS - 1));

    cols.push(mem_time_and_channel(MEM_CODE_CHANNEL_IDX));

    cols
}

pub fn ctl_data_gp_memory<F: Field>(channel: usize) -> Vec<Column<F>> {
    let channel_map = COL_MAP.mem_channels[channel];
    let mut cols: Vec<_> = Column::singles([
        channel_map.is_read,
        channel_map.addr_context,
        channel_map.addr_segment,
        channel_map.addr_virtual,
    ])
    .collect();

    cols.extend(Column::singles(channel_map.value));

    cols.push(mem_time_and_channel(MEM_GP_CHANNELS_IDX_START + channel));

    cols
}

pub fn ctl_filter_code_memory<F: Field>() -> Column<F> {
    Column::single(COL_MAP.is_cpu_cycle)
}

pub fn ctl_filter_gp_memory<F: Field>(channel: usize) -> Column<F> {
    Column::single(COL_MAP.mem_channels[channel].used)
}

#[derive(Copy, Clone, Default)]
pub struct CpuStark<F, const D: usize> {
    pub f: PhantomData<F>,
}

impl<F: RichField, const D: usize> CpuStark<F, D> {
    pub fn generate(&self, local_values: &mut [F; NUM_CPU_COLUMNS]) {
        let local_values: &mut CpuColumnsView<_> = local_values.borrow_mut();
        decode::generate(local_values);
        membus::generate(local_values);
        simple_logic::generate(local_values);
        stack_bounds::generate(local_values); // Must come after `decode`.
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Stark<F, D> for CpuStark<F, D> {
    const COLUMNS: usize = NUM_CPU_COLUMNS;

    fn eval_packed_generic<FE, P, const D2: usize>(
        &self,
        vars: StarkEvaluationVars<FE, P, { Self::COLUMNS }>,
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
        jumps::eval_packed(local_values, next_values, yield_constr);
        membus::eval_packed(local_values, yield_constr);
        simple_logic::eval_packed(local_values, yield_constr);
        stack_bounds::eval_packed(local_values, yield_constr);
        syscalls::eval_packed(local_values, next_values, yield_constr);
    }

    fn eval_ext_circuit(
        &self,
        builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
        vars: StarkEvaluationTargets<D, { Self::COLUMNS }>,
        yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    ) {
        let local_values = vars.local_values.borrow();
        let next_values = vars.next_values.borrow();
        bootstrap_kernel::eval_bootstrap_kernel_circuit(builder, vars, yield_constr);
        control_flow::eval_ext_circuit(builder, local_values, next_values, yield_constr);
        decode::eval_ext_circuit(builder, local_values, yield_constr);
        jumps::eval_ext_circuit(builder, local_values, next_values, yield_constr);
        membus::eval_ext_circuit(builder, local_values, yield_constr);
        simple_logic::eval_ext_circuit(builder, local_values, yield_constr);
        stack_bounds::eval_ext_circuit(builder, local_values, yield_constr);
        syscalls::eval_ext_circuit(builder, local_values, next_values, yield_constr);
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
