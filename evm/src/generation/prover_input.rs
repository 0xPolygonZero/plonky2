use std::mem::transmute;
use std::str::FromStr;

use anyhow::{bail, Error};
use ethereum_types::{BigEndianHash, H256, U256, U512};
use itertools::Itertools;
use plonky2::field::types::Field;

use crate::extension_tower::{FieldExt, Fp12, BLS381, BN254};
use crate::generation::prover_input::EvmField::{
    Bls381Base, Bls381Scalar, Bn254Base, Bn254Scalar, Secp256k1Base, Secp256k1Scalar,
};
use crate::generation::prover_input::FieldOp::{Inverse, Sqrt};
use crate::generation::state::GenerationState;
use crate::memory::segments::Segment;
use crate::memory::segments::Segment::BnPairing;
use crate::util::{biguint_to_mem_vec, mem_vec_to_biguint};
use crate::witness::util::{kernel_peek, stack_peek};

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
            "sf" => self.run_sf(input_fn),
            "ffe" => self.run_ffe(input_fn),
            "mpt" => self.run_mpt(),
            "rlp" => self.run_rlp(),
            "account_code" => self.run_account_code(input_fn),
            "bignum_modmul" => self.run_bignum_modmul(),
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

    /// Special finite field operations.
    fn run_sf(&self, input_fn: &ProverInputFn) -> U256 {
        let field = EvmField::from_str(input_fn.0[1].as_str()).unwrap();
        let inputs: [U256; 4] = match field {
            Bls381Base => std::array::from_fn(|i| {
                stack_peek(self, i).expect("Insufficient number of items on stack")
            }),
            _ => todo!(),
        };
        match input_fn.0[2].as_str() {
            "add_lo" => field.add_lo(inputs),
            "add_hi" => field.add_hi(inputs),
            "mul_lo" => field.mul_lo(inputs),
            "mul_hi" => field.mul_hi(inputs),
            "sub_lo" => field.sub_lo(inputs),
            "sub_hi" => field.sub_hi(inputs),
            _ => todo!(),
        }
    }

    /// Finite field extension operations.
    fn run_ffe(&self, input_fn: &ProverInputFn) -> U256 {
        let field = EvmField::from_str(input_fn.0[1].as_str()).unwrap();
        let n = input_fn.0[2]
            .as_str()
            .split('_')
            .nth(1)
            .unwrap()
            .parse::<usize>()
            .unwrap();
        let ptr = stack_peek(self, 11 - n)
            .expect("Insufficient number of items on stack")
            .as_usize();

        let f: [U256; 12] = match field {
            Bn254Base => std::array::from_fn(|i| kernel_peek(self, BnPairing, ptr + i)),
            _ => todo!(),
        };
        field.field_extension_inverse(n, f)
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
                self.inputs
                    .contract_code
                    .get(&H256::from_uint(&codehash))
                    .unwrap_or_else(|| panic!("No code found with hash {codehash}"))
                    .len()
                    .into()
            }
            "get" => {
                // Return `code[i]`.
                // stack: i, code_length, codehash, ...
                let i = stack_peek(self, 0).expect("Unexpected stack").as_usize();
                let codehash = stack_peek(self, 2).expect("Unexpected stack");
                self.inputs
                    .contract_code
                    .get(&H256::from_uint(&codehash))
                    .unwrap_or_else(|| panic!("No code found with hash {codehash}"))[i]
                    .into()
            }
            _ => panic!("Invalid prover input function."),
        }
    }

    // Bignum modular multiplication.
    // On the first call, calculates the remainder and quotient of the given inputs.
    // These are stored, as limbs, in self.bignum_modmul_result_limbs.
    // Subsequent calls return one limb at a time, in order (first remainder and then quotient).
    fn run_bignum_modmul(&mut self) -> U256 {
        if self.bignum_modmul_result_limbs.is_empty() {
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

            let (remainder, quotient) =
                self.bignum_modmul(len, a_start_loc, b_start_loc, m_start_loc);

            self.bignum_modmul_result_limbs = remainder
                .iter()
                .cloned()
                .pad_using(len, |_| 0.into())
                .chain(quotient.iter().cloned().pad_using(2 * len, |_| 0.into()))
                .collect();
            self.bignum_modmul_result_limbs.reverse();
        }

        self.bignum_modmul_result_limbs.pop().unwrap()
    }

    fn bignum_modmul(
        &mut self,
        len: usize,
        a_start_loc: usize,
        b_start_loc: usize,
        m_start_loc: usize,
    ) -> (Vec<U256>, Vec<U256>) {
        let a = &self.memory.contexts[0].segments[Segment::KernelGeneral as usize].content
            [a_start_loc..a_start_loc + len];
        let b = &self.memory.contexts[0].segments[Segment::KernelGeneral as usize].content
            [b_start_loc..b_start_loc + len];
        let m = &self.memory.contexts[0].segments[Segment::KernelGeneral as usize].content
            [m_start_loc..m_start_loc + len];

        let a_biguint = mem_vec_to_biguint(a);
        let b_biguint = mem_vec_to_biguint(b);
        let m_biguint = mem_vec_to_biguint(m);

        let prod = a_biguint * b_biguint;
        let quo = &prod / &m_biguint;
        let rem = prod - m_biguint * &quo;

        (biguint_to_mem_vec(rem), biguint_to_mem_vec(quo))
    }
}

enum EvmField {
    Bls381Base,
    Bls381Scalar,
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
            "bls381_base" => Bls381Base,
            "bls381_scalar" => Bls381Scalar,
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
            EvmField::Bls381Base => todo!(),
            EvmField::Bls381Scalar => todo!(),
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

    fn add_lo(&self, inputs: [U256; 4]) -> U256 {
        let [y1, x0, x1, y0] = inputs;
        let x = U512::from(x0) + (U512::from(x1) << 256);
        let y = U512::from(y0) + (U512::from(y1) << 256);
        let z = BLS381 { val: x } + BLS381 { val: y };
        z.lo()
    }

    fn add_hi(&self, inputs: [U256; 4]) -> U256 {
        let [x0, x1, y0, y1] = inputs;
        let x = U512::from(x0) + (U512::from(x1) << 256);
        let y = U512::from(y0) + (U512::from(y1) << 256);
        let z = BLS381 { val: x } + BLS381 { val: y };
        z.hi()
    }

    fn mul_lo(&self, inputs: [U256; 4]) -> U256 {
        let [y1, x0, x1, y0] = inputs;
        let x = U512::from(x0) + (U512::from(x1) << 256);
        let y = U512::from(y0) + (U512::from(y1) << 256);
        let z = BLS381 { val: x } * BLS381 { val: y };
        z.lo()
    }

    fn mul_hi(&self, inputs: [U256; 4]) -> U256 {
        let [x0, x1, y0, y1] = inputs;
        let x = U512::from(x0) + (U512::from(x1) << 256);
        let y = U512::from(y0) + (U512::from(y1) << 256);
        let z = BLS381 { val: x } * BLS381 { val: y };
        z.hi()
    }

    fn sub_lo(&self, inputs: [U256; 4]) -> U256 {
        let [y1, x0, x1, y0] = inputs;
        let x = U512::from(x0) + (U512::from(x1) << 256);
        let y = U512::from(y0) + (U512::from(y1) << 256);
        let z = BLS381 { val: x } - BLS381 { val: y };
        z.lo()
    }

    fn sub_hi(&self, inputs: [U256; 4]) -> U256 {
        let [x0, x1, y0, y1] = inputs;
        let x = U512::from(x0) + (U512::from(x1) << 256);
        let y = U512::from(y0) + (U512::from(y1) << 256);
        let z = BLS381 { val: x } - BLS381 { val: y };
        z.hi()
    }

    fn field_extension_inverse(&self, n: usize, f: [U256; 12]) -> U256 {
        let f: Fp12<BN254> = unsafe { transmute(f) };
        let f_inv: [U256; 12] = unsafe { transmute(f.inv()) };
        f_inv[n]
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
