use ethereum_types::U256;

use crate::bn254::BN_BASE;
use crate::util::{addmod, mulmod, submod};

mod add;
mod compare;
mod modular;
mod mul;
mod sub;
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
            BinaryOperator::AddFp254 => addmod(input0, input1, BN_BASE),
            BinaryOperator::MulFp254 => mulmod(input0, input1, BN_BASE),
            BinaryOperator::SubFp254 => submod(input0, input1, BN_BASE),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TernaryOperator {
    AddMod,
    MulMod,
}

impl TernaryOperator {
    pub(crate) fn result(&self, input0: U256, input1: U256, input2: U256) -> U256 {
        match self {
            TernaryOperator::AddMod => addmod(input0, input1, input2),
            TernaryOperator::MulMod => mulmod(input0, input1, input2),
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
