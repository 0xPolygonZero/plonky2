use std::marker::PhantomData;

use ethereum_types::U256;
use itertools::izip;
use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::packed::PackedField;
use plonky2::field::polynomial::PolynomialValues;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::timed;
use plonky2::util::timing::TimingTree;
use plonky2_util::ceil_div_usize;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cross_table_lookup::Column;
use crate::logic::columns::NUM_COLUMNS;
use crate::stark::Stark;
use crate::util::{limb_from_bits_le, limb_from_bits_le_recursive, trace_rows_to_poly_values};
use crate::vars::{StarkEvaluationTargets, StarkEvaluationVars};

// Total number of bits per input/output.
const VAL_BITS: usize = 256;
// Number of bits stored per field element. Ensure that this fits; it is not checked.
pub(crate) const PACKED_LIMB_BITS: usize = 32;
// Number of field elements needed to store each input/output at the specified packing.
const PACKED_LEN: usize = ceil_div_usize(VAL_BITS, PACKED_LIMB_BITS);

pub(crate) mod columns {
    use std::cmp::min;
    use std::ops::Range;

    use super::{PACKED_LEN, PACKED_LIMB_BITS, VAL_BITS};

    pub const IS_AND: usize = 0;
    pub const IS_OR: usize = IS_AND + 1;
    pub const IS_XOR: usize = IS_OR + 1;
    // The inputs are decomposed into bits.
    pub const INPUT0: Range<usize> = (IS_XOR + 1)..(IS_XOR + 1) + VAL_BITS;
    pub const INPUT1: Range<usize> = INPUT0.end..INPUT0.end + VAL_BITS;
    // The result is packed in limbs of `PACKED_LIMB_BITS` bits.
    pub const RESULT: Range<usize> = INPUT1.end..INPUT1.end + PACKED_LEN;

    pub fn limb_bit_cols_for_input(input_bits: Range<usize>) -> impl Iterator<Item = Range<usize>> {
        (0..PACKED_LEN).map(move |i| {
            let start = input_bits.start + i * PACKED_LIMB_BITS;
            let end = min(start + PACKED_LIMB_BITS, input_bits.end);
            start..end
        })
    }

    pub const NUM_COLUMNS: usize = RESULT.end;
}

pub fn ctl_data<F: Field>() -> Vec<Column<F>> {
    let mut res = vec![
        Column::single(columns::IS_AND),
        Column::single(columns::IS_OR),
        Column::single(columns::IS_XOR),
    ];
    res.extend(columns::limb_bit_cols_for_input(columns::INPUT0).map(Column::le_bits));
    res.extend(columns::limb_bit_cols_for_input(columns::INPUT1).map(Column::le_bits));
    res.extend(columns::RESULT.map(Column::single));
    res
}

pub fn ctl_filter<F: Field>() -> Column<F> {
    Column::sum([columns::IS_AND, columns::IS_OR, columns::IS_XOR])
}

#[derive(Copy, Clone, Default)]
pub struct LogicStark<F, const D: usize> {
    pub f: PhantomData<F>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum Op {
    And,
    Or,
    Xor,
}

impl Op {
    pub(crate) fn result(&self, a: U256, b: U256) -> U256 {
        match self {
            Op::And => a & b,
            Op::Or => a | b,
            Op::Xor => a ^ b,
        }
    }
}

#[derive(Debug)]
pub(crate) struct Operation {
    operator: Op,
    input0: U256,
    input1: U256,
    pub(crate) result: U256,
}

impl Operation {
    pub(crate) fn new(operator: Op, input0: U256, input1: U256) -> Self {
        let result = operator.result(input0, input1);
        Operation {
            operator,
            input0,
            input1,
            result,
        }
    }

    fn into_row<F: Field>(self) -> [F; NUM_COLUMNS] {
        let Operation {
            operator,
            input0,
            input1,
            result,
        } = self;
        let mut row = [F::ZERO; NUM_COLUMNS];
        row[match operator {
            Op::And => columns::IS_AND,
            Op::Or => columns::IS_OR,
            Op::Xor => columns::IS_XOR,
        }] = F::ONE;
        for i in 0..256 {
            row[columns::INPUT0.start + i] = F::from_bool(input0.bit(i));
            row[columns::INPUT1.start + i] = F::from_bool(input1.bit(i));
        }
        let result_limbs: &[u64] = result.as_ref();
        for (i, &limb) in result_limbs.iter().enumerate() {
            row[columns::RESULT.start + 2 * i] = F::from_canonical_u32(limb as u32);
            row[columns::RESULT.start + 2 * i + 1] = F::from_canonical_u32((limb >> 32) as u32);
        }
        row
    }
}

impl<F: RichField, const D: usize> LogicStark<F, D> {
    pub(crate) fn generate_trace(
        &self,
        operations: Vec<Operation>,
        min_rows: usize,
        timing: &mut TimingTree,
    ) -> Vec<PolynomialValues<F>> {
        let trace_rows = timed!(
            timing,
            "generate trace rows",
            self.generate_trace_rows(operations, min_rows)
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
        operations: Vec<Operation>,
        min_rows: usize,
    ) -> Vec<[F; NUM_COLUMNS]> {
        let len = operations.len();
        let padded_len = len.max(min_rows).next_power_of_two();

        let mut rows = Vec::with_capacity(padded_len);
        for op in operations {
            rows.push(op.into_row());
        }

        // Pad to a power of two.
        for _ in len..padded_len {
            rows.push([F::ZERO; NUM_COLUMNS]);
        }

        rows
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Stark<F, D> for LogicStark<F, D> {
    const COLUMNS: usize = NUM_COLUMNS;

    fn eval_packed_generic<FE, P, const D2: usize>(
        &self,
        vars: StarkEvaluationVars<FE, P, { Self::COLUMNS }>,
        yield_constr: &mut ConstraintConsumer<P>,
    ) where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>,
    {
        let lv = &vars.local_values;

        // IS_AND, IS_OR, and IS_XOR come from the CPU table, so we assume they're valid.
        let is_and = lv[columns::IS_AND];
        let is_or = lv[columns::IS_OR];
        let is_xor = lv[columns::IS_XOR];

        // The result will be `in0 OP in1 = sum_coeff * (in0 + in1) + and_coeff * (in0 AND in1)`.
        // `AND => sum_coeff = 0, and_coeff = 1`
        // `OR  => sum_coeff = 1, and_coeff = -1`
        // `XOR => sum_coeff = 1, and_coeff = -2`
        let sum_coeff = is_or + is_xor;
        let and_coeff = is_and - is_or - is_xor * FE::TWO;

        // Ensure that all bits are indeed bits.
        for input_bits_cols in [columns::INPUT0, columns::INPUT1] {
            for i in input_bits_cols {
                let bit = lv[i];
                yield_constr.constraint(bit * (bit - P::ONES));
            }
        }

        // Form the result
        for (result_col, x_bits_cols, y_bits_cols) in izip!(
            columns::RESULT,
            columns::limb_bit_cols_for_input(columns::INPUT0),
            columns::limb_bit_cols_for_input(columns::INPUT1),
        ) {
            let x: P = limb_from_bits_le(x_bits_cols.clone().map(|col| lv[col]));
            let y: P = limb_from_bits_le(y_bits_cols.clone().map(|col| lv[col]));

            let x_bits = x_bits_cols.map(|i| lv[i]);
            let y_bits = y_bits_cols.map(|i| lv[i]);

            let x_land_y: P = izip!(0.., x_bits, y_bits)
                .map(|(i, x_bit, y_bit)| x_bit * y_bit * FE::from_canonical_u64(1 << i))
                .sum();
            let x_op_y = sum_coeff * (x + y) + and_coeff * x_land_y;

            yield_constr.constraint(lv[result_col] - x_op_y);
        }
    }

    fn eval_ext_circuit(
        &self,
        builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
        vars: StarkEvaluationTargets<D, { Self::COLUMNS }>,
        yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    ) {
        let lv = &vars.local_values;

        // IS_AND, IS_OR, and IS_XOR come from the CPU table, so we assume they're valid.
        let is_and = lv[columns::IS_AND];
        let is_or = lv[columns::IS_OR];
        let is_xor = lv[columns::IS_XOR];

        // The result will be `in0 OP in1 = sum_coeff * (in0 + in1) + and_coeff * (in0 AND in1)`.
        // `AND => sum_coeff = 0, and_coeff = 1`
        // `OR  => sum_coeff = 1, and_coeff = -1`
        // `XOR => sum_coeff = 1, and_coeff = -2`
        let sum_coeff = builder.add_extension(is_or, is_xor);
        let and_coeff = {
            let and_coeff = builder.sub_extension(is_and, is_or);
            builder.mul_const_add_extension(-F::TWO, is_xor, and_coeff)
        };

        // Ensure that all bits are indeed bits.
        for input_bits_cols in [columns::INPUT0, columns::INPUT1] {
            for i in input_bits_cols {
                let bit = lv[i];
                let constr = builder.mul_sub_extension(bit, bit, bit);
                yield_constr.constraint(builder, constr);
            }
        }

        // Form the result
        for (result_col, x_bits_cols, y_bits_cols) in izip!(
            columns::RESULT,
            columns::limb_bit_cols_for_input(columns::INPUT0),
            columns::limb_bit_cols_for_input(columns::INPUT1),
        ) {
            let x = limb_from_bits_le_recursive(builder, x_bits_cols.clone().map(|i| lv[i]));
            let y = limb_from_bits_le_recursive(builder, y_bits_cols.clone().map(|i| lv[i]));
            let x_bits = x_bits_cols.map(|i| lv[i]);
            let y_bits = y_bits_cols.map(|i| lv[i]);

            let x_land_y = izip!(0usize.., x_bits, y_bits).fold(
                builder.zero_extension(),
                |acc, (i, x_bit, y_bit)| {
                    builder.arithmetic_extension(
                        F::from_canonical_u64(1 << i),
                        F::ONE,
                        x_bit,
                        y_bit,
                        acc,
                    )
                },
            );
            let x_op_y = {
                let x_op_y = builder.mul_extension(sum_coeff, x);
                let x_op_y = builder.mul_add_extension(sum_coeff, y, x_op_y);
                builder.mul_add_extension(and_coeff, x_land_y, x_op_y)
            };
            let constr = builder.sub_extension(lv[result_col], x_op_y);
            yield_constr.constraint(builder, constr);
        }
    }

    fn constraint_degree(&self) -> usize {
        3
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};

    use crate::logic::LogicStark;
    use crate::stark_testing::{test_stark_circuit_constraints, test_stark_low_degree};

    #[test]
    fn test_stark_degree() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type S = LogicStark<F, D>;

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
        type S = LogicStark<F, D>;

        let stark = S {
            f: Default::default(),
        };
        test_stark_circuit_constraints::<F, C, S, D>(stark)
    }
}
