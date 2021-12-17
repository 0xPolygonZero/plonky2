use crate::field::extension_field::target::{ExtensionAlgebraTarget, ExtensionTarget};
use crate::field::extension_field::Extendable;
use crate::iop::target::Target;
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::util::reducing::ReducingFactorTarget;

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
        let point = builder.convert_to_ext(point);
        let mut point = ReducingFactorTarget::new(point);
        point.reduce(&self.0, builder)
    }

    pub fn eval<F: Extendable<D>>(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        point: ExtensionTarget<D>,
    ) -> ExtensionTarget<D> {
        let mut point = ReducingFactorTarget::new(point);
        point.reduce(&self.0, builder)
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
            acc = builder.scalar_mul_add_ext_algebra(point, acc, c);
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
            acc = builder.mul_add_ext_algebra(point, acc, c);
        }
        acc
    }

    /// Evaluate the polynomial at a point given its powers. The first power is the point itself, not 1.
    pub fn eval_with_powers<F>(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        powers: &[ExtensionAlgebraTarget<D>],
    ) -> ExtensionAlgebraTarget<D>
    where
        F: Extendable<D>,
    {
        debug_assert_eq!(self.0.len(), powers.len() + 1);
        let acc = self.0[0];
        self.0[1..]
            .iter()
            .zip(powers)
            .fold(acc, |acc, (&x, &c)| builder.mul_add_ext_algebra(c, x, acc))
    }
}
