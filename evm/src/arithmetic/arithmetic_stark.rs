use std::marker::PhantomData;
use std::ops::Add;

use itertools::Itertools;
use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::packed::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::util::transpose;

use crate::arithmetic::{add, columns, compare, modular, mul, sub};
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::lookup::{eval_lookups, eval_lookups_circuit, permuted_cols};
use crate::permutation::PermutationPair;
use crate::stark::Stark;
use crate::vars::{StarkEvaluationTargets, StarkEvaluationVars};

#[derive(Copy, Clone)]
pub struct ArithmeticStark<F, const D: usize> {
    pub f: PhantomData<F>,
}

impl<F: RichField, const D: usize> ArithmeticStark<F, D> {
    /// Expects input in *column*-major layout
    fn generate_range_checks(&self, cols: &mut Vec<Vec<F>>) {
        assert!(cols.len() == columns::NUM_SHARED_COLS);

        // TODO: This column is constant; do I really need to set it each time?
        // TODO: Could I just append this to cols instead? It would make the resizing
        // for loop below simpler.
        cols[columns::RANGE_COUNTER] = (0..1 << 16).map(|i| F::from_canonical_usize(i)).collect();

        // All columns should be the same length, but we may need to pad to
        // ensure that length is bigger than the range check length.
        assert!(
            columns::RANGE_COUNTER != 0,
            "range counter column cannot be first in the table"
        );
        let old_len = cols[0].len();
        let new_len = std::cmp::max(1 << 16, old_len);
        for col in cols.iter_mut() {
            // FIXME: one of the columns is the RANGE_COUNTER, whose
            // length will not equal old_len
            debug_assert!(col.len() == old_len);
            col.resize(new_len, F::ZERO);
        }

        // For each column c in cols, generate the range-check
        // permuations and put them in columns rc_c and rc_c+1.
        for (c, rc_c) in (0..cols.len()).zip(columns::RC_COLS.step_by(2)) {
            let (col_perm, table_perm) = permuted_cols(&cols[c], &cols[columns::RANGE_COUNTER]);
            cols[rc_c].copy_from_slice(&col_perm);
            cols[rc_c + 1].copy_from_slice(&table_perm);
        }
    }

    pub fn generate(
        &self,
        local_values: &mut [F; columns::NUM_ARITH_COLUMNS],
        next_values: &mut [F; columns::NUM_ARITH_COLUMNS],
    ) {
        // Check that at most one operation column is "one" and that the
        // rest are "zero".
        assert_eq!(
            columns::ALL_OPERATIONS
                .iter()
                .map(|&c| {
                    if local_values[c] == F::ONE {
                        Ok(1u64)
                    } else if local_values[c] == F::ZERO {
                        Ok(0u64)
                    } else {
                        Err("column was not 0 nor 1")
                    }
                })
                .fold_ok(0u64, Add::add),
            Ok(1)
        );

        if local_values[columns::IS_ADD].is_one() {
            add::generate(local_values);
        } else if local_values[columns::IS_SUB].is_one() {
            sub::generate(local_values);
        } else if local_values[columns::IS_MUL].is_one() {
            mul::generate(local_values);
        } else if local_values[columns::IS_LT].is_one() {
            compare::generate(local_values, columns::IS_LT);
        } else if local_values[columns::IS_GT].is_one() {
            compare::generate(local_values, columns::IS_GT);
        } else if local_values[columns::IS_ADDMOD].is_one() {
            modular::generate(local_values, next_values, columns::IS_ADDMOD);
        } else if local_values[columns::IS_SUBMOD].is_one() {
            modular::generate(local_values, next_values, columns::IS_SUBMOD);
        } else if local_values[columns::IS_MULMOD].is_one() {
            modular::generate(local_values, next_values, columns::IS_MULMOD);
        } else if local_values[columns::IS_MOD].is_one() {
            modular::generate(local_values, next_values, columns::IS_MOD);
        } else if local_values[columns::IS_DIV].is_one() {
            modular::generate(local_values, next_values, columns::IS_DIV);
        } else {
            panic!("the requested operation should not be handled by the arithmetic table");
        }

        // FIXME: need to transpose before passing to the range check code
        let mut local_values_cols = vec![vec![F::ZERO]]; //transpose(local_values);
        self.generate_range_checks(&mut local_values_cols);
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
