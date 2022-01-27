use itertools::{unfold, Itertools};
use num::BigUint;

use crate::curve::curve_types::{base_to_scalar, AffinePoint, Curve, CurveScalar};
use crate::field::field_types::Field;
use crate::hash::hash_types::RichField;
use crate::hash::hashing::{hash_n_to_m, PlonkyPermutation};
use crate::hash::poseidon::PoseidonPermutation;

pub struct ECDSASignature<C: Curve> {
    pub r: C::ScalarField,
    pub s: C::ScalarField,
}

pub struct ECDSASecretKey<C: Curve>(pub C::ScalarField);
pub struct ECDSAPublicKey<C: Curve>(pub AffinePoint<C>);

pub fn hash_to_bits<F: RichField, P: PlonkyPermutation<F>>(x: F, num_bits: usize) -> Vec<bool> {
    let hashed = hash_n_to_m::<F, P>(&vec![x], 1, true)[0];

    let mut val = hashed.to_canonical_u64();
    unfold((), move |_| {
        let ret = val % 2 != 0;
        val /= 2;
        Some(ret)
    })
    .take(num_bits)
    .collect()
}

pub fn hash_to_scalar<F: RichField, C: Curve, P: PlonkyPermutation<F>>(
    x: F,
    num_bits: usize,
) -> C::ScalarField {
    let h_bits = hash_to_bits::<F, P>(x, num_bits);
    let h_vals: Vec<_> = h_bits
        .iter()
        .chunks(32)
        .into_iter()
        .map(|chunk| {
            chunk
                .enumerate()
                .fold(0u32, |acc, (pow, &bit)| acc + (bit as u32) * (2 << pow))
        })
        .collect();
    C::ScalarField::from_biguint(BigUint::new(h_vals))
}

pub fn sign_message<F: RichField, C: Curve>(msg: F, sk: ECDSASecretKey<C>) -> ECDSASignature<C> {
    let h = hash_to_scalar::<F, C, PoseidonPermutation>(msg, 256);

    let k = C::ScalarField::rand();
    let rr = (CurveScalar(k) * C::GENERATOR_PROJECTIVE).to_affine();
    let r = base_to_scalar::<C>(rr.x);
    let s = k.inverse() * (h + r * sk.0);

    ECDSASignature { r, s }
}

pub fn verify_message<F: RichField, C: Curve>(
    msg: F,
    sig: ECDSASignature<C>,
    pk: ECDSAPublicKey<C>,
) -> bool {
    let ECDSASignature { r, s } = sig;

    let h = hash_to_scalar::<F, C, PoseidonPermutation>(msg, 256);

    let c = s.inverse();
    let u1 = h * c;
    let u2 = r * c;

    let g = C::GENERATOR_PROJECTIVE;
    let point_proj = CurveScalar(u1) * g + CurveScalar(u2) * pk.0.to_projective();
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
    use crate::field::goldilocks_field::GoldilocksField;
    use crate::field::secp256k1_scalar::Secp256K1Scalar;

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
