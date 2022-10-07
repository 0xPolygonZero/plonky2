use std::iter;

use plonky2::field::extension::Extendable;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;

use crate::config::StarkConfig;
use crate::cpu::cpu_stark;
use crate::cpu::cpu_stark::CpuStark;
use crate::cpu::membus::NUM_GP_CHANNELS;
use crate::cross_table_lookup::{CrossTableLookup, TableWithColumns};
use crate::keccak::keccak_stark;
use crate::keccak::keccak_stark::KeccakStark;
use crate::keccak_memory::columns::KECCAK_WIDTH_BYTES;
use crate::keccak_memory::keccak_memory_stark;
use crate::keccak_memory::keccak_memory_stark::KeccakMemoryStark;
use crate::logic;
use crate::logic::LogicStark;
use crate::memory::memory_stark;
use crate::memory::memory_stark::MemoryStark;
use crate::stark::Stark;

#[derive(Clone)]
pub struct AllStark<F: RichField + Extendable<D>, const D: usize> {
    pub cpu_stark: CpuStark<F, D>,
    pub keccak_stark: KeccakStark<F, D>,
    pub keccak_memory_stark: KeccakMemoryStark<F, D>,
    pub logic_stark: LogicStark<F, D>,
    pub memory_stark: MemoryStark<F, D>,
    pub cross_table_lookups: Vec<CrossTableLookup<F>>,
}

impl<F: RichField + Extendable<D>, const D: usize> Default for AllStark<F, D> {
    fn default() -> Self {
        Self {
            cpu_stark: CpuStark::default(),
            keccak_stark: KeccakStark::default(),
            keccak_memory_stark: KeccakMemoryStark::default(),
            logic_stark: LogicStark::default(),
            memory_stark: MemoryStark::default(),
            cross_table_lookups: all_cross_table_lookups(),
        }
    }
}

impl<F: RichField + Extendable<D>, const D: usize> AllStark<F, D> {
    pub(crate) fn nums_permutation_zs(&self, config: &StarkConfig) -> [usize; NUM_TABLES] {
        [
            self.cpu_stark.num_permutation_batches(config),
            self.keccak_stark.num_permutation_batches(config),
            self.keccak_memory_stark.num_permutation_batches(config),
            self.logic_stark.num_permutation_batches(config),
            self.memory_stark.num_permutation_batches(config),
        ]
    }

    pub(crate) fn permutation_batch_sizes(&self) -> [usize; NUM_TABLES] {
        [
            self.cpu_stark.permutation_batch_size(),
            self.keccak_stark.permutation_batch_size(),
            self.keccak_memory_stark.permutation_batch_size(),
            self.logic_stark.permutation_batch_size(),
            self.memory_stark.permutation_batch_size(),
        ]
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Table {
    Cpu = 0,
    Keccak = 1,
    KeccakMemory = 2,
    Logic = 3,
    Memory = 4,
}

pub(crate) const NUM_TABLES: usize = Table::Memory as usize + 1;

#[allow(unused)] // TODO: Should be used soon.
pub(crate) fn all_cross_table_lookups<F: Field>() -> Vec<CrossTableLookup<F>> {
    vec![ctl_keccak(), ctl_logic(), ctl_memory(), ctl_keccak_memory()]
}

fn ctl_keccak<F: Field>() -> CrossTableLookup<F> {
    let cpu_looking = TableWithColumns::new(
        Table::Cpu,
        cpu_stark::ctl_data_keccak(),
        Some(cpu_stark::ctl_filter_keccak()),
    );
    let keccak_memory_looking = TableWithColumns::new(
        Table::KeccakMemory,
        keccak_memory_stark::ctl_looking_keccak(),
        Some(keccak_memory_stark::ctl_filter()),
    );
    CrossTableLookup::new(
        vec![cpu_looking, keccak_memory_looking],
        TableWithColumns::new(
            Table::Keccak,
            keccak_stark::ctl_data(),
            Some(keccak_stark::ctl_filter()),
        ),
        None,
    )
}

fn ctl_keccak_memory<F: Field>() -> CrossTableLookup<F> {
    CrossTableLookup::new(
        vec![TableWithColumns::new(
            Table::Cpu,
            cpu_stark::ctl_data_keccak_memory(),
            Some(cpu_stark::ctl_filter_keccak_memory()),
        )],
        TableWithColumns::new(
            Table::KeccakMemory,
            keccak_memory_stark::ctl_looked_data(),
            Some(keccak_memory_stark::ctl_filter()),
        ),
        None,
    )
}

fn ctl_logic<F: Field>() -> CrossTableLookup<F> {
    CrossTableLookup::new(
        vec![TableWithColumns::new(
            Table::Cpu,
            cpu_stark::ctl_data_logic(),
            Some(cpu_stark::ctl_filter_logic()),
        )],
        TableWithColumns::new(Table::Logic, logic::ctl_data(), Some(logic::ctl_filter())),
        None,
    )
}

fn ctl_memory<F: Field>() -> CrossTableLookup<F> {
    let cpu_memory_code_read = TableWithColumns::new(
        Table::Cpu,
        cpu_stark::ctl_data_code_memory(),
        Some(cpu_stark::ctl_filter_code_memory()),
    );
    let cpu_memory_gp_ops = (0..NUM_GP_CHANNELS).map(|channel| {
        TableWithColumns::new(
            Table::Cpu,
            cpu_stark::ctl_data_gp_memory(channel),
            Some(cpu_stark::ctl_filter_gp_memory(channel)),
        )
    });
    let keccak_memory_reads = (0..KECCAK_WIDTH_BYTES).map(|i| {
        TableWithColumns::new(
            Table::KeccakMemory,
            keccak_memory_stark::ctl_looking_memory(i, true),
            Some(keccak_memory_stark::ctl_filter()),
        )
    });
    let keccak_memory_writes = (0..KECCAK_WIDTH_BYTES).map(|i| {
        TableWithColumns::new(
            Table::KeccakMemory,
            keccak_memory_stark::ctl_looking_memory(i, false),
            Some(keccak_memory_stark::ctl_filter()),
        )
    });
    let all_lookers = iter::once(cpu_memory_code_read)
        .chain(cpu_memory_gp_ops)
        .chain(keccak_memory_reads)
        .chain(keccak_memory_writes)
        .collect();
    CrossTableLookup::new(
        all_lookers,
        TableWithColumns::new(
            Table::Memory,
            memory_stark::ctl_data(),
            Some(memory_stark::ctl_filter()),
        ),
        None,
    )
}

#[cfg(test)]
mod tests {
    use std::borrow::BorrowMut;

    use anyhow::Result;
    use ethereum_types::U256;
    use itertools::Itertools;
    use plonky2::field::polynomial::PolynomialValues;
    use plonky2::field::types::{Field, PrimeField64};
    use plonky2::iop::witness::PartialWitness;
    use plonky2::plonk::circuit_builder::CircuitBuilder;
    use plonky2::plonk::circuit_data::{CircuitConfig, VerifierCircuitData};
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use plonky2::util::timing::TimingTree;
    use rand::{thread_rng, Rng};

    use crate::all_stark::{AllStark, NUM_TABLES};
    use crate::config::StarkConfig;
    use crate::cpu::cpu_stark::CpuStark;
    use crate::cpu::kernel::aggregator::KERNEL;
    use crate::cross_table_lookup::testutils::check_ctls;
    use crate::keccak::keccak_stark::{KeccakStark, NUM_INPUTS, NUM_ROUNDS};
    use crate::keccak_memory::keccak_memory_stark::KeccakMemoryStark;
    use crate::logic::{self, LogicStark, Operation};
    use crate::memory::memory_stark::tests::generate_random_memory_ops;
    use crate::memory::memory_stark::MemoryStark;
    use crate::memory::NUM_CHANNELS;
    use crate::proof::{AllProof, PublicValues};
    use crate::prover::prove_with_traces;
    use crate::recursive_verifier::tests::recursively_verify_all_proof;
    use crate::recursive_verifier::{
        add_virtual_recursive_all_proof, all_verifier_data_recursive_stark_proof,
        set_recursive_all_proof_target, RecursiveAllProof,
    };
    use crate::stark::Stark;
    use crate::util::{limb_from_bits_le, trace_rows_to_poly_values};
    use crate::verifier::verify_proof;
    use crate::{cpu, keccak, memory};

    const D: usize = 2;
    type C = PoseidonGoldilocksConfig;
    type F = <C as GenericConfig<D>>::F;

    fn make_keccak_trace<R: Rng>(
        num_keccak_perms: usize,
        keccak_stark: &KeccakStark<F, D>,
        rng: &mut R,
    ) -> Vec<PolynomialValues<F>> {
        let keccak_inputs = (0..num_keccak_perms)
            .map(|_| [0u64; NUM_INPUTS].map(|_| rng.gen()))
            .collect_vec();
        keccak_stark.generate_trace(keccak_inputs, &mut TimingTree::default())
    }

    fn make_keccak_memory_trace(
        keccak_memory_stark: &KeccakMemoryStark<F, D>,
        config: &StarkConfig,
    ) -> Vec<PolynomialValues<F>> {
        keccak_memory_stark.generate_trace(
            vec![],
            config.fri_config.num_cap_elements(),
            &mut TimingTree::default(),
        )
    }

    fn make_logic_trace<R: Rng>(
        num_rows: usize,
        logic_stark: &LogicStark<F, D>,
        rng: &mut R,
    ) -> Vec<PolynomialValues<F>> {
        let all_ops = [logic::Op::And, logic::Op::Or, logic::Op::Xor];
        let ops = (0..num_rows)
            .map(|_| {
                let op = all_ops[rng.gen_range(0..all_ops.len())];
                let input0 = U256(rng.gen());
                let input1 = U256(rng.gen());
                Operation::new(op, input0, input1)
            })
            .collect();
        logic_stark.generate_trace(ops, &mut TimingTree::default())
    }

    fn make_memory_trace<R: Rng>(
        num_memory_ops: usize,
        memory_stark: &MemoryStark<F, D>,
        rng: &mut R,
    ) -> (Vec<PolynomialValues<F>>, usize) {
        let memory_ops = generate_random_memory_ops(num_memory_ops, rng);
        let trace = memory_stark.generate_trace(memory_ops, &mut TimingTree::default());
        let num_ops = trace[0].values.len();
        (trace, num_ops)
    }

    fn bits_from_opcode(opcode: u8) -> [F; 8] {
        [
            F::from_bool(opcode & (1 << 0) != 0),
            F::from_bool(opcode & (1 << 1) != 0),
            F::from_bool(opcode & (1 << 2) != 0),
            F::from_bool(opcode & (1 << 3) != 0),
            F::from_bool(opcode & (1 << 4) != 0),
            F::from_bool(opcode & (1 << 5) != 0),
            F::from_bool(opcode & (1 << 6) != 0),
            F::from_bool(opcode & (1 << 7) != 0),
        ]
    }

    fn make_cpu_trace(
        num_keccak_perms: usize,
        num_logic_rows: usize,
        num_memory_ops: usize,
        cpu_stark: &CpuStark<F, D>,
        keccak_trace: &[PolynomialValues<F>],
        logic_trace: &[PolynomialValues<F>],
        memory_trace: &mut [PolynomialValues<F>],
    ) -> Vec<PolynomialValues<F>> {
        let keccak_input_limbs: Vec<[F; 2 * NUM_INPUTS]> = (0..num_keccak_perms)
            .map(|i| {
                (0..2 * NUM_INPUTS)
                    .map(|j| {
                        keccak::columns::reg_input_limb(j)
                            .eval_table(keccak_trace, (i + 1) * NUM_ROUNDS - 1)
                    })
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap()
            })
            .collect();
        let keccak_output_limbs: Vec<[F; 2 * NUM_INPUTS]> = (0..num_keccak_perms)
            .map(|i| {
                (0..2 * NUM_INPUTS)
                    .map(|j| {
                        keccak_trace[keccak::columns::reg_output_limb(j)].values
                            [(i + 1) * NUM_ROUNDS - 1]
                    })
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap()
            })
            .collect();

        let mut cpu_trace_rows: Vec<[F; CpuStark::<F, D>::COLUMNS]> = vec![];
        let mut bootstrap_row: cpu::columns::CpuColumnsView<F> =
            [F::ZERO; CpuStark::<F, D>::COLUMNS].into();
        bootstrap_row.is_bootstrap_kernel = F::ONE;
        cpu_trace_rows.push(bootstrap_row.into());

        for i in 0..num_keccak_perms {
            let mut row: cpu::columns::CpuColumnsView<F> =
                [F::ZERO; CpuStark::<F, D>::COLUMNS].into();
            row.is_keccak = F::ONE;
            let keccak = row.general.keccak_mut();
            for j in 0..2 * NUM_INPUTS {
                keccak.input_limbs[j] = keccak_input_limbs[i][j];
                keccak.output_limbs[j] = keccak_output_limbs[i][j];
            }
            cpu_stark.generate(row.borrow_mut());
            cpu_trace_rows.push(row.into());
        }

        // Pad to `num_memory_ops` for memory testing.
        for _ in cpu_trace_rows.len()..num_memory_ops {
            let mut row: cpu::columns::CpuColumnsView<F> =
                [F::ZERO; CpuStark::<F, D>::COLUMNS].into();
            row.opcode_bits = bits_from_opcode(0x5b);
            row.is_cpu_cycle = F::ONE;
            row.is_kernel_mode = F::ONE;
            row.program_counter = F::from_canonical_usize(KERNEL.global_labels["main"]);
            cpu_stark.generate(row.borrow_mut());
            cpu_trace_rows.push(row.into());
        }

        for i in 0..num_memory_ops {
            let mem_timestamp: usize = memory_trace[memory::columns::TIMESTAMP].values[i]
                .to_canonical_u64()
                .try_into()
                .unwrap();
            let clock = mem_timestamp / NUM_CHANNELS;
            let channel = mem_timestamp % NUM_CHANNELS;

            let filter = memory_trace[memory::columns::FILTER].values[i];
            assert!(filter.is_one() || filter.is_zero());
            let is_actual_op = filter.is_one();

            if is_actual_op {
                let row: &mut cpu::columns::CpuColumnsView<F> = cpu_trace_rows[clock].borrow_mut();
                row.clock = F::from_canonical_usize(clock);

                dbg!(channel, row.mem_channels.len());
                let channel = &mut row.mem_channels[channel];
                channel.used = F::ONE;
                channel.is_read = memory_trace[memory::columns::IS_READ].values[i];
                channel.addr_context = memory_trace[memory::columns::ADDR_CONTEXT].values[i];
                channel.addr_segment = memory_trace[memory::columns::ADDR_SEGMENT].values[i];
                channel.addr_virtual = memory_trace[memory::columns::ADDR_VIRTUAL].values[i];
                for j in 0..8 {
                    channel.value[j] = memory_trace[memory::columns::value_limb(j)].values[i];
                }
            }
        }

        for i in 0..num_logic_rows {
            let mut row: cpu::columns::CpuColumnsView<F> =
                [F::ZERO; CpuStark::<F, D>::COLUMNS].into();
            row.is_cpu_cycle = F::ONE;
            row.is_kernel_mode = F::ONE;

            // Since these are the first cycle rows, we must start with PC=main then increment.
            row.program_counter = F::from_canonical_usize(KERNEL.global_labels["main"] + i);
            row.opcode_bits = bits_from_opcode(
                if logic_trace[logic::columns::IS_AND].values[i] != F::ZERO {
                    0x16
                } else if logic_trace[logic::columns::IS_OR].values[i] != F::ZERO {
                    0x17
                } else if logic_trace[logic::columns::IS_XOR].values[i] != F::ZERO {
                    0x18
                } else {
                    panic!()
                },
            );

            let input0_bit_cols = logic::columns::limb_bit_cols_for_input(logic::columns::INPUT0);
            for (col_cpu, limb_cols_logic) in
                row.mem_channels[0].value.iter_mut().zip(input0_bit_cols)
            {
                *col_cpu = limb_from_bits_le(limb_cols_logic.map(|col| logic_trace[col].values[i]));
            }

            let input1_bit_cols = logic::columns::limb_bit_cols_for_input(logic::columns::INPUT1);
            for (col_cpu, limb_cols_logic) in
                row.mem_channels[1].value.iter_mut().zip(input1_bit_cols)
            {
                *col_cpu = limb_from_bits_le(limb_cols_logic.map(|col| logic_trace[col].values[i]));
            }

            for (col_cpu, col_logic) in row.mem_channels[2]
                .value
                .iter_mut()
                .zip(logic::columns::RESULT)
            {
                *col_cpu = logic_trace[col_logic].values[i];
            }

            cpu_stark.generate(row.borrow_mut());
            cpu_trace_rows.push(row.into());
        }

        // Trap to kernel
        {
            let mut row: cpu::columns::CpuColumnsView<F> =
                [F::ZERO; CpuStark::<F, D>::COLUMNS].into();
            let last_row: cpu::columns::CpuColumnsView<F> =
                cpu_trace_rows[cpu_trace_rows.len() - 1].into();
            row.is_cpu_cycle = F::ONE;
            row.opcode_bits = bits_from_opcode(0x0a); // `EXP` is implemented in software
            row.is_kernel_mode = F::ONE;
            row.program_counter = last_row.program_counter + F::ONE;
            row.mem_channels[0].value = [
                row.program_counter,
                F::ONE,
                F::ZERO,
                F::ZERO,
                F::ZERO,
                F::ZERO,
                F::ZERO,
                F::ZERO,
            ];
            cpu_stark.generate(row.borrow_mut());
            cpu_trace_rows.push(row.into());
        }

        // `EXIT_KERNEL` (to kernel)
        {
            let mut row: cpu::columns::CpuColumnsView<F> =
                [F::ZERO; CpuStark::<F, D>::COLUMNS].into();
            row.is_cpu_cycle = F::ONE;
            row.opcode_bits = bits_from_opcode(0xf9);
            row.is_kernel_mode = F::ONE;
            row.program_counter = F::from_canonical_usize(KERNEL.global_labels["sys_exp"]);
            row.mem_channels[0].value = [
                F::from_canonical_u16(15682),
                F::ONE,
                F::ZERO,
                F::ZERO,
                F::ZERO,
                F::ZERO,
                F::ZERO,
                F::ZERO,
            ];
            cpu_stark.generate(row.borrow_mut());
            cpu_trace_rows.push(row.into());
        }

        // `JUMP` (in kernel mode)
        {
            let mut row: cpu::columns::CpuColumnsView<F> =
                [F::ZERO; CpuStark::<F, D>::COLUMNS].into();
            row.is_cpu_cycle = F::ONE;
            row.opcode_bits = bits_from_opcode(0x56);
            row.is_kernel_mode = F::ONE;
            row.program_counter = F::from_canonical_u16(15682);
            row.mem_channels[0].value = [
                F::from_canonical_u16(15106),
                F::ZERO,
                F::ZERO,
                F::ZERO,
                F::ZERO,
                F::ZERO,
                F::ZERO,
                F::ZERO,
            ];
            row.mem_channels[1].value = [
                F::ONE,
                F::ZERO,
                F::ZERO,
                F::ZERO,
                F::ZERO,
                F::ZERO,
                F::ZERO,
                F::ZERO,
            ];
            row.general.jumps_mut().input0_upper_zero = F::ONE;
            row.general.jumps_mut().dst_valid_or_kernel = F::ONE;
            row.general.jumps_mut().input0_jumpable = F::ONE;
            row.general.jumps_mut().input1_sum_inv = F::ONE;
            row.general.jumps_mut().should_jump = F::ONE;
            cpu_stark.generate(row.borrow_mut());
            cpu_trace_rows.push(row.into());
        }

        // `EXIT_KERNEL` (to userspace)
        {
            let mut row: cpu::columns::CpuColumnsView<F> =
                [F::ZERO; CpuStark::<F, D>::COLUMNS].into();
            row.is_cpu_cycle = F::ONE;
            row.opcode_bits = bits_from_opcode(0xf9);
            row.is_kernel_mode = F::ONE;
            row.program_counter = F::from_canonical_u16(15106);
            row.mem_channels[0].value = [
                F::from_canonical_u16(63064),
                F::ZERO,
                F::ZERO,
                F::ZERO,
                F::ZERO,
                F::ZERO,
                F::ZERO,
                F::ZERO,
            ];
            cpu_stark.generate(row.borrow_mut());
            cpu_trace_rows.push(row.into());
        }

        // `JUMP` (taken)
        {
            let mut row: cpu::columns::CpuColumnsView<F> =
                [F::ZERO; CpuStark::<F, D>::COLUMNS].into();
            row.is_cpu_cycle = F::ONE;
            row.opcode_bits = bits_from_opcode(0x56);
            row.is_kernel_mode = F::ZERO;
            row.program_counter = F::from_canonical_u16(63064);
            row.mem_channels[0].value = [
                F::from_canonical_u16(3754),
                F::ZERO,
                F::ZERO,
                F::ZERO,
                F::ZERO,
                F::ZERO,
                F::ZERO,
                F::ZERO,
            ];
            row.mem_channels[1].value = [
                F::ONE,
                F::ZERO,
                F::ZERO,
                F::ZERO,
                F::ZERO,
                F::ZERO,
                F::ZERO,
                F::ZERO,
            ];
            row.general.jumps_mut().input0_upper_zero = F::ONE;
            row.general.jumps_mut().dst_valid = F::ONE;
            row.general.jumps_mut().dst_valid_or_kernel = F::ONE;
            row.general.jumps_mut().input0_jumpable = F::ONE;
            row.general.jumps_mut().input1_sum_inv = F::ONE;
            row.general.jumps_mut().should_jump = F::ONE;
            cpu_stark.generate(row.borrow_mut());
            cpu_trace_rows.push(row.into());
        }

        // `JUMPI` (taken)
        {
            let mut row: cpu::columns::CpuColumnsView<F> =
                [F::ZERO; CpuStark::<F, D>::COLUMNS].into();
            row.is_cpu_cycle = F::ONE;
            row.opcode_bits = bits_from_opcode(0x57);
            row.is_kernel_mode = F::ZERO;
            row.program_counter = F::from_canonical_u16(3754);
            row.mem_channels[0].value = [
                F::from_canonical_u16(37543),
                F::ZERO,
                F::ZERO,
                F::ZERO,
                F::ZERO,
                F::ZERO,
                F::ZERO,
                F::ZERO,
            ];
            row.mem_channels[1].value = [
                F::ZERO,
                F::ZERO,
                F::ZERO,
                F::ONE,
                F::ZERO,
                F::ZERO,
                F::ZERO,
                F::ZERO,
            ];
            row.general.jumps_mut().input0_upper_zero = F::ONE;
            row.general.jumps_mut().dst_valid = F::ONE;
            row.general.jumps_mut().dst_valid_or_kernel = F::ONE;
            row.general.jumps_mut().input0_jumpable = F::ONE;
            row.general.jumps_mut().input1_sum_inv = F::ONE;
            row.general.jumps_mut().should_jump = F::ONE;
            cpu_stark.generate(row.borrow_mut());
            cpu_trace_rows.push(row.into());
        }

        // `JUMPI` (not taken)
        {
            let mut row: cpu::columns::CpuColumnsView<F> =
                [F::ZERO; CpuStark::<F, D>::COLUMNS].into();
            row.is_cpu_cycle = F::ONE;
            row.opcode_bits = bits_from_opcode(0x57);
            row.is_kernel_mode = F::ZERO;
            row.program_counter = F::from_canonical_u16(37543);
            row.mem_channels[0].value = [
                F::from_canonical_u16(37543),
                F::ZERO,
                F::ZERO,
                F::ZERO,
                F::ONE,
                F::ZERO,
                F::ZERO,
                F::ZERO,
            ];
            row.general.jumps_mut().input0_upper_sum_inv = F::ONE;
            row.general.jumps_mut().dst_valid = F::ONE;
            row.general.jumps_mut().dst_valid_or_kernel = F::ONE;
            row.general.jumps_mut().input0_jumpable = F::ZERO;
            row.general.jumps_mut().should_continue = F::ONE;
            cpu_stark.generate(row.borrow_mut());
            cpu_trace_rows.push(row.into());
        }

        // `JUMP` (trapping)
        {
            let mut row: cpu::columns::CpuColumnsView<F> =
                [F::ZERO; CpuStark::<F, D>::COLUMNS].into();
            let last_row: cpu::columns::CpuColumnsView<F> =
                cpu_trace_rows[cpu_trace_rows.len() - 1].into();
            row.is_cpu_cycle = F::ONE;
            row.opcode_bits = bits_from_opcode(0x56);
            row.is_kernel_mode = F::ZERO;
            row.program_counter = last_row.program_counter + F::ONE;
            row.mem_channels[0].value = [
                F::from_canonical_u16(37543),
                F::ZERO,
                F::ZERO,
                F::ZERO,
                F::ONE,
                F::ZERO,
                F::ZERO,
                F::ZERO,
            ];
            row.mem_channels[1].value = [
                F::ONE,
                F::ZERO,
                F::ZERO,
                F::ZERO,
                F::ZERO,
                F::ZERO,
                F::ZERO,
                F::ZERO,
            ];
            row.general.jumps_mut().input0_upper_sum_inv = F::ONE;
            row.general.jumps_mut().dst_valid = F::ONE;
            row.general.jumps_mut().dst_valid_or_kernel = F::ONE;
            row.general.jumps_mut().input0_jumpable = F::ZERO;
            row.general.jumps_mut().input1_sum_inv = F::ONE;
            row.general.jumps_mut().should_trap = F::ONE;
            cpu_stark.generate(row.borrow_mut());
            cpu_trace_rows.push(row.into());
        }

        // Pad to a power of two.
        for i in 0..cpu_trace_rows.len().next_power_of_two() - cpu_trace_rows.len() {
            let mut row: cpu::columns::CpuColumnsView<F> =
                [F::ZERO; CpuStark::<F, D>::COLUMNS].into();
            row.opcode_bits = bits_from_opcode(0xff);
            row.is_cpu_cycle = F::ONE;
            row.is_kernel_mode = F::ONE;
            row.program_counter =
                F::from_canonical_usize(KERNEL.global_labels["fault_exception"] + i);
            cpu_stark.generate(row.borrow_mut());
            cpu_trace_rows.push(row.into());
        }

        // Ensure we finish in a halted state.
        {
            let num_rows = cpu_trace_rows.len();
            let halt_label = F::from_canonical_usize(KERNEL.global_labels["halt_pc0"]);

            let last_row: &mut cpu::columns::CpuColumnsView<F> =
                cpu_trace_rows[num_rows - 1].borrow_mut();
            last_row.program_counter = halt_label;
        }

        trace_rows_to_poly_values(cpu_trace_rows)
    }

    fn get_proof(config: &StarkConfig) -> Result<(AllStark<F, D>, AllProof<F, C, D>)> {
        let all_stark = AllStark::default();

        let num_logic_rows = 62;
        let num_memory_ops = 1 << 5;

        let mut rng = thread_rng();
        let num_keccak_perms = 2;

        let keccak_trace = make_keccak_trace(num_keccak_perms, &all_stark.keccak_stark, &mut rng);
        let keccak_memory_trace = make_keccak_memory_trace(&all_stark.keccak_memory_stark, config);
        let logic_trace = make_logic_trace(num_logic_rows, &all_stark.logic_stark, &mut rng);
        let mem_trace = make_memory_trace(num_memory_ops, &all_stark.memory_stark, &mut rng);
        let mut memory_trace = mem_trace.0;
        let num_memory_ops = mem_trace.1;
        let cpu_trace = make_cpu_trace(
            num_keccak_perms,
            num_logic_rows,
            num_memory_ops,
            &all_stark.cpu_stark,
            &keccak_trace,
            &logic_trace,
            &mut memory_trace,
        );

        let traces = [
            cpu_trace,
            keccak_trace,
            keccak_memory_trace,
            logic_trace,
            memory_trace,
        ];
        check_ctls(&traces, &all_stark.cross_table_lookups);

        let public_values = PublicValues::default();
        let proof = prove_with_traces::<F, C, D>(
            &all_stark,
            config,
            traces,
            public_values,
            &mut TimingTree::default(),
        )?;

        Ok((all_stark, proof))
    }

    #[test]
    #[ignore] // Ignoring but not deleting so the test can serve as an API usage example
    fn test_all_stark() -> Result<()> {
        let config = StarkConfig::standard_fast_config();
        let (all_stark, proof) = get_proof(&config)?;
        verify_proof(all_stark, proof, &config)
    }

    #[test]
    #[ignore] // Ignoring but not deleting so the test can serve as an API usage example
    fn test_all_stark_recursive_verifier() -> Result<()> {
        init_logger();

        let config = StarkConfig::standard_fast_config();
        let (all_stark, proof) = get_proof(&config)?;
        verify_proof(all_stark.clone(), proof.clone(), &config)?;

        recursive_proof(all_stark, proof, &config)
    }

    fn recursive_proof(
        inner_all_stark: AllStark<F, D>,
        inner_proof: AllProof<F, C, D>,
        inner_config: &StarkConfig,
    ) -> Result<()> {
        let circuit_config = CircuitConfig::standard_recursion_config();
        let recursive_all_proof = recursively_verify_all_proof(
            &inner_all_stark,
            &inner_proof,
            inner_config,
            &circuit_config,
        )?;

        let verifier_data: [VerifierCircuitData<F, C, D>; NUM_TABLES] =
            all_verifier_data_recursive_stark_proof(
                &inner_all_stark,
                inner_proof.degree_bits(inner_config),
                inner_config,
                &circuit_config,
            );
        let circuit_config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<F, D>::new(circuit_config);
        let mut pw = PartialWitness::new();
        let recursive_all_proof_target =
            add_virtual_recursive_all_proof(&mut builder, &verifier_data);
        set_recursive_all_proof_target(&mut pw, &recursive_all_proof_target, &recursive_all_proof);
        RecursiveAllProof::verify_circuit(
            &mut builder,
            recursive_all_proof_target,
            &verifier_data,
            inner_all_stark.cross_table_lookups,
            inner_config,
        );

        let data = builder.build::<C>();
        let proof = data.prove(pw)?;
        data.verify(proof)
    }

    fn init_logger() {
        let _ = env_logger::builder().format_timestamp(None).try_init();
    }
}
