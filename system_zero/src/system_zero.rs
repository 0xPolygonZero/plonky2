use std::marker::PhantomData;

use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::packed::PackedField;
use plonky2::field::polynomial::PolynomialValues;
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::timed;
use plonky2::util::timing::TimingTree;
use plonky2::util::transpose;
use starky::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use starky::permutation::PermutationPair;
use starky::stark::Stark;
use starky::vars::{StarkEvaluationTargets, StarkEvaluationVars};

use crate::alu::{eval_alu, eval_alu_circuit, generate_alu};
use crate::core_registers::{
    eval_core_registers, eval_core_registers_circuit, generate_first_row_core_registers,
    generate_next_row_core_registers,
};
use crate::lookup::{eval_lookups, eval_lookups_circuit, generate_lookups};
use crate::memory::TransactionMemory;
use crate::permutation_unit::{
    eval_permutation_unit, eval_permutation_unit_circuit, generate_permutation_unit,
};
use crate::public_input_layout::NUM_PUBLIC_INPUTS;
use crate::registers::{lookup, NUM_COLUMNS};

/// We require at least 2^16 rows as it helps support efficient 16-bit range checks.
const MIN_TRACE_ROWS: usize = 1 << 16;

#[derive(Copy, Clone)]
pub struct SystemZero<F: RichField + Extendable<D>, const D: usize> {
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> SystemZero<F, D> {
    /// Generate the rows of the trace. Note that this does not generate the permuted columns used
    /// in our lookup arguments, as those are computed after transposing to column-wise form.
    fn generate_trace_rows(&self) -> Vec<[F; NUM_COLUMNS]> {
        #[allow(unused)] // TODO
        let memory = TransactionMemory::default();

        let mut row = [F::ZERO; NUM_COLUMNS];
        generate_first_row_core_registers(&mut row);
        generate_alu(&mut row);
        generate_permutation_unit(&mut row);

        let mut trace = Vec::with_capacity(MIN_TRACE_ROWS);

        loop {
            let mut next_row = [F::ZERO; NUM_COLUMNS];
            generate_next_row_core_registers(&row, &mut next_row);
            generate_alu(&mut next_row);
            generate_permutation_unit(&mut next_row);

            trace.push(row);
            row = next_row;

            // TODO: Replace with proper termination condition.
            if trace.len() == (1 << 16) - 1 {
                break;
            }
        }

        trace.push(row);
        trace
    }

    pub fn generate_trace(&self) -> Vec<PolynomialValues<F>> {
        let mut timing = TimingTree::new("generate trace", log::Level::Debug);

        // Generate the witness, except for permuted columns in the lookup argument.
        let trace_rows = timed!(
            &mut timing,
            "generate trace rows",
            self.generate_trace_rows()
        );

        // Transpose from row-wise to column-wise.
        let trace_row_vecs: Vec<_> = timed!(
            &mut timing,
            "convert to Vecs",
            trace_rows.into_iter().map(|row| row.to_vec()).collect()
        );
        let mut trace_col_vecs: Vec<Vec<F>> =
            timed!(&mut timing, "transpose", transpose(&trace_row_vecs));

        // Generate permuted columns in the lookup argument.
        timed!(
            &mut timing,
            "generate lookup columns",
            generate_lookups(&mut trace_col_vecs)
        );

        let trace_polys = timed!(
            &mut timing,
            "convert to PolynomialValues",
            trace_col_vecs
                .into_iter()
                .map(|column| PolynomialValues::new(column))
                .collect()
        );

        timing.print();
        trace_polys
    }
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

    fn eval_ext_circuit(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: StarkEvaluationTargets<D, NUM_COLUMNS, NUM_PUBLIC_INPUTS>,
        yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    ) {
        eval_core_registers_circuit(builder, vars, yield_constr);
        eval_alu_circuit(builder, vars, yield_constr);
        eval_permutation_unit_circuit(builder, vars, yield_constr);
        eval_lookups_circuit(builder, vars, yield_constr);
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
    use plonky2::field::goldilocks_field::GoldilocksField;
    use plonky2::field::types::Field;
    use plonky2::plonk::config::PoseidonGoldilocksConfig;
    use plonky2::util::timing::TimingTree;
    use starky::config::StarkConfig;
    use starky::prover::prove;
    use starky::stark::Stark;
    use starky::stark_testing::test_stark_low_degree;
    use starky::verifier::verify_stark_proof;

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
        let trace = system.generate_trace();
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
