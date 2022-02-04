use std::marker::PhantomData;

use plonky2_field::extension_field::Extendable;
use plonky2_field::secp256k1_base::Secp256K1Base;
use plonky2_field::secp256k1_scalar::Secp256K1Scalar;

use crate::curve::glv::{decompose_secp256k1_scalar, BETA};
use crate::curve::secp256k1::Secp256K1;
use crate::gadgets::curve::AffinePointTarget;
use crate::gadgets::nonnative::NonNativeTarget;
use crate::hash::hash_types::RichField;
use crate::iop::generator::{GeneratedValues, SimpleGenerator};
use crate::iop::target::Target;
use crate::iop::witness::{PartitionWitness, Witness};
use crate::plonk::circuit_builder::CircuitBuilder;

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    pub fn secp256k1_glv_beta(&mut self) -> NonNativeTarget<Secp256K1Base> {
        self.constant_nonnative(BETA)
    }

    pub fn decompose_secp256k1_scalar(
        &mut self,
        k: &NonNativeTarget<Secp256K1Scalar>,
    ) -> (
        NonNativeTarget<Secp256K1Scalar>,
        NonNativeTarget<Secp256K1Scalar>,
    ) {
        let k1 = self.add_virtual_nonnative_target::<Secp256K1Scalar>();
        let k2 = self.add_virtual_nonnative_target::<Secp256K1Scalar>();

        self.add_simple_generator(GLVDecompositionGenerator::<F, D> {
            k: k.clone(),
            k1: k1.clone(),
            k2: k2.clone(),
            _phantom: PhantomData,
        });

        (k1, k2)
    }

    pub fn glv_mul(
        &mut self,
        k: &NonNativeTarget<Secp256K1Scalar>,
        p: &AffinePointTarget<Secp256K1>,
    ) -> AffinePointTarget<Secp256K1> {
        let (k1, k2) = self.decompose_secp256k1_scalar(k);

        let beta = self.secp256k1_glv_beta();
        let beta_px = self.mul_nonnative(&beta, &p.x);
        let sp = AffinePointTarget::<Secp256K1> {
            x: beta_px,
            y: p.y.clone(),
        };

        // TODO: replace with MSM
        let part1 = self.curve_scalar_mul(p, &k1);
        let part2 = self.curve_scalar_mul(&sp, &k2);

        self.curve_add(&part1, &part2)
    }
}

#[derive(Debug)]
struct GLVDecompositionGenerator<F: RichField + Extendable<D>, const D: usize> {
    k: NonNativeTarget<Secp256K1Scalar>,
    k1: NonNativeTarget<Secp256K1Scalar>,
    k2: NonNativeTarget<Secp256K1Scalar>,
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
        let (k1, k2) = decompose_secp256k1_scalar(k);

        out_buffer.set_nonnative_target(self.k1.clone(), k1);
        out_buffer.set_nonnative_target(self.k2.clone(), k2);
    }
}

#[cfg(test)]
mod tests {}
