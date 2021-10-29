use crate::curve::curve_types::{AffinePoint, Curve};
use crate::field::field_types::Field;
use crate::field::secp256k1_base::Secp256K1Base;
use crate::field::secp256k1_scalar::Secp256K1Scalar;

// Parameters taken from the implementation of Bls12-377 in Zexe found here:
// https://github.com/scipr-lab/zexe/blob/master/algebra/src/curves/bls12_377/g1.rs

#[derive(Debug, Copy, Clone)]
pub struct Secp256K1;

impl Curve for Bls12377 {
    type BaseField = Bls12377Base;
    type ScalarField = Bls12377Scalar;

    const A: Bls12377Base = Bls12377Base::ZERO;
    const B: Bls12377Base = Bls12377Base::ONE;
    const GENERATOR_AFFINE: AffinePoint<Self> = AffinePoint {
        x: BLS12_377_GENERATOR_X,
        y: BLS12_377_GENERATOR_Y,
        zero: false,
    };
}

/// 81937999373150964239938255573465948239988671502647976594219695644855304257327692006745978603320413799295628339695
const BLS12_377_GENERATOR_X: Bls12377Base = Bls12377Base {
    limbs: [2742467569752756724, 14217256487979144792, 6635299530028159197, 8509097278468658840,
        14518893593143693938, 46181716169194829]
};

/// 241266749859715473739788878240585681733927191168601896383759122102112907357779751001206799952863815012735208165030
const BLS12_377_GENERATOR_Y: Bls12377Base = Bls12377Base {
    limbs: [9336971515457667571, 28021381849722296, 18085035374859187530, 14013031479170682136,
        3369780711397861396, 35370409237953649]
};

#[cfg(test)]
mod tests {
    use crate::{blake_hash_usize_to_curve, Bls12377, Bls12377Scalar, Curve, Field, ProjectivePoint};

    #[test]
    fn test_double_affine() {
        for i in 0..100 {
            let p = blake_hash_usize_to_curve::<Bls12377>(i);
            assert_eq!(
                p.double(),
                p.to_projective().double().to_affine());
        }
    }

    #[test]
    fn test_naive_multiplication() {
        let g = Bls12377::GENERATOR_PROJECTIVE;
        let ten = Bls12377Scalar::from_canonical_u64(10);
        let product = mul_naive(ten, g);
        let sum = g + g + g + g + g + g + g + g + g + g;
        assert_eq!(product, sum);
    }

    #[test]
    fn test_g1_multiplication() {
        let lhs = Bls12377Scalar::from_canonical([11111111, 22222222, 33333333, 44444444]);
        assert_eq!(Bls12377::convert(lhs) * Bls12377::GENERATOR_PROJECTIVE, mul_naive(lhs, Bls12377::GENERATOR_PROJECTIVE));
    }

    /// A simple, somewhat inefficient implementation of multiplication which is used as a reference
    /// for correctness.
    fn mul_naive(lhs: Bls12377Scalar, rhs: ProjectivePoint<Bls12377>) -> ProjectivePoint<Bls12377> {
        let mut g = rhs;
        let mut sum = ProjectivePoint::ZERO;
        for limb in lhs.to_canonical().iter() {
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
