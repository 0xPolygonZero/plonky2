use std::str::FromStr;

use ethereum_types::U256;

use crate::cpu::kernel::prover_input::Field::{
    Bn254Base, Bn254Scalar, Secp256k1Base, Secp256k1Scalar,
};
use crate::cpu::kernel::prover_input::FieldOp::{Inverse, Sqrt};

#[derive(PartialEq, Eq, Debug, Clone)]
pub(crate) struct ProverInputFn(Vec<String>);

impl From<Vec<String>> for ProverInputFn {
    fn from(v: Vec<String>) -> Self {
        Self(v)
    }
}

impl ProverInputFn {
    pub(crate) fn run(&self, mut stack: Vec<U256>) -> U256 {
        match self.0[0].as_str() {
            "ff" => self.run_ff(stack),
            "storage" => todo!(),
            _ => panic!("Unrecognized prover input function."),
        }
    }

    fn run_ff(&self, mut stack: Vec<U256>) -> U256 {
        let field = Field::from_str(self.0[1].as_str()).unwrap();
        let op = FieldOp::from_str(self.0[2].as_str()).unwrap();
        let x = stack.pop().expect("Empty stack");
        field.op(op, x)
    }
}

enum Field {
    Bn254Base,
    Bn254Scalar,
    Secp256k1Base,
    Secp256k1Scalar,
}

enum FieldOp {
    Inverse,
    Sqrt,
}

impl FromStr for Field {
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

impl Field {
    fn order(&self) -> U256 {
        match self {
            Field::Bn254Base => {
                U256::from_str("0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47")
                    .unwrap()
            }
            Field::Bn254Scalar => todo!(),
            Field::Secp256k1Base => todo!(),
            Field::Secp256k1Scalar => todo!(),
        }
    }

    fn op(&self, op: FieldOp, x: U256) -> U256 {
        match op {
            FieldOp::Inverse => self.inverse(x),
            FieldOp::Sqrt => todo!(),
        }
    }

    fn inverse(&self, x: U256) -> U256 {
        let n = self.order();
        assert!(x < n);
        modexp(x, n - 2, n)
    }
}

fn modexp(x: U256, e: U256, n: U256) -> U256 {
    let mut current = x;
    let mut product = U256::one();

    for j in 0..256 {
        if !(e >> j & U256::one()).is_zero() {
            product = U256::try_from(product.full_mul(current) % n).unwrap();
        }
        current = U256::try_from(current.full_mul(current) % n).unwrap();
    }
    product
}
