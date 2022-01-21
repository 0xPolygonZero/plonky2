use std::marker::PhantomData;

use crate::curve::curve_types::Curve;
use crate::field::extension_field::Extendable;
use crate::field::field_types::RichField;
use crate::gadgets::arithmetic_u32::U32Target;
use crate::gadgets::biguint::BigUintTarget;
use crate::gadgets::curve::AffinePointTarget;
use crate::gadgets::nonnative::NonNativeTarget;
use crate::iop::target::{BoolTarget, Target};
use crate::plonk::circuit_builder::CircuitBuilder;

pub struct ECDSASecretKeyTarget<C: Curve>(NonNativeTarget<C::ScalarField>);
pub struct ECDSAPublicKeyTarget<C: Curve>(AffinePointTarget<C>);

pub struct ECDSASignatureTarget<C: Curve> {
    pub r: NonNativeTarget<C::ScalarField>,
    pub s: NonNativeTarget<C::ScalarField>,
}

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    pub fn hash_to_bits(&mut self, x: Target, num_bits: usize) -> Vec<BoolTarget> {
        let inputs = vec![x];
        let hashed = self.hash_n_to_m(inputs, 1, true)[0];
        self.split_le(hashed, num_bits)
    }

    pub fn hash_to_scalar<C: Curve>(
        &mut self,
        x: Target,
        num_bits: usize,
    ) -> NonNativeTarget<C::ScalarField> {
        let h_bits = self.hash_to_bits(x, num_bits);

        let two = self.two();
        let mut rev_bits = h_bits.iter().rev();
        let mut sum = rev_bits.next().unwrap().target;
        for &bit in rev_bits {
            sum = self.mul_add(two, sum, bit.target);
        }
        let limbs = vec![U32Target(sum)];
        let value = BigUintTarget { limbs };

        NonNativeTarget {
            value,
            _phantom: PhantomData,
        }
    }

    pub fn verify_message<C: Curve>(
        &mut self,
        msg: Target,
        sig: ECDSASignatureTarget<C>,
        pk: ECDSAPublicKeyTarget<C>,
    ) {
        let ECDSASignatureTarget { r, s } = sig;

        let h = self.hash_to_scalar::<C>(msg, 32);

        let c = self.inv_nonnative(&s);
        let u1 = self.mul_nonnative(&h, &c);
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
    use crate::field::goldilocks_field::GoldilocksField;
    use crate::field::secp256k1_scalar::Secp256K1Scalar;
    use crate::gadgets::ecdsa::{ECDSAPublicKeyTarget, ECDSASignatureTarget};
    use crate::iop::witness::PartialWitness;
    use crate::plonk::circuit_builder::CircuitBuilder;
    use crate::plonk::circuit_data::CircuitConfig;
    use crate::plonk::verifier::verify;

    #[test]
    fn test_ecdsa_circuit() -> Result<()> {
        type F = GoldilocksField;
        const D: usize = 4;
        type C = Secp256K1;

        let config = CircuitConfig::standard_recursion_config();

        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let msg = F::rand();
        let msg_target = builder.constant(msg);

        let sk = ECDSASecretKey::<C>(Secp256K1Scalar::rand());
        let pk = ECDSAPublicKey((CurveScalar(sk.0) * C::GENERATOR_PROJECTIVE).to_affine());

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

        let data = builder.build();
        let proof = data.prove(pw).unwrap();

        verify(proof, &data.verifier_only, &data.common)
    }
}
