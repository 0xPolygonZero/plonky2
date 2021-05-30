use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::target::{ExtensionExtensionTarget, ExtensionTarget};
use crate::field::extension_field::Extendable;
use crate::target::Target;

pub struct PolynomialCoeffsExtTarget<const D: usize>(pub Vec<ExtensionTarget<D>>);

impl<const D: usize> PolynomialCoeffsExtTarget<D> {
    pub fn eval_scalar<F: Extendable<D>>(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        point: Target,
    ) -> ExtensionTarget<D> {
        let mut acc = builder.zero_extension();
        for &c in self.0.iter().rev() {
            let tmp = builder.scalar_mul_ext(point, acc);
            acc = builder.add_extension(tmp, c);
        }
        acc
    }

    pub fn eval<F: Extendable<D>>(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        point: ExtensionTarget<D>,
    ) -> ExtensionTarget<D> {
        let mut acc = builder.zero_extension();
        for &c in self.0.iter().rev() {
            let tmp = builder.mul_extension(point, acc);
            acc = builder.add_extension(tmp, c);
        }
        acc
    }
}

pub struct PolynomialCoeffsExtExtTarget<const D: usize>(pub Vec<ExtensionExtensionTarget<D>>);

impl<const D: usize> PolynomialCoeffsExtExtTarget<D> {
    pub fn eval_scalar<F>(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        point: ExtensionTarget<D>,
    ) -> ExtensionExtensionTarget<D>
    where
        F: Extendable<D>,
        F::Extension: Extendable<D>,
    {
        let mut acc = builder.zero_ext_ext();
        for &c in self.0.iter().rev() {
            let tmp = builder.scalar_mul_ext_ext(point, acc);
            acc = builder.add_ext_ext(tmp, c);
        }
        acc
    }

    pub fn eval<F>(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        point: ExtensionExtensionTarget<D>,
    ) -> ExtensionExtensionTarget<D>
    where
        F: Extendable<D>,
        F::Extension: Extendable<D>,
    {
        let mut acc = builder.zero_ext_ext();
        for &c in self.0.iter().rev() {
            let tmp = builder.mul_ext_ext(point, acc);
            acc = builder.add_ext_ext(tmp, c);
        }
        acc
    }
}
