use std::str::FromStr;

use ethereum_types::{BigEndianHash, H256, U256};
use plonky2::field::types::Field;

use crate::bn254::{fp12_to_array, inv_fp12, vec_to_fp12};
use crate::generation::prover_input::EvmField::{
    Bn254Base, Bn254Scalar, Secp256k1Base, Secp256k1Scalar,
};
use crate::generation::prover_input::FieldExtOp::{
    ExtInv0, ExtInv1, ExtInv10, ExtInv11, ExtInv2, ExtInv3, ExtInv4, ExtInv5, ExtInv6, ExtInv7,
    ExtInv8, ExtInv9,
};
use crate::generation::prover_input::FieldOp::{Inverse, Sqrt};
use crate::generation::state::GenerationState;
use crate::witness::util::{stack_peek, stack_peeks};

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
            "ffe" => self.run_ffe(input_fn),
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

    /// Finite field extension operations.
    fn run_ffe(&self, input_fn: &ProverInputFn) -> U256 {
        let field = EvmField::from_str(input_fn.0[1].as_str()).unwrap();
        let op = FieldExtOp::from_str(input_fn.0[2].as_str()).unwrap();
        let xs = stack_peeks(self).expect("Empty stack");
        field.extop(op, xs)
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

enum FieldExtOp {
    ExtInv0,
    ExtInv1,
    ExtInv2,
    ExtInv3,
    ExtInv4,
    ExtInv5,
    ExtInv6,
    ExtInv7,
    ExtInv8,
    ExtInv9,
    ExtInv10,
    ExtInv11,
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

impl FromStr for FieldExtOp {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "ext_inv0" => ExtInv0,
            "ext_inv1" => ExtInv1,
            "ext_inv2" => ExtInv2,
            "ext_inv3" => ExtInv3,
            "ext_inv4" => ExtInv4,
            "ext_inv5" => ExtInv5,
            "ext_inv6" => ExtInv6,
            "ext_inv7" => ExtInv7,
            "ext_inv8" => ExtInv8,
            "ext_inv9" => ExtInv9,
            "ext_inv10" => ExtInv10,
            "ext_inv11" => ExtInv11,
            _ => panic!("Unrecognized field extension operation."),
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

    fn extop(&self, op: FieldExtOp, xs: Vec<U256>) -> U256 {
        match op {
            FieldExtOp::ExtInv0 => self.ext_inv(0, xs),
            FieldExtOp::ExtInv1 => self.ext_inv(1, xs),
            FieldExtOp::ExtInv2 => self.ext_inv(2, xs),
            FieldExtOp::ExtInv3 => self.ext_inv(3, xs),
            FieldExtOp::ExtInv4 => self.ext_inv(4, xs),
            FieldExtOp::ExtInv5 => self.ext_inv(5, xs),
            FieldExtOp::ExtInv6 => self.ext_inv(6, xs),
            FieldExtOp::ExtInv7 => self.ext_inv(7, xs),
            FieldExtOp::ExtInv8 => self.ext_inv(8, xs),
            FieldExtOp::ExtInv9 => self.ext_inv(9, xs),
            FieldExtOp::ExtInv10 => self.ext_inv(10, xs),
            FieldExtOp::ExtInv11 => self.ext_inv(11, xs),
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

    fn ext_inv(&self, n: usize, xs: Vec<U256>) -> U256 {
        let offset = 12 - n;
        let vec: Vec<U256> = xs[offset..].to_vec();
        let f = fp12_to_array(inv_fp12(vec_to_fp12(vec)));
        f[n]
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
