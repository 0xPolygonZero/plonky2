use std::marker::PhantomData;

use plonky2_field::extension_field::Extendable;
use plonky2_field::secp256k1_base::Secp256K1Base;
use plonky2_field::secp256k1_scalar::Secp256K1Scalar;

use crate::curve::glv::{decompose_secp256k1_scalar, GLV_BETA, GLV_S};
use crate::curve::secp256k1::Secp256K1;
use crate::gadgets::curve::AffinePointTarget;
use crate::gadgets::nonnative::NonNativeTarget;
use crate::hash::hash_types::RichField;
use crate::iop::generator::{GeneratedValues, SimpleGenerator};
use crate::iop::target::{BoolTarget, Target};
use crate::iop::witness::{PartitionWitness, Witness};
use crate::plonk::circuit_builder::CircuitBuilder;

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    pub fn secp256k1_glv_beta(&mut self) -> NonNativeTarget<Secp256K1Base> {
        self.constant_nonnative(GLV_BETA)
    }

    // TODO: Add decomposition check.
    pub fn decompose_secp256k1_scalar(
        &mut self,
        k: &NonNativeTarget<Secp256K1Scalar>,
    ) -> (
        NonNativeTarget<Secp256K1Scalar>,
        NonNativeTarget<Secp256K1Scalar>,
        BoolTarget,
        BoolTarget,
    ) {
        let k1 = self.add_virtual_nonnative_target_sized::<Secp256K1Scalar>(4);
        let k2 = self.add_virtual_nonnative_target_sized::<Secp256K1Scalar>(4);
        let k1_neg = self.add_virtual_bool_target();
        let k2_neg = self.add_virtual_bool_target();

        self.add_simple_generator(GLVDecompositionGenerator::<F, D> {
            k: k.clone(),
            k1: k1.clone(),
            k2: k2.clone(),
            k1_neg,
            k2_neg,
            _phantom: PhantomData,
        });

        // Check that `k1_raw + GLV_S * k2_raw == k`.
        let k1_raw = self.nonnative_conditional_neg(&k1, k1_neg);
        let k2_raw = self.nonnative_conditional_neg(&k2, k2_neg);
        let s = self.constant_nonnative(GLV_S);
        let mut should_be_k = self.mul_nonnative(&s, &k2_raw);
        should_be_k = self.add_nonnative(&should_be_k, &k1_raw);
        self.connect_nonnative(&should_be_k, k);

        (k1, k2, k1_neg, k2_neg)
    }

    pub fn glv_mul(
        &mut self,
        p: &AffinePointTarget<Secp256K1>,
        k: &NonNativeTarget<Secp256K1Scalar>,
    ) -> AffinePointTarget<Secp256K1> {
        let (k1, k2, k1_neg, k2_neg) = self.decompose_secp256k1_scalar(k);

        let beta = self.secp256k1_glv_beta();
        let beta_px = self.mul_nonnative(&beta, &p.x);
        let sp = AffinePointTarget::<Secp256K1> {
            x: beta_px,
            y: p.y.clone(),
        };

        let p_neg = self.curve_conditional_neg(p, k1_neg);
        let sp_neg = self.curve_conditional_neg(&sp, k2_neg);
        self.curve_msm(&p_neg, &sp_neg, &k1.value, &k2.value)
    }
}

#[derive(Debug)]
struct GLVDecompositionGenerator<F: RichField + Extendable<D>, const D: usize> {
    k: NonNativeTarget<Secp256K1Scalar>,
    k1: NonNativeTarget<Secp256K1Scalar>,
    k2: NonNativeTarget<Secp256K1Scalar>,
    k1_neg: BoolTarget,
    k2_neg: BoolTarget,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> SimpleGenerator<F>
    for GLVDecompositionGenerator<F, D>
{
    fn dependencies(&self) -> Vec<Target> {
        self.k.value.limbs.iter().map(|l| l.0).collect()
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let k = witness.get_nonnative_target(self.k.clone());
        let (k1, k2, k1_neg, k2_neg) = decompose_secp256k1_scalar(k);

        out_buffer.set_nonnative_target(self.k1.clone(), k1);
        out_buffer.set_nonnative_target(self.k2.clone(), k2);
        out_buffer.set_bool_target(self.k1_neg, k1_neg);
        out_buffer.set_bool_target(self.k2_neg, k2_neg);
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use plonky2_field::field_types::Field;
    use plonky2_field::secp256k1_scalar::Secp256K1Scalar;

    use crate::curve::curve_types::{Curve, CurveScalar};
    use crate::curve::glv::glv_mul;
    use crate::curve::secp256k1::Secp256K1;
    use crate::iop::witness::PartialWitness;
    use crate::plonk::circuit_builder::CircuitBuilder;
    use crate::plonk::circuit_data::CircuitConfig;
    use crate::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use crate::plonk::verifier::verify;

    #[test]
    fn test_glv_gadget() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;

        let config = CircuitConfig::standard_ecc_config();

        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let rando =
            (CurveScalar(Secp256K1Scalar::rand()) * Secp256K1::GENERATOR_PROJECTIVE).to_affine();
        let randot = builder.constant_affine_point(rando);

        let scalar = Secp256K1Scalar::rand();
        let scalar_target = builder.constant_nonnative(scalar);

        let rando_glv_scalar = glv_mul(rando.to_projective(), scalar);
        let expected = builder.constant_affine_point(rando_glv_scalar.to_affine());
        let actual = builder.glv_mul(&randot, &scalar_target);
        builder.connect_affine_point(&expected, &actual);

        dbg!(builder.num_gates());
        let data = builder.build::<C>();
        let proof = data.prove(pw).unwrap();

        verify(proof, &data.verifier_only, &data.common)
    }
}
