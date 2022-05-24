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
    use plonky2::field::field_types::Field;
    use plonky2::field::polynomial::PolynomialValues;
    use plonky2::iop::witness::PartialWitness;
    use plonky2::plonk::circuit_builder::CircuitBuilder;
    use plonky2::plonk::circuit_data::CircuitConfig;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use plonky2::util::timing::TimingTree;
    use rand::{thread_rng, Rng};

    use crate::all_stark::{AllStark, Table};
    use crate::config::StarkConfig;
    use crate::cpu::cpu_stark::CpuStark;
    use crate::cross_table_lookup::CrossTableLookup;
    use crate::keccak::keccak_stark::KeccakStark;
    use crate::proof::AllProof;
    use crate::prover::prove;
    use crate::recursive_verifier::{
        add_virtual_all_proof, set_all_proof_target, verify_proof_circuit,
    };
    use crate::verifier::verify_proof;

    const D: usize = 2;
    type C = PoseidonGoldilocksConfig;
    type F = <C as GenericConfig<D>>::F;

    fn get_proof(config: &StarkConfig) -> Result<(AllStark<F, D>, AllProof<F, C, D>)> {
        let cpu_stark = CpuStark::<F, D> {
            f: Default::default(),
        };
        let cpu_rows = 1 << 4;

        let keccak_stark = KeccakStark::<F, D> {
            f: Default::default(),
        };
        let keccak_rows = 1 << 3;

        let mut cpu_trace = vec![PolynomialValues::zero(cpu_rows); 10];
        let mut keccak_trace = vec![PolynomialValues::zero(keccak_rows); 7];

        let vs0 = (0..keccak_rows)
            .map(F::from_canonical_usize)
            .collect::<Vec<_>>();
        let vs1 = (1..=keccak_rows)
            .map(F::from_canonical_usize)
            .collect::<Vec<_>>();
        let start = thread_rng().gen_range(0..cpu_rows - keccak_rows);

        let default = vec![F::ONE; 2];

        cpu_trace[2].values = vec![default[0]; cpu_rows];
        cpu_trace[2].values[start..start + keccak_rows].copy_from_slice(&vs0);
        cpu_trace[4].values = vec![default[1]; cpu_rows];
        cpu_trace[4].values[start..start + keccak_rows].copy_from_slice(&vs1);

        keccak_trace[3].values[..].copy_from_slice(&vs0);
        keccak_trace[5].values[..].copy_from_slice(&vs1);

        let cross_table_lookups = vec![CrossTableLookup {
            looking_table: Table::Cpu,
            looking_columns: vec![2, 4],
            looked_table: Table::Keccak,
            looked_columns: vec![3, 5],
            default: vec![F::ONE; 2],
        }];

        let all_stark = AllStark {
            cpu_stark,
            keccak_stark,
            cross_table_lookups,
        };

        let proof = prove::<F, C, D>(
            &all_stark,
            &config,
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
    fn test_recursive_all_stark_verifier() -> Result<()> {
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
        let degree_bits = inner_proof
            .stark_proofs
            .iter()
            .map(|proof| proof.proof.recover_degree_bits(inner_config))
            .collect::<Vec<_>>();
        let pt = add_virtual_all_proof(&mut builder, &inner_all_stark, inner_config, &degree_bits);
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
