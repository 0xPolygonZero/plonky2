use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::field::field::Field;
use crate::target::Target;

pub struct PolynomialCoeffsTarget<const D: usize>(pub Vec<ExtensionTarget<D>>);

impl<const D: usize> PolynomialCoeffsTarget<D> {
    pub fn eval_scalar<F: Field + Extendable<D>>(
        &self,
        builder: &mut CircuitBuilder<F>,
        point: Target,
    ) -> ExtensionTarget<D> {
        let mut acc = builder.zero_ext();
        for &c in self.0.iter().rev() {
            let tmp = builder.scalar_mul(point, acc);
            acc = builder.add_extension(tmp, c);
        }
        acc
    }

    pub fn eval<F: Field + Extendable<D>>(
        &self,
        builder: &mut CircuitBuilder<F>,
        point: ExtensionTarget<D>,
    ) -> ExtensionTarget<D> {
        let mut acc = builder.zero_ext();
        for &c in self.0.iter().rev() {
            let tmp = builder.mul_extension(point, acc);
            acc = builder.add_extension(tmp, c);
        }
        acc
    }
}
