use crate::curve::curve_types::{AffinePoint, Curve};
use crate::field::extension_field::Extendable;
use crate::field::field_types::{Field, RichField};
use crate::gadgets::nonnative::NonNativeTarget;
use crate::plonk::circuit_builder::CircuitBuilder;

/// A Target representing an affine point on the curve `C`.
#[derive(Clone, Debug)]
pub struct AffinePointTarget<C: Curve> {
    pub x: NonNativeTarget<C::BaseField>,
    pub y: NonNativeTarget<C::BaseField>,
}

impl<C: Curve> AffinePointTarget<C> {
    pub fn to_vec(&self) -> Vec<NonNativeTarget<C::BaseField>> {
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

    pub fn curve_double<C: Curve>(
        &mut self,
        p: &AffinePointTarget<C>,
        p_orig: AffinePoint<C>,
    ) -> AffinePointTarget<C> {
        let AffinePointTarget { x, y } = p;
        let double_y = self.add_nonnative(y, y);
        let inv_double_y = self.inv_nonnative(&double_y);
        let x_squared = self.mul_nonnative(x, x);
        let double_x_squared = self.add_nonnative(&x_squared, &x_squared);
        let triple_x_squared = self.add_nonnative(&double_x_squared, &x_squared);

        let a = self.constant_nonnative(C::A);
        let triple_xx_a = self.add_nonnative(&triple_x_squared, &a);
        let lambda = self.mul_nonnative(&triple_xx_a, &inv_double_y);
        let lambda_squared = self.mul_nonnative(&lambda, &lambda);
        let x_double = self.add_nonnative(x, x);

        let x3 = self.sub_nonnative(&lambda_squared, &x_double);

        let x_diff = self.sub_nonnative(x, &x3);
        let lambda_x_diff = self.mul_nonnative(&lambda, &x_diff);

        let y3 = self.sub_nonnative(&lambda_x_diff, y);

        AffinePointTarget { x: x3, y: y3 }
    }

    pub fn curve_add<C: Curve>(
        &mut self,
        p1: &AffinePointTarget<C>,
        p2: &AffinePointTarget<C>,
    ) -> AffinePointTarget<C> {
        let AffinePointTarget { x: x1, y: y1 } = p1;
        let AffinePointTarget { x: x2, y: y2 } = p2;

        let u = self.sub_nonnative(y2, y1);
        let uu = self.mul_nonnative(&u, &u);
        let v = self.sub_nonnative(x2, x1);
        let vv = self.mul_nonnative(&v, &v);
        let vvv = self.mul_nonnative(&v, &vv);
        let r = self.mul_nonnative(&vv, x1);
        let diff = self.sub_nonnative(&uu, &vvv);
        let r2 = self.add_nonnative(&r, &r);
        let a = self.sub_nonnative(&diff, &r2);
        let x3 = self.mul_nonnative(&v, &a);

        let r_a = self.sub_nonnative(&r, &a);
        let y3_first = self.mul_nonnative(&u, &r_a);
        let y3_second = self.mul_nonnative(&vvv, y1);
        let y3 = self.sub_nonnative(&y3_first, &y3_second);

        let z3_inv = self.inv_nonnative(&vvv);
        let x3_norm = self.mul_nonnative(&x3, &z3_inv);
        let y3_norm = self.mul_nonnative(&y3, &z3_inv);

        AffinePointTarget {
            x: x3_norm,
            y: y3_norm,
        }
    }
}

mod tests {
    use std::ops::Neg;

    use anyhow::Result;

    use crate::curve::curve_types::{AffinePoint, Curve};
    use crate::curve::secp256k1::Secp256K1;
    use crate::field::field_types::Field;
    use crate::field::goldilocks_field::GoldilocksField;
    use crate::field::secp256k1_base::Secp256K1Base;
    use crate::iop::witness::PartialWitness;
    use crate::plonk::circuit_builder::CircuitBuilder;
    use crate::plonk::circuit_data::CircuitConfig;
    use crate::plonk::verifier::verify;

    #[test]
    fn test_curve_point_is_valid() -> Result<()> {
        type F = GoldilocksField;
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
        type F = GoldilocksField;
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

    #[test]
    fn test_curve_double() -> Result<()> {
        type F = GoldilocksField;
        const D: usize = 4;

        let config = CircuitConfig::large_config();

        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let g = Secp256K1::GENERATOR_AFFINE;
        let neg_g = g.neg();
        let g_target = builder.constant_affine_point(g);
        let neg_g_target = builder.curve_neg(&g_target);

        let double_g = g.double();
        let double_g_other_target = builder.constant_affine_point(double_g);
        builder.curve_assert_valid(&double_g_other_target);

        let double_g_target = builder.curve_double(&g_target, g);
        let double_neg_g_target = builder.curve_double(&neg_g_target, neg_g);

        builder.curve_assert_valid(&double_g_target);
        builder.curve_assert_valid(&double_neg_g_target);

        let data = builder.build();
        let proof = data.prove(pw).unwrap();

        verify(proof, &data.verifier_only, &data.common)
    }
}
