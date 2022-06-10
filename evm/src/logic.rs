use std::marker::PhantomData;

use plonky2::field::extension_field::{Extendable, FieldExtension};
use plonky2::field::packed_field::PackedField;
use plonky2::hash::hash_types::RichField;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::stark::Stark;
use crate::vars::{StarkEvaluationTargets, StarkEvaluationVars};

// Total number of bits per input/output.
const VAL_BITS: usize = 256;
// Number of bits stored per field element. Ensure that this fits; it is not checked.
const PACKED_LIMB_BITS: usize = 16;
// Number of field elements needed to store each input/output at the specified packing.
const PACKED_LEN: usize = (VAL_BITS + PACKED_LIMB_BITS - 1) / PACKED_LIMB_BITS;
// Number of bits in the last limb, which may be smaller than `PACKED_LIMB_BITS`.
const LAST_LIMB_BITS: usize = VAL_BITS - PACKED_LIMB_BITS * (PACKED_LEN - 1);

pub(crate) mod columns {
    use std::ops::Range;

    use super::{PACKED_LEN, VAL_BITS};

    pub const IS_AND: usize = 0;
    pub const IS_OR: usize = IS_AND + 1;
    pub const IS_XOR: usize = IS_OR + 1;
    pub const AND_COEFF: usize = IS_XOR + 1;
    pub const INPUT0_PACKED: Range<usize> = (AND_COEFF + 1)..(AND_COEFF + 1) + PACKED_LEN;
    pub const INPUT1_PACKED: Range<usize> = INPUT0_PACKED.end..INPUT0_PACKED.end + PACKED_LEN;
    pub const RESULT: Range<usize> = INPUT1_PACKED.end..INPUT1_PACKED.end + PACKED_LEN;
    pub const INPUT0_BITS: Range<usize> = RESULT.end..RESULT.end + VAL_BITS;
    pub const INPUT1_BITS: Range<usize> = INPUT0_BITS.end..INPUT0_BITS.end + VAL_BITS;

    pub const NUM_COLUMNS: usize = INPUT1_BITS.end;
}

#[derive(Copy, Clone)]
pub struct LogicStark<F, const D: usize> {
    pub f: PhantomData<F>,
}

impl<F: RichField, const D: usize> LogicStark<F, D> {
    pub fn generate(&self, lv: &mut [F; columns::NUM_COLUMNS]) {
        let is_and = lv[columns::IS_AND].to_canonical_u64();
        assert!(is_and <= 1);
        let is_or = lv[columns::IS_OR].to_canonical_u64();
        assert!(is_or <= 1);
        let is_xor = lv[columns::IS_XOR].to_canonical_u64();
        assert!(is_xor <= 1);
        assert_eq!(is_and + is_or + is_xor, 1);

        for packed_input_cols in [columns::INPUT0_PACKED, columns::INPUT1_PACKED] {
            let packed_input = packed_input_cols.map(|i| lv[i].to_canonical_u64());
            for (i, packed_limb) in packed_input.enumerate() {
                let bits_in_limb = if i == PACKED_LEN - 1 {
                    LAST_LIMB_BITS
                } else {
                    PACKED_LIMB_BITS
                };
                assert_eq!(packed_limb >> bits_in_limb, 0);
            }
        }

        for (packed_input_cols, bit_cols) in [
            (columns::INPUT0_PACKED, columns::INPUT0_BITS),
            (columns::INPUT1_PACKED, columns::INPUT1_BITS),
        ] {
            for (i, limb_col) in packed_input_cols.enumerate() {
                let limb = lv[limb_col].to_canonical_u64();
                let limb_bits_cols = bit_cols
                    .clone()
                    .skip(i * PACKED_LIMB_BITS)
                    .take(PACKED_LIMB_BITS);
                for (j, col) in limb_bits_cols.enumerate() {
                    let bit = (limb >> j) & 1;
                    lv[col] = F::from_canonical_u64(bit);
                }
            }
        }

        for (res_col, (limb_in0_col, limb_in1_col)) in
            columns::RESULT.zip(columns::INPUT0_PACKED.zip(columns::INPUT1_PACKED))
        {
            let limb_in0 = lv[limb_in0_col].to_canonical_u64();
            let limb_in1 = lv[limb_in1_col].to_canonical_u64();
            let res = if is_and == 1 {
                limb_in0 & limb_in1
            } else if is_or == 1 {
                limb_in0 | limb_in1
            } else if is_xor == 1 {
                limb_in0 ^ limb_in1
            } else {
                panic!()
            };
            lv[res_col] = F::from_canonical_u64(res);
        }
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Stark<F, D> for LogicStark<F, D> {
    const COLUMNS: usize = columns::NUM_COLUMNS;
    const PUBLIC_INPUTS: usize = 0;

    fn eval_packed_generic<FE, P, const D2: usize>(
        &self,
        vars: StarkEvaluationVars<FE, P, { Self::COLUMNS }, { Self::PUBLIC_INPUTS }>,
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

        for (input_bits_cols, input_packed_cols) in [
            (columns::INPUT0_BITS, columns::INPUT0_PACKED),
            (columns::INPUT1_BITS, columns::INPUT1_PACKED),
        ] {
            // Ensure that all bits are indeed bits.
            for i in input_bits_cols.clone() {
                yield_constr.constraint(lv[i] * (lv[i] - P::ONES));
            }

            // Check that the bits match the packed inputs.
            let mut reconstructed_inputs = [P::ZEROS; PACKED_LEN];
            for (bit_i, col_i) in input_bits_cols.enumerate() {
                let packed_i = bit_i / PACKED_LIMB_BITS;
                let bit_in_packed = bit_i % PACKED_LIMB_BITS;
                reconstructed_inputs[packed_i] +=
                    lv[col_i] * FE::from_canonical_u64(1 << bit_in_packed);
            }
            for (col_i, reconstructed) in input_packed_cols.zip(reconstructed_inputs) {
                yield_constr.constraint(lv[col_i] - reconstructed);
            }
        }

        // Form the result
        for (i, (result_col, (x_col, y_col))) in columns::RESULT
            .zip(columns::INPUT0_PACKED.zip(columns::INPUT1_PACKED))
            .enumerate()
        {
            let x = lv[x_col];
            let y = lv[y_col];

            let x_bits_cols = columns::INPUT0_PACKED
                .skip(i * PACKED_LIMB_BITS)
                .take(PACKED_LIMB_BITS);
            let y_bits_cols = columns::INPUT1_PACKED
                .skip(i * PACKED_LIMB_BITS)
                .take(PACKED_LIMB_BITS);
            let x_bits = x_bits_cols.map(|i| lv[i]);
            let y_bits = y_bits_cols.map(|i| lv[i]);

            let x_land_y: P = x_bits
                .zip(y_bits)
                .enumerate()
                .map(|(i, (x_bit, y_bit))| x_bit * y_bit * FE::from_canonical_u64(1 << i))
                .sum();
            let x_op_y = sum_coeff * (x + y) + and_coeff * x_land_y;

            yield_constr.constraint(lv[result_col] - x_op_y);
        }
    }

    fn eval_ext_circuit(
        &self,
        builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
        vars: StarkEvaluationTargets<D, { Self::COLUMNS }, { Self::PUBLIC_INPUTS }>,
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

        for (input_bits_cols, input_packed_cols) in [
            (columns::INPUT0_BITS, columns::INPUT0_PACKED),
            (columns::INPUT1_BITS, columns::INPUT1_PACKED),
        ] {
            // Ensure that all bits are indeed bits.
            for i in input_bits_cols.clone() {
                let constr = builder.mul_sub_extension(lv[i], lv[i], lv[i]);
                yield_constr.constraint(builder, constr);
            }

            // Check that the bits match the packed inputs.
            let mut reconstructed_inputs = [builder.zero_extension(); PACKED_LEN];
            for (bit_i, col_i) in input_bits_cols.enumerate() {
                let packed_i = bit_i / PACKED_LIMB_BITS;
                let bit_in_packed = bit_i % PACKED_LIMB_BITS;
                reconstructed_inputs[packed_i] = builder.mul_const_add_extension(
                    F::from_canonical_u64(1 << bit_in_packed),
                    lv[col_i],
                    reconstructed_inputs[packed_i],
                );
            }
            for (col_i, reconstructed) in input_packed_cols.zip(reconstructed_inputs) {
                let constr = builder.sub_extension(lv[col_i], reconstructed);
                yield_constr.constraint(builder, constr);
            }
        }

        // Form the result
        for (i, (result_col, (x_col, y_col))) in columns::RESULT
            .zip(columns::INPUT0_PACKED.zip(columns::INPUT1_PACKED))
            .enumerate()
        {
            let x = lv[x_col];
            let y = lv[y_col];

            let x_bits_cols = columns::INPUT0_PACKED
                .skip(i * PACKED_LIMB_BITS)
                .take(PACKED_LIMB_BITS);
            let y_bits_cols = columns::INPUT1_PACKED
                .skip(i * PACKED_LIMB_BITS)
                .take(PACKED_LIMB_BITS);
            let x_bits = x_bits_cols.map(|i| lv[i]);
            let y_bits = y_bits_cols.map(|i| lv[i]);

            let x_land_y = x_bits.zip(y_bits).enumerate().fold(
                builder.zero_extension(),
                |acc, (i, (x_bit, y_bit))| {
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
