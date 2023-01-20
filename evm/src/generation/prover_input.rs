use std::str::FromStr;

use anyhow::{bail, Error};
use ethereum_types::{BigEndianHash, H256, U256};
use plonky2::field::types::Field;

use crate::generation::prover_input::EvmField::{
    Bn254Base, Bn254Scalar, Secp256k1Base, Secp256k1Scalar,
};
use crate::generation::prover_input::FieldOp::{Inverse, Sqrt};
use crate::generation::state::GenerationState;
use crate::memory::segments::Segment;
use crate::util::{biguint_to_mem_vec, mem_vec_to_biguint};
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
            "bignum_modmul" => self.run_bignum_modmul(input_fn),
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

    // Bignum-related code.
    fn run_bignum_modmul(&mut self, input_fn: &ProverInputFn) -> U256 {
        if self.bignum_modmul_prover_inputs.is_empty() {
            let function = input_fn.0[1].as_str();

            let len = stack_peek(self, 1)
                .expect("Stack does not have enough items")
                .try_into()
                .unwrap();
            let a_start_loc = stack_peek(self, 2)
                .expect("Stack does not have enough items")
                .try_into()
                .unwrap();
            let b_start_loc = stack_peek(self, 3)
                .expect("Stack does not have enough items")
                .try_into()
                .unwrap();
            let m_start_loc = stack_peek(self, 4)
                .expect("Stack does not have enough items")
                .try_into()
                .unwrap();

            let result = match function {
                "remainder" => {
                    self.bignum_modmul_remainder(len, a_start_loc, b_start_loc, m_start_loc)
                }
                "quotient" => {
                    self.bignum_modmul_quotient(len, a_start_loc, b_start_loc, m_start_loc)
                }
                _ => panic!("Invalid prover input function."),
            };

            self.bignum_modmul_prover_inputs = result.to_vec();
            self.bignum_modmul_prover_inputs.reverse();
        }

        self.bignum_modmul_prover_inputs.pop().unwrap()
    }

    fn bignum_modmul_remainder(
        &mut self,
        len: usize,
        a_start_loc: usize,
        b_start_loc: usize,
        m_start_loc: usize,
    ) -> Vec<U256> {
        let a = &self.memory.contexts[0].segments[Segment::KernelGeneral as usize].content
            [a_start_loc..a_start_loc + len];
        let b = &self.memory.contexts[0].segments[Segment::KernelGeneral as usize].content
            [b_start_loc..b_start_loc + len];
        let m = &self.memory.contexts[0].segments[Segment::KernelGeneral as usize].content
            [m_start_loc..m_start_loc + len];

        let a_biguint = mem_vec_to_biguint(a);
        let b_biguint = mem_vec_to_biguint(b);
        let m_biguint = mem_vec_to_biguint(m);

        let result_biguint = (a_biguint * b_biguint) % m_biguint;
        biguint_to_mem_vec(result_biguint)
    }

    fn bignum_modmul_quotient(
        &mut self,
        len: usize,
        a_start_loc: usize,
        b_start_loc: usize,
        m_start_loc: usize,
    ) -> Vec<U256> {
        let a = &self.memory.contexts[0].segments[Segment::KernelGeneral as usize].content
            [a_start_loc..a_start_loc + len];
        let b = &self.memory.contexts[0].segments[Segment::KernelGeneral as usize].content
            [b_start_loc..b_start_loc + len];
        let m = &self.memory.contexts[0].segments[Segment::KernelGeneral as usize].content
            [m_start_loc..m_start_loc + len];

        let a_biguint = mem_vec_to_biguint(a);
        let b_biguint = mem_vec_to_biguint(b);
        let m_biguint = mem_vec_to_biguint(m);

        let result_biguint = (a_biguint * b_biguint) / m_biguint;
        biguint_to_mem_vec(result_biguint)
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
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "bn254_base" => Bn254Base,
            "bn254_scalar" => Bn254Scalar,
            "secp256k1_base" => Secp256k1Base,
            "secp256k1_scalar" => Secp256k1Scalar,
            _ => bail!("Unrecognized field."),
        })
    }
}

impl FromStr for FieldOp {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "inverse" => Inverse,
            "sqrt" => Sqrt,
            _ => bail!("Unrecognized field operation."),
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
