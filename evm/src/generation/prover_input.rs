use std::str::FromStr;

use ethereum_types::{BigEndianHash, H256, U256};
use plonky2::field::types::Field;

use crate::generation::prover_input::EvmField::{
    Bn254Base, Bn254Scalar, Secp256k1Base, Secp256k1Scalar,
};
use crate::generation::prover_input::FieldOp::{Inverse, Sqrt};
use crate::generation::state::GenerationState;
use crate::witness::util::stack_peek;

/// Prover input function represented as a scoped function name.
/// Example: `PROVER_INPUT(ff::bn254_base::inverse)` is represented as `ProverInputFn([ff, bn254_base, inverse])`.
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct ProverInputFn(Vec<String>);

impl From<Vec<String>> for ProverInputFn {
    fn from(v: Vec<String>) -> Self {
        Self(v)
    }
}

impl<F: Field> GenerationState<F> {
    pub(crate) fn prover_input(&mut self, input_fn: &ProverInputFn) -> U256 {
        match input_fn.0[0].as_str() {
            "end_of_txns" => self.run_end_of_txns(),
            "ff" => self.run_ff(input_fn),
            "mpt" => self.run_mpt(),
            "rlp" => self.run_rlp(),
            "account_code" => self.run_account_code(input_fn),
            _ => panic!("Unrecognized prover input function."),
        }
    }

    fn run_end_of_txns(&mut self) -> U256 {
        let end = self.next_txn_index == self.inputs.signed_txns.len();
        if end {
            U256::one()
        } else {
            self.next_txn_index += 1;
            U256::zero()
        }
    }

    /// Finite field operations.
    fn run_ff(&self, input_fn: &ProverInputFn) -> U256 {
        let field = EvmField::from_str(input_fn.0[1].as_str()).unwrap();
        let op = FieldOp::from_str(input_fn.0[2].as_str()).unwrap();
        let x = stack_peek(self, 0).expect("Empty stack");
        field.op(op, x)
    }

    /// MPT data.
    fn run_mpt(&mut self) -> U256 {
        self.mpt_prover_inputs
            .pop()
            .unwrap_or_else(|| panic!("Out of MPT data"))
    }

    /// RLP data.
    fn run_rlp(&mut self) -> U256 {
        self.rlp_prover_inputs
            .pop()
            .unwrap_or_else(|| panic!("Out of RLP data"))
    }

    /// Account code.
    fn run_account_code(&mut self, input_fn: &ProverInputFn) -> U256 {
        match input_fn.0[1].as_str() {
            "length" => {
                // Return length of code.
                // stack: codehash, ...
                let codehash = stack_peek(self, 0).expect("Empty stack");
                self.inputs.contract_code[&H256::from_uint(&codehash)]
                    .len()
                    .into()
            }
            "get" => {
                // Return `code[i]`.
                // stack: i, code_length, codehash, ...
                let i = stack_peek(self, 0).expect("Unexpected stack").as_usize();
                let codehash = stack_peek(self, 2).expect("Unexpected stack");
                self.inputs.contract_code[&H256::from_uint(&codehash)][i].into()
            }
            _ => panic!("Invalid prover input function."),
        }
    }
}

enum EvmField {
    Bn254Base,
    Bn254Scalar,
    Secp256k1Base,
    Secp256k1Scalar,
}

enum FieldOp {
    Inverse,
    Sqrt,
}

impl FromStr for EvmField {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "bn254_base" => Bn254Base,
            "bn254_scalar" => Bn254Scalar,
            "secp256k1_base" => Secp256k1Base,
            "secp256k1_scalar" => Secp256k1Scalar,
            _ => panic!("Unrecognized field."),
        })
    }
}

impl FromStr for FieldOp {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "inverse" => Inverse,
            "sqrt" => Sqrt,
            _ => panic!("Unrecognized field operation."),
        })
    }
}

impl EvmField {
    fn order(&self) -> U256 {
        match self {
            EvmField::Bn254Base => {
                U256::from_str("0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47")
                    .unwrap()
            }
            EvmField::Bn254Scalar => todo!(),
            EvmField::Secp256k1Base => {
                U256::from_str("0xfffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f")
                    .unwrap()
            }
            EvmField::Secp256k1Scalar => {
                U256::from_str("0xfffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141")
                    .unwrap()
            }
        }
    }

    fn op(&self, op: FieldOp, x: U256) -> U256 {
        match op {
            FieldOp::Inverse => self.inverse(x),
            FieldOp::Sqrt => self.sqrt(x),
        }
    }

    fn inverse(&self, x: U256) -> U256 {
        let n = self.order();
        assert!(x < n);
        modexp(x, n - 2, n)
    }

    fn sqrt(&self, x: U256) -> U256 {
        let n = self.order();
        assert!(x < n);
        let (q, r) = (n + 1).div_mod(4.into());
        assert!(
            r.is_zero(),
            "Only naive sqrt implementation for now. If needed implement Tonelli-Shanks."
        );
        modexp(x, q, n)
    }
}

fn modexp(x: U256, e: U256, n: U256) -> U256 {
    let mut current = x;
    let mut product = U256::one();

    for j in 0..256 {
        if e.bit(j) {
            product = U256::try_from(product.full_mul(current) % n).unwrap();
        }
        current = U256::try_from(current.full_mul(current) % n).unwrap();
    }
    product
}
