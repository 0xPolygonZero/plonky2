use std::marker::PhantomData;

use itertools::Itertools;
use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::{columns, decode, simple_logic};
use crate::cross_table_lookup::Column;
use crate::permutation::PermutationPair;
use crate::stark::Stark;
use crate::vars::{StarkEvaluationTargets, StarkEvaluationVars};

pub fn ctl_data_keccak<F: Field>() -> Vec<Column<F>> {
    let mut res: Vec<_> = columns::KECCAK_INPUT_LIMBS.map(Column::single).collect();
    res.extend(columns::KECCAK_OUTPUT_LIMBS.map(Column::single));
    res
}

pub fn ctl_filter_keccak<F: Field>() -> Column<F> {
    Column::single(columns::IS_KECCAK)
}

pub fn ctl_data_logic<F: Field>() -> Vec<Column<F>> {
    let mut res = Column::singles([columns::IS_AND, columns::IS_OR, columns::IS_XOR]).collect_vec();
    res.extend(columns::LOGIC_INPUT0.map(Column::single));
    res.extend(columns::LOGIC_INPUT1.map(Column::single));
    res.extend(columns::LOGIC_OUTPUT.map(Column::single));
    res
}

pub fn ctl_filter_logic<F: Field>() -> Column<F> {
    Column::sum([columns::IS_AND, columns::IS_OR, columns::IS_XOR])
}

pub fn ctl_data_memory<F: Field>(op: usize) -> Vec<Column<F>> {
    let mut cols: Vec<Column<F>> = Column::singles([
        columns::CLOCK,
        columns::memop_is_read(op),
        columns::memop_addr_context(op),
        columns::memop_addr_segment(op),
        columns::memop_addr_virtual(op),
    ])
    .collect_vec();
    cols.extend(Column::singles((0..8).map(|j| columns::memop_value(op, j))));
    cols
}

pub fn ctl_filter_memory<F: Field>(op: usize) -> Column<F> {
    Column::single(columns::uses_memop(op))
}

#[derive(Copy, Clone)]
pub struct CpuStark<F, const D: usize> {
    pub f: PhantomData<F>,
}

impl<F: RichField, const D: usize> CpuStark<F, D> {
    pub fn generate(&self, local_values: &mut [F; columns::NUM_CPU_COLUMNS]) {
        decode::generate(local_values);
        simple_logic::generate(local_values);
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Stark<F, D> for CpuStark<F, D> {
    const COLUMNS: usize = columns::NUM_CPU_COLUMNS;
    const PUBLIC_INPUTS: usize = 0;

    fn eval_packed_generic<FE, P, const D2: usize>(
        &self,
        vars: StarkEvaluationVars<FE, P, { Self::COLUMNS }, { Self::PUBLIC_INPUTS }>,
        yield_constr: &mut ConstraintConsumer<P>,
    ) where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>,
    {
        decode::eval_packed_generic(vars.local_values, yield_constr);
        simple_logic::eval_packed(vars.local_values, yield_constr);
    }

    fn eval_ext_circuit(
        &self,
        builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
        vars: StarkEvaluationTargets<D, { Self::COLUMNS }, { Self::PUBLIC_INPUTS }>,
        yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    ) {
        decode::eval_ext_circuit(builder, vars.local_values, yield_constr);
        simple_logic::eval_ext_circuit(builder, vars.local_values, yield_constr);
    }

    fn constraint_degree(&self) -> usize {
        3
    }

    fn permutation_pairs(&self) -> Vec<PermutationPair> {
        vec![]
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
