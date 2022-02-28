use num::Integer;
use plonky2_field::field_types::PrimeField;
use plonky2_field::secp256k1_scalar::Secp256K1Scalar;
use serde::{Deserialize, Serialize};

use crate::curve::curve_msm::msm_parallel;
use crate::curve::curve_types::{base_to_scalar, scalar_to_base, AffinePoint, Curve, CurveScalar};
use crate::curve::secp256k1::Secp256K1;
use crate::field::field_types::Field;

#[derive(Copy, Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct ECDSASignature<C: Curve> {
    pub r: C::ScalarField,
    pub s: C::ScalarField,
}

#[derive(Copy, Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct ECDSASecretKey<C: Curve>(pub C::ScalarField);

#[derive(Copy, Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct ECDSAPublicKey<C: Curve>(pub AffinePoint<C>);

pub fn secret_to_public<C: Curve>(sk: ECDSASecretKey<C>) -> ECDSAPublicKey<C> {
    ECDSAPublicKey((CurveScalar(sk.0) * C::GENERATOR_PROJECTIVE).to_affine())
}

pub fn sign_message<C: Curve>(
    msg: C::ScalarField,
    sk: ECDSASecretKey<C>,
) -> (ECDSASignature<C>, RecoveryId) {
    let (k, rr) = {
        let mut k = C::ScalarField::rand();
        let mut rr = (CurveScalar(k) * C::GENERATOR_PROJECTIVE).to_affine();
        while rr.x == C::BaseField::ZERO {
            k = C::ScalarField::rand();
            rr = (CurveScalar(k) * C::GENERATOR_PROJECTIVE).to_affine();
        }
        (k, rr)
    };
    let recovery_id = if rr.y.to_canonical_biguint().is_odd() {
        RecoveryId::Odd
    } else {
        RecoveryId::Even
    };
    let r = base_to_scalar::<C>(rr.x);

    let s = k.inverse() * (msg + r * sk.0);

    (ECDSASignature { r, s }, recovery_id)
}

pub fn verify_message<C: Curve>(
    msg: C::ScalarField,
    sig: ECDSASignature<C>,
    pk: ECDSAPublicKey<C>,
) -> bool {
    let ECDSASignature { r, s } = sig;

    assert!(pk.0.is_valid());

    let c = s.inverse();
    let u1 = msg * c;
    let u2 = r * c;

    let g = C::GENERATOR_PROJECTIVE;
    let w = 5; // Experimentally fastest
    let point_proj = msm_parallel(&[u1, u2], &[g, pk.0.to_projective()], w);
    let point = point_proj.to_affine();

    let x = base_to_scalar::<C>(point.x);
    r == x
}

#[derive(Copy, Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum RecoveryId {
    Even,
    Odd,
}

// TODO: Check for overflow, see https://crypto.stackexchange.com/a/18138.
pub fn recover_public_key(
    msg: Secp256K1Scalar,
    sig: ECDSASignature<Secp256K1>,
    recovery_id: RecoveryId,
) -> ECDSAPublicKey<Secp256K1> {
    let ECDSASignature { r, s } = sig;
    let r_scalar = scalar_to_base::<Secp256K1>(r);
    let point = AffinePoint::<Secp256K1>::lift_x(r_scalar, recovery_id);
    let r_inv = r.inverse();
    let u1 = s * r_inv;
    let u2 = -msg * r_inv;

    let g = Secp256K1::GENERATOR_PROJECTIVE;
    ECDSAPublicKey(msm_parallel(&[u1, u2], &[point.to_projective(), g], 5).to_affine())
}

#[cfg(test)]
mod tests {
    use crate::curve::ecdsa::{
        recover_public_key, secret_to_public, sign_message, verify_message, ECDSASecretKey,
    };
    use crate::curve::secp256k1::Secp256K1;
    use crate::field::field_types::Field;
    use crate::field::secp256k1_scalar::Secp256K1Scalar;

    #[test]
    fn test_ecdsa_native() {
        type C = Secp256K1;

        let msg = Secp256K1Scalar::rand();
        let sk = ECDSASecretKey::<C>(Secp256K1Scalar::rand());
        let pk = secret_to_public(sk);

        let (sig, _) = sign_message(msg, sk);
        let result = verify_message(msg, sig, pk);
        assert!(result);
    }

    #[test]
    fn test_ecdsa_ecrecover() {
        type C = Secp256K1;

        let msg = Secp256K1Scalar::rand();
        let sk = ECDSASecretKey::<C>(Secp256K1Scalar::rand());
        let pk = secret_to_public(sk);
        let (sig, rid) = sign_message(msg, sk);
        let recovered_pk = recover_public_key(msg, sig, rid);

        assert_eq!(pk, recovered_pk);
    }
}
