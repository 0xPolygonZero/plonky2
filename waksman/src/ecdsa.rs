pub struct ECDSASecretKeyTarget<C: Curve>(NonNativeTarget<C::ScalarField>);
pub struct ECDSAPublicKeyTarget<C: Curve>(AffinePointTarget<C>);

pub struct ECDSASignatureTarget<C: Curve> {
    pub r: NonNativeTarget<C::ScalarField>,
    pub s: NonNativeTarget<C::ScalarField>,
}



impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    
}

mod tests {
    use std::ops::{Mul, Neg};

    use anyhow::Result;

    use crate::curve::curve_types::{AffinePoint, Curve, CurveScalar};
    use crate::curve::secp256k1::Secp256K1;
    use crate::field::field_types::Field;
    use crate::field::goldilocks_field::GoldilocksField;
    use crate::field::secp256k1_base::Secp256K1Base;
    use crate::field::secp256k1_scalar::Secp256K1Scalar;
    use crate::iop::witness::PartialWitness;
    use crate::plonk::circuit_builder::CircuitBuilder;
    use crate::plonk::circuit_data::CircuitConfig;
    use crate::plonk::verifier::verify;

    /*#[test]
    fn test_curve_point_is_valid() -> Result<()> {
        type F = GoldilocksField;
        const D: usize = 4;

        let config = CircuitConfig::standard_recursion_config();

        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let g = Secp256K1::GENERATOR_AFFINE;
        let g_target = builder.constant_affine_point(g);
        let neg_g_target = builder.curve_neg(&g_target);

        builder.curve_assert_valid(&g_target);
        builder.curve_assert_valid(&neg_g_target);

        let data = builder.build();
        let proof = data.prove(pw).unwrap();

        verify(proof, &data.verifier_only, &data.common)
    }*/
}
