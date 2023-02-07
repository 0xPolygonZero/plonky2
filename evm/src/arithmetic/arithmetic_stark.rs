use std::marker::PhantomData;

use itertools::Itertools;
use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::packed::PackedField;
use plonky2::field::polynomial::PolynomialValues;
use plonky2::hash::hash_types::RichField;
use plonky2::util::transpose;

use crate::arithmetic::{addcy, columns, modular, mul, Operation, Traceable};
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::lookup::{eval_lookups, eval_lookups_circuit, permuted_cols};
use crate::permutation::PermutationPair;
use crate::stark::Stark;
use crate::vars::{StarkEvaluationTargets, StarkEvaluationVars};

#[derive(Copy, Clone)]
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

        // For each column c in cols, generate the range-check
        // permutations and put them in the corresponding range-check
        // columns rc_c and rc_c+1.
        for (c, rc_c) in columns::SHARED_COLS.zip(columns::RC_COLS.step_by(2)) {
            let (col_perm, table_perm) = permuted_cols(&cols[c], &cols[columns::RANGE_COUNTER]);
            cols[rc_c].copy_from_slice(&col_perm);
            cols[rc_c + 1].copy_from_slice(&table_perm);
        }
    }

    #[allow(unused)]
    pub(crate) fn generate(&self, operations: Vec<Operation>) -> Vec<PolynomialValues<F>> {
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
        // to accommodate the range check columns.
        for _ in trace_rows.len()..RANGE_MAX {
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

        mul::eval_packed_generic(lv, yield_constr);
        addcy::eval_packed_generic(lv, yield_constr);
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
        mul::eval_ext_circuit(builder, lv, yield_constr);
        addcy::eval_ext_circuit(builder, lv, yield_constr);
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
    use plonky2::field::types::{Field, PrimeField64};
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaCha8Rng;

    use super::{columns, ArithmeticStark};
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
        // 128 % 13 == 11
        let modop = Operation::binary(BinaryOperator::Mod, U256::from(128), U256::from(13));

        // 128 / 13 == 9
        let div = Operation::binary(BinaryOperator::Div, U256::from(128), U256::from(13));
        let ops: Vec<Operation> = vec![add, mulmod, addmod, mul, div, modop];

        let pols = stark.generate(ops);

        // Trace should always have NUM_ARITH_COLUMNS columns and
        // min(RANGE_MAX, operations.len()) rows. In this case there
        // are only 6 rows, so we should have RANGE_MAX rows.
        assert!(
            pols.len() == columns::NUM_ARITH_COLUMNS
                && pols.iter().all(|v| v.len() == super::RANGE_MAX)
        );

        // Each operation has a single word answer that we can check
        let expected_output = [
            // Row (some ops take two rows), col, expected
            (0, columns::GENERAL_REGISTER_2, 579), // ADD_OUTPUT
            (1, columns::MODULAR_OUTPUT, 703),
            (3, columns::MODULAR_OUTPUT, 794),
            (5, columns::MUL_OUTPUT, 56088),
            (6, columns::MODULAR_OUTPUT, 11),
            (8, columns::DIV_OUTPUT, 9),
        ];

        for (row, col, expected) in expected_output {
            // First register should match expected value...
            let first = col.start;
            let out = pols[first].values[row].to_canonical_u64();
            assert_eq!(
                out, expected,
                "expected column {} on row {} to be {} but it was {}",
                first, row, expected, out,
            );
            // ...other registers should be zero
            let rest = col.start + 1..col.end;
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

        let pols = stark.generate(ops);

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

        let pols = stark.generate(ops);

        // Trace should always have NUM_ARITH_COLUMNS columns and
        // min(RANGE_MAX, operations.len()) rows. In this case there
        // are RANGE_MAX operations with two rows each, so 2*RANGE_MAX.
        assert!(
            pols.len() == columns::NUM_ARITH_COLUMNS
                && pols.iter().all(|v| v.len() == 2 * super::RANGE_MAX)
        );
    }
}
