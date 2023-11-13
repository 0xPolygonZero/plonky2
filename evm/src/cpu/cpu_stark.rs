use std::borrow::Borrow;
use std::iter::repeat;
use std::marker::PhantomData;

use itertools::Itertools;
use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use super::columns::CpuColumnsView;
use super::halt;
use super::membus::NUM_GP_CHANNELS;
use crate::all_stark::Table;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::{COL_MAP, NUM_CPU_COLUMNS};
use crate::cpu::{
    bootstrap_kernel, byte_unpacking, clock, contextops, control_flow, decode, dup_swap, gas,
    jumps, membus, memio, modfp254, pc, push0, shift, simple_logic, stack, stack_bounds,
    syscalls_exceptions,
};
use crate::cross_table_lookup::{Column, TableWithColumns};
use crate::evaluation_frame::{StarkEvaluationFrame, StarkFrame};
use crate::memory::segments::Segment;
use crate::memory::{NUM_CHANNELS, VALUE_LIMBS};
use crate::stark::Stark;

/// Creates the vector of `Columns` corresponding to the General Purpose channels when calling the Keccak sponge:
/// the CPU reads the output of the sponge directly from the `KeccakSpongeStark` table.
pub fn ctl_data_keccak_sponge<F: Field>() -> Vec<Column<F>> {
    // When executing KECCAK_GENERAL, the GP memory channels are used as follows:
    // GP channel 0: stack[-1] = context
    // GP channel 1: stack[-2] = segment
    // GP channel 2: stack[-3] = virt
    // GP channel 3: stack[-4] = len
    // Next GP channel 0: pushed = outputs
    let context = Column::single(COL_MAP.mem_channels[0].value[0]);
    let segment = Column::single(COL_MAP.mem_channels[1].value[0]);
    let virt = Column::single(COL_MAP.mem_channels[2].value[0]);
    let len = Column::single(COL_MAP.mem_channels[3].value[0]);

    let num_channels = F::from_canonical_usize(NUM_CHANNELS);
    let timestamp = Column::linear_combination([(COL_MAP.clock, num_channels)]);

    let mut cols = vec![context, segment, virt, len, timestamp];
    cols.extend(Column::singles_next_row(COL_MAP.mem_channels[0].value));
    cols
}

/// CTL filter for a call to the Keccak sponge.
pub fn ctl_filter_keccak_sponge<F: Field>() -> Column<F> {
    Column::single(COL_MAP.is_keccak_sponge)
}

/// Creates the vector of `Columns` corresponding to the two inputs and
/// one output of a binary operation.
fn ctl_data_binops<F: Field>() -> Vec<Column<F>> {
    let mut res = Column::singles(COL_MAP.mem_channels[0].value).collect_vec();
    res.extend(Column::singles(COL_MAP.mem_channels[1].value));
    res.extend(Column::singles_next_row(COL_MAP.mem_channels[0].value));
    res
}

/// Creates the vector of `Columns` corresponding to the three inputs and
/// one output of a ternary operation. By default, ternary operations use
/// the first three memory channels, and the next top of the stack for the
/// result (binary operations do not use the third inputs).
fn ctl_data_ternops<F: Field>() -> Vec<Column<F>> {
    let mut res = Column::singles(COL_MAP.mem_channels[0].value).collect_vec();
    res.extend(Column::singles(COL_MAP.mem_channels[1].value));
    res.extend(Column::singles(COL_MAP.mem_channels[2].value));
    res.extend(Column::singles_next_row(COL_MAP.mem_channels[0].value));
    res
}

/// Creates the vector of columns corresponding to the opcode, the two inputs and the output of the logic operation.
pub fn ctl_data_logic<F: Field>() -> Vec<Column<F>> {
    // Instead of taking single columns, we reconstruct the entire opcode value directly.
    let mut res = vec![Column::le_bits(COL_MAP.opcode_bits)];
    res.extend(ctl_data_binops());
    res
}

/// CTL filter for logic operations.
pub fn ctl_filter_logic<F: Field>() -> Column<F> {
    Column::single(COL_MAP.op.logic_op)
}

/// Returns the `TableWithColumns` for the CPU rows calling arithmetic operations.
pub fn ctl_arithmetic_base_rows<F: Field>() -> TableWithColumns<F> {
    // Instead of taking single columns, we reconstruct the entire opcode value directly.
    let mut columns = vec![Column::le_bits(COL_MAP.opcode_bits)];
    columns.extend(ctl_data_ternops());
    // Create the CPU Table whose columns are those with the three
    // inputs and one output of the ternary operations listed in `ops`
    // (also `ops` is used as the operation filter). The list of
    // operations includes binary operations which will simply ignore
    // the third input.
    TableWithColumns::new(
        Table::Cpu,
        columns,
        Some(Column::sum([
            COL_MAP.op.binary_op,
            COL_MAP.op.fp254_op,
            COL_MAP.op.ternary_op,
            COL_MAP.op.shift,
            COL_MAP.op.prover_input,
            COL_MAP.op.syscall,
            COL_MAP.op.exception,
        ])),
    )
}

/// Creates the vector of `Columns` corresponding to the contents of General Purpose channels when calling byte packing.
/// We use `ctl_data_keccak_sponge` because the `Columns` are the same as the ones computed for `KeccakSpongeStark`.
pub fn ctl_data_byte_packing<F: Field>() -> Vec<Column<F>> {
    ctl_data_keccak_sponge()
}

/// CTL filter for the `MLOAD_32BYTES` operation.
pub fn ctl_filter_byte_packing<F: Field>() -> Column<F> {
    Column::single(COL_MAP.op.mload_32bytes)
}

/// Creates the vector of `Columns` corresponding to the contents of General Purpose channels when calling byte unpacking.
pub fn ctl_data_byte_unpacking<F: Field>() -> Vec<Column<F>> {
    // When executing MSTORE_32BYTES, the GP memory channels are used as follows:
    // GP channel 0: stack[-1] = context
    // GP channel 1: stack[-2] = segment
    // GP channel 2: stack[-3] = virt
    // GP channel 3: stack[-4] = val
    // Next GP channel 0: pushed = new_offset (virt + len)
    let context = Column::single(COL_MAP.mem_channels[0].value[0]);
    let segment = Column::single(COL_MAP.mem_channels[1].value[0]);
    let virt = Column::single(COL_MAP.mem_channels[2].value[0]);
    let val = Column::singles(COL_MAP.mem_channels[3].value);

    // len can be reconstructed as new_offset - virt.
    let len = Column::linear_combination_and_next_row_with_constant(
        [(COL_MAP.mem_channels[2].value[0], -F::ONE)],
        [(COL_MAP.mem_channels[0].value[0], F::ONE)],
        F::ZERO,
    );

    let num_channels = F::from_canonical_usize(NUM_CHANNELS);
    let timestamp = Column::linear_combination([(COL_MAP.clock, num_channels)]);

    let mut res = vec![context, segment, virt, len, timestamp];
    res.extend(val);

    res
}

/// CTL filter for the `MSTORE_32BYTES` operation.
pub fn ctl_filter_byte_unpacking<F: Field>() -> Column<F> {
    Column::single(COL_MAP.op.mstore_32bytes)
}

/// Index of the memory channel storing code.
pub const MEM_CODE_CHANNEL_IDX: usize = 0;
/// Index of the first general purpose memory channel.
pub const MEM_GP_CHANNELS_IDX_START: usize = MEM_CODE_CHANNEL_IDX + 1;

/// Make the time/channel column for memory lookups.
fn mem_time_and_channel<F: Field>(channel: usize) -> Column<F> {
    let scalar = F::from_canonical_usize(NUM_CHANNELS);
    let addend = F::from_canonical_usize(channel);
    Column::linear_combination_with_constant([(COL_MAP.clock, scalar)], addend)
}

/// Creates the vector of `Columns` corresponding to the contents of the code channel when reading code values.
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

/// Creates the vector of `Columns` corresponding to the contents of General Purpose channels.
pub(crate) fn ctl_data_gp_memory<F: Field>(channel: usize) -> Vec<Column<F>> {
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

pub(crate) fn ctl_data_partial_memory<F: Field>() -> Vec<Column<F>> {
    let channel_map = COL_MAP.partial_channel;
    let values = COL_MAP.mem_channels[0].value;
    let mut cols: Vec<_> = Column::singles([
        channel_map.is_read,
        channel_map.addr_context,
        channel_map.addr_segment,
        channel_map.addr_virtual,
    ])
    .collect();

    cols.extend(Column::singles(values));

    cols.push(mem_time_and_channel(
        MEM_GP_CHANNELS_IDX_START + NUM_GP_CHANNELS,
    ));

    cols
}

/// CTL filter for code read and write operations.
pub(crate) fn ctl_filter_code_memory<F: Field>() -> Column<F> {
    Column::sum(COL_MAP.op.iter())
}

/// CTL filter for General Purpose memory read and write operations.
pub(crate) fn ctl_filter_gp_memory<F: Field>(channel: usize) -> Column<F> {
    Column::single(COL_MAP.mem_channels[channel].used)
}

pub(crate) fn ctl_filter_partial_memory<F: Field>() -> Column<F> {
    Column::single(COL_MAP.partial_channel.used)
}

/// Structure representing the CPU Stark.
#[derive(Copy, Clone, Default)]
pub struct CpuStark<F, const D: usize> {
    pub f: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> Stark<F, D> for CpuStark<F, D> {
    type EvaluationFrame<FE, P, const D2: usize> = StarkFrame<P, NUM_CPU_COLUMNS>
    where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>;

    type EvaluationFrameTarget = StarkFrame<ExtensionTarget<D>, NUM_CPU_COLUMNS>;

    /// Evaluates all CPU constraints.
    fn eval_packed_generic<FE, P, const D2: usize>(
        &self,
        vars: &Self::EvaluationFrame<FE, P, D2>,
        yield_constr: &mut ConstraintConsumer<P>,
    ) where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>,
    {
        let local_values: &[P; NUM_CPU_COLUMNS] = vars.get_local_values().try_into().unwrap();
        let local_values: &CpuColumnsView<P> = local_values.borrow();
        let next_values: &[P; NUM_CPU_COLUMNS] = vars.get_next_values().try_into().unwrap();
        let next_values: &CpuColumnsView<P> = next_values.borrow();

        bootstrap_kernel::eval_bootstrap_kernel_packed(local_values, next_values, yield_constr);
        byte_unpacking::eval_packed(local_values, next_values, yield_constr);
        clock::eval_packed(local_values, next_values, yield_constr);
        contextops::eval_packed(local_values, next_values, yield_constr);
        control_flow::eval_packed_generic(local_values, next_values, yield_constr);
        decode::eval_packed_generic(local_values, yield_constr);
        dup_swap::eval_packed(local_values, next_values, yield_constr);
        gas::eval_packed(local_values, next_values, yield_constr);
        halt::eval_packed(local_values, next_values, yield_constr);
        jumps::eval_packed(local_values, next_values, yield_constr);
        membus::eval_packed(local_values, yield_constr);
        memio::eval_packed(local_values, next_values, yield_constr);
        modfp254::eval_packed(local_values, yield_constr);
        pc::eval_packed(local_values, next_values, yield_constr);
        push0::eval_packed(local_values, next_values, yield_constr);
        shift::eval_packed(local_values, yield_constr);
        simple_logic::eval_packed(local_values, next_values, yield_constr);
        stack::eval_packed(local_values, next_values, yield_constr);
        stack_bounds::eval_packed(local_values, yield_constr);
        syscalls_exceptions::eval_packed(local_values, next_values, yield_constr);
    }

    /// Circuit version of `eval_packed_generic`.
    /// Evaluates all CPU constraints.
    fn eval_ext_circuit(
        &self,
        builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
        vars: &Self::EvaluationFrameTarget,
        yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    ) {
        let local_values: &[ExtensionTarget<D>; NUM_CPU_COLUMNS] =
            vars.get_local_values().try_into().unwrap();
        let local_values: &CpuColumnsView<ExtensionTarget<D>> = local_values.borrow();
        let next_values: &[ExtensionTarget<D>; NUM_CPU_COLUMNS] =
            vars.get_next_values().try_into().unwrap();
        let next_values: &CpuColumnsView<ExtensionTarget<D>> = next_values.borrow();

        bootstrap_kernel::eval_bootstrap_kernel_ext_circuit(
            builder,
            local_values,
            next_values,
            yield_constr,
        );
        byte_unpacking::eval_ext_circuit(builder, local_values, next_values, yield_constr);
        clock::eval_ext_circuit(builder, local_values, next_values, yield_constr);
        contextops::eval_ext_circuit(builder, local_values, next_values, yield_constr);
        control_flow::eval_ext_circuit(builder, local_values, next_values, yield_constr);
        decode::eval_ext_circuit(builder, local_values, yield_constr);
        dup_swap::eval_ext_circuit(builder, local_values, next_values, yield_constr);
        gas::eval_ext_circuit(builder, local_values, next_values, yield_constr);
        halt::eval_ext_circuit(builder, local_values, next_values, yield_constr);
        jumps::eval_ext_circuit(builder, local_values, next_values, yield_constr);
        membus::eval_ext_circuit(builder, local_values, yield_constr);
        memio::eval_ext_circuit(builder, local_values, next_values, yield_constr);
        modfp254::eval_ext_circuit(builder, local_values, yield_constr);
        pc::eval_ext_circuit(builder, local_values, next_values, yield_constr);
        push0::eval_ext_circuit(builder, local_values, next_values, yield_constr);
        shift::eval_ext_circuit(builder, local_values, yield_constr);
        simple_logic::eval_ext_circuit(builder, local_values, next_values, yield_constr);
        stack::eval_ext_circuit(builder, local_values, next_values, yield_constr);
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
