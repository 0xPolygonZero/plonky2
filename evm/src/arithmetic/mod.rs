use ethereum_types::U256;
use plonky2::field::types::PrimeField64;

use crate::extension_tower::BN_BASE;
use crate::util::{addmod, mulmod, submod};

mod addcy;
mod byte;
mod divmod;
mod modular;
mod mul;
mod utils;

pub mod arithmetic_stark;
pub(crate) mod columns;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BinaryOperator {
    Add,
    Mul,
    Sub,
    Div,
    Mod,
    Lt,
    Gt,
    AddFp254,
    MulFp254,
    SubFp254,
    Byte,
}

impl BinaryOperator {
    pub(crate) fn result(&self, input0: U256, input1: U256) -> U256 {
        match self {
            BinaryOperator::Add => input0.overflowing_add(input1).0,
            BinaryOperator::Mul => input0.overflowing_mul(input1).0,
            BinaryOperator::Sub => input0.overflowing_sub(input1).0,
            BinaryOperator::Div => {
                if input1.is_zero() {
                    U256::zero()
                } else {
                    input0 / input1
                }
            }
            BinaryOperator::Mod => {
                if input1.is_zero() {
                    U256::zero()
                } else {
                    input0 % input1
                }
            }
            BinaryOperator::Lt => U256::from((input0 < input1) as u8),
            BinaryOperator::Gt => U256::from((input0 > input1) as u8),
            BinaryOperator::AddFp254 => addmod(input0, input1, BN_BASE),
            BinaryOperator::MulFp254 => mulmod(input0, input1, BN_BASE),
            BinaryOperator::SubFp254 => submod(input0, input1, BN_BASE),
            BinaryOperator::Byte => {
                if input0 >= 32.into() {
                    U256::zero()
                } else {
                    input1.byte(31 - input0.as_usize()).into()
                }
            }
        }
    }

    pub(crate) fn row_filter(&self) -> usize {
        match self {
            BinaryOperator::Add => columns::IS_ADD,
            BinaryOperator::Mul => columns::IS_MUL,
            BinaryOperator::Sub => columns::IS_SUB,
            BinaryOperator::Div => columns::IS_DIV,
            BinaryOperator::Mod => columns::IS_MOD,
            BinaryOperator::Lt => columns::IS_LT,
            BinaryOperator::Gt => columns::IS_GT,
            BinaryOperator::AddFp254 => columns::IS_ADDFP254,
            BinaryOperator::MulFp254 => columns::IS_MULFP254,
            BinaryOperator::SubFp254 => columns::IS_SUBFP254,
            BinaryOperator::Byte => columns::IS_BYTE,
        }
    }
}

#[allow(clippy::enum_variant_names)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TernaryOperator {
    AddMod,
    MulMod,
    SubMod,
}

impl TernaryOperator {
    pub(crate) fn result(&self, input0: U256, input1: U256, input2: U256) -> U256 {
        match self {
            TernaryOperator::AddMod => addmod(input0, input1, input2),
            TernaryOperator::MulMod => mulmod(input0, input1, input2),
            TernaryOperator::SubMod => submod(input0, input1, input2),
        }
    }

    pub(crate) fn row_filter(&self) -> usize {
        match self {
            TernaryOperator::AddMod => columns::IS_ADDMOD,
            TernaryOperator::MulMod => columns::IS_MULMOD,
            TernaryOperator::SubMod => columns::IS_SUBMOD,
        }
    }
}

#[derive(Debug)]
pub(crate) enum Operation {
    BinaryOperation {
        operator: BinaryOperator,
        input0: U256,
        input1: U256,
        result: U256,
    },
    TernaryOperation {
        operator: TernaryOperator,
        input0: U256,
        input1: U256,
        input2: U256,
        result: U256,
    },
}

impl Operation {
    pub(crate) fn binary(operator: BinaryOperator, input0: U256, input1: U256) -> Self {
        let result = operator.result(input0, input1);
        Self::BinaryOperation {
            operator,
            input0,
            input1,
            result,
        }
    }

    pub(crate) fn ternary(
        operator: TernaryOperator,
        input0: U256,
        input1: U256,
        input2: U256,
    ) -> Self {
        let result = operator.result(input0, input1, input2);
        Self::TernaryOperation {
            operator,
            input0,
            input1,
            input2,
            result,
        }
    }

    pub(crate) fn result(&self) -> U256 {
        match self {
            Operation::BinaryOperation { result, .. } => *result,
            Operation::TernaryOperation { result, .. } => *result,
        }
    }

    /// Convert operation into one or two rows of the trace.
    ///
    /// Morally these types should be [F; NUM_ARITH_COLUMNS], but we
    /// use vectors because that's what utils::transpose (who consumes
    /// the result of this function as part of the range check code)
    /// expects.
    fn to_rows<F: PrimeField64>(&self) -> (Vec<F>, Option<Vec<F>>) {
        match *self {
            Operation::BinaryOperation {
                operator,
                input0,
                input1,
                result,
            } => binary_op_to_rows(operator, input0, input1, result),
            Operation::TernaryOperation {
                operator,
                input0,
                input1,
                input2,
                result,
            } => ternary_op_to_rows(operator.row_filter(), input0, input1, input2, result),
        }
    }
}

fn ternary_op_to_rows<F: PrimeField64>(
    row_filter: usize,
    input0: U256,
    input1: U256,
    input2: U256,
    _result: U256,
) -> (Vec<F>, Option<Vec<F>>) {
    let mut row1 = vec![F::ZERO; columns::NUM_ARITH_COLUMNS];
    let mut row2 = vec![F::ZERO; columns::NUM_ARITH_COLUMNS];

    row1[row_filter] = F::ONE;

    modular::generate(&mut row1, &mut row2, row_filter, input0, input1, input2);

    (row1, Some(row2))
}

fn binary_op_to_rows<F: PrimeField64>(
    op: BinaryOperator,
    input0: U256,
    input1: U256,
    result: U256,
) -> (Vec<F>, Option<Vec<F>>) {
    let mut row = vec![F::ZERO; columns::NUM_ARITH_COLUMNS];
    row[op.row_filter()] = F::ONE;

    match op {
        BinaryOperator::Add | BinaryOperator::Sub | BinaryOperator::Lt | BinaryOperator::Gt => {
            addcy::generate(&mut row, op.row_filter(), input0, input1);
            (row, None)
        }
        BinaryOperator::Mul => {
            mul::generate(&mut row, input0, input1);
            (row, None)
        }
        BinaryOperator::Div | BinaryOperator::Mod => {
            let mut nv = vec![F::ZERO; columns::NUM_ARITH_COLUMNS];
            divmod::generate(&mut row, &mut nv, op.row_filter(), input0, input1, result);
            (row, Some(nv))
        }
        BinaryOperator::AddFp254 | BinaryOperator::MulFp254 | BinaryOperator::SubFp254 => {
            ternary_op_to_rows::<F>(op.row_filter(), input0, input1, BN_BASE, result)
        }
        BinaryOperator::Byte => {
            byte::generate(&mut row, input0, input1);
            (row, None)
        }
    }
}
