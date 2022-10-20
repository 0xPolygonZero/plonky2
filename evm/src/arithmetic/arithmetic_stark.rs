use std::marker::PhantomData;
use std::ops::Add;

use itertools::Itertools;
use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::packed::PackedField;
use plonky2::hash::hash_types::RichField;

use crate::arithmetic::add;
use crate::arithmetic::columns;
use crate::arithmetic::compare;
use crate::arithmetic::modular;
use crate::arithmetic::mul;
use crate::arithmetic::sub;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::stark::Stark;
use crate::vars::{StarkEvaluationTargets, StarkEvaluationVars};

#[derive(Copy, Clone)]
pub struct ArithmeticStark<F, const D: usize> {
    pub f: PhantomData<F>,
}

impl<F: RichField, const D: usize> ArithmeticStark<F, D> {
    pub fn generate(&self, local_values: &mut [F; columns::NUM_ARITH_COLUMNS]) {
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
            modular::generate(local_values, columns::IS_ADDMOD);
        } else if local_values[columns::IS_SUBMOD].is_one() {
            modular::generate(local_values, columns::IS_SUBMOD);
        } else if local_values[columns::IS_MULMOD].is_one() {
            modular::generate(local_values, columns::IS_MULMOD);
        } else if local_values[columns::IS_MOD].is_one() {
            modular::generate(local_values, columns::IS_MOD);
        } else if local_values[columns::IS_DIV].is_one() {
            modular::generate(local_values, columns::IS_DIV);
        } else {
            todo!("the requested operation has not yet been implemented");
        }
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
        let lv = vars.local_values;
        add::eval_packed_generic(lv, yield_constr);
        sub::eval_packed_generic(lv, yield_constr);
        mul::eval_packed_generic(lv, yield_constr);
        compare::eval_packed_generic(lv, yield_constr);
        modular::eval_packed_generic(lv, yield_constr);
    }

    fn eval_ext_circuit(
        &self,
        builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
        vars: StarkEvaluationTargets<D, { Self::COLUMNS }>,
        yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    ) {
        let lv = vars.local_values;
        add::eval_ext_circuit(builder, lv, yield_constr);
        sub::eval_ext_circuit(builder, lv, yield_constr);
        mul::eval_ext_circuit(builder, lv, yield_constr);
        compare::eval_ext_circuit(builder, lv, yield_constr);
        modular::eval_ext_circuit(builder, lv, yield_constr);
    }

    fn constraint_degree(&self) -> usize {
        3
    }
}
