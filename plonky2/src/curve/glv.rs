use num::rational::Ratio;
use plonky2_field::field_types::Field;
use plonky2_field::secp256k1_scalar::Secp256K1Scalar;

const BETA: Secp256K1Scalar = Secp256K1Scalar([
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
    let c2_biguint = Ratio::new(MINUS_B1.to_biguint() * k.to_biguint(), p.clone())
        .round()
        .to_integer();
    let c2 = Secp256K1Scalar::from_biguint(c2_biguint);

    let k1 = k - c1 * A1 - c2 * A2;
    let k2 = c1 * MINUS_B1 - c2 * B2;
    debug_assert!(k1 + S * k2 == k);
    (k1, k2)
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use plonky2_field::field_types::Field;
    use plonky2_field::secp256k1_scalar::Secp256K1Scalar;

    use crate::curve::glv::{decompose_secp256k1_scalar, S};

    #[test]
    fn test_glv_decompose() -> Result<()> {
        let k = Secp256K1Scalar::rand();
        let (k1, k2) = decompose_secp256k1_scalar(k);

        assert!(k1 + S * k2 == k);

        Ok(())
    }
}
