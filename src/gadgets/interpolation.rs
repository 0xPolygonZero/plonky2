use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::gadgets::polynomial::PolynomialCoeffsExtTarget;

impl<F: Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    /// Interpolate two points. No need for an `InterpolationGate` since the coefficients
    /// of the linear interpolation polynomial can be easily computed with arithmetic operations.
    pub fn interpolate2(
        &mut self,
        points: [(ExtensionTarget<D>, ExtensionTarget<D>); 2],
    ) -> PolynomialCoeffsExtTarget<D> {
        todo!()
    }
}
