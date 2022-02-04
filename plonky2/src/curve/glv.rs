use num::rational::Ratio;
use plonky2_field::field_types::Field;
use plonky2_field::secp256k1_base::Secp256K1Base;
use plonky2_field::secp256k1_scalar::Secp256K1Scalar;

use crate::curve::curve_msm::msm_parallel;
use crate::curve::curve_types::{ProjectivePoint, AffinePoint};
use crate::curve::secp256k1::Secp256K1;

pub const BETA: Secp256K1Base = Secp256K1Base([
    13923278643952681454,
    11308619431505398165,
    7954561588662645993,
    8856726876819556112,
]);

const S: Secp256K1Scalar = Secp256K1Scalar([
    16069571880186789234,
    1310022930574435960,
    11900229862571533402,
    6008836872998760672,
]);

const A1: Secp256K1Scalar = Secp256K1Scalar([16747920425669159701, 3496713202691238861, 0, 0]);

const MINUS_B1: Secp256K1Scalar =
    Secp256K1Scalar([8022177200260244675, 16448129721693014056, 0, 0]);

const A2: Secp256K1Scalar = Secp256K1Scalar([6323353552219852760, 1498098850674701302, 1, 0]);

const B2: Secp256K1Scalar = Secp256K1Scalar([16747920425669159701, 3496713202691238861, 0, 0]);

pub fn decompose_secp256k1_scalar(k: Secp256K1Scalar) -> (Secp256K1Scalar, Secp256K1Scalar) {
    let p = Secp256K1Scalar::order();
    let c1_biguint = Ratio::new(B2.to_biguint() * k.to_biguint(), p.clone())
        .round()
        .to_integer();
    let c1 = Secp256K1Scalar::from_biguint(c1_biguint);
    let c2_biguint = Ratio::new(MINUS_B1.to_biguint() * k.to_biguint(), p)
        .round()
        .to_integer();
    let c2 = Secp256K1Scalar::from_biguint(c2_biguint);

    let k1 = k - c1 * A1 - c2 * A2;
    let k2 = c1 * MINUS_B1 - c2 * B2;
    debug_assert!(k1 + S * k2 == k);
    (k1, k2)
}

pub fn glv_mul(p: ProjectivePoint<Secp256K1>, k: Secp256K1Scalar) -> ProjectivePoint<Secp256K1> {
    let (k1, k2) = decompose_secp256k1_scalar(k);
    assert!(k1 + S * k2 == k);

    let p_affine = p.to_affine();
    let sp = AffinePoint::<Secp256K1> {
        x: p_affine.x * BETA,
        y: p_affine.y,
        zero: p_affine.zero,
    };

    msm_parallel(&[k1, k2], &[p, sp.to_projective()], 5)
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use plonky2_field::field_types::Field;
    use plonky2_field::secp256k1_scalar::Secp256K1Scalar;

    use crate::curve::curve_types::{Curve, CurveScalar};
    use crate::curve::glv::{decompose_secp256k1_scalar, glv_mul, S};
    use crate::curve::secp256k1::Secp256K1;

    #[test]
    fn test_glv_decompose() -> Result<()> {
        let k = Secp256K1Scalar::rand();
        let (k1, k2) = decompose_secp256k1_scalar(k);

        assert!(k1 + S * k2 == k);

        Ok(())
    }

    #[test]
    fn test_glv_mul() -> Result<()> {
        for _ in 0..20 {
            let k = Secp256K1Scalar::rand();

            let p = CurveScalar(Secp256K1Scalar::rand()) * Secp256K1::GENERATOR_PROJECTIVE;

            let kp = CurveScalar(k) * p;
            let glv = glv_mul(p, k);

            assert!(kp == glv);
        }

        Ok(())
    }
}
