use crate::curve::curve_types::{AffinePoint, Curve};
use crate::field::field_types::Field;
use crate::field::secp256k1_base::Secp256K1Base;
use crate::field::secp256k1_scalar::Secp256K1Scalar;

// Parameters taken from the implementation of Bls12-377 in Zexe found here:
// https://github.com/scipr-lab/zexe/blob/master/algebra/src/curves/bls12_377/g1.rs

#[derive(Debug, Copy, Clone)]
pub struct Secp256K1;

impl Curve for Secp256K1 {
    type BaseField = Secp256K1Base;
    type ScalarField = Secp256K1Scalar;

    const A: Secp256K1Base = Secp256K1Base::ZERO;
    const B: Secp256K1Base = Secp256K1Base([7, 0, 0, 0]);
    const GENERATOR_AFFINE: AffinePoint<Self> = AffinePoint {
        x: SECP256K1_GENERATOR_X,
        y: SECP256K1_GENERATOR_Y,
        zero: false,
    };
}

const SECP256K1_GENERATOR_X: Secp256K1Base = Secp256K1Base([
    0x59F2815B16F81798,
    0x029BFCDB2DCE28D9,
    0x55A06295CE870B07,
    0x79BE667EF9DCBBAC,
]);

/// 241266749859715473739788878240585681733927191168601896383759122102112907357779751001206799952863815012735208165030
const SECP256K1_GENERATOR_Y: Secp256K1Base = Secp256K1Base([
    0x9C47D08FFB10D4B8,
    0xFD17B448A6855419,
    0x5DA4FBFC0E1108A8,
    0x483ADA7726A3C465,
]);

#[cfg(test)]
mod tests {
    use num::BigUint;

    use crate::curve::curve_types::{Curve, ProjectivePoint};
    use crate::curve::secp256k1_curve::Secp256K1;
    use crate::field::field_types::Field;
    use crate::field::secp256k1_scalar::Secp256K1Scalar;

    /*#[test]
    fn test_double_affine() {
        for i in 0..100 {
            //let p = blake_hash_usize_to_curve::<Secp256K1>(i);
            assert_eq!(
                p.double(),
                p.to_projective().double().to_affine());
        }
    }*/

    #[test]
    fn test_naive_multiplication() {
        let g = Secp256K1::GENERATOR_PROJECTIVE;
        let ten = Secp256K1Scalar::from_canonical_u64(10);
        let product = mul_naive(ten, g);
        let sum = g + g + g + g + g + g + g + g + g + g;
        assert_eq!(product, sum);
    }

    #[test]
    fn test_g1_multiplication() {
        let lhs = Secp256K1Scalar::from_biguint(
            BigUint::from_slice(&[1111, 2222, 3333, 4444, 5555, 6666, 7777, 8888])
        );
        assert_eq!(Secp256K1::convert(lhs) * Secp256K1::GENERATOR_PROJECTIVE, mul_naive(lhs, Secp256K1::GENERATOR_PROJECTIVE));
    }

    /// A simple, somewhat inefficient implementation of multiplication which is used as a reference
    /// for correctness.
    fn mul_naive(lhs: Secp256K1Scalar, rhs: ProjectivePoint<Secp256K1>) -> ProjectivePoint<Secp256K1> {
        let mut g = rhs;
        let mut sum = ProjectivePoint::ZERO;
        for limb in lhs.to_biguint().to_u64_digits().iter() {
            for j in 0..64 {
                if (limb >> j & 1u64) != 0u64 {
                    sum = sum + g;
                }
                g = g.double();
            }
        }
        sum
    }
}
