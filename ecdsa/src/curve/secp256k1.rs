use plonky2_field::secp256k1_base::Secp256K1Base;
use plonky2_field::secp256k1_scalar::Secp256K1Scalar;
use plonky2_field::types::Field;
use serde::{Deserialize, Serialize};

use crate::curve::curve_types::{AffinePoint, Curve};

#[derive(Debug, Copy, Clone, Deserialize, Eq, Hash, PartialEq, Serialize)]
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

// 55066263022277343669578718895168534326250603453777594175500187360389116729240
const SECP256K1_GENERATOR_X: Secp256K1Base = Secp256K1Base([
    0x59F2815B16F81798,
    0x029BFCDB2DCE28D9,
    0x55A06295CE870B07,
    0x79BE667EF9DCBBAC,
]);

/// 32670510020758816978083085130507043184471273380659243275938904335757337482424
const SECP256K1_GENERATOR_Y: Secp256K1Base = Secp256K1Base([
    0x9C47D08FFB10D4B8,
    0xFD17B448A6855419,
    0x5DA4FBFC0E1108A8,
    0x483ADA7726A3C465,
]);

#[cfg(test)]
mod tests {
    use num::BigUint;
    use plonky2_field::secp256k1_scalar::Secp256K1Scalar;
    use plonky2_field::types::Field;
    use plonky2_field::types::PrimeField;

    use crate::curve::curve_types::{AffinePoint, Curve, ProjectivePoint};
    use crate::curve::secp256k1::Secp256K1;

    #[test]
    fn test_generator() {
        let g = Secp256K1::GENERATOR_AFFINE;
        assert!(g.is_valid());

        let neg_g = AffinePoint::<Secp256K1> {
            x: g.x,
            y: -g.y,
            zero: g.zero,
        };
        assert!(neg_g.is_valid());
    }

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
        let lhs = Secp256K1Scalar::from_noncanonical_biguint(BigUint::from_slice(&[
            1111, 2222, 3333, 4444, 5555, 6666, 7777, 8888,
        ]));
        assert_eq!(
            Secp256K1::convert(lhs) * Secp256K1::GENERATOR_PROJECTIVE,
            mul_naive(lhs, Secp256K1::GENERATOR_PROJECTIVE)
        );
    }

    /// A simple, somewhat inefficient implementation of multiplication which is used as a reference
    /// for correctness.
    fn mul_naive(
        lhs: Secp256K1Scalar,
        rhs: ProjectivePoint<Secp256K1>,
    ) -> ProjectivePoint<Secp256K1> {
        let mut g = rhs;
        let mut sum = ProjectivePoint::ZERO;
        for limb in lhs.to_canonical_biguint().to_u64_digits().iter() {
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
