use ethereum_types::U256;
use plonky2::field::types::PrimeField64;

use self::columns::{
    INPUT_REGISTER_0, INPUT_REGISTER_1, INPUT_REGISTER_2, OPCODE_COL, OUTPUT_REGISTER,
};
use self::utils::u256_to_array;
use crate::arithmetic::columns::IS_RANGE_CHECK;
use crate::extension_tower::BN_BASE;
use crate::util::{addmod, mulmod, submod};

mod addcy;
mod byte;
mod divmod;
mod modular;
mod mul;
mod shift;
mod utils;

pub mod arithmetic_stark;
pub(crate) mod columns;

/// An enum representing different binary operations.
///
/// `Shl` and `Shr` are handled differently, by leveraging `Mul` and `Div` respectively.
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
    Shl, // simulated with MUL
    Shr, // simulated with DIV
}

impl BinaryOperator {
    /// Computes the result of a binary arithmetic operation given two inputs.
    pub(crate) fn result(&self, input0: U256, input1: U256) -> U256 {
        match self {
            BinaryOperator::Add => input0.overflowing_add(input1).0,
            BinaryOperator::Mul => input0.overflowing_mul(input1).0,
            BinaryOperator::Shl => {
                if input0 < U256::from(256usize) {
                    input1 << input0
                } else {
                    U256::zero()
                }
            }
            BinaryOperator::Sub => input0.overflowing_sub(input1).0,
            BinaryOperator::Div => {
                if input1.is_zero() {
                    U256::zero()
                } else {
                    input0 / input1
                }
            }
            BinaryOperator::Shr => {
                if input0 < U256::from(256usize) {
                    input1 >> input0
                } else {
                    U256::zero()
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

    /// Maps a binary arithmetic operation to its associated flag column in the trace.
    pub(crate) const fn row_filter(&self) -> usize {
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
            BinaryOperator::Shl => columns::IS_SHL,
            BinaryOperator::Shr => columns::IS_SHR,
        }
    }
}

/// An enum representing different ternary operations.
#[allow(clippy::enum_variant_names)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TernaryOperator {
    AddMod,
    MulMod,
    SubMod,
}

impl TernaryOperator {
    /// Computes the result of a ternary arithmetic operation given three inputs.
    pub(crate) fn result(&self, input0: U256, input1: U256, input2: U256) -> U256 {
        match self {
            TernaryOperator::AddMod => addmod(input0, input1, input2),
            TernaryOperator::MulMod => mulmod(input0, input1, input2),
            TernaryOperator::SubMod => submod(input0, input1, input2),
        }
    }

    /// Maps a ternary arithmetic operation to its associated flag column in the trace.
    pub(crate) const fn row_filter(&self) -> usize {
        match self {
            TernaryOperator::AddMod => columns::IS_ADDMOD,
            TernaryOperator::MulMod => columns::IS_MULMOD,
            TernaryOperator::SubMod => columns::IS_SUBMOD,
        }
    }
}

/// An enum representing arithmetic operations that can be either binary or ternary.
#[allow(clippy::enum_variant_names)]
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
    RangeCheckOperation {
        input0: U256,
        input1: U256,
        input2: U256,
        opcode: U256,
        result: U256,
    },
}

impl Operation {
    /// Creates a binary operator with given inputs.
    ///
    /// NB: This works as you would expect, EXCEPT for SHL and SHR,
    /// whose inputs need a small amount of preprocessing. Specifically,
    /// to create `SHL(shift, value)`, call (note the reversal of
    /// argument order):
    ///
    ///    `Operation::binary(BinaryOperator::Shl, value, 1 << shift)`
    ///
    /// Similarly, to create `SHR(shift, value)`, call
    ///
    ///    `Operation::binary(BinaryOperator::Shr, value, 1 << shift)`
    ///
    /// See witness/operation.rs::append_shift() for an example (indeed
    /// the only call site for such inputs).
    pub(crate) fn binary(operator: BinaryOperator, input0: U256, input1: U256) -> Self {
        let result = operator.result(input0, input1);
        Self::BinaryOperation {
            operator,
            input0,
            input1,
            result,
        }
    }

    /// Creates a ternary operator with given inputs.
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

    pub(crate) const fn range_check(
        input0: U256,
        input1: U256,
        input2: U256,
        opcode: U256,
        result: U256,
    ) -> Self {
        Self::RangeCheckOperation {
            input0,
            input1,
            input2,
            opcode,
            result,
        }
    }

    /// Gets the result of an arithmetic operation.
    pub(crate) fn result(&self) -> U256 {
        match self {
            Operation::BinaryOperation { result, .. } => *result,
            Operation::TernaryOperation { result, .. } => *result,
            _ => panic!("This function should not be called for range checks."),
        }
    }

    /// Convert operation into one or two rows of the trace.
    ///
    /// Morally these types should be [F; NUM_ARITH_COLUMNS], but we
    /// use vectors because that's what utils::transpose (who consumes
    /// the result of this function as part of the range check code)
    /// expects.
    ///
    /// The `is_simulated` bool indicates whether we use a native arithmetic
    /// operation or simulate one with another. This is used to distinguish
    /// SHL and SHR operations that are simulated through MUL and DIV respectively.
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
            Operation::RangeCheckOperation {
                input0,
                input1,
                input2,
                opcode,
                result,
            } => range_check_to_rows(input0, input1, input2, opcode, result),
        }
    }
}

/// Converts a ternary arithmetic operation to one or two rows of the `ArithmeticStark` table.
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

/// Converts a binary arithmetic operation to one or two rows of the `ArithmeticStark` table.
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
        BinaryOperator::Shl => {
            let mut nv = vec![F::ZERO; columns::NUM_ARITH_COLUMNS];
            shift::generate(&mut row, &mut nv, true, input0, input1, result);
            (row, None)
        }
        BinaryOperator::Div | BinaryOperator::Mod => {
            let mut nv = vec![F::ZERO; columns::NUM_ARITH_COLUMNS];
            divmod::generate(&mut row, &mut nv, op.row_filter(), input0, input1, result);
            (row, Some(nv))
        }
        BinaryOperator::Shr => {
            let mut nv = vec![F::ZERO; columns::NUM_ARITH_COLUMNS];
            shift::generate(&mut row, &mut nv, false, input0, input1, result);
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

fn range_check_to_rows<F: PrimeField64>(
    input0: U256,
    input1: U256,
    input2: U256,
    opcode: U256,
    result: U256,
) -> (Vec<F>, Option<Vec<F>>) {
    let mut row = vec![F::ZERO; columns::NUM_ARITH_COLUMNS];
    row[IS_RANGE_CHECK] = F::ONE;
    row[OPCODE_COL] = F::from_canonical_u64(opcode.as_u64());
    u256_to_array(&mut row[INPUT_REGISTER_0], input0);
    u256_to_array(&mut row[INPUT_REGISTER_1], input1);
    u256_to_array(&mut row[INPUT_REGISTER_2], input2);
    u256_to_array(&mut row[OUTPUT_REGISTER], result);

    (row, None)
}
