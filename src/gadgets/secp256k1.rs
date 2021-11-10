use crate::curve::curve_types::{AffinePoint, Curve};
use crate::field::extension_field::Extendable;
use crate::field::field_types::RichField;
use crate::gadgets::nonnative::ForeignFieldTarget;
use crate::plonk::circuit_builder::CircuitBuilder;

#[derive(Clone, Debug)]
pub struct AffinePointTarget<C: Curve> {
    pub x: ForeignFieldTarget<C::ScalarField>,
    pub y: ForeignFieldTarget<C::ScalarField>,
}

impl<C: Curve> AffinePointTarget<C> {
    pub fn to_vec(&self) -> Vec<ForeignFieldTarget<C::ScalarField>> {
        vec![self.x.clone(), self.y.clone()]
    }
}

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    pub fn constant_affine_point<C: Curve, InnerC: Curve<BaseField = C::ScalarField>>(
        &mut self,
        point: AffinePoint<InnerC>,
    ) -> AffinePointTarget<C> {
        debug_assert!(!point.zero);
        AffinePointTarget {
            x: self.constant_ff(point.x),
            y: self.constant_ff(point.y),
        }
    }
}

mod tests {}
