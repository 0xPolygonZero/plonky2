use std::ops::Mul;
use std::str::FromStr;

use ethereum_types::{U256, U512};
use num::BigUint;

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
            BinaryOperator::Add => input0 + input1,
            BinaryOperator::Mul => input0 * input1,
            BinaryOperator::Sub => input0 - input1,
            BinaryOperator::Div => input0 / input1,
            BinaryOperator::Mod => input0 % input1,
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
            BinaryOperator::Shl => input0 << input1,
            BinaryOperator::Shr => input0 >> input1,
            BinaryOperator::AddFp254 => addmod(input0, input1, bn_base_order()),
            BinaryOperator::MulFp254 => mulmod(input0, input1, bn_base_order()),
            BinaryOperator::SubFp254 => submod(input0, input1, bn_base_order()),
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

fn addmod(x: U256, y: U256, m: U256) -> U256 {
    if m.is_zero() {
        return m;
    }
    let x = to_biguint(x);
    let y = to_biguint(y);
    let m = to_biguint(m);
    from_biguint((x + y) % m)
}

fn mulmod(x: U256, y: U256, m: U256) -> U256 {
    if m.is_zero() {
        return m;
    }
    let x = to_biguint(x);
    let y = to_biguint(y);
    let m = to_biguint(m);
    from_biguint(x * y % m)
}

fn submod(x: U256, y: U256, m: U256) -> U256 {
    if m.is_zero() {
        return m;
    }
    let mut x = to_biguint(x);
    let y = to_biguint(y);
    let m = to_biguint(m);
    while x < y {
        x += &m;
    }
    from_biguint((x - y) % m)
}

fn to_biguint(x: U256) -> BigUint {
    let mut bytes = [0u8; 32];
    x.to_little_endian(&mut bytes);
    BigUint::from_bytes_le(&bytes)
}

fn from_biguint(x: BigUint) -> U256 {
    let bytes = x.to_bytes_le();
    U256::from_little_endian(&bytes)
}

fn bn_base_order() -> U256 {
    U256::from_str("0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47").unwrap()
}
