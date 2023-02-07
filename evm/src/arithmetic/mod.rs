use std::str::FromStr;

use ethereum_types::U256;
use plonky2::hash::hash_types::RichField;

use crate::util::{addmod, mulmod, submod};

mod addcy;
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
    Shl,
    Shr,
    AddFp254,
    MulFp254,
    SubFp254,
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
            BinaryOperator::Lt => {
                if input0 < input1 {
                    U256::one()
                } else {
                    U256::zero()
                }
            }
            BinaryOperator::Gt => {
                if input0 > input1 {
                    U256::one()
                } else {
                    U256::zero()
                }
            }
            BinaryOperator::Shl => {
                if input0 > 255.into() {
                    U256::zero()
                } else {
                    input1 << input0
                }
            }
            BinaryOperator::Shr => {
                if input0 > 255.into() {
                    U256::zero()
                } else {
                    input1 >> input0
                }
            }
            BinaryOperator::AddFp254 => addmod(input0, input1, bn_base_order()),
            BinaryOperator::MulFp254 => mulmod(input0, input1, bn_base_order()),
            BinaryOperator::SubFp254 => submod(input0, input1, bn_base_order()),
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
            BinaryOperator::Shl => panic!("SHL is not an arithmetic operation"),
            BinaryOperator::Shr => panic!("SHR is not an arithmetic operation"),
            BinaryOperator::AddFp254 => columns::IS_ADDMOD,
            BinaryOperator::MulFp254 => columns::IS_MULMOD,
            BinaryOperator::SubFp254 => columns::IS_SUBMOD,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TernaryOperator {
    AddMod,
    MulMod,
    //SubMod,
}

impl TernaryOperator {
    pub(crate) fn result(&self, input0: U256, input1: U256, input2: U256) -> U256 {
        match self {
            TernaryOperator::AddMod => addmod(input0, input1, input2),
            TernaryOperator::MulMod => mulmod(input0, input1, input2),
            //TernaryOperator::SubMod => submod(input0, input1, input2),
        }
    }

    pub(crate) fn row_filter(&self) -> usize {
        match self {
            TernaryOperator::AddMod => columns::IS_ADDMOD,
            TernaryOperator::MulMod => columns::IS_MULMOD,
            //TernaryOperator::SubMod => columns::IS_SUBMOD,
        }
    }
}

#[derive(Debug)]
#[allow(unused)] // TODO: Should be used soon.
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
}

pub trait Traceable<F: RichField> {
    /// Convert operation into one or two rows of the trace.
    ///
    /// Morally these types should be [F; NUM_ARITH_COLUMNS], but we
    /// use vectors because that's what utils::transpose (who consumes
    /// the result of this function as part of the range check code)
    /// expects.
    fn to_rows(&self) -> (Vec<F>, Option<Vec<F>>);
}

fn ternary_op_to_rows<F: RichField>(
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

fn binary_op_to_rows<F: RichField>(
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
            ternary_op_to_rows::<F>(op.row_filter(), input0, U256::zero(), input1, result)
        }
        BinaryOperator::AddFp254 | BinaryOperator::MulFp254 | BinaryOperator::SubFp254 => {
            ternary_op_to_rows::<F>(op.row_filter(), input0, input1, bn_base_order(), result)
        }
        BinaryOperator::Shl | BinaryOperator::Shr => {
            panic!("shl and shr are not handled by the arithmetic table")
        }
    }
}

impl<F: RichField> Traceable<F> for Operation {
    fn to_rows(&self) -> (Vec<F>, Option<Vec<F>>) {
        match self {
            &Operation::BinaryOperation {
                operator,
                input0,
                input1,
                result,
            } => binary_op_to_rows(operator, input0, input1, result),
            &Operation::TernaryOperation {
                operator,
                input0,
                input1,
                input2,
                result,
            } => ternary_op_to_rows(operator.row_filter(), input0, input1, input2, result),
        }
    }
}

fn bn_base_order() -> U256 {
    U256::from_str("0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47").unwrap()
}
