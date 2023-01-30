use ethereum_types::U256;
use plonky2::hash::hash_types::RichField;

use crate::arithmetic::columns::*;
use crate::arithmetic::{add, compare, modular, mul, sub};

#[inline]
fn u64_to_array<F: RichField>(out: &mut [F], x: u64) {
    debug_assert!(LIMB_BITS == 16);
    debug_assert!(out.len() == 4);

    out[0] = F::from_canonical_u16(x as u16);
    out[1] = F::from_canonical_u16((x >> 16) as u16);
    out[2] = F::from_canonical_u16((x >> 32) as u16);
    out[3] = F::from_canonical_u16((x >> 48) as u16);
}

fn u256_to_array<F: RichField>(out: &mut [F], x: U256) {
    debug_assert!(N_LIMBS == 16);
    debug_assert!(out.len() == N_LIMBS);

    u64_to_array(&mut out[0..4], x.0[0]);
    u64_to_array(&mut out[4..8], x.0[1]);
    u64_to_array(&mut out[8..12], x.0[2]);
    u64_to_array(&mut out[12..16], x.0[3]);
}

pub trait Operation<F: RichField> {
    /// Convert operation into one or two rows of the trace.
    ///
    /// Morally these types should be [F; NUM_ARITH_COLUMNS], but we
    /// use vectors because that's what utils::transpose expects.
    fn to_rows(&self) -> (Vec<F>, Option<Vec<F>>);
}

pub struct SimpleBinaryOp {
    /// The operation is identified using the associated filter from
    /// `columns::IS_ADD` etc., stored in `op_filter`.
    op_filter: usize,
    input0: U256,
    input1: U256,
}

impl SimpleBinaryOp {
    pub fn new(op_filter: usize, input0: U256, input1: U256) -> Self {
        assert!(
            op_filter == IS_ADD
                || op_filter == IS_SUB
                || op_filter == IS_MUL
                || op_filter == IS_LT
                || op_filter == IS_GT
        );
        Self {
            op_filter,
            input0,
            input1,
        }
    }
}

impl<F: RichField> Operation<F> for SimpleBinaryOp {
    fn to_rows(&self) -> (Vec<F>, Option<Vec<F>>) {
        let mut row = vec![F::ZERO; NUM_ARITH_COLUMNS];
        row[self.op_filter] = F::ONE;

        // Each of these operations uses the same columns for input; the
        // asserts ensure no-one changes this.
        // TODO: This is ugly; should just remove the other
        // *_INPUT_[01] variables and remove this.
        debug_assert!([ADD_INPUT_0, SUB_INPUT_0, MUL_INPUT_0, CMP_INPUT_0,]
            .iter()
            .all(|x| *x == GENERAL_INPUT_0));
        debug_assert!([ADD_INPUT_1, SUB_INPUT_1, MUL_INPUT_1, CMP_INPUT_1,]
            .iter()
            .all(|x| *x == GENERAL_INPUT_1));

        u256_to_array(&mut row[GENERAL_INPUT_0], self.input0);
        u256_to_array(&mut row[GENERAL_INPUT_1], self.input1);

        // This is ugly, but it avoids the huge amount of boilerplate
        // required to dispatch directly to each add/sub/etc. operation.
        match self.op_filter {
            IS_ADD => add::generate(&mut row),
            IS_SUB => sub::generate(&mut row),
            IS_MUL => mul::generate(&mut row),
            IS_LT | IS_GT => compare::generate(&mut row, self.op_filter),
            _ => panic!("unrecognised operation"),
        }

        (row, None)
    }
}

pub struct ModularBinaryOp {
    op_filter: usize,
    input0: U256,
    input1: U256,
    modulus: U256,
}

impl ModularBinaryOp {
    pub fn new(op_filter: usize, input0: U256, input1: U256, modulus: U256) -> Self {
        assert!(op_filter == IS_ADDMOD || op_filter == IS_SUBMOD || op_filter == IS_MULMOD);
        Self {
            op_filter,
            input0,
            input1,
            modulus,
        }
    }
}

fn modular_to_rows_helper<F: RichField>(
    op_filter: usize,
    input0: U256,
    input1: U256,
    modulus: U256,
) -> (Vec<F>, Option<Vec<F>>) {
    let mut row1 = vec![F::ZERO; NUM_ARITH_COLUMNS];
    let mut row2 = vec![F::ZERO; NUM_ARITH_COLUMNS];

    row1[op_filter] = F::ONE;

    u256_to_array(&mut row1[MODULAR_INPUT_0], input0);
    u256_to_array(&mut row1[MODULAR_INPUT_1], input1);
    u256_to_array(&mut row1[MODULAR_MODULUS], modulus);

    modular::generate(&mut row1, &mut row2, op_filter);

    (row1, Some(row2))
}

impl<F: RichField> Operation<F> for ModularBinaryOp {
    fn to_rows(&self) -> (Vec<F>, Option<Vec<F>>) {
        modular_to_rows_helper(self.op_filter, self.input0, self.input1, self.modulus)
    }
}

pub struct ModOp {
    pub input: U256,
    pub modulus: U256,
}

impl<F: RichField> Operation<F> for ModOp {
    fn to_rows(&self) -> (Vec<F>, Option<Vec<F>>) {
        modular_to_rows_helper(IS_MOD, self.input, U256::zero(), self.modulus)
    }
}

pub struct DivOp {
    pub numerator: U256,
    pub denominator: U256,
}

impl<F: RichField> Operation<F> for DivOp {
    fn to_rows(&self) -> (Vec<F>, Option<Vec<F>>) {
        let mut row1 = vec![F::ZERO; NUM_ARITH_COLUMNS];
        let mut row2 = vec![F::ZERO; NUM_ARITH_COLUMNS];

        row1[IS_DIV] = F::ONE;

        u256_to_array(&mut row1[DIV_NUMERATOR], self.numerator);
        u256_to_array(&mut row1[DIV_DENOMINATOR], self.denominator);

        modular::generate(&mut row1, &mut row2, IS_DIV);

        (row1, Some(row2))
    }
}
