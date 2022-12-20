use std::str::FromStr;

use ethereum_types::{BigEndianHash, H256, U256};
use plonky2::field::types::Field;

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
            FieldExtOp::ExtInv0 => self.ext_inv0(xs),
            FieldExtOp::ExtInv1 => self.ext_inv1(xs),
            FieldExtOp::ExtInv2 => self.ext_inv2(xs),
            FieldExtOp::ExtInv3 => self.ext_inv3(xs),
            FieldExtOp::ExtInv4 => self.ext_inv4(xs),
            FieldExtOp::ExtInv5 => self.ext_inv5(xs),
            FieldExtOp::ExtInv6 => self.ext_inv6(xs),
            FieldExtOp::ExtInv7 => self.ext_inv7(xs),
            FieldExtOp::ExtInv8 => self.ext_inv8(xs),
            FieldExtOp::ExtInv9 => self.ext_inv9(xs),
            FieldExtOp::ExtInv10 => self.ext_inv10(xs),
            FieldExtOp::ExtInv11 => self.ext_inv11(xs),
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

    fn ext_inv(&self, xs: Vec<U256>, offset: usize) -> [U256; 12] {
        let f0 = xs.clone().into_iter().nth(offset).unwrap();
        let f1 = xs.clone().into_iter().nth(offset + 1).unwrap();
        let f2 = xs.clone().into_iter().nth(offset + 2).unwrap();
        let f3 = xs.clone().into_iter().nth(offset + 3).unwrap();
        let f4 = xs.clone().into_iter().nth(offset + 4).unwrap();
        let f5 = xs.clone().into_iter().nth(offset + 5).unwrap();
        let f6 = xs.clone().into_iter().nth(offset + 6).unwrap();
        let f7 = xs.clone().into_iter().nth(offset + 7).unwrap();
        let f8 = xs.clone().into_iter().nth(offset + 8).unwrap();
        let f9 = xs.clone().into_iter().nth(offset + 9).unwrap();
        let f10 = xs.clone().into_iter().nth(offset + 10).unwrap();
        let f11 = xs.clone().into_iter().nth(offset + 11).unwrap();

        let f = [
            [[f0, f1], [f2, f3], [f4, f5]],
            [[f6, f7], [f8, f9], [f10, f11]],
        ];

        let g = inv_fp12(f);

        [
            g[0][0][0], g[0][0][1], g[0][1][0], g[0][1][1], g[0][2][0], g[0][2][1], g[1][0][0],
            g[1][0][1], g[1][1][0], g[1][1][1], g[1][2][0], g[1][2][1],
        ]
    }

    fn ext_inv0(&self, xs: Vec<U256>) -> U256 {
        Self::ext_inv(&self, xs, 12)[0]
    }

    fn ext_inv1(&self, xs: Vec<U256>) -> U256 {
        Self::ext_inv(&self, xs, 11)[1]
    }

    fn ext_inv2(&self, xs: Vec<U256>) -> U256 {
        Self::ext_inv(&self, xs, 10)[2]
    }

    fn ext_inv3(&self, xs: Vec<U256>) -> U256 {
        Self::ext_inv(&self, xs, 9)[3]
    }

    fn ext_inv4(&self, xs: Vec<U256>) -> U256 {
        Self::ext_inv(&self, xs, 8)[4]
    }

    fn ext_inv5(&self, xs: Vec<U256>) -> U256 {
        Self::ext_inv(&self, xs, 7)[5]
    }

    fn ext_inv6(&self, xs: Vec<U256>) -> U256 {
        Self::ext_inv(&self, xs, 6)[6]
    }

    fn ext_inv7(&self, xs: Vec<U256>) -> U256 {
        Self::ext_inv(&self, xs, 5)[7]
    }

    fn ext_inv8(&self, xs: Vec<U256>) -> U256 {
        Self::ext_inv(&self, xs, 4)[8]
    }

    fn ext_inv9(&self, xs: Vec<U256>) -> U256 {
        Self::ext_inv(&self, xs, 3)[9]
    }

    fn ext_inv10(&self, xs: Vec<U256>) -> U256 {
        Self::ext_inv(&self, xs, 2)[10]
    }

    fn ext_inv11(&self, xs: Vec<U256>) -> U256 {
        Self::ext_inv(&self, xs, 1)[11]
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

type Fp = U256;
type Fp2 = [U256; 2];
type Fp6 = [Fp2; 3];
type Fp12 = [Fp6; 2];

const ZERO: Fp = U256([0, 0, 0, 0]);

const BN_BASE: U256 = U256([
    4332616871279656263,
    10917124144477883021,
    13281191951274694749,
    3486998266802970665,
]);

fn embed_fp2(x: Fp) -> Fp2 {
    [x, ZERO]
}

fn embed_fp2_fp6(a: Fp2) -> Fp6 {
    [a, embed_fp2(ZERO), embed_fp2(ZERO)]
}

fn embed_fp6(x: Fp) -> Fp6 {
    embed_fp2_fp6(embed_fp2(x))
}

fn embed_fp12(x: Fp) -> Fp12 {
    [embed_fp6(x), embed_fp6(ZERO)]
}

fn add_fp(x: Fp, y: Fp) -> Fp {
    (x + y) % BN_BASE
}

fn add3_fp(x: Fp, y: Fp, z: Fp) -> Fp {
    (x + y + z) % BN_BASE
}

fn mul_fp(x: Fp, y: Fp) -> Fp {
    U256::try_from(x.full_mul(y) % BN_BASE).unwrap()
}

fn sub_fp(x: Fp, y: Fp) -> Fp {
    (BN_BASE + x - y) % BN_BASE
}

fn neg_fp(x: Fp) -> Fp {
    (BN_BASE - x) % BN_BASE
}

fn conj_fp2(a: Fp2) -> Fp2 {
    let [a, a_] = a;
    [a, neg_fp(a_)]
}

fn add_fp2(a: Fp2, b: Fp2) -> Fp2 {
    let [a, a_] = a;
    let [b, b_] = b;
    [add_fp(a, b), add_fp(a_, b_)]
}

fn add3_fp2(a: Fp2, b: Fp2, c: Fp2) -> Fp2 {
    let [a, a_] = a;
    let [b, b_] = b;
    let [c, c_] = c;
    [add3_fp(a, b, c), add3_fp(a_, b_, c_)]
}

fn sub_fp2(a: Fp2, b: Fp2) -> Fp2 {
    let [a, a_] = a;
    let [b, b_] = b;
    [sub_fp(a, b), sub_fp(a_, b_)]
}

fn neg_fp2(a: Fp2) -> Fp2 {
    sub_fp2(embed_fp2(ZERO), a)
}

fn mul_fp2(a: Fp2, b: Fp2) -> Fp2 {
    let [a, a_] = a;
    let [b, b_] = b;
    [
        sub_fp(mul_fp(a, b), mul_fp(a_, b_)),
        add_fp(mul_fp(a, b_), mul_fp(a_, b)),
    ]
}

fn i9(a: Fp2) -> Fp2 {
    let [a, a_] = a;
    let nine = U256::from(9);
    [sub_fp(mul_fp(nine, a), a_), add_fp(a, mul_fp(nine, a_))]
}

fn add_fp6(c: Fp6, d: Fp6) -> Fp6 {
    let [c0, c1, c2] = c;
    let [d0, d1, d2] = d;

    let e0 = add_fp2(c0, d0);
    let e1 = add_fp2(c1, d1);
    let e2 = add_fp2(c2, d2);
    [e0, e1, e2]
}

fn sub_fp6(c: Fp6, d: Fp6) -> Fp6 {
    let [c0, c1, c2] = c;
    let [d0, d1, d2] = d;

    let e0 = sub_fp2(c0, d0);
    let e1 = sub_fp2(c1, d1);
    let e2 = sub_fp2(c2, d2);
    [e0, e1, e2]
}

fn neg_fp6(a: Fp6) -> Fp6 {
    sub_fp6(embed_fp6(ZERO), a)
}

fn mul_fp6(c: Fp6, d: Fp6) -> Fp6 {
    let [c0, c1, c2] = c;
    let [d0, d1, d2] = d;

    let c0d0 = mul_fp2(c0, d0);
    let c0d1 = mul_fp2(c0, d1);
    let c0d2 = mul_fp2(c0, d2);
    let c1d0 = mul_fp2(c1, d0);
    let c1d1 = mul_fp2(c1, d1);
    let c1d2 = mul_fp2(c1, d2);
    let c2d0 = mul_fp2(c2, d0);
    let c2d1 = mul_fp2(c2, d1);
    let c2d2 = mul_fp2(c2, d2);
    let cd12 = add_fp2(c1d2, c2d1);

    [
        add_fp2(c0d0, i9(cd12)),
        add3_fp2(c0d1, c1d0, i9(c2d2)),
        add3_fp2(c0d2, c1d1, c2d0),
    ]
}

fn sh(c: Fp6) -> Fp6 {
    let [c0, c1, c2] = c;
    [i9(c2), c0, c1]
}

fn sparse_embed(x: [U256; 5]) -> Fp12 {
    let [g0, g1, g1_, g2, g2_] = x;
    [
        [embed_fp2(g0), [g1, g1_], embed_fp2(ZERO)],
        [embed_fp2(ZERO), [g2, g2_], embed_fp2(ZERO)],
    ]
}

fn mul_fp12(f: Fp12, g: Fp12) -> Fp12 {
    let [f0, f1] = f;
    let [g0, g1] = g;

    let h0 = mul_fp6(f0, g0);
    let h1 = mul_fp6(f1, g1);
    let h01 = mul_fp6(add_fp6(f0, f1), add_fp6(g0, g1));
    [add_fp6(h0, sh(h1)), sub_fp6(h01, add_fp6(h0, h1))]
}

fn frob_t1(n: usize) -> Fp2 {
    match n {
        0 => [
            U256::from_str("0x1").unwrap(),
            U256::from_str("0x0").unwrap(),
        ],
        1 => [
            U256::from_str("0x2fb347984f7911f74c0bec3cf559b143b78cc310c2c3330c99e39557176f553d")
                .unwrap(),
            U256::from_str("0x16c9e55061ebae204ba4cc8bd75a079432ae2a1d0b7c9dce1665d51c640fcba2")
                .unwrap(),
        ],
        2 => [
            U256::from_str("0x30644e72e131a0295e6dd9e7e0acccb0c28f069fbb966e3de4bd44e5607cfd48")
                .unwrap(),
            U256::from_str("0x0").unwrap(),
        ],
        3 => [
            U256::from_str("0x856e078b755ef0abaff1c77959f25ac805ffd3d5d6942d37b746ee87bdcfb6d")
                .unwrap(),
            U256::from_str("0x4f1de41b3d1766fa9f30e6dec26094f0fdf31bf98ff2631380cab2baaa586de")
                .unwrap(),
        ],
        4 => [
            U256::from_str("0x59e26bcea0d48bacd4f263f1acdb5c4f5763473177fffffe").unwrap(),
            U256::from_str("0x0").unwrap(),
        ],
        5 => [
            U256::from_str("0x28be74d4bb943f51699582b87809d9caf71614d4b0b71f3a62e913ee1dada9e4")
                .unwrap(),
            U256::from_str("0x14a88ae0cb747b99c2b86abcbe01477a54f40eb4c3f6068dedae0bcec9c7aac7")
                .unwrap(),
        ],
        _ => panic!(),
    }
}

fn frob_t2(n: usize) -> Fp2 {
    match n {
        0 => [
            U256::from_str("0x1").unwrap(),
            U256::from_str("0x0").unwrap(),
        ],
        1 => [
            U256::from_str("0x5b54f5e64eea80180f3c0b75a181e84d33365f7be94ec72848a1f55921ea762")
                .unwrap(),
            U256::from_str("0x2c145edbe7fd8aee9f3a80b03b0b1c923685d2ea1bdec763c13b4711cd2b8126")
                .unwrap(),
        ],
        2 => [
            U256::from_str("0x59e26bcea0d48bacd4f263f1acdb5c4f5763473177fffffe").unwrap(),
            U256::from_str("0x0").unwrap(),
        ],
        3 => [
            U256::from_str("0xbc58c6611c08dab19bee0f7b5b2444ee633094575b06bcb0e1a92bc3ccbf066")
                .unwrap(),
            U256::from_str("0x23d5e999e1910a12feb0f6ef0cd21d04a44a9e08737f96e55fe3ed9d730c239f")
                .unwrap(),
        ],
        4 => [
            U256::from_str("0x30644e72e131a0295e6dd9e7e0acccb0c28f069fbb966e3de4bd44e5607cfd48")
                .unwrap(),
            U256::from_str("0x0").unwrap(),
        ],
        5 => [
            U256::from_str("0x1ee972ae6a826a7d1d9da40771b6f589de1afb54342c724fa97bda050992657f")
                .unwrap(),
            U256::from_str("0x10de546ff8d4ab51d2b513cdbb25772454326430418536d15721e37e70c255c9")
                .unwrap(),
        ],
        _ => panic!(),
    }
}

fn frob_z(n: usize) -> Fp2 {
    match n {
        0 => [
            U256::from_str("0x1").unwrap(),
            U256::from_str("0x0").unwrap(),
        ],
        1 => [
            U256::from_str("0x1284b71c2865a7dfe8b99fdd76e68b605c521e08292f2176d60b35dadcc9e470")
                .unwrap(),
            U256::from_str("0x246996f3b4fae7e6a6327cfe12150b8e747992778eeec7e5ca5cf05f80f362ac")
                .unwrap(),
        ],
        2 => [
            U256::from_str("0x30644e72e131a0295e6dd9e7e0acccb0c28f069fbb966e3de4bd44e5607cfd49")
                .unwrap(),
            U256::from_str("0x0").unwrap(),
        ],
        3 => [
            U256::from_str("0x19dc81cfcc82e4bbefe9608cd0acaa90894cb38dbe55d24ae86f7d391ed4a67f")
                .unwrap(),
            U256::from_str("0xabf8b60be77d7306cbeee33576139d7f03a5e397d439ec7694aa2bf4c0c101")
                .unwrap(),
        ],
        4 => [
            U256::from_str("0x30644e72e131a0295e6dd9e7e0acccb0c28f069fbb966e3de4bd44e5607cfd48")
                .unwrap(),
            U256::from_str("0x0").unwrap(),
        ],
        5 => [
            U256::from_str("0x757cab3a41d3cdc072fc0af59c61f302cfa95859526b0d41264475e420ac20f")
                .unwrap(),
            U256::from_str("0xca6b035381e35b618e9b79ba4e2606ca20b7dfd71573c93e85845e34c4a5b9c")
                .unwrap(),
        ],
        6 => [
            U256::from_str("0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd46")
                .unwrap(),
            U256::from_str("0x0").unwrap(),
        ],
        7 => [
            U256::from_str("0x1ddf9756b8cbf849cf96a5d90a9accfd3b2f4c893f42a9166615563bfbb318d7")
                .unwrap(),
            U256::from_str("0xbfab77f2c36b843121dc8b86f6c4ccf2307d819d98302a771c39bb757899a9b")
                .unwrap(),
        ],
        8 => [
            U256::from_str("0x59e26bcea0d48bacd4f263f1acdb5c4f5763473177fffffe").unwrap(),
            U256::from_str("0x0").unwrap(),
        ],
        9 => [
            U256::from_str("0x1687cca314aebb6dc866e529b0d4adcd0e34b703aa1bf84253b10eddb9a856c8")
                .unwrap(),
            U256::from_str("0x2fb855bcd54a22b6b18456d34c0b44c0187dc4add09d90a0c58be1eae3bc3c46")
                .unwrap(),
        ],
        10 => [
            U256::from_str("0x59e26bcea0d48bacd4f263f1acdb5c4f5763473177ffffff").unwrap(),
            U256::from_str("0x0").unwrap(),
        ],
        11 => [
            U256::from_str("0x290c83bf3d14634db120850727bb392d6a86d50bd34b19b929bc44b896723b38")
                .unwrap(),
            U256::from_str("0x23bd9e3da9136a739f668e1adc9ef7f0f575ec93f71a8df953c846338c32a1ab")
                .unwrap(),
        ],
        _ => panic!(),
    }
}

fn frob_fp6(n: usize, c: Fp6) -> Fp6 {
    let [c0, c1, c2] = c;
    let _c0 = conj_fp2(c0);
    let _c1 = conj_fp2(c1);
    let _c2 = conj_fp2(c2);

    let n = n % 6;
    let frob_t1 = frob_t1(n);
    let frob_t2 = frob_t2(n);

    if n % 2 != 0 {
        [_c0, mul_fp2(frob_t1, _c1), mul_fp2(frob_t2, _c2)]
    } else {
        [c0, mul_fp2(frob_t1, c1), mul_fp2(frob_t2, c2)]
    }
}

fn frob_fp12(n: usize, f: Fp12) -> Fp12 {
    let [f0, f1] = f;
    let scale = embed_fp2_fp6(frob_z(n));

    [frob_fp6(n, f0), mul_fp6(scale, frob_fp6(n, f1))]
}

fn exp_fp(x: Fp, e: U256) -> Fp {
    let mut current = x;
    let mut product = U256::one();

    for j in 0..256 {
        if e.bit(j) {
            product = U256::try_from(product.full_mul(current) % BN_BASE).unwrap();
        }
        current = U256::try_from(current.full_mul(current) % BN_BASE).unwrap();
    }
    product
}

fn inv_fp(x: Fp) -> Fp {
    exp_fp(x, BN_BASE - 2)
}

fn inv_fp2(a: Fp2) -> Fp2 {
    let [a0, a1] = a;
    let norm = inv_fp(mul_fp(a0, a0) + mul_fp(a1, a1));
    [mul_fp(norm, a0), neg_fp(mul_fp(norm, a1))]
}

fn inv_fp6(c: Fp6) -> Fp6 {
    let b = mul_fp6(frob_fp6(1, c), frob_fp6(3, c));
    let e = mul_fp6(b, frob_fp6(5, c))[0];
    let n = mul_fp2(e, conj_fp2(e))[0];
    let i = inv_fp(n);
    let d = mul_fp2(embed_fp2(i), e);
    let [f0, f1, f2] = frob_fp6(1, b);
    [mul_fp2(d, f0), mul_fp2(d, f1), mul_fp2(d, f2)]
}

fn inv_fp12(f: Fp12) -> Fp12 {
    let a = mul_fp12(frob_fp12(1, f), frob_fp12(7, f))[0];
    let b = mul_fp6(a, frob_fp6(2, a));
    let c = mul_fp6(b, frob_fp6(4, a))[0];
    let n = mul_fp2(c, conj_fp2(c))[0];
    let i = inv_fp(n);
    let d = mul_fp2(embed_fp2(i), c);
    let [g0, g1, g2] = frob_fp6(1, b);
    let e = [mul_fp2(d, g0), mul_fp2(d, g1), mul_fp2(d, g2)];
    [mul_fp6(e, f[0]), neg_fp6(mul_fp6(e, f[1]))]
}
