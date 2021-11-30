use std::ops::Mul;

use itertools::unfold;

use crate::curve::curve_types::{AffinePoint, Curve, CurveScalar};
use crate::field::field_types::{Field, RichField};
use crate::hash::hashing::hash_n_to_1;

pub struct ECDSASignature<C: Curve> {
    pub r: C::ScalarField,
    pub s: C::ScalarField,
}

pub struct ECDSASecretKey<C: Curve>(C::ScalarField);
pub struct ECDSAPublicKey<C: Curve>(AffinePoint<C>);

pub fn base_to_scalar<C: Curve>(x: C::BaseField) -> C::ScalarField {
    C::ScalarField::from_biguint(x.to_biguint())
}

pub fn scalar_to_base<C: Curve>(x: C::ScalarField) -> C::BaseField {
    C::BaseField::from_biguint(x.to_biguint())
}

pub fn hash_to_scalar<F: RichField, C: Curve>(msg: F, num_bits: usize) -> C::ScalarField {
    let h_bits = hash_to_bits(msg, num_bits);
    let h_u32 = h_bits
        .iter()
        .zip(0..32)
        .fold(0u32, |acc, (&bit, pow)| acc + (bit as u32) * (2 << pow));
    C::ScalarField::from_canonical_u32(h_u32)
}

pub fn hash_to_bits<F: RichField>(x: F, num_bits: usize) -> Vec<bool> {
    let hashed = hash_n_to_1(vec![x], true);

    let mut val = hashed.to_canonical_u64();
    unfold((), move |_| {
        let ret = val % 2 != 0;
        val /= 2;
        Some(ret)
    })
    .take(num_bits)
    .collect()
}

pub fn sign_message<F: RichField, C: Curve>(msg: F, sk: ECDSASecretKey<C>) -> ECDSASignature<C> {
    let h = hash_to_scalar::<F, C>(msg, 32);
    println!("SIGNING   h: {:?}", h);

    let k = C::ScalarField::rand();
    let rr = (CurveScalar(k) * C::GENERATOR_PROJECTIVE).to_affine();
    let r = base_to_scalar::<C>(rr.x);
    let s = k.inverse() * (h + r * sk.0);

    println!("SIGNING            s: {:?}", s);
    println!("SIGNING         s^-1: {:?}", s.inverse());
    println!("SIGNING      s^-1^-1: {:?}", s.inverse().inverse());

    ECDSASignature { r, s }
}

pub fn verify_message<F: RichField, C: Curve>(
    msg: F,
    sig: ECDSASignature<C>,
    pk: ECDSAPublicKey<C>,
) -> bool {
    let ECDSASignature { r, s } = sig;

    let h = hash_to_scalar::<F, C>(msg, 32);
    println!("VERIFYING h: {:?}", h);

    let c = s.inverse();

    println!("VERIFYING c^-1: {:?}", c.inverse());
    let u1 = h * c;
    let u2 = r * c;

    let g = C::GENERATOR_PROJECTIVE;
    let point_proj = CurveScalar(u1) * g + CurveScalar(u2) * pk.0.to_projective();
    let point = point_proj.to_affine();

    let x = base_to_scalar::<C>(point.x);
    r == x
}

mod tests {
    use anyhow::Result;

    use crate::curve::curve_types::{Curve, CurveScalar};
    use crate::curve::ecdsa::{sign_message, verify_message, ECDSAPublicKey, ECDSASecretKey};
    use crate::curve::secp256k1::Secp256K1;
    use crate::field::field_types::Field;
    use crate::field::goldilocks_field::GoldilocksField;
    use crate::field::secp256k1_scalar::Secp256K1Scalar;
    use crate::plonk::circuit_data::CircuitConfig;

    #[test]
    fn test_ecdsa_native() {
        type F = GoldilocksField;
        type C = Secp256K1;

        let msg = F::rand();
        let sk = ECDSASecretKey(Secp256K1Scalar::rand());
        let pk = ECDSAPublicKey((CurveScalar(sk.0) * C::GENERATOR_PROJECTIVE).to_affine());

        let sig = sign_message(msg, sk);
        let result = verify_message(msg, sig, pk);
        assert!(result);
    }
}
