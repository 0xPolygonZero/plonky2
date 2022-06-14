use plonky2::field::extension_field::Extendable;
use plonky2::hash::hash_types::RichField;

use crate::config::StarkConfig;
use crate::cpu::cpu_stark::CpuStark;
use crate::cross_table_lookup::CrossTableLookup;
use crate::keccak::keccak_stark::KeccakStark;
use crate::stark::Stark;

#[derive(Clone)]
pub struct AllStark<F: RichField + Extendable<D>, const D: usize> {
    pub cpu_stark: CpuStark<F, D>,
    pub keccak_stark: KeccakStark<F, D>,
    pub cross_table_lookups: Vec<CrossTableLookup<F>>,
}

impl<F: RichField + Extendable<D>, const D: usize> AllStark<F, D> {
    pub(crate) fn nums_permutation_zs(&self, config: &StarkConfig) -> Vec<usize> {
        let ans = vec![
            self.cpu_stark.num_permutation_batches(config),
            self.keccak_stark.num_permutation_batches(config),
        ];
        debug_assert_eq!(ans.len(), Table::num_tables());
        ans
    }

    pub(crate) fn permutation_batch_sizes(&self) -> Vec<usize> {
        let ans = vec![
            self.cpu_stark.permutation_batch_size(),
            self.keccak_stark.permutation_batch_size(),
        ];
        debug_assert_eq!(ans.len(), Table::num_tables());
        ans
    }
}

#[derive(Copy, Clone)]
pub enum Table {
    Cpu = 0,
    Keccak = 1,
}

impl Table {
    pub(crate) fn num_tables() -> usize {
        Table::Keccak as usize + 1
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use itertools::{izip, Itertools};
    use plonky2::field::field_types::Field;
    use plonky2::iop::witness::PartialWitness;
    use plonky2::plonk::circuit_builder::CircuitBuilder;
    use plonky2::plonk::circuit_data::CircuitConfig;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use plonky2::util::timing::TimingTree;
    use rand::{thread_rng, Rng};

    use crate::all_stark::{AllStark, Table};
    use crate::config::StarkConfig;
    use crate::cpu::columns::{KECCAK_INPUT_LIMBS, KECCAK_OUTPUT_LIMBS};
    use crate::cpu::cpu_stark::CpuStark;
    use crate::cross_table_lookup::{Column, CrossTableLookup, TableWithColumns};
    use crate::keccak::keccak_stark::{KeccakStark, NUM_INPUTS, NUM_ROUNDS};
    use crate::proof::AllProof;
    use crate::prover::prove;
    use crate::recursive_verifier::{
        add_virtual_all_proof, set_all_proof_target, verify_proof_circuit,
    };
    use crate::stark::Stark;
    use crate::util::trace_rows_to_poly_values;
    use crate::verifier::verify_proof;
    use crate::{cpu, keccak};

    const D: usize = 2;
    type C = PoseidonGoldilocksConfig;
    type F = <C as GenericConfig<D>>::F;

    fn get_proof(config: &StarkConfig) -> Result<(AllStark<F, D>, AllProof<F, C, D>)> {
        let cpu_stark = CpuStark::<F, D> {
            f: Default::default(),
        };
        let cpu_rows = 256;

        let keccak_stark = KeccakStark::<F, D> {
            f: Default::default(),
        };

        let mut rng = thread_rng();
        let num_keccak_perms = 2;
        let keccak_inputs = (0..num_keccak_perms)
            .map(|_| [0u64; NUM_INPUTS].map(|_| rng.gen()))
            .collect_vec();
        let keccak_trace = keccak_stark.generate_trace(keccak_inputs);
        let keccak_input_limbs: Vec<[F; 2 * NUM_INPUTS]> = (0..num_keccak_perms)
            .map(|i| {
                (0..2 * NUM_INPUTS)
                    .map(|j| {
                        keccak::registers::reg_input_limb(j)
                            .eval_table(&keccak_trace, (i + 1) * NUM_ROUNDS - 1)
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
                        keccak_trace[keccak::registers::reg_output_limb(j)].values
                            [(i + 1) * NUM_ROUNDS - 1]
                    })
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap()
            })
            .collect();

        let mut cpu_trace_rows = vec![];
        for i in 0..cpu_rows {
            let mut cpu_trace_row = [F::ZERO; CpuStark::<F, D>::COLUMNS];
            cpu_trace_row[cpu::columns::IS_CPU_CYCLE] = F::ONE;
            cpu_trace_row[cpu::columns::OPCODE] = F::from_canonical_usize(i);
            cpu_stark.generate(&mut cpu_trace_row);
            cpu_trace_rows.push(cpu_trace_row);
        }
        for i in 0..num_keccak_perms {
            cpu_trace_rows[i][cpu::columns::IS_KECCAK] = F::ONE;
            for (j, input, output) in
                izip!(0..2 * NUM_INPUTS, KECCAK_INPUT_LIMBS, KECCAK_OUTPUT_LIMBS)
            {
                cpu_trace_rows[i][input] = keccak_input_limbs[i][j];
                cpu_trace_rows[i][output] = keccak_output_limbs[i][j];
            }
        }
        let cpu_trace = trace_rows_to_poly_values(cpu_trace_rows);

        let mut cpu_keccak_input_output = cpu::columns::KECCAK_INPUT_LIMBS.collect::<Vec<_>>();
        cpu_keccak_input_output.extend(cpu::columns::KECCAK_OUTPUT_LIMBS);
        let mut keccak_keccak_input_output = (0..2 * NUM_INPUTS)
            .map(keccak::registers::reg_input_limb)
            .collect::<Vec<_>>();
        keccak_keccak_input_output.extend(Column::singles(
            (0..2 * NUM_INPUTS)
                .map(keccak::registers::reg_output_limb)
                .collect(),
        ));
        let cross_table_lookups = vec![CrossTableLookup::new(
            vec![TableWithColumns::new(
                Table::Cpu,
                Column::singles(cpu_keccak_input_output),
                Some(Column::single(cpu::columns::IS_KECCAK)),
            )],
            TableWithColumns::new(
                Table::Keccak,
                keccak_keccak_input_output,
                Some(Column::single(keccak::registers::reg_step(NUM_ROUNDS - 1))),
            ),
            None,
        )];

        let all_stark = AllStark {
            cpu_stark,
            keccak_stark,
            cross_table_lookups,
        };

        let proof = prove::<F, C, D>(
            &all_stark,
            config,
            vec![cpu_trace, keccak_trace],
            vec![vec![]; 2],
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
