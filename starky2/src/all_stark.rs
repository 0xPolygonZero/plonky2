use std::marker::PhantomData;

use plonky2::field::extension_field::{Extendable, FieldExtension};
use plonky2::field::packed_field::PackedField;
use plonky2::hash::hash_types::RichField;

use crate::config::StarkConfig;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cross_table_lookup::CrossTableLookup;
use crate::permutation::PermutationPair;
use crate::stark::Stark;
use crate::vars::{StarkEvaluationTargets, StarkEvaluationVars};

#[derive(Clone)]
pub struct AllStark<F: RichField + Extendable<D>, const D: usize> {
    pub cpu_stark: CpuStark<F, D>,
    pub keccak_stark: KeccakStark<F, D>,
    pub cross_table_lookups: Vec<CrossTableLookup>,
}

impl<F: RichField + Extendable<D>, const D: usize> AllStark<F, D> {
    pub(crate) fn nums_permutation_zs(&self, config: &StarkConfig) -> Vec<usize> {
        vec![
            self.cpu_stark.num_permutation_batches(config),
            self.keccak_stark.num_permutation_batches(config),
        ]
    }
}

#[derive(Copy, Clone)]
pub struct CpuStark<F, const D: usize> {
    #[allow(dead_code)]
    num_rows: usize,
    f: PhantomData<F>,
}

#[derive(Copy, Clone)]
pub struct KeccakStark<F, const D: usize> {
    #[allow(dead_code)]
    num_rows: usize,
    f: PhantomData<F>,
}

#[derive(Copy, Clone)]
pub enum Table {
    Cpu = 0,
    Keccak = 1,
}

impl<F: RichField + Extendable<D>, const D: usize> Stark<F, D> for CpuStark<F, D> {
    const COLUMNS: usize = 10;
    const PUBLIC_INPUTS: usize = 0;

    fn eval_packed_generic<FE, P, const D2: usize>(
        &self,
        _vars: StarkEvaluationVars<FE, P, { Self::COLUMNS }, { Self::PUBLIC_INPUTS }>,
        _yield_constr: &mut ConstraintConsumer<P>,
    ) where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>,
    {
    }

    fn eval_ext_recursively(
        &self,
        _builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
        _vars: StarkEvaluationTargets<D, { Self::COLUMNS }, { Self::PUBLIC_INPUTS }>,
        _yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    ) {
    }

    fn constraint_degree(&self) -> usize {
        3
    }

    fn permutation_pairs(&self) -> Vec<PermutationPair> {
        vec![PermutationPair::singletons(8, 9)]
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Stark<F, D> for KeccakStark<F, D> {
    const COLUMNS: usize = 7;
    const PUBLIC_INPUTS: usize = 0;

    fn eval_packed_generic<FE, P, const D2: usize>(
        &self,
        _vars: StarkEvaluationVars<FE, P, { Self::COLUMNS }, { Self::PUBLIC_INPUTS }>,
        _yield_constr: &mut ConstraintConsumer<P>,
    ) where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>,
    {
    }

    fn eval_ext_recursively(
        &self,
        _builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
        _vars: StarkEvaluationTargets<D, { Self::COLUMNS }, { Self::PUBLIC_INPUTS }>,
        _yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    ) {
    }

    fn constraint_degree(&self) -> usize {
        3
    }

    fn permutation_pairs(&self) -> Vec<PermutationPair> {
        vec![PermutationPair::singletons(0, 6)]
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use plonky2::field::field_types::Field;
    use plonky2::field::polynomial::PolynomialValues;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use plonky2::util::timing::TimingTree;
    use rand::{thread_rng, Rng};

    use crate::all_stark::{AllStark, CpuStark, KeccakStark, Table};
    use crate::config::StarkConfig;
    use crate::cross_table_lookup::CrossTableLookup;
    use crate::prover::prove;
    use crate::verifier::verify_proof;

    #[test]
    fn test_all_stark() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;

        let config = StarkConfig::standard_fast_config();

        let cpu_stark = CpuStark::<F, D> {
            num_rows: 1 << 4,
            f: Default::default(),
        };
        let keccak_stark = KeccakStark::<F, D> {
            num_rows: 1 << 3,
            f: Default::default(),
        };

        // let mut cpu_trace = vec![PolynomialValues::zero(cpu_stark.num_rows); CpuStark::COLUMNS];
        let mut cpu_trace = vec![PolynomialValues::zero(cpu_stark.num_rows); 10];
        // let mut keccak_trace =
        //     vec![PolynomialValues::zero(keccak_stark.num_rows); KeccakStark::COLUMNS];
        let mut keccak_trace = vec![PolynomialValues::zero(keccak_stark.num_rows); 7];

        let vs0 = (0..keccak_stark.num_rows)
            .map(F::from_canonical_usize)
            .collect::<Vec<_>>();
        let vs1 = (1..=keccak_stark.num_rows)
            .map(F::from_canonical_usize)
            .collect::<Vec<_>>();
        let start = thread_rng().gen_range(0..cpu_stark.num_rows - keccak_stark.num_rows);

        cpu_trace[2].values[start..start + keccak_stark.num_rows].copy_from_slice(&vs0);
        cpu_trace[4].values[start..start + keccak_stark.num_rows].copy_from_slice(&vs1);

        keccak_trace[3].values[..].copy_from_slice(&vs0);
        keccak_trace[5].values[..].copy_from_slice(&vs1);

        let cross_table_lookups = vec![CrossTableLookup {
            looking_table: Table::Cpu,
            looking_columns: vec![2, 4],
            looked_table: Table::Keccak,
            looked_columns: vec![3, 5],
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

        verify_proof(all_stark, proof, &config)
    }
}
