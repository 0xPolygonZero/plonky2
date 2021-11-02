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
        lhs: &AffinePointTarget<C>,
        rhs: &AffinePointTarget<C>,
    ) {
        self.connect_nonnative(&lhs.x, &rhs.x);
        self.connect_nonnative(&lhs.y, &rhs.y);
    }

    pub fn curve_assert_valid<C: Curve>(&mut self, p: &AffinePointTarget<C>) {
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

    pub fn curve_neg<C: Curve>(&mut self, p: &AffinePointTarget<C>) -> AffinePointTarget<C> {
        let neg_y = self.neg_nonnative(&p.y);
        AffinePointTarget {
            x: p.x.clone(),
            y: neg_y,
        }
    }
}

mod tests {
    use anyhow::Result;

    use crate::curve::curve_types::{AffinePoint, Curve};
    use crate::curve::secp256k1::Secp256K1;
    use crate::field::crandall_field::CrandallField;
    use crate::field::field_types::Field;
    use crate::field::secp256k1_base::Secp256K1Base;
    use crate::iop::witness::PartialWitness;
    use crate::plonk::circuit_builder::CircuitBuilder;
    use crate::plonk::circuit_data::CircuitConfig;
    use crate::plonk::verifier::verify;

    #[test]
    fn test_curve_point_is_valid() -> Result<()> {
        type F = CrandallField;
        const D: usize = 4;

        let config = CircuitConfig::large_config();

        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let g = Secp256K1::GENERATOR_AFFINE;
        let g_target = builder.constant_affine_point(g);
        let neg_g_target = builder.curve_neg(&g_target);

        builder.curve_assert_valid(&g_target);
        builder.curve_assert_valid(&neg_g_target);

        let data = builder.build();
        let proof = data.prove(pw).unwrap();

        verify(proof, &data.verifier_only, &data.common)
    }

    #[test]
    #[should_panic]
    fn test_curve_point_is_not_valid() {
        type F = CrandallField;
        const D: usize = 4;

        let config = CircuitConfig::large_config();

        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let g = Secp256K1::GENERATOR_AFFINE;
        let not_g = AffinePoint::<Secp256K1> {
            x: g.x,
            y: g.y + Secp256K1Base::ONE,
            zero: g.zero,
        };
        let not_g_target = builder.constant_affine_point(not_g);

        builder.curve_assert_valid(&not_g_target);

        let data = builder.build();
        let proof = data.prove(pw).unwrap();

        verify(proof, &data.verifier_only, &data.common).unwrap();
    }
}
