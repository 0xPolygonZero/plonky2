use std::marker::PhantomData;

use ethereum_types::U256;
use itertools::Itertools;
use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::packed::PackedField;
use plonky2::field::polynomial::PolynomialValues;
use plonky2::hash::hash_types::RichField;
use plonky2::util::transpose;

use crate::arithmetic::{add, columns, compare, modular, mul, sub};
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::lookup::{eval_lookups, eval_lookups_circuit, permuted_cols};
use crate::permutation::PermutationPair;
use crate::stark::Stark;
use crate::vars::{StarkEvaluationTargets, StarkEvaluationVars};

#[inline]
fn u64_to_array<F: RichField>(out: &mut [F], x: u64) {
    debug_assert!(out.len() == 4);

    const MASK: u64 = (1 << 16) - 1;
    out[0] = F::from_canonical_u64(x & MASK);
    out[1] = F::from_canonical_u64((x >> 16) % MASK);
    out[2] = F::from_canonical_u64((x >> 32) % MASK);
    out[3] = F::from_canonical_u64((x >> 48) % MASK);
}

fn u256_to_array<F: RichField>(out: &mut [F], x: U256) {
    debug_assert!(out.len() == columns::N_LIMBS);

    u64_to_array(&mut out[0..4], x.0[0]);
    u64_to_array(&mut out[4..8], x.0[1]);
    u64_to_array(&mut out[8..12], x.0[2]);
    u64_to_array(&mut out[12..16], x.0[3]);
}

#[derive(Copy, Clone)]
pub struct ArithmeticStark<F, const D: usize> {
    pub f: PhantomData<F>,
}

pub trait Operation<F: RichField> {
    /// Convert operation into one or two rows of the trace.
    ///
    /// Morally these types should be [F; columns::NUM_ARITH_COLUMNS], but we
    /// use vectors because that's what utils::transpose expects.
    fn to_rows(&self) -> (Vec<F>, Option<Vec<F>>);
}

struct SimpleBinaryOp {
    op: usize,
    input0: U256,
    input1: U256,
}

impl SimpleBinaryOp {
    pub fn new(op: usize, input0: U256, input1: U256) -> Self {
        assert!(
            op == columns::IS_ADD
                || op == columns::IS_SUB
                || op == columns::IS_MUL
                || op == columns::IS_LT
                || op == columns::IS_GT
        );
        Self { op, input0, input1 }
    }
}

impl<F: RichField> Operation<F> for SimpleBinaryOp {
    fn to_rows(&self) -> (Vec<F>, Option<Vec<F>>) {
        let mut row = vec![F::ZERO; columns::NUM_ARITH_COLUMNS];
        row[self.op] = F::ONE;

        // Each of these operations uses the same columns for input; the
        // asserts ensure no-one changes this.
        debug_assert!([
            columns::ADD_INPUT_0,
            columns::SUB_INPUT_0,
            columns::MUL_INPUT_0,
            columns::CMP_INPUT_0,
        ]
        .iter()
        .all(|x| *x == columns::GENERAL_INPUT_0));
        debug_assert!([
            columns::ADD_INPUT_1,
            columns::SUB_INPUT_1,
            columns::MUL_INPUT_1,
            columns::CMP_INPUT_1,
        ]
        .iter()
        .all(|x| *x == columns::GENERAL_INPUT_1));

        u256_to_array(&mut row[columns::GENERAL_INPUT_0], self.input0);
        u256_to_array(&mut row[columns::GENERAL_INPUT_1], self.input1);

        // This is ugly, but it avoids the huge amount of boilderplate
        // required to dispatch directly to each add/sub/etc. operation.
        match self.op {
            columns::IS_ADD => add::generate(&mut row),
            columns::IS_SUB => sub::generate(&mut row),
            columns::IS_MUL => mul::generate(&mut row),
            columns::IS_LT | columns::IS_GT => compare::generate(&mut row, self.op),
            _ => panic!("unrecognised operation"),
        }

        (row, None)
    }
}

pub struct ModularBinaryOp {
    op: usize,
    input0: U256,
    input1: U256,
    modulus: U256,
}

impl ModularBinaryOp {
    pub fn new(op: usize, input0: U256, input1: U256, modulus: U256) -> Self {
        assert!(op == columns::IS_ADDMOD || op == columns::IS_SUBMOD || op == columns::IS_MULMOD);
        Self {
            op,
            input0,
            input1,
            modulus,
        }
    }
}

fn modular_to_rows_helper<F: RichField>(
    op: usize,
    input0: U256,
    input1: U256,
    modulus: U256,
) -> (Vec<F>, Option<Vec<F>>) {
    let mut row1 = vec![F::ZERO; columns::NUM_ARITH_COLUMNS];
    let mut row2 = vec![F::ZERO; columns::NUM_ARITH_COLUMNS];

    row1[op] = F::ONE;

    u256_to_array(&mut row1[columns::MODULAR_INPUT_0], input0);
    u256_to_array(&mut row1[columns::MODULAR_INPUT_1], input1);
    u256_to_array(&mut row1[columns::MODULAR_MODULUS], modulus);

    modular::generate(&mut row1, &mut row2, op);

    (row1, Some(row2))
}

impl<F: RichField> Operation<F> for ModularBinaryOp {
    fn to_rows(&self) -> (Vec<F>, Option<Vec<F>>) {
        modular_to_rows_helper(self.op, self.input0, self.input1, self.modulus)
    }
}

pub struct ModOp {
    input: U256,
    modulus: U256,
}

impl<F: RichField> Operation<F> for ModOp {
    fn to_rows(&self) -> (Vec<F>, Option<Vec<F>>) {
        modular_to_rows_helper(columns::IS_MOD, self.input, U256::zero(), self.modulus)
    }
}

pub struct DivOp {
    numerator: U256,
    denominator: U256,
}

impl<F: RichField> Operation<F> for DivOp {
    fn to_rows(&self) -> (Vec<F>, Option<Vec<F>>) {
        let mut row1 = vec![F::ZERO; columns::NUM_ARITH_COLUMNS];
        let mut row2 = vec![F::ZERO; columns::NUM_ARITH_COLUMNS];

        row1[columns::IS_DIV] = F::ONE;

        u256_to_array(&mut row1[columns::DIV_NUMERATOR], self.numerator);
        u256_to_array(&mut row1[columns::DIV_DENOMINATOR], self.denominator);

        modular::generate(&mut row1, &mut row2, columns::IS_DIV);

        (row1, Some(row2))
    }
}

const RANGE_MAX: usize = 1usize << 16; // Range check strict upper bound

impl<F: RichField, const D: usize> ArithmeticStark<F, D> {
    /// Expects input in *column*-major layout
    fn generate_range_checks(&self, cols: &mut Vec<Vec<F>>) {
        debug_assert!(cols.len() == columns::NUM_ARITH_COLUMNS);

        let n_rows = cols[0].len();
        debug_assert!(cols.iter().all(|col| col.len() == n_rows));

        // TODO: This column is constant; do I really need to set it each time?
        for i in 0..RANGE_MAX {
            cols[columns::RANGE_COUNTER][i] = F::from_canonical_usize(i);
        }

        // For each column c in cols, generate the range-check
        // permuations and put them in the corresponding range-check
        // columns rc_c and rc_c+1.
        for (c, rc_c) in (0..cols.len()).zip(columns::RC_COLS.step_by(2)) {
            let (col_perm, table_perm) = permuted_cols(&cols[c], &cols[columns::RANGE_COUNTER]);
            cols[rc_c].copy_from_slice(&col_perm);
            cols[rc_c + 1].copy_from_slice(&table_perm);
        }
    }

    pub fn generate(&self, operations: Vec<&dyn Operation<F>>) -> Vec<PolynomialValues<F>> {
        // The number of rows reserved is the smallest value that's
        // guaranteed to avoid a reallocation: The only ops that use
        // two rows are the modular operations and DIV, so the only
        // way to reach capacity is when every op is modular or DIV
        // (which is obviously unlikely in normal
        // circumstances). (Also need at least RANGE_MAX rows to
        // accommodate range checks.)
        let max_rows = std::cmp::max(2 * operations.len(), RANGE_MAX);
        let mut trace_rows = Vec::with_capacity(max_rows);

        for op in operations.iter() {
            let (row1, maybe_row2) = op.to_rows();
            trace_rows.push(row1);

            if let Some(row2) = maybe_row2 {
                trace_rows.push(row2);
            }
        }

        // Pad the trace with zero rows if it doesn't have enough rows
        // to accommodate the range check columns.
        for _ in trace_rows.len()..RANGE_MAX {
            trace_rows.push(vec![F::ZERO; columns::NUM_ARITH_COLUMNS]);
        }

        let mut trace_cols = transpose(&trace_rows);
        self.generate_range_checks(&mut trace_cols);

        trace_cols
            .into_iter()
            .map(|col| PolynomialValues::new(col))
            .collect()
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

        add::eval_packed_generic(lv, yield_constr);
        sub::eval_packed_generic(lv, yield_constr);
        mul::eval_packed_generic(lv, yield_constr);
        compare::eval_packed_generic(lv, yield_constr);
        modular::eval_packed_generic(lv, nv, yield_constr);
    }

    fn eval_ext_circuit(
        &self,
        builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
        vars: StarkEvaluationTargets<D, { Self::COLUMNS }>,
        yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    ) {
        // Range check all the columns
        for col in columns::RC_COLS.step_by(2) {
            eval_lookups_circuit(builder, vars, yield_constr, col, col + 1);
        }

        let lv = vars.local_values;
        let nv = vars.next_values;
        add::eval_ext_circuit(builder, lv, yield_constr);
        sub::eval_ext_circuit(builder, lv, yield_constr);
        mul::eval_ext_circuit(builder, lv, yield_constr);
        compare::eval_ext_circuit(builder, lv, yield_constr);
        modular::eval_ext_circuit(builder, lv, nv, yield_constr);
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
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};

    use super::{
        columns, ArithmeticStark, DivOp, ModOp, ModularBinaryOp, Operation, SimpleBinaryOp,
    };
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

        let add = SimpleBinaryOp::new(columns::IS_ADD, U256::from(123), U256::from(456));
        let mulmod = ModularBinaryOp::new(
            columns::IS_MULMOD,
            U256::from(123),
            U256::from(456),
            U256::from(1007),
        );
        let submod = ModularBinaryOp::new(
            columns::IS_SUBMOD,
            U256::from(123),
            U256::from(456),
            U256::from(1007),
        );
        let modop = ModOp {
            input: U256::from(128),
            modulus: U256::from(13),
        };
        let div = DivOp {
            numerator: U256::from(128),
            denominator: U256::from(13),
        };
        let ops: Vec<&dyn Operation<F>> = vec![&add, &mulmod, &submod, &div, &modop];
        let pols = stark.generate(ops);
        assert!(
            pols.len() == columns::NUM_ARITH_COLUMNS
                && pols.iter().all(|v| v.len() == super::RANGE_MAX)
        );
    }
}
