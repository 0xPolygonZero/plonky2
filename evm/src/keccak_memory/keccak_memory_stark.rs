use std::marker::PhantomData;

use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::packed::PackedField;
use plonky2::field::polynomial::PolynomialValues;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::timed;
use plonky2::util::timing::TimingTree;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cross_table_lookup::Column;
use crate::keccak::keccak_stark::NUM_INPUTS;
use crate::keccak_memory::columns::*;
use crate::memory::segments::Segment;
use crate::stark::Stark;
use crate::util::trace_rows_to_poly_values;
use crate::vars::StarkEvaluationTargets;
use crate::vars::StarkEvaluationVars;

pub(crate) fn ctl_looked_data<F: Field>() -> Vec<Column<F>> {
    Column::singles([COL_CONTEXT, COL_SEGMENT, COL_VIRTUAL, COL_READ_TIMESTAMP]).collect()
}

pub(crate) fn ctl_looking_keccak<F: Field>() -> Vec<Column<F>> {
    let input_cols = (0..50).map(|i| {
        Column::le_bytes((0..4).map(|j| {
            let byte_index = i * 4 + j;
            col_input_byte(byte_index)
        }))
    });
    let output_cols = (0..50).map(|i| {
        Column::le_bytes((0..4).map(|j| {
            let byte_index = i * 4 + j;
            col_output_byte(byte_index)
        }))
    });
    input_cols.chain(output_cols).collect()
}

pub(crate) fn ctl_looking_memory<F: Field>(i: usize, is_read: bool) -> Vec<Column<F>> {
    let mut res = vec![Column::constant(F::from_bool(is_read))];
    res.extend(Column::singles([COL_CONTEXT, COL_SEGMENT, COL_VIRTUAL]));

    res.push(Column::single(col_input_byte(i)));
    // Since we're reading or writing a single byte, the higher limbs must be zero.
    res.extend((1..8).map(|_| Column::zero()));

    // Since COL_READ_TIMESTAMP is the read time, we add 1 if this is a write.
    let is_write_f = F::from_bool(!is_read);
    res.push(Column::linear_combination_with_constant(
        [(COL_READ_TIMESTAMP, F::ONE)],
        is_write_f,
    ));

    assert_eq!(
        res.len(),
        crate::memory::memory_stark::ctl_data::<F>().len()
    );
    res
}

/// CTL filter used for both directions (looked and looking).
pub(crate) fn ctl_filter<F: Field>() -> Column<F> {
    Column::single(COL_IS_REAL)
}

/// Information about a Keccak memory operation needed for witness generation.
#[derive(Debug)]
pub(crate) struct KeccakMemoryOp {
    // The address at which we will read inputs and write outputs.
    pub(crate) context: usize,
    pub(crate) segment: Segment,
    pub(crate) virt: usize,

    /// The timestamp at which inputs should be read from memory.
    /// Outputs will be written at the following timestamp.
    pub(crate) read_timestamp: usize,

    /// The input that was read at that address.
    pub(crate) input: [u64; NUM_INPUTS],
    pub(crate) output: [u64; NUM_INPUTS],
}

#[derive(Copy, Clone, Default)]
pub struct KeccakMemoryStark<F, const D: usize> {
    pub(crate) f: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> KeccakMemoryStark<F, D> {
    #[allow(unused)] // TODO: Should be used soon.
    pub(crate) fn generate_trace(
        &self,
        operations: Vec<KeccakMemoryOp>,
        min_rows: usize,
    ) -> Vec<PolynomialValues<F>> {
        let mut timing = TimingTree::new("generate trace", log::Level::Debug);

        // Generate the witness row-wise.
        let trace_rows = timed!(
            &mut timing,
            "generate trace rows",
            self.generate_trace_rows(operations, min_rows)
        );

        let trace_polys = timed!(
            &mut timing,
            "convert to PolynomialValues",
            trace_rows_to_poly_values(trace_rows)
        );

        timing.print();
        trace_polys
    }

    fn generate_trace_rows(
        &self,
        operations: Vec<KeccakMemoryOp>,
        min_rows: usize,
    ) -> Vec<[F; NUM_COLUMNS]> {
        let num_rows = operations.len().max(min_rows).next_power_of_two();
        let mut rows = Vec::with_capacity(num_rows);
        for op in operations {
            rows.push(self.generate_row_for_op(op));
        }

        let padding_row = self.generate_padding_row();
        for _ in rows.len()..num_rows {
            rows.push(padding_row);
        }
        rows
    }

    fn generate_row_for_op(&self, op: KeccakMemoryOp) -> [F; NUM_COLUMNS] {
        let mut row = [F::ZERO; NUM_COLUMNS];
        row[COL_IS_REAL] = F::ONE;
        row[COL_CONTEXT] = F::from_canonical_usize(op.context);
        row[COL_SEGMENT] = F::from_canonical_usize(op.segment as usize);
        row[COL_VIRTUAL] = F::from_canonical_usize(op.virt);
        row[COL_READ_TIMESTAMP] = F::from_canonical_usize(op.read_timestamp);
        for i in 0..25 {
            let input_u64 = op.input[i];
            let output_u64 = op.output[i];
            for j in 0..8 {
                let byte_index = i * 8 + j;
                row[col_input_byte(byte_index)] = F::from_canonical_u8(input_u64.to_le_bytes()[j]);
                row[col_output_byte(byte_index)] =
                    F::from_canonical_u8(output_u64.to_le_bytes()[j]);
            }
        }
        row
    }

    fn generate_padding_row(&self) -> [F; NUM_COLUMNS] {
        // We just need COL_IS_REAL to be zero, which it is by default.
        // The other fields will have no effect.
        [F::ZERO; NUM_COLUMNS]
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Stark<F, D> for KeccakMemoryStark<F, D> {
    const COLUMNS: usize = NUM_COLUMNS;

    fn eval_packed_generic<FE, P, const D2: usize>(
        &self,
        vars: StarkEvaluationVars<FE, P, { Self::COLUMNS }>,
        yield_constr: &mut ConstraintConsumer<P>,
    ) where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>,
    {
        // is_real must be 0 or 1.
        let is_real = vars.local_values[COL_IS_REAL];
        yield_constr.constraint(is_real * (is_real - P::ONES));
    }

    fn eval_ext_circuit(
        &self,
        builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
        vars: StarkEvaluationTargets<D, { Self::COLUMNS }>,
        yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    ) {
        // is_real must be 0 or 1.
        let is_real = vars.local_values[COL_IS_REAL];
        let constraint = builder.mul_sub_extension(is_real, is_real, is_real);
        yield_constr.constraint(builder, constraint);
    }

    fn constraint_degree(&self) -> usize {
        2
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};

    use crate::keccak_memory::keccak_memory_stark::KeccakMemoryStark;
    use crate::stark_testing::{test_stark_circuit_constraints, test_stark_low_degree};

    #[test]
    fn test_stark_degree() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type S = KeccakMemoryStark<F, D>;

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
        type S = KeccakMemoryStark<F, D>;

        let stark = S {
            f: Default::default(),
        };
        test_stark_circuit_constraints::<F, C, S, D>(stark)
    }
}
