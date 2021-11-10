use crate::curve::curve_types::{AffinePoint, Curve};
use crate::field::extension_field::Extendable;
use crate::field::field_types::RichField;
use crate::gadgets::nonnative::ForeignFieldTarget;
use crate::plonk::circuit_builder::CircuitBuilder;

/// A Target representing an affine point on the curve `C`.
#[derive(Clone, Debug)]
pub struct AffinePointTarget<C: Curve> {
    pub x: ForeignFieldTarget<C::BaseField>,
    pub y: ForeignFieldTarget<C::BaseField>,
}

impl<C: Curve> AffinePointTarget<C> {
    pub fn to_vec(&self) -> Vec<ForeignFieldTarget<C::BaseField>> {
        vec![self.x.clone(), self.y.clone()]
    }
}

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    pub fn constant_affine_point<C: Curve>(
        &mut self,
        point: AffinePoint<C>,
    ) -> AffinePointTarget<C> {
        debug_assert!(!point.zero);
        AffinePointTarget {
            x: self.constant_nonnative(point.x),
            y: self.constant_nonnative(point.y),
        }
    }

    pub fn connect_affine_point<C: Curve>(
        &mut self,
        lhs: AffinePointTarget<C>,
        rhs: AffinePointTarget<C>,
    ) {
        self.connect_nonnative(&lhs.x, &rhs.x);
        self.connect_nonnative(&lhs.y, &rhs.y);
    }

    pub fn curve_assert_valid<C: Curve>(&mut self, p: AffinePointTarget<C>) {
        let a = self.constant_nonnative(C::A);
        let b = self.constant_nonnative(C::B);

        let y_squared = self.mul_nonnative(&p.y, &p.y);
        let x_squared = self.mul_nonnative(&p.x, &p.x);
        let x_cubed = self.mul_nonnative(&x_squared, &p.x);
        let a_x = self.mul_nonnative(&a, &p.x);
        let a_x_plus_b = self.add_nonnative(&a_x, &b);
        let rhs = self.add_nonnative(&x_cubed, &a_x_plus_b);

        self.connect_nonnative(&y_squared, &rhs);
    }
}

mod tests {}
