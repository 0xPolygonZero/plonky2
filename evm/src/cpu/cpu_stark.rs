use std::borrow::{Borrow, BorrowMut};
use std::iter::repeat;
use std::marker::PhantomData;

use itertools::Itertools;
use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;

use crate::all_stark::Table;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::{CpuColumnsView, COL_MAP, NUM_CPU_COLUMNS};
use crate::cpu::membus::NUM_GP_CHANNELS;
use crate::cpu::{
    bootstrap_kernel, contextops, control_flow, decode, dup_swap, gas, jumps, membus, memio,
    modfp254, pc, push0, shift, simple_logic, stack, stack_bounds, syscalls_exceptions,
};
use crate::cross_table_lookup::{Column, TableWithColumns};
use crate::memory::segments::Segment;
use crate::memory::{NUM_CHANNELS, VALUE_LIMBS};
use crate::stark::Stark;
use crate::vars::{StarkEvaluationTargets, StarkEvaluationVars};

pub fn ctl_data_keccak_sponge<F: Field>() -> Vec<Column<F>> {
    // When executing KECCAK_GENERAL, the GP memory channels are used as follows:
    // GP channel 0: stack[-1] = context
    // GP channel 1: stack[-2] = segment
    // GP channel 2: stack[-3] = virt
    // GP channel 3: stack[-4] = len
    // GP channel 4: pushed = outputs
    let context = Column::single(COL_MAP.mem_channels[0].value[0]);
    let segment = Column::single(COL_MAP.mem_channels[1].value[0]);
    let virt = Column::single(COL_MAP.mem_channels[2].value[0]);
    let len = Column::single(COL_MAP.mem_channels[3].value[0]);

    let num_channels = F::from_canonical_usize(NUM_CHANNELS);
    let timestamp = Column::linear_combination([(COL_MAP.clock, num_channels)]);

    let mut cols = vec![context, segment, virt, len, timestamp];
    cols.extend(COL_MAP.mem_channels[4].value.map(Column::single));
    cols
}

pub fn ctl_filter_keccak_sponge<F: Field>() -> Column<F> {
    Column::single(COL_MAP.is_keccak_sponge)
}

/// Create the vector of Columns corresponding to the two inputs and
/// one output of a binary operation.
fn ctl_data_binops<F: Field>() -> Vec<Column<F>> {
    let mut res = Column::singles(COL_MAP.mem_channels[0].value).collect_vec();
    res.extend(Column::singles(COL_MAP.mem_channels[1].value));
    res.extend(Column::singles(
        COL_MAP.mem_channels[NUM_GP_CHANNELS - 1].value,
    ));
    res
}

/// Create the vector of Columns corresponding to the three inputs and
/// one output of a ternary operation. By default, ternary operations use
/// the first three memory channels, and the last one for the result (binary
/// operations do not use the third inputs).
///
/// Shift operations are different, as they are simulated with `MUL` or `DIV`
/// on the arithmetic side. We first convert the shift into the multiplicand
/// (in case of `SHL`) or the divisor (in case of `SHR`), making the first memory
/// channel not directly usable. We overcome this by adding an offset of 1 in
/// case of shift operations, which will skip the first memory channel and use the
/// next three as ternary inputs. Because both `MUL` and `DIV` are binary operations,
/// the last memory channel used for the inputs will be safely ignored.
fn ctl_data_ternops<F: Field>(is_shift: bool) -> Vec<Column<F>> {
    let offset = is_shift as usize;
    let mut res = Column::singles(COL_MAP.mem_channels[offset].value).collect_vec();
    res.extend(Column::singles(COL_MAP.mem_channels[offset + 1].value));
    res.extend(Column::singles(COL_MAP.mem_channels[offset + 2].value));
    res.extend(Column::singles(
        COL_MAP.mem_channels[NUM_GP_CHANNELS - 1].value,
    ));
    res
}

pub fn ctl_data_logic<F: Field>() -> Vec<Column<F>> {
    // Instead of taking single columns, we reconstruct the entire opcode value directly.
    let mut res = vec![Column::le_bits(COL_MAP.opcode_bits)];
    res.extend(ctl_data_binops());
    res
}

pub fn ctl_filter_logic<F: Field>() -> Column<F> {
    Column::single(COL_MAP.op.logic_op)
}

pub fn ctl_arithmetic_base_rows<F: Field>() -> TableWithColumns<F> {
    const OPS: [usize; 4] = [
        COL_MAP.op.byte,
        COL_MAP.op.binary_op,
        COL_MAP.op.ternary_op,
        COL_MAP.op.fp254_op,
    ];

    // Instead of taking single columns, we reconstruct the entire opcode value directly.
    let mut columns = vec![Column::le_bits(COL_MAP.opcode_bits)];
    columns.extend(ctl_data_ternops(false));
    // Create the CPU Table whose columns are those with the three
    // inputs and one output of the ternary operations listed in `ops`
    // (also `ops` is used as the operation filter). The list of
    // operations includes binary operations which will simply ignore
    // the third input.
    TableWithColumns::new(Table::Cpu, columns, Some(Column::sum(OPS)))
}

pub fn ctl_arithmetic_shift_rows<F: Field>() -> TableWithColumns<F> {
    // Instead of taking single columns, we reconstruct the entire opcode value directly.
    let mut columns = vec![Column::le_bits(COL_MAP.opcode_bits)];
    columns.extend(ctl_data_ternops(true));
    // Create the CPU Table whose columns are those with the three
    // inputs and one output of the ternary operations listed in `ops`
    // (also `ops` is used as the operation filter). The list of
    // operations includes binary operations which will simply ignore
    // the third input.
    TableWithColumns::new(
        Table::Cpu,
        columns,
        Some(Column::sum([COL_MAP.op.shl, COL_MAP.op.shr])),
    )
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
    Column::sum(COL_MAP.op.iter())
}

pub fn ctl_filter_gp_memory<F: Field>(channel: usize) -> Column<F> {
    Column::single(COL_MAP.mem_channels[channel].used)
}

#[derive(Copy, Clone, Default)]
pub struct CpuStark<F, const D: usize> {
    pub f: PhantomData<F>,
}

impl<F: RichField, const D: usize> CpuStark<F, D> {
    // TODO: Remove?
    pub fn generate(&self, local_values: &mut [F; NUM_CPU_COLUMNS]) {
        let local_values: &mut CpuColumnsView<_> = local_values.borrow_mut();
        decode::generate(local_values);
        membus::generate(local_values);
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
        contextops::eval_packed(local_values, next_values, yield_constr);
        control_flow::eval_packed_generic(local_values, next_values, yield_constr);
        decode::eval_packed_generic(local_values, yield_constr);
        dup_swap::eval_packed(local_values, yield_constr);
        gas::eval_packed(local_values, next_values, yield_constr);
        jumps::eval_packed(local_values, next_values, yield_constr);
        membus::eval_packed(local_values, yield_constr);
        memio::eval_packed(local_values, yield_constr);
        modfp254::eval_packed(local_values, yield_constr);
        pc::eval_packed(local_values, yield_constr);
        push0::eval_packed(local_values, yield_constr);
        shift::eval_packed(local_values, yield_constr);
        simple_logic::eval_packed(local_values, yield_constr);
        stack::eval_packed(local_values, yield_constr);
        stack_bounds::eval_packed(local_values, yield_constr);
        syscalls_exceptions::eval_packed(local_values, next_values, yield_constr);
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
        contextops::eval_ext_circuit(builder, local_values, next_values, yield_constr);
        control_flow::eval_ext_circuit(builder, local_values, next_values, yield_constr);
        decode::eval_ext_circuit(builder, local_values, yield_constr);
        dup_swap::eval_ext_circuit(builder, local_values, yield_constr);
        gas::eval_ext_circuit(builder, local_values, next_values, yield_constr);
        jumps::eval_ext_circuit(builder, local_values, next_values, yield_constr);
        membus::eval_ext_circuit(builder, local_values, yield_constr);
        memio::eval_ext_circuit(builder, local_values, yield_constr);
        modfp254::eval_ext_circuit(builder, local_values, yield_constr);
        pc::eval_ext_circuit(builder, local_values, yield_constr);
        push0::eval_ext_circuit(builder, local_values, yield_constr);
        shift::eval_ext_circuit(builder, local_values, yield_constr);
        simple_logic::eval_ext_circuit(builder, local_values, yield_constr);
        stack::eval_ext_circuit(builder, local_values, yield_constr);
        stack_bounds::eval_ext_circuit(builder, local_values, yield_constr);
        syscalls_exceptions::eval_ext_circuit(builder, local_values, next_values, yield_constr);
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
