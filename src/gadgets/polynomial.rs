use crate::field::extension_field::target::{ExtensionAlgebraTarget, ExtensionTarget};
use crate::field::extension_field::Extendable;
use crate::iop::target::Target;
use crate::plonk::circuit_builder::CircuitBuilder;

pub struct PolynomialCoeffsExtTarget<const D: usize>(pub Vec<ExtensionTarget<D>>);

impl<const D: usize> PolynomialCoeffsExtTarget<D> {
    pub fn len(&self) -> usize {
        self.0.len()
    }

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

pub struct PolynomialCoeffsExtAlgebraTarget<const D: usize>(pub Vec<ExtensionAlgebraTarget<D>>);

impl<const D: usize> PolynomialCoeffsExtAlgebraTarget<D> {
    pub fn eval_scalar<F>(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        point: ExtensionTarget<D>,
    ) -> ExtensionAlgebraTarget<D>
    where
        F: Extendable<D>,
    {
        let mut acc = builder.zero_ext_algebra();
        for &c in self.0.iter().rev() {
            let tmp = builder.scalar_mul_ext_algebra(point, acc);
            acc = builder.add_ext_algebra(tmp, c);
        }
        acc
    }

    pub fn eval<F>(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        point: ExtensionAlgebraTarget<D>,
    ) -> ExtensionAlgebraTarget<D>
    where
        F: Extendable<D>,
    {
        let mut acc = builder.zero_ext_algebra();
        for &c in self.0.iter().rev() {
            let tmp = builder.mul_ext_algebra(point, acc);
            acc = builder.add_ext_algebra(tmp, c);
        }
        acc
    }
}
