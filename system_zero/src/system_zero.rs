use std::marker::PhantomData;

use plonky2::field::extension_field::{Extendable, FieldExtension};
use plonky2::field::packed_field::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use starky::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use starky::permutation::PermutationPair;
use starky::stark::Stark;
use starky::vars::StarkEvaluationTargets;
use starky::vars::StarkEvaluationVars;

use crate::alu::{eval_alu, eval_alu_recursively};
use crate::core_registers::{eval_core_registers, eval_core_registers_recursively};
use crate::lookup::{eval_lookups, eval_lookups_recursively};
use crate::permutation_unit::{eval_permutation_unit, eval_permutation_unit_recursively};
use crate::public_input_layout::NUM_PUBLIC_INPUTS;
use crate::registers::{lookup, NUM_COLUMNS};

#[derive(Copy, Clone)]
pub struct SystemZero<F: RichField + Extendable<D>, const D: usize> {
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> Default for SystemZero<F, D> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Stark<F, D> for SystemZero<F, D> {
    const COLUMNS: usize = NUM_COLUMNS;
    const PUBLIC_INPUTS: usize = NUM_PUBLIC_INPUTS;

    fn eval_packed_generic<FE, P, const D2: usize>(
        &self,
        vars: StarkEvaluationVars<FE, P, NUM_COLUMNS, NUM_PUBLIC_INPUTS>,
        yield_constr: &mut ConstraintConsumer<P>,
    ) where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>,
    {
        eval_core_registers(vars, yield_constr);
        eval_alu(vars, yield_constr);
        eval_permutation_unit::<F, FE, P, D2>(vars, yield_constr);
        eval_lookups(vars, yield_constr);
        // TODO: Other units
    }

    fn eval_ext_recursively(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: StarkEvaluationTargets<D, NUM_COLUMNS, NUM_PUBLIC_INPUTS>,
        yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    ) {
        eval_core_registers_recursively(builder, vars, yield_constr);
        eval_alu_recursively(builder, vars, yield_constr);
        eval_permutation_unit_recursively(builder, vars, yield_constr);
        eval_lookups_recursively(builder, vars, yield_constr);
        // TODO: Other units
    }

    fn constraint_degree(&self) -> usize {
        3
    }

    fn permutation_pairs(&self) -> Vec<PermutationPair> {
        let mut pairs = Vec::new();

        for i in 0..lookup::NUM_LOOKUPS {
            pairs.push(PermutationPair::singletons(
                lookup::col_input(i),
                lookup::col_permuted_input(i),
            ));
            pairs.push(PermutationPair::singletons(
                lookup::col_table(i),
                lookup::col_permuted_table(i),
            ));
        }

        // TODO: Add permutation pairs for memory.

        pairs
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use log::Level;
    use plonky2::field::field_types::Field;
    use plonky2::field::goldilocks_field::GoldilocksField;
    use plonky2::plonk::config::PoseidonGoldilocksConfig;
    use plonky2::util::timing::TimingTree;
    use starky::config::StarkConfig;
    use starky::prover::prove;
    use starky::stark::Stark;
    use starky::stark_testing::test_stark_low_degree;
    use starky::verifier::verify_stark_proof;

    use crate::env::memory::MemoryEnvironment;
    use crate::generate::generate_trace;
    use crate::system_zero::SystemZero;

    #[test]
    fn run() -> Result<()> {
        init_logger();

        type F = GoldilocksField;
        type C = PoseidonGoldilocksConfig;
        const D: usize = 2;

        type S = SystemZero<F, D>;
        let system = S::default();
        let public_inputs = [F::ZERO; S::PUBLIC_INPUTS];
        let config = StarkConfig::standard_fast_config();
        let mut timing = TimingTree::new("prove", Level::Debug);
        let mut env = MemoryEnvironment::new();
        let trace = generate_trace(&mut env);
        let proof = prove::<F, C, S, D>(system, &config, trace, public_inputs, &mut timing)?;

        verify_stark_proof(system, proof, &config)
    }

    #[test]
    fn degree() -> Result<()> {
        type F = GoldilocksField;
        const D: usize = 2;

        type S = SystemZero<F, D>;
        let system = S::default();
        test_stark_low_degree(system)
    }

    fn init_logger() {
        let _ = env_logger::builder().format_timestamp(None).try_init();
    }
}
