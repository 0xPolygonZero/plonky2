use std::marker::PhantomData;

use plonky2::hash::hash_types::RichField;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2_field::extension::Extendable;
use plonky2_field::secp256k1_scalar::Secp256K1Scalar;

use crate::curve::curve_types::Curve;
use crate::curve::secp256k1::Secp256K1;
use crate::gadgets::curve::{AffinePointTarget, CircuitBuilderCurve};
use crate::gadgets::curve_fixed_base::fixed_base_curve_mul_circuit;
use crate::gadgets::glv::CircuitBuilderGlv;
use crate::gadgets::nonnative::{CircuitBuilderNonNative, NonNativeTarget};

#[derive(Clone, Debug)]
pub struct ECDSASecretKeyTarget<C: Curve>(pub NonNativeTarget<C::ScalarField>);

#[derive(Clone, Debug)]
pub struct ECDSAPublicKeyTarget<C: Curve>(pub AffinePointTarget<C>);

#[derive(Clone, Debug)]
pub struct ECDSASignatureTarget<C: Curve> {
    pub r: NonNativeTarget<C::ScalarField>,
    pub s: NonNativeTarget<C::ScalarField>,
}

pub fn verify_message_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    msg: NonNativeTarget<Secp256K1Scalar>,
    sig: ECDSASignatureTarget<Secp256K1>,
    pk: ECDSAPublicKeyTarget<Secp256K1>,
) {
    let ECDSASignatureTarget { r, s } = sig;

    builder.curve_assert_valid(&pk.0);

    let c = builder.inv_nonnative(&s);
    let u1 = builder.mul_nonnative(&msg, &c);
    let u2 = builder.mul_nonnative(&r, &c);

    let point1 = fixed_base_curve_mul_circuit(builder, Secp256K1::GENERATOR_AFFINE, &u1);
    let point2 = builder.glv_mul(&pk.0, &u2);
    let point = builder.curve_add(&point1, &point2);

    let x = NonNativeTarget::<Secp256K1Scalar> {
        value: point.x.value,
        _phantom: PhantomData,
    };
    builder.connect_nonnative(&r, &x);
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use plonky2::iop::witness::PartialWitness;
    use plonky2::plonk::circuit_builder::CircuitBuilder;
    use plonky2::plonk::circuit_data::CircuitConfig;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use plonky2_field::secp256k1_scalar::Secp256K1Scalar;
    use plonky2_field::types::Field;

    use super::{ECDSAPublicKeyTarget, ECDSASignatureTarget};
    use crate::curve::curve_types::{Curve, CurveScalar};
    use crate::curve::ecdsa::{sign_message, ECDSAPublicKey, ECDSASecretKey, ECDSASignature};
    use crate::curve::secp256k1::Secp256K1;
    use crate::gadgets::curve::CircuitBuilderCurve;
    use crate::gadgets::ecdsa::verify_message_circuit;
    use crate::gadgets::nonnative::CircuitBuilderNonNative;

    fn test_ecdsa_circuit_with_config(config: CircuitConfig) -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;

        type Curve = Secp256K1;

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

        verify_message_circuit(&mut builder, msg_target, sig_target, pk_target);

        dbg!(builder.num_gates());
        let data = builder.build::<C>();
        let proof = data.prove(pw).unwrap();
        data.verify(proof)
    }

    #[test]
    #[ignore]
    fn test_ecdsa_circuit_narrow() -> Result<()> {
        test_ecdsa_circuit_with_config(CircuitConfig::standard_ecc_config())
    }

    #[test]
    #[ignore]
    fn test_ecdsa_circuit_wide() -> Result<()> {
        test_ecdsa_circuit_with_config(CircuitConfig::wide_ecc_config())
    }
}
