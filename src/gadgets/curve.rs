use crate::curve::curve_types::{AffinePoint, Curve, CurveScalar};
use crate::field::extension_field::Extendable;
use crate::field::field_types::{Field, RichField};
use crate::gadgets::nonnative::NonNativeTarget;
use crate::plonk::circuit_builder::CircuitBuilder;

/// A Target representing an affine point on the curve `C`. We use incomplete arithmetic for efficiency,
/// so we assume these points are not zero.
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

    pub fn add_virtual_affine_point_target<C: Curve>(&mut self) -> AffinePointTarget<C> {
        let x = self.add_virtual_nonnative_target();
        let y = self.add_virtual_nonnative_target();

        AffinePointTarget { x, y }
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

    pub fn curve_double<C: Curve>(&mut self, p: &AffinePointTarget<C>) -> AffinePointTarget<C> {
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

    // Add two points, which are assumed to be non-equal.
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

    pub fn curve_scalar_mul<C: Curve>(
        &mut self,
        p: &AffinePointTarget<C>,
        n: &NonNativeTarget<C::ScalarField>,
    ) -> AffinePointTarget<C> {
        let one = self.constant_nonnative(C::BaseField::ONE);

        let bits = self.split_nonnative_to_bits(&n);
        let bits_as_base: Vec<NonNativeTarget<C::BaseField>> =
            bits.iter().map(|b| self.bool_to_nonnative(b)).collect();

        let rando = (CurveScalar(C::ScalarField::rand()) * C::GENERATOR_PROJECTIVE).to_affine();
        let randot = self.constant_affine_point(rando);
        // Result starts at `rando`, which is later subtracted, because we don't support arithmetic with the zero point.
        let mut result = self.add_virtual_affine_point_target();
        self.connect_affine_point(&randot, &result);

        let mut two_i_times_p = self.add_virtual_affine_point_target();
        self.connect_affine_point(p, &two_i_times_p);

        for bit in bits_as_base.iter() {
            let not_bit = self.sub_nonnative(&one, bit);

            let result_plus_2_i_p = self.curve_add(&result, &two_i_times_p);

            let new_x_if_bit = self.mul_nonnative(bit, &result_plus_2_i_p.x);
            let new_x_if_not_bit = self.mul_nonnative(&not_bit, &result.x);
            let new_y_if_bit = self.mul_nonnative(bit, &result_plus_2_i_p.y);
            let new_y_if_not_bit = self.mul_nonnative(&not_bit, &result.y);

            let new_x = self.add_nonnative(&new_x_if_bit, &new_x_if_not_bit);
            let new_y = self.add_nonnative(&new_y_if_bit, &new_y_if_not_bit);

            result = AffinePointTarget { x: new_x, y: new_y };

            two_i_times_p = self.curve_double(&two_i_times_p);
        }

        // Subtract off result's intial value of `rando`.
        let neg_r = self.curve_neg(&randot);
        result = self.curve_add(&result, &neg_r);

        result
    }
}

mod tests {
    use std::ops::{Mul, Neg};

    use anyhow::Result;

    use crate::curve::curve_types::{AffinePoint, Curve, CurveScalar};
    use crate::curve::secp256k1::Secp256K1;
    use crate::field::field_types::Field;
    use crate::field::goldilocks_field::GoldilocksField;
    use crate::field::secp256k1_base::Secp256K1Base;
    use crate::field::secp256k1_scalar::Secp256K1Scalar;
    use crate::iop::witness::PartialWitness;
    use crate::plonk::circuit_builder::CircuitBuilder;
    use crate::plonk::circuit_data::CircuitConfig;
    use crate::plonk::verifier::verify;

    #[test]
    fn test_curve_point_is_valid() -> Result<()> {
        type F = GoldilocksField;
        const D: usize = 4;

        let config = CircuitConfig::standard_recursion_config();

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

        let config = CircuitConfig::standard_recursion_config();

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

        let config = CircuitConfig::standard_recursion_config();

        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let g = Secp256K1::GENERATOR_AFFINE;
        let g_target = builder.constant_affine_point(g);
        let neg_g_target = builder.curve_neg(&g_target);

        let double_g = g.double();
        let double_g_expected = builder.constant_affine_point(double_g);
        builder.curve_assert_valid(&double_g_expected);

        let double_neg_g = (-g).double();
        let double_neg_g_expected = builder.constant_affine_point(double_neg_g);
        builder.curve_assert_valid(&double_neg_g_expected);

        let double_g_actual = builder.curve_double(&g_target);
        let double_neg_g_actual = builder.curve_double(&neg_g_target);
        builder.curve_assert_valid(&double_g_actual);
        builder.curve_assert_valid(&double_neg_g_actual);

        builder.connect_affine_point(&double_g_expected, &double_g_actual);
        builder.connect_affine_point(&double_neg_g_expected, &double_neg_g_actual);

        let data = builder.build();
        let proof = data.prove(pw).unwrap();

        verify(proof, &data.verifier_only, &data.common)
    }

    #[test]
    fn test_curve_add() -> Result<()> {
        type F = GoldilocksField;
        const D: usize = 4;

        let config = CircuitConfig::standard_recursion_config();

        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let g = Secp256K1::GENERATOR_AFFINE;
        let double_g = g.double();
        let g_plus_2g = (g + double_g).to_affine();
        let g_plus_2g_expected = builder.constant_affine_point(g_plus_2g);
        builder.curve_assert_valid(&g_plus_2g_expected);

        let g_target = builder.constant_affine_point(g);
        let double_g_target = builder.curve_double(&g_target);
        let g_plus_2g_actual = builder.curve_add(&g_target, &double_g_target);
        builder.curve_assert_valid(&g_plus_2g_actual);

        builder.connect_affine_point(&g_plus_2g_expected, &g_plus_2g_actual);

        let data = builder.build();
        let proof = data.prove(pw).unwrap();

        verify(proof, &data.verifier_only, &data.common)
    }

    #[test]
    fn test_curve_mul() -> Result<()> {
        type F = GoldilocksField;
        const D: usize = 4;

        let config = CircuitConfig {
            num_routed_wires: 33,
            ..CircuitConfig::standard_recursion_config()
        };

        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let g = Secp256K1::GENERATOR_AFFINE;
        let five = Secp256K1Scalar::from_canonical_usize(5);
        let five_scalar = CurveScalar::<Secp256K1>(five);
        let five_g = (five_scalar * g.to_projective()).to_affine();
        let five_g_expected = builder.constant_affine_point(five_g);
        builder.curve_assert_valid(&five_g_expected);

        let g_target = builder.constant_affine_point(g);
        let five_target = builder.constant_nonnative(five);
        let five_g_actual = builder.curve_scalar_mul(&g_target, &five_target);
        builder.curve_assert_valid(&five_g_actual);

        builder.connect_affine_point(&five_g_expected, &five_g_actual);

        let data = builder.build();
        let proof = data.prove(pw).unwrap();

        verify(proof, &data.verifier_only, &data.common)
    }

    #[test]
    fn test_curve_random() -> Result<()> {
        type F = GoldilocksField;
        const D: usize = 4;

        let config = CircuitConfig {
            num_routed_wires: 33,
            ..CircuitConfig::standard_recursion_config()
        };

        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let rando =
            (CurveScalar(Secp256K1Scalar::rand()) * Secp256K1::GENERATOR_PROJECTIVE).to_affine();
        let randot = builder.constant_affine_point(rando);

        let two_target = builder.constant_nonnative(Secp256K1Scalar::TWO);
        let randot_doubled = builder.curve_double(&randot);
        let randot_times_two = builder.curve_scalar_mul(&randot, &two_target);
        builder.connect_affine_point(&randot_doubled, &randot_times_two);

        let data = builder.build();
        let proof = data.prove(pw).unwrap();

        verify(proof, &data.verifier_only, &data.common)
    }
}
