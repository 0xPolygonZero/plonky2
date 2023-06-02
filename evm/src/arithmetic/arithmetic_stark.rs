use std::marker::PhantomData;
use std::ops::Range;

use itertools::Itertools;
use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::packed::PackedField;
use plonky2::field::polynomial::PolynomialValues;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::util::transpose;
use static_assertions::const_assert;

use crate::all_stark::Table;
use crate::arithmetic::{addcy, byte, columns, divmod, modular, mul, Operation};
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cross_table_lookup::{Column, TableWithColumns};
use crate::lookup::{eval_lookups, eval_lookups_circuit, permuted_cols};
use crate::permutation::PermutationPair;
use crate::stark::Stark;
use crate::vars::{StarkEvaluationTargets, StarkEvaluationVars};

/// Link the 16-bit columns of the arithmetic table, split into groups
/// of N_LIMBS at a time in `regs`, with the corresponding 32-bit
/// columns of the CPU table. Does this for all ops in `ops`.
///
/// This is done by taking pairs of columns (x, y) of the arithmetic
/// table and combining them as x + y*2^16 to ensure they equal the
/// corresponding 32-bit number in the CPU table.
fn cpu_arith_data_link<F: Field>(ops: &[usize], regs: &[Range<usize>]) -> Vec<Column<F>> {
    let limb_base = F::from_canonical_u64(1 << columns::LIMB_BITS);

    let mut res = Column::singles(ops).collect_vec();

    // The inner for loop below assumes N_LIMBS is even.
    const_assert!(columns::N_LIMBS % 2 == 0);

    for reg_cols in regs {
        // Loop below assumes we're operating on a "register" of N_LIMBS columns.
        debug_assert_eq!(reg_cols.len(), columns::N_LIMBS);

        for i in 0..(columns::N_LIMBS / 2) {
            let c0 = reg_cols.start + 2 * i;
            let c1 = reg_cols.start + 2 * i + 1;
            res.push(Column::linear_combination([(c0, F::ONE), (c1, limb_base)]));
        }
    }
    res
}

pub fn ctl_arithmetic_rows<F: Field>() -> TableWithColumns<F> {
    const ARITH_OPS: [usize; 14] = [
        columns::IS_ADD,
        columns::IS_SUB,
        columns::IS_MUL,
        columns::IS_LT,
        columns::IS_GT,
        columns::IS_ADDFP254,
        columns::IS_MULFP254,
        columns::IS_SUBFP254,
        columns::IS_ADDMOD,
        columns::IS_MULMOD,
        columns::IS_SUBMOD,
        columns::IS_DIV,
        columns::IS_MOD,
        columns::IS_BYTE,
    ];

    const REGISTER_MAP: [Range<usize>; 4] = [
        columns::INPUT_REGISTER_0,
        columns::INPUT_REGISTER_1,
        columns::INPUT_REGISTER_2,
        columns::OUTPUT_REGISTER,
    ];

    // Create the Arithmetic Table whose columns are those of the
    // operations listed in `ops` whose inputs and outputs are given
    // by `regs`, where each element of `regs` is a range of columns
    // corresponding to a 256-bit input or output register (also `ops`
    // is used as the operation filter).
    TableWithColumns::new(
        Table::Arithmetic,
        cpu_arith_data_link(&ARITH_OPS, &REGISTER_MAP),
        Some(Column::sum(ARITH_OPS)),
    )
}

#[derive(Copy, Clone, Default)]
pub struct ArithmeticStark<F, const D: usize> {
    pub f: PhantomData<F>,
}

const RANGE_MAX: usize = 1usize << 16; // Range check strict upper bound

impl<F: RichField, const D: usize> ArithmeticStark<F, D> {
    /// Expects input in *column*-major layout
    fn generate_range_checks(&self, cols: &mut Vec<Vec<F>>) {
        debug_assert!(cols.len() == columns::NUM_ARITH_COLUMNS);

        let n_rows = cols[0].len();
        debug_assert!(cols.iter().all(|col| col.len() == n_rows));

        for i in 0..RANGE_MAX {
            cols[columns::RANGE_COUNTER][i] = F::from_canonical_usize(i);
        }
        for i in RANGE_MAX..n_rows {
            cols[columns::RANGE_COUNTER][i] = F::from_canonical_usize(RANGE_MAX - 1);
        }

        // For each column c in cols, generate the range-check
        // permutations and put them in the corresponding range-check
        // columns rc_c and rc_c+1.
        for (c, rc_c) in columns::SHARED_COLS.zip(columns::RC_COLS.step_by(2)) {
            let (col_perm, table_perm) = permuted_cols(&cols[c], &cols[columns::RANGE_COUNTER]);
            cols[rc_c].copy_from_slice(&col_perm);
            cols[rc_c + 1].copy_from_slice(&table_perm);
        }
    }

    pub(crate) fn generate_trace(&self, operations: Vec<Operation>) -> Vec<PolynomialValues<F>> {
        // The number of rows reserved is the smallest value that's
        // guaranteed to avoid a reallocation: The only ops that use
        // two rows are the modular operations and DIV, so the only
        // way to reach capacity is when every op is modular or DIV
        // (which is obviously unlikely in normal
        // circumstances). (Also need at least RANGE_MAX rows to
        // accommodate range checks.)
        let max_rows = std::cmp::max(2 * operations.len(), RANGE_MAX);
        let mut trace_rows = Vec::with_capacity(max_rows);

        for op in operations {
            let (row1, maybe_row2) = op.to_rows();
            trace_rows.push(row1);

            if let Some(row2) = maybe_row2 {
                trace_rows.push(row2);
            }
        }

        // Pad the trace with zero rows if it doesn't have enough rows
        // to accommodate the range check columns. Also make sure the
        // trace length is a power of two.
        let padded_len = trace_rows.len().next_power_of_two();
        for _ in trace_rows.len()..std::cmp::max(padded_len, RANGE_MAX) {
            trace_rows.push(vec![F::ZERO; columns::NUM_ARITH_COLUMNS]);
        }

        let mut trace_cols = transpose(&trace_rows);
        self.generate_range_checks(&mut trace_cols);

        trace_cols.into_iter().map(PolynomialValues::new).collect()
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Stark<F, D> for ArithmeticStark<F, D> {
    const COLUMNS: usize = columns::NUM_ARITH_COLUMNS;

    fn eval_packed_generic<FE, P, const D2: usize>(
        &self,
        vars: StarkEvaluationVars<FE, P, { Self::COLUMNS }>,
        yield_constr: &mut ConstraintConsumer<P>,
    ) where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>,
    {
        // Range check all the columns
        for col in columns::RC_COLS.step_by(2) {
            eval_lookups(vars, yield_constr, col, col + 1);
        }

        let lv = vars.local_values;
        let nv = vars.next_values;

        // Check the range column: First value must be 0, last row
        // must be 2^16-1, and intermediate rows must increment by 0
        // or 1.
        let rc1 = lv[columns::RANGE_COUNTER];
        let rc2 = nv[columns::RANGE_COUNTER];
        yield_constr.constraint_first_row(rc1);
        let incr = rc2 - rc1;
        yield_constr.constraint_transition(incr * incr - incr);
        let range_max = P::Scalar::from_canonical_u64((RANGE_MAX - 1) as u64);
        yield_constr.constraint_last_row(rc1 - range_max);

        mul::eval_packed_generic(lv, yield_constr);
        addcy::eval_packed_generic(lv, yield_constr);
        divmod::eval_packed(lv, nv, yield_constr);
        modular::eval_packed(lv, nv, yield_constr);
        byte::eval_packed(lv, yield_constr);
    }

    fn eval_ext_circuit(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: StarkEvaluationTargets<D, { Self::COLUMNS }>,
        yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    ) {
        // Range check all the columns
        for col in columns::RC_COLS.step_by(2) {
            eval_lookups_circuit(builder, vars, yield_constr, col, col + 1);
        }

        let lv = vars.local_values;
        let nv = vars.next_values;

        let rc1 = lv[columns::RANGE_COUNTER];
        let rc2 = nv[columns::RANGE_COUNTER];
        yield_constr.constraint_first_row(builder, rc1);
        let incr = builder.sub_extension(rc2, rc1);
        let t = builder.mul_sub_extension(incr, incr, incr);
        yield_constr.constraint_transition(builder, t);
        let range_max =
            builder.constant_extension(F::Extension::from_canonical_usize(RANGE_MAX - 1));
        let t = builder.sub_extension(rc1, range_max);
        yield_constr.constraint_last_row(builder, t);

        mul::eval_ext_circuit(builder, lv, yield_constr);
        addcy::eval_ext_circuit(builder, lv, yield_constr);
        divmod::eval_ext_circuit(builder, lv, nv, yield_constr);
        modular::eval_ext_circuit(builder, lv, nv, yield_constr);
        byte::eval_ext_circuit(builder, lv, yield_constr);
    }

    fn constraint_degree(&self) -> usize {
        3
    }

    fn permutation_pairs(&self) -> Vec<PermutationPair> {
        const START: usize = columns::START_SHARED_COLS;
        const END: usize = START + columns::NUM_SHARED_COLS;
        let mut pairs = Vec::with_capacity(2 * columns::NUM_SHARED_COLS);
        for (c, c_perm) in (START..END).zip_eq(columns::RC_COLS.step_by(2)) {
            pairs.push(PermutationPair::singletons(c, c_perm));
            pairs.push(PermutationPair::singletons(
                c_perm + 1,
                columns::RANGE_COUNTER,
            ));
        }
        pairs
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use ethereum_types::U256;
    use plonky2::field::types::{Field, PrimeField64};
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaCha8Rng;

    use super::{columns, ArithmeticStark};
    use crate::arithmetic::columns::OUTPUT_REGISTER;
    use crate::arithmetic::*;
    use crate::stark_testing::{test_stark_circuit_constraints, test_stark_low_degree};

    #[test]
    fn degree() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type S = ArithmeticStark<F, D>;

        let stark = S {
            f: Default::default(),
        };
        test_stark_low_degree(stark)
    }

    #[test]
    fn circuit() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type S = ArithmeticStark<F, D>;

        let stark = S {
            f: Default::default(),
        };
        test_stark_circuit_constraints::<F, C, S, D>(stark)
    }

    #[test]
    fn basic_trace() {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type S = ArithmeticStark<F, D>;

        let stark = S {
            f: Default::default(),
        };

        // 123 + 456 == 579
        let add = Operation::binary(BinaryOperator::Add, U256::from(123), U256::from(456));
        // (123 * 456) % 1007 == 703
        let mulmod = Operation::ternary(
            TernaryOperator::MulMod,
            U256::from(123),
            U256::from(456),
            U256::from(1007),
        );
        // (1234 + 567) % 1007 == 794
        let addmod = Operation::ternary(
            TernaryOperator::AddMod,
            U256::from(1234),
            U256::from(567),
            U256::from(1007),
        );
        // 123 * 456 == 56088
        let mul = Operation::binary(BinaryOperator::Mul, U256::from(123), U256::from(456));
        // 128 / 13 == 9
        let div = Operation::binary(BinaryOperator::Div, U256::from(128), U256::from(13));

        // 128 < 13 == 0
        let lt1 = Operation::binary(BinaryOperator::Lt, U256::from(128), U256::from(13));
        // 13 < 128 == 1
        let lt2 = Operation::binary(BinaryOperator::Lt, U256::from(13), U256::from(128));
        // 128 < 128 == 0
        let lt3 = Operation::binary(BinaryOperator::Lt, U256::from(128), U256::from(128));

        // 128 % 13 == 11
        let modop = Operation::binary(BinaryOperator::Mod, U256::from(128), U256::from(13));

        // byte(30, 0xABCD) = 0xAB
        let byte = Operation::binary(BinaryOperator::Byte, U256::from(30), U256::from(0xABCD));

        let ops: Vec<Operation> = vec![add, mulmod, addmod, mul, modop, lt1, lt2, lt3, div, byte];

        let pols = stark.generate_trace(ops);

        // Trace should always have NUM_ARITH_COLUMNS columns and
        // min(RANGE_MAX, operations.len()) rows. In this case there
        // are only 6 rows, so we should have RANGE_MAX rows.
        assert!(
            pols.len() == columns::NUM_ARITH_COLUMNS
                && pols.iter().all(|v| v.len() == super::RANGE_MAX)
        );

        // Each operation has a single word answer that we can check
        let expected_output = [
            // Row (some ops take two rows), expected
            (0, 579), // ADD_OUTPUT
            (1, 703),
            (3, 794),
            (5, 56088),
            (6, 11),
            (8, 0),
            (9, 1),
            (10, 0),
            (11, 9),
            (13, 0xAB),
        ];

        for (row, expected) in expected_output {
            // First register should match expected value...
            let first = OUTPUT_REGISTER.start;
            let out = pols[first].values[row].to_canonical_u64();
            assert_eq!(
                out, expected,
                "expected column {} on row {} to be {} but it was {}",
                first, row, expected, out,
            );
            // ...other registers should be zero
            let rest = OUTPUT_REGISTER.start + 1..OUTPUT_REGISTER.end;
            assert!(pols[rest].iter().all(|v| v.values[row] == F::ZERO));
        }
    }

    #[test]
    fn big_traces() {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type S = ArithmeticStark<F, D>;

        let stark = S {
            f: Default::default(),
        };

        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);

        let ops = (0..super::RANGE_MAX)
            .map(|_| {
                Operation::binary(
                    BinaryOperator::Mul,
                    U256::from(rng.gen::<[u8; 32]>()),
                    U256::from(rng.gen::<[u8; 32]>()),
                )
            })
            .collect::<Vec<_>>();

        let pols = stark.generate_trace(ops);

        // Trace should always have NUM_ARITH_COLUMNS columns and
        // min(RANGE_MAX, operations.len()) rows. In this case there
        // are RANGE_MAX operations with one row each, so RANGE_MAX.
        assert!(
            pols.len() == columns::NUM_ARITH_COLUMNS
                && pols.iter().all(|v| v.len() == super::RANGE_MAX)
        );

        let ops = (0..super::RANGE_MAX)
            .map(|_| {
                Operation::ternary(
                    TernaryOperator::MulMod,
                    U256::from(rng.gen::<[u8; 32]>()),
                    U256::from(rng.gen::<[u8; 32]>()),
                    U256::from(rng.gen::<[u8; 32]>()),
                )
            })
            .collect::<Vec<_>>();

        let pols = stark.generate_trace(ops);

        // Trace should always have NUM_ARITH_COLUMNS columns and
        // min(RANGE_MAX, operations.len()) rows. In this case there
        // are RANGE_MAX operations with two rows each, so 2*RANGE_MAX.
        assert!(
            pols.len() == columns::NUM_ARITH_COLUMNS
                && pols.iter().all(|v| v.len() == 2 * super::RANGE_MAX)
        );
    }
}
