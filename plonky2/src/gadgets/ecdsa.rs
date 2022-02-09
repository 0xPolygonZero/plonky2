use std::marker::PhantomData;

use crate::curve::curve_types::Curve;
use crate::field::extension_field::Extendable;
use crate::gadgets::curve::AffinePointTarget;
use crate::gadgets::nonnative::NonNativeTarget;
use crate::hash::hash_types::RichField;
use crate::plonk::circuit_builder::CircuitBuilder;

#[derive(Clone, Debug)]
pub struct ECDSASecretKeyTarget<C: Curve>(NonNativeTarget<C::ScalarField>);

#[derive(Clone, Debug)]
pub struct ECDSAPublicKeyTarget<C: Curve>(AffinePointTarget<C>);

#[derive(Clone, Debug)]
pub struct ECDSASignatureTarget<C: Curve> {
    pub r: NonNativeTarget<C::ScalarField>,
    pub s: NonNativeTarget<C::ScalarField>,
}

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    pub fn verify_message<C: Curve>(
        &mut self,
        msg: NonNativeTarget<C::ScalarField>,
        sig: ECDSASignatureTarget<C>,
        pk: ECDSAPublicKeyTarget<C>,
    ) {
        let ECDSASignatureTarget { r, s } = sig;

        self.curve_assert_valid(&pk.0);

        let c = self.inv_nonnative(&s);
        let u1 = self.mul_nonnative(&msg, &c);
        let u2 = self.mul_nonnative(&r, &c);

        let g = self.constant_affine_point(C::GENERATOR_AFFINE);
        let point1 = self.curve_scalar_mul(&g, &u1);
        let point2 = self.curve_scalar_mul(&pk.0, &u2);
        let point = self.curve_add(&point1, &point2);

        let x = NonNativeTarget::<C::ScalarField> {
            value: point.x.value,
            _phantom: PhantomData,
        };
        self.connect_nonnative(&r, &x);
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use crate::curve::curve_types::{Curve, CurveScalar};
    use crate::curve::ecdsa::{sign_message, ECDSAPublicKey, ECDSASecretKey, ECDSASignature};
    use crate::curve::secp256k1::Secp256K1;
    use crate::field::field_types::Field;
    use crate::field::secp256k1_scalar::Secp256K1Scalar;
    use crate::gadgets::ecdsa::{ECDSAPublicKeyTarget, ECDSASignatureTarget};
    use crate::iop::witness::PartialWitness;
    use crate::plonk::circuit_builder::CircuitBuilder;
    use crate::plonk::circuit_data::CircuitConfig;
    use crate::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use crate::plonk::verifier::verify;

    #[test]
    #[ignore]
    fn test_ecdsa_circuit() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;

        type Curve = Secp256K1;

        let config = CircuitConfig::standard_ecc_config();

        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let msg = Secp256K1Scalar::rand();
        let msg_target = builder.constant_nonnative(msg);

        let sk = ECDSASecretKey::<Curve>(Secp256K1Scalar::rand());
        let pk = ECDSAPublicKey((CurveScalar(sk.0) * Curve::GENERATOR_PROJECTIVE).to_affine());

        let pk_target = ECDSAPublicKeyTarget(builder.constant_affine_point(pk.0));

        let sig = sign_message(msg, sk);

        let ECDSASignature { r, s } = sig;
        let r_target = builder.constant_nonnative(r);
        let s_target = builder.constant_nonnative(s);
        let sig_target = ECDSASignatureTarget {
            r: r_target,
            s: s_target,
        };

        builder.verify_message(msg_target, sig_target, pk_target);

        let data = builder.build::<C>();
        let proof = data.prove(pw).unwrap();
        verify(proof, &data.verifier_only, &data.common)
    }
}
