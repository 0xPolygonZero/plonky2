use std::marker::PhantomData;

use itertools::Itertools;
use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::{decode, registers, simple_logic};
use crate::cross_table_lookup::Column;
use crate::memory::NUM_CHANNELS;
use crate::permutation::PermutationPair;
use crate::stark::Stark;
use crate::vars::{StarkEvaluationTargets, StarkEvaluationVars};

pub fn ctl_data_keccak<F: Field>() -> Vec<Column<F>> {
    let mut res: Vec<_> = registers::KECCAK_INPUT_LIMBS.map(Column::single).collect();
    res.extend(registers::KECCAK_OUTPUT_LIMBS.map(Column::single));
    res
}

pub fn ctl_filter_keccak<F: Field>() -> Column<F> {
    Column::single(registers::IS_KECCAK)
}

pub fn ctl_data_logic<F: Field>() -> Vec<Column<F>> {
    let mut res =
        Column::singles([registers::IS_AND, registers::IS_OR, registers::IS_XOR]).collect_vec();
    res.extend(registers::LOGIC_INPUT0.map(Column::single));
    res.extend(registers::LOGIC_INPUT1.map(Column::single));
    res.extend(registers::LOGIC_OUTPUT.map(Column::single));
    res
}

pub fn ctl_filter_logic<F: Field>() -> Column<F> {
    Column::sum([registers::IS_AND, registers::IS_OR, registers::IS_XOR])
}

pub fn ctl_data_memory<F: Field>(channel: usize) -> Vec<Column<F>> {
    debug_assert!(channel < NUM_CHANNELS);
    let mut cols: Vec<Column<F>> = Column::singles([
        registers::CLOCK,
        registers::mem_is_read(channel),
        registers::mem_addr_context(channel),
        registers::mem_addr_segment(channel),
        registers::mem_addr_virtual(channel),
    ])
    .collect_vec();
    cols.extend(Column::singles(
        (0..8).map(|j| registers::mem_value(channel, j)),
    ));
    cols
}

pub fn ctl_filter_memory<F: Field>(channel: usize) -> Column<F> {
    Column::single(registers::mem_channel_used(channel))
}

#[derive(Copy, Clone)]
pub struct CpuStark<F, const D: usize> {
    pub f: PhantomData<F>,
}

impl<F: RichField, const D: usize> CpuStark<F, D> {
    pub fn generate(&self, local_values: &mut [F; registers::NUM_CPU_COLUMNS]) {
        decode::generate(local_values);
        simple_logic::generate(local_values);
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Stark<F, D> for CpuStark<F, D> {
    const COLUMNS: usize = registers::NUM_CPU_COLUMNS;
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
