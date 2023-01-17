use ethereum_types::U256;
use plonky2::hash::hash_types::RichField;

use crate::arithmetic::columns::*;
use crate::arithmetic::{add, compare, modular, mul, sub};

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
    op: usize,
    input0: U256,
    input1: U256,
}

impl SimpleBinaryOp {
    pub fn new(op: usize, input0: U256, input1: U256) -> Self {
        assert!(op == IS_ADD || op == IS_SUB || op == IS_MUL || op == IS_LT || op == IS_GT);
        Self { op, input0, input1 }
    }
}

impl<F: RichField> Operation<F> for SimpleBinaryOp {
    fn to_rows(&self) -> (Vec<F>, Option<Vec<F>>) {
        let mut row = vec![F::ZERO; NUM_ARITH_COLUMNS];
        row[self.op] = F::ONE;

        // Each of these operations uses the same columns for input; the
        // asserts ensure no-one changes this.
        debug_assert!([ADD_INPUT_0, SUB_INPUT_0, MUL_INPUT_0, CMP_INPUT_0,]
            .iter()
            .all(|x| *x == GENERAL_INPUT_0));
        debug_assert!([ADD_INPUT_1, SUB_INPUT_1, MUL_INPUT_1, CMP_INPUT_1,]
            .iter()
            .all(|x| *x == GENERAL_INPUT_1));

        u256_to_array(&mut row[GENERAL_INPUT_0], self.input0);
        u256_to_array(&mut row[GENERAL_INPUT_1], self.input1);

        // This is ugly, but it avoids the huge amount of boilderplate
        // required to dispatch directly to each add/sub/etc. operation.
        match self.op {
            IS_ADD => add::generate(&mut row),
            IS_SUB => sub::generate(&mut row),
            IS_MUL => mul::generate(&mut row),
            IS_LT | IS_GT => compare::generate(&mut row, self.op),
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
        assert!(op == IS_ADDMOD || op == IS_SUBMOD || op == IS_MULMOD);
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
    let mut row1 = vec![F::ZERO; NUM_ARITH_COLUMNS];
    let mut row2 = vec![F::ZERO; NUM_ARITH_COLUMNS];

    row1[op] = F::ONE;

    u256_to_array(&mut row1[MODULAR_INPUT_0], input0);
    u256_to_array(&mut row1[MODULAR_INPUT_1], input1);
    u256_to_array(&mut row1[MODULAR_MODULUS], modulus);

    modular::generate(&mut row1, &mut row2, op);

    (row1, Some(row2))
}

impl<F: RichField> Operation<F> for ModularBinaryOp {
    fn to_rows(&self) -> (Vec<F>, Option<Vec<F>>) {
        modular_to_rows_helper(self.op, self.input0, self.input1, self.modulus)
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
