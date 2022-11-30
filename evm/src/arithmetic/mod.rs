use ethereum_types::U256;

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
    Mul,
    Sub,
    Div,
    Mod,
    Lt,
    Gt,
    Shl,
    Shr,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TernaryOperator {
    AddMod,
    SubMod,
    MulMod,
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
