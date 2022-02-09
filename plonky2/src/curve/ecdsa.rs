use crate::curve::curve_msm::msm_parallel;
use crate::curve::curve_types::{base_to_scalar, AffinePoint, Curve, CurveScalar};
use crate::field::field_types::Field;

#[derive(Copy, Clone, Debug)]
pub struct ECDSASignature<C: Curve> {
    pub r: C::ScalarField,
    pub s: C::ScalarField,
}

#[derive(Copy, Clone, Debug)]
pub struct ECDSASecretKey<C: Curve>(pub C::ScalarField);

#[derive(Copy, Clone, Debug)]
pub struct ECDSAPublicKey<C: Curve>(pub AffinePoint<C>);

pub fn sign_message<C: Curve>(msg: C::ScalarField, sk: ECDSASecretKey<C>) -> ECDSASignature<C> {
    let (k, rr) = {
        let mut k = C::ScalarField::rand();
        let mut rr = (CurveScalar(k) * C::GENERATOR_PROJECTIVE).to_affine();
        while rr.x == C::BaseField::ZERO {
            k = C::ScalarField::rand();
            rr = (CurveScalar(k) * C::GENERATOR_PROJECTIVE).to_affine();
        }
        (k, rr)
    };
    let r = base_to_scalar::<C>(rr.x);

    let s = k.inverse() * (msg + r * sk.0);

    ECDSASignature { r, s }
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

#[cfg(test)]
mod tests {
    use crate::curve::curve_types::{Curve, CurveScalar};
    use crate::curve::ecdsa::{sign_message, verify_message, ECDSAPublicKey, ECDSASecretKey};
    use crate::curve::secp256k1::Secp256K1;
    use crate::field::field_types::Field;
    use crate::field::secp256k1_scalar::Secp256K1Scalar;

    #[test]
    fn test_ecdsa_native() {
        type C = Secp256K1;

        let msg = Secp256K1Scalar::rand();
        let sk = ECDSASecretKey(Secp256K1Scalar::rand());
        let pk = ECDSAPublicKey((CurveScalar(sk.0) * C::GENERATOR_PROJECTIVE).to_affine());

        let sig = sign_message(msg, sk);
        let result = verify_message(msg, sig, pk);
        assert!(result);
    }
}
