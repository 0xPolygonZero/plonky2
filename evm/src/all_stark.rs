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
    use itertools::Itertools;
    use plonky2::field::field_types::Field;
    use plonky2::field::polynomial::PolynomialValues;
    use plonky2::iop::witness::PartialWitness;
    use plonky2::plonk::circuit_builder::CircuitBuilder;
    use plonky2::plonk::circuit_data::CircuitConfig;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use plonky2::util::timing::TimingTree;

    use crate::all_stark::{AllStark, Table};
    use crate::config::StarkConfig;
    use crate::cpu;
    use crate::cpu::cpu_stark::CpuStark;
    use crate::cross_table_lookup::CrossTableLookup;
    use crate::keccak::keccak_stark::{KeccakStark, NUM_ROUNDS, INPUT_LIMBS};
    use crate::proof::AllProof;
    use crate::prover::prove;
    use crate::recursive_verifier::{
        add_virtual_all_proof, set_all_proof_target, verify_proof_circuit,
    };
    use crate::stark::Stark;
    use crate::util::trace_rows_to_poly_values;
    use crate::verifier::verify_proof;

    const D: usize = 2;
    type C = PoseidonGoldilocksConfig;
    type F = <C as GenericConfig<D>>::F;

    fn get_proof(config: &StarkConfig) -> Result<(AllStark<F, D>, AllProof<F, C, D>)> {
        let cpu_stark = CpuStark::<F, D> {
            f: Default::default(),
        };
        let cpu_rows = 1 << 6;

        let keccak_stark = KeccakStark::<F, D> {
            f: Default::default(),
        };
        let keccak_rows = (NUM_ROUNDS + 1).next_power_of_two();

        let mut cpu_trace = vec![PolynomialValues::<F>::zero(cpu_rows); 10];
        
        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);

        let num_inpts = 1;
        let keccak_inputs = (0..num_inpts)
            .map(|_| [0u64; INPUT_LIMBS].map(|_| rng.gen()))
            .collect_vec();
        let keccak_trace = keccak_stark.generate_trace(keccak_inputs);
        
        let vs0: Vec<_> = keccak_trace[3].values[..].into();
        let vs1: Vec<_> = keccak_trace[5].values[..].into();
        
        let start = thread_rng().gen_range(0..cpu_rows - keccak_rows);

        let default = vec![F::ONE; 2];

        cpu_trace[2].values = vec![default[0]; cpu_rows];
        cpu_trace[2].values[start..start + keccak_rows].copy_from_slice(&vs0);
        cpu_trace[4].values = vec![default[1]; cpu_rows];
        cpu_trace[4].values[start..start + keccak_rows].copy_from_slice(&vs1);

        let cross_table_lookups = vec![CrossTableLookup {
            looking_tables: vec![Table::Cpu],
            looking_columns: vec![vec![cpu::columns::OPCODE]],
            looked_table: Table::Keccak,
            looked_columns: vec![keccak_looked_col],
            default,
        }];

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
