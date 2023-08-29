// TODO: Remove
#![allow(unused)]

use std::marker::PhantomData;

use itertools::Itertools;
use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::packed::PackedField;
use plonky2::field::polynomial::PolynomialValues;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::timed;
use plonky2::util::timing::TimingTree;

use super::{VALUE_BYTES, VALUE_LIMBS};
use crate::byte_packing::columns::{value_bytes, value_limb, FILTER, NUM_COLUMNS, REMAINING_LEN};
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cross_table_lookup::Column;
use crate::stark::Stark;
use crate::util::trace_rows_to_poly_values;
use crate::vars::{StarkEvaluationTargets, StarkEvaluationVars};

// TODO: change
pub fn ctl_data<F: Field>() -> Vec<Column<F>> {
    let mut res = Column::singles([FILTER, FILTER, FILTER, FILTER]).collect_vec();
    res.extend(Column::singles((0..8).map(value_limb)));
    res.push(Column::single(FILTER));
    res
}

pub fn ctl_filter<F: Field>() -> Column<F> {
    Column::single(FILTER)
}

#[derive(Copy, Clone, Default)]
pub struct BytePackingStark<F, const D: usize> {
    pub(crate) f: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> BytePackingStark<F, D> {
    pub(crate) fn generate_trace(
        &self,
        grouped_bytes: Vec<Vec<u8>>,
        min_rows: usize,
        timing: &mut TimingTree,
    ) -> Vec<PolynomialValues<F>> {
        // Generate most of the trace in row-major form.
        let trace_rows = timed!(
            timing,
            "generate trace rows",
            self.generate_trace_rows(grouped_bytes, min_rows)
        );

        let trace_polys = timed!(
            timing,
            "convert to PolynomialValues",
            trace_rows_to_poly_values(trace_rows)
        );

        trace_polys
    }

    fn generate_trace_rows(
        &self,
        grouped_bytes: Vec<Vec<u8>>,
        min_rows: usize,
    ) -> Vec<[F; NUM_COLUMNS]> {
        let base_len: usize = grouped_bytes.iter().map(|bytes| bytes.len()).sum();
        let mut rows = Vec::with_capacity(base_len.max(min_rows).next_power_of_two());

        for bytes in grouped_bytes {
            rows.extend(self.generate_rows_for_bytes(bytes));
        }

        let padded_rows = rows.len().max(min_rows).next_power_of_two();
        for _ in rows.len()..padded_rows {
            rows.push(self.generate_padding_row());
        }
        rows
    }

    fn generate_rows_for_bytes(&self, bytes: Vec<u8>) -> Vec<[F; NUM_COLUMNS]> {
        let mut rows = Vec::with_capacity(bytes.len());
        let mut row = [F::ZERO; NUM_COLUMNS];
        row[FILTER] = F::ONE;

        for (i, &byte) in bytes.iter().enumerate() {
            row[REMAINING_LEN] = F::from_canonical_usize(bytes.len() - 1);
            row[value_bytes(i)] = F::from_canonical_u8(byte);
            row[value_limb(i / 4)] += F::from_canonical_u32((byte as u32) << (8 * (i % 4)));

            rows.push(row.into());
        }

        rows
    }

    fn generate_padding_row(&self) -> [F; NUM_COLUMNS] {
        [F::ZERO; NUM_COLUMNS]
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Stark<F, D> for BytePackingStark<F, D> {
    const COLUMNS: usize = NUM_COLUMNS;

    fn eval_packed_generic<FE, P, const D2: usize>(
        &self,
        vars: StarkEvaluationVars<FE, P, { Self::COLUMNS }>,
        yield_constr: &mut ConstraintConsumer<P>,
    ) where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>,
    {
        let one = P::ONES;

        // The filter must be boolean.
        let filter = vars.local_values[FILTER];
        yield_constr.constraint(filter * (filter - P::ONES));

        // The remaining length of a byte sequence must decrease by one or be zero.
        let current_remaining_length = vars.local_values[REMAINING_LEN];
        let next_remaining_length = vars.local_values[REMAINING_LEN];
        yield_constr.constraint_transition(
            current_remaining_length * (current_remaining_length - next_remaining_length - one),
        );

        // The remaining length on the last row must be zero.
        let final_remaining_length = vars.local_values[REMAINING_LEN];
        yield_constr.constraint_last_row(final_remaining_length);

        // Each byte must be zero or equal to the previous one when reading through a sequence.
        for i in 0..VALUE_BYTES {
            let current_byte = vars.local_values[value_bytes(i)];
            let next_byte = vars.next_values[value_bytes(i)];
            yield_constr.constraint_transition(next_byte * (next_byte - current_byte));
        }

        // Each limb must correspond to the big-endian u32 value of each chunk of 4 bytes.
        for i in 0..VALUE_LIMBS {
            let current_limb = vars.local_values[value_limb(i)];
            let value = vars.local_values[value_bytes(4 * i)..value_bytes(4 * i + 4)]
                .iter()
                .enumerate()
                .map(|(i, &v)| v * P::Scalar::from_canonical_usize(1 << (8 * (i % 4))))
                .sum::<P>();
            yield_constr.constraint(current_limb - value);
        }
    }

    fn eval_ext_circuit(
        &self,
        builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
        vars: StarkEvaluationTargets<D, { Self::COLUMNS }>,
        yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    ) {
        let one = builder.one_extension();

        // The filter must be boolean.
        let filter = vars.local_values[FILTER];
        let constraint = builder.mul_sub_extension(filter, filter, filter);
        yield_constr.constraint(builder, constraint);

        // The remaining length of a byte sequence must decrease by one or be zero.
        let current_remaining_length = vars.local_values[REMAINING_LEN];
        let next_remaining_length = vars.local_values[REMAINING_LEN];
        let length_diff = builder.sub_extension(current_remaining_length, next_remaining_length);
        let length_diff_minus_one = builder.add_const_extension(length_diff, F::NEG_ONE);
        let constraint = builder.mul_extension(current_remaining_length, length_diff_minus_one);
        yield_constr.constraint(builder, constraint);

        // The remaining length on the last row must be zero.
        let final_remaining_length = vars.local_values[REMAINING_LEN];
        yield_constr.constraint_last_row(builder, final_remaining_length);

        // Each byte must be zero or equal to the previous one when reading through a sequence.
        for i in 0..VALUE_BYTES {
            let current_byte = vars.local_values[value_bytes(i)];
            let next_byte = vars.next_values[value_bytes(i)];
            let byte_diff = builder.sub_extension(current_byte, next_byte);
            let constraint = builder.mul_extension(next_byte, byte_diff);
            yield_constr.constraint(builder, constraint);
        }

        // Each limb must correspond to the big-endian u32 value of each chunk of 4 bytes.
        for i in 0..VALUE_LIMBS {
            let current_limb = vars.local_values[value_limb(i)];
            let mut value = vars.local_values[value_bytes(4 * i)];
            for (i, &v) in vars.local_values[value_bytes(4 * i)..value_bytes(4 * i + 4)]
                .iter()
                .enumerate()
                .skip(1)
            {
                let scaled_v =
                    builder.mul_const_extension(F::from_canonical_usize(1 << (8 * (i % 4))), v);
                value = builder.add_extension(value, scaled_v);
            }
            let byte_diff = builder.sub_extension(current_limb, value);
            yield_constr.constraint(builder, constraint);
        }
    }

    fn constraint_degree(&self) -> usize {
        3
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use anyhow::Result;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};

    use crate::byte_packing::byte_packing_stark::BytePackingStark;
    use crate::stark_testing::{test_stark_circuit_constraints, test_stark_low_degree};

    #[test]
    fn test_stark_degree() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type S = BytePackingStark<F, D>;

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
        type S = BytePackingStark<F, D>;

        let stark = S {
            f: Default::default(),
        };
        test_stark_circuit_constraints::<F, C, S, D>(stark)
    }
}
