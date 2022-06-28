use plonky2::field::extension::Extendable;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;

use crate::config::StarkConfig;
use crate::cpu::cpu_stark;
use crate::cpu::cpu_stark::CpuStark;
use crate::cross_table_lookup::{CrossTableLookup, TableWithColumns};
use crate::keccak::keccak_stark;
use crate::keccak::keccak_stark::KeccakStark;
use crate::logic;
use crate::logic::LogicStark;
use crate::memory::memory_stark::MemoryStark;
use crate::memory::{memory_stark, NUM_CHANNELS};
use crate::stark::Stark;

#[derive(Clone)]
pub struct AllStark<F: RichField + Extendable<D>, const D: usize> {
    pub cpu_stark: CpuStark<F, D>,
    pub keccak_stark: KeccakStark<F, D>,
    pub logic_stark: LogicStark<F, D>,
    pub memory_stark: MemoryStark<F, D>,
    pub cross_table_lookups: Vec<CrossTableLookup<F>>,
}

impl<F: RichField + Extendable<D>, const D: usize> AllStark<F, D> {
    pub(crate) fn nums_permutation_zs(&self, config: &StarkConfig) -> Vec<usize> {
        let ans = vec![
            self.cpu_stark.num_permutation_batches(config),
            self.keccak_stark.num_permutation_batches(config),
            self.logic_stark.num_permutation_batches(config),
            self.memory_stark.num_permutation_batches(config),
        ];
        debug_assert_eq!(ans.len(), Table::num_tables());
        ans
    }

    pub(crate) fn permutation_batch_sizes(&self) -> Vec<usize> {
        let ans = vec![
            self.cpu_stark.permutation_batch_size(),
            self.keccak_stark.permutation_batch_size(),
            self.logic_stark.permutation_batch_size(),
            self.memory_stark.permutation_batch_size(),
        ];
        debug_assert_eq!(ans.len(), Table::num_tables());
        ans
    }
}

#[derive(Copy, Clone)]
pub enum Table {
    Cpu = 0,
    Keccak = 1,
    Logic = 2,
    Memory = 3,
}

impl Table {
    pub(crate) fn num_tables() -> usize {
        Table::Memory as usize + 1
    }
}

#[allow(unused)] // TODO: Should be used soon.
pub(crate) fn all_cross_table_lookups<F: Field>() -> Vec<CrossTableLookup<F>> {
    let mut cross_table_lookups = vec![ctl_keccak(), ctl_logic()];
    cross_table_lookups.extend((0..NUM_CHANNELS).map(ctl_memory));
    cross_table_lookups
}

fn ctl_keccak<F: Field>() -> CrossTableLookup<F> {
    CrossTableLookup::new(
        vec![TableWithColumns::new(
            Table::Cpu,
            cpu_stark::ctl_data_keccak(),
            Some(cpu_stark::ctl_filter_keccak()),
        )],
        TableWithColumns::new(
            Table::Keccak,
            keccak_stark::ctl_data(),
            Some(keccak_stark::ctl_filter()),
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

fn ctl_memory<F: Field>(channel: usize) -> CrossTableLookup<F> {
    CrossTableLookup::new(
        vec![TableWithColumns::new(
            Table::Cpu,
            cpu_stark::ctl_data_memory(channel),
            Some(cpu_stark::ctl_filter_memory(channel)),
        )],
        TableWithColumns::new(
            Table::Memory,
            memory_stark::ctl_data(),
            Some(memory_stark::ctl_filter(channel)),
        ),
        None,
    )
}

#[cfg(test)]
mod tests {
    use std::borrow::BorrowMut;

    use anyhow::Result;
    use itertools::{izip, Itertools};
    use plonky2::field::polynomial::PolynomialValues;
    use plonky2::field::types::Field;
    use plonky2::iop::witness::PartialWitness;
    use plonky2::plonk::circuit_builder::CircuitBuilder;
    use plonky2::plonk::circuit_data::CircuitConfig;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use plonky2::util::timing::TimingTree;
    use rand::{thread_rng, Rng};

    use crate::all_stark::{all_cross_table_lookups, AllStark};
    use crate::config::StarkConfig;
    use crate::cpu::cpu_stark::CpuStark;
    use crate::keccak::keccak_stark::{KeccakStark, NUM_INPUTS, NUM_ROUNDS};
    use crate::logic::{self, LogicStark};
    use crate::memory::memory_stark::{generate_random_memory_ops, MemoryStark};
    use crate::memory::NUM_CHANNELS;
    use crate::proof::AllProof;
    use crate::prover::prove;
    use crate::recursive_verifier::{
        add_virtual_all_proof, set_all_proof_target, verify_proof_circuit,
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
        keccak_stark.generate_trace(keccak_inputs)
    }

    fn make_logic_trace<R: Rng>(
        num_rows: usize,
        logic_stark: &LogicStark<F, D>,
        rng: &mut R,
    ) -> Vec<PolynomialValues<F>> {
        let mut trace_rows = vec![];
        for _ in 0..num_rows {
            let mut row = [F::ZERO; logic::columns::NUM_COLUMNS];

            assert_eq!(logic::PACKED_LIMB_BITS, 16);
            for col in logic::columns::INPUT0 {
                row[col] = F::from_bool(rng.gen());
            }
            for col in logic::columns::INPUT1 {
                row[col] = F::from_bool(rng.gen());
            }
            let op: usize = rng.gen_range(0..3);
            let op_col = [
                logic::columns::IS_AND,
                logic::columns::IS_OR,
                logic::columns::IS_XOR,
            ][op];
            row[op_col] = F::ONE;
            logic_stark.generate(&mut row);
            trace_rows.push(row);
        }

        for _ in num_rows..num_rows.next_power_of_two() {
            trace_rows.push([F::ZERO; logic::columns::NUM_COLUMNS])
        }
        trace_rows_to_poly_values(trace_rows)
    }

    fn make_memory_trace<R: Rng>(
        num_memory_ops: usize,
        memory_stark: &MemoryStark<F, D>,
        rng: &mut R,
    ) -> Vec<PolynomialValues<F>> {
        let memory_ops = generate_random_memory_ops(num_memory_ops, rng);
        memory_stark.generate_trace(memory_ops)
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
        for i in 0..num_keccak_perms {
            let mut row: cpu::columns::CpuColumnsView<F> =
                [F::ZERO; CpuStark::<F, D>::COLUMNS].into();
            row.is_keccak = F::ONE;
            for (j, input, output) in izip!(
                0..2 * NUM_INPUTS,
                row.keccak_input_limbs.iter_mut(),
                row.keccak_output_limbs.iter_mut()
            ) {
                *input = keccak_input_limbs[i][j];
                *output = keccak_output_limbs[i][j];
            }
            cpu_stark.generate(&mut row);
            cpu_trace_rows.push(row.into());
        }

        for i in 0..num_logic_rows {
            let mut row: cpu::columns::CpuColumnsView<F> =
                [F::ZERO; CpuStark::<F, D>::COLUMNS].into();
            row.is_cpu_cycle = F::ONE;
            row.opcode = [
                (logic::columns::IS_AND, 0x16),
                (logic::columns::IS_OR, 0x17),
                (logic::columns::IS_XOR, 0x18),
            ]
            .into_iter()
            .map(|(col, opcode)| logic_trace[col].values[i] * F::from_canonical_u64(opcode))
            .sum();
            for (cols_cpu, cols_logic) in [
                (&mut row.logic_input0, logic::columns::INPUT0),
                (&mut row.logic_input1, logic::columns::INPUT1),
            ] {
                for (col_cpu, limb_cols_logic) in cols_cpu
                    .iter_mut()
                    .zip(logic::columns::limb_bit_cols_for_input(cols_logic))
                {
                    *col_cpu =
                        limb_from_bits_le(limb_cols_logic.map(|col| logic_trace[col].values[i]));
                }
            }
            for (col_cpu, col_logic) in row.logic_output.iter_mut().zip(logic::columns::RESULT) {
                *col_cpu = logic_trace[col_logic].values[i];
            }
            cpu_stark.generate(&mut row);
            cpu_trace_rows.push(row.into());
        }

        let mut current_cpu_index = 0;
        let mut last_timestamp = memory_trace[memory::columns::TIMESTAMP].values[0];
        for i in 0..num_memory_ops {
            let mem_timestamp = memory_trace[memory::columns::TIMESTAMP].values[i];
            let clock = mem_timestamp;
            let op = (0..NUM_CHANNELS)
                .filter(|&o| memory_trace[memory::columns::is_channel(o)].values[i] == F::ONE)
                .collect_vec()[0];

            if mem_timestamp != last_timestamp {
                current_cpu_index += 1;
                last_timestamp = mem_timestamp;
            }

            let row: &mut cpu::columns::CpuColumnsView<F> =
                cpu_trace_rows[current_cpu_index].borrow_mut();

            row.mem_channel_used[op] = F::ONE;
            row.clock = clock;
            row.mem_is_read[op] = memory_trace[memory::columns::IS_READ].values[i];
            row.mem_addr_context[op] = memory_trace[memory::columns::ADDR_CONTEXT].values[i];
            row.mem_addr_segment[op] = memory_trace[memory::columns::ADDR_SEGMENT].values[i];
            row.mem_addr_virtual[op] = memory_trace[memory::columns::ADDR_VIRTUAL].values[i];
            for j in 0..8 {
                row.mem_value[op][j] = memory_trace[memory::columns::value_limb(j)].values[i];
            }
        }
        trace_rows_to_poly_values(cpu_trace_rows)
    }

    fn get_proof(config: &StarkConfig) -> Result<(AllStark<F, D>, AllProof<F, C, D>)> {
        let cpu_stark = CpuStark::<F, D> {
            f: Default::default(),
        };

        let keccak_stark = KeccakStark::<F, D> {
            f: Default::default(),
        };

        let logic_stark = LogicStark::<F, D> {
            f: Default::default(),
        };
        let num_logic_rows = 62;

        let memory_stark = MemoryStark::<F, D> {
            f: Default::default(),
        };
        let num_memory_ops = 1 << 5;

        let mut rng = thread_rng();
        let num_keccak_perms = 2;

        let keccak_trace = make_keccak_trace(num_keccak_perms, &keccak_stark, &mut rng);
        let logic_trace = make_logic_trace(num_logic_rows, &logic_stark, &mut rng);
        let mut memory_trace = make_memory_trace(num_memory_ops, &memory_stark, &mut rng);
        let cpu_trace = make_cpu_trace(
            num_keccak_perms,
            num_logic_rows,
            num_memory_ops,
            &cpu_stark,
            &keccak_trace,
            &logic_trace,
            &mut memory_trace,
        );

        let all_stark = AllStark {
            cpu_stark,
            keccak_stark,
            logic_stark,
            memory_stark,
            cross_table_lookups: all_cross_table_lookups(),
        };

        let proof = prove::<F, C, D>(
            &all_stark,
            config,
            vec![cpu_trace, keccak_trace, logic_trace, memory_trace],
            vec![vec![]; 4],
            &mut TimingTree::default(),
        )?;

        Ok((all_stark, proof))
    }

    #[test]
    fn test_all_stark() -> Result<()> {
        let config = StarkConfig::standard_fast_config();
        let (all_stark, proof) = get_proof(&config)?;
        verify_proof(all_stark, proof, &config)
    }

    #[test]
    fn test_all_stark_recursive_verifier() -> Result<()> {
        init_logger();

        let config = StarkConfig::standard_fast_config();
        let (all_stark, proof) = get_proof(&config)?;
        verify_proof(all_stark.clone(), proof.clone(), &config)?;

        recursive_proof(all_stark, proof, &config, true)
    }

    fn recursive_proof(
        inner_all_stark: AllStark<F, D>,
        inner_proof: AllProof<F, C, D>,
        inner_config: &StarkConfig,
        print_gate_counts: bool,
    ) -> Result<()> {
        let circuit_config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<F, D>::new(circuit_config);
        let mut pw = PartialWitness::new();
        let degree_bits = inner_proof.degree_bits(inner_config);
        let nums_ctl_zs = inner_proof.nums_ctl_zs();
        let pt = add_virtual_all_proof(
            &mut builder,
            &inner_all_stark,
            inner_config,
            &degree_bits,
            &nums_ctl_zs,
        );
        set_all_proof_target(&mut pw, &pt, &inner_proof, builder.zero());

        verify_proof_circuit::<F, C, D>(&mut builder, inner_all_stark, pt, inner_config);

        if print_gate_counts {
            builder.print_gate_counts(0);
        }

        let data = builder.build::<C>();
        let proof = data.prove(pw)?;
        data.verify(proof)
    }

    fn init_logger() {
        let _ = env_logger::builder().format_timestamp(None).try_init();
    }
}
