use std::marker::PhantomData;

use num::BigUint;
use plonky2_field::extension_field::Extendable;
use plonky2_field::field_types::Field;

use crate::curve::curve_types::{Curve, CurveScalar};
use crate::gadgets::arithmetic_u32::U32Target;
use crate::gadgets::biguint::BigUintTarget;
use crate::gadgets::curve::AffinePointTarget;
use crate::gadgets::nonnative::NonNativeTarget;
use crate::hash::hash_types::RichField;
use crate::hash::keccak::KeccakHash;
use crate::iop::target::{BoolTarget, Target};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::config::{GenericHashOut, Hasher};

const WINDOW_SIZE: usize = 4;

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    pub fn precompute_window<C: Curve>(
        &mut self,
        p: &AffinePointTarget<C>,
    ) -> Vec<AffinePointTarget<C>> {
        let g = (CurveScalar(C::ScalarField::rand()) * C::GENERATOR_PROJECTIVE).to_affine();
        let neg = {
            let mut neg = g;
            neg.y = -neg.y;
            self.constant_affine_point(neg)
        };

        let mut multiples = vec![self.constant_affine_point(g)];
        for i in 1..1 << WINDOW_SIZE {
            multiples.push(self.curve_add(p, &multiples[i - 1]));
        }
        for i in 1..1 << WINDOW_SIZE {
            multiples[i] = self.curve_add(&neg, &multiples[i]);
        }
        multiples
    }

    pub fn random_access_curve_points<C: Curve>(
        &mut self,
        access_index: Target,
        v: Vec<AffinePointTarget<C>>,
    ) -> AffinePointTarget<C> {
        let num_limbs = v[0].x.value.num_limbs();
        let x_limbs: Vec<Vec<_>> = (0..num_limbs)
            .map(|i| v.iter().map(|p| p.x.value.limbs[i].0).collect())
            .collect();
        let y_limbs: Vec<Vec<_>> = (0..num_limbs)
            .map(|i| v.iter().map(|p| p.y.value.limbs[i].0).collect())
            .collect();

        let selected_x_limbs: Vec<_> = x_limbs
            .iter()
            .map(|limbs| U32Target(self.random_access(access_index, limbs.clone())))
            .collect();
        let selected_y_limbs: Vec<_> = y_limbs
            .iter()
            .map(|limbs| U32Target(self.random_access(access_index, limbs.clone())))
            .collect();

        let x = NonNativeTarget {
            value: BigUintTarget {
                limbs: selected_x_limbs,
            },
            _phantom: PhantomData,
        };
        let y = NonNativeTarget {
            value: BigUintTarget {
                limbs: selected_y_limbs,
            },
            _phantom: PhantomData,
        };
        AffinePointTarget { x, y }
    }

    pub fn if_affine_point<C: Curve>(
        &mut self,
        b: BoolTarget,
        p1: &AffinePointTarget<C>,
        p2: &AffinePointTarget<C>,
    ) -> AffinePointTarget<C> {
        let new_x = self.if_nonnative(b, &p1.x, &p2.x);
        let new_y = self.if_nonnative(b, &p1.y, &p2.y);
        AffinePointTarget { x: new_x, y: new_y }
    }

    pub fn curve_scalar_mul_windowed<C: Curve>(
        &mut self,
        p: &AffinePointTarget<C>,
        n: &NonNativeTarget<C::ScalarField>,
    ) -> AffinePointTarget<C> {
        let hash_0 = KeccakHash::<25>::hash(&[F::ZERO], false);
        let hash_0_scalar = C::ScalarField::from_biguint(BigUint::from_bytes_le(
            &GenericHashOut::<F>::to_bytes(&hash_0),
        ));
        let starting_point = CurveScalar(hash_0_scalar) * C::GENERATOR_PROJECTIVE;
        let starting_point_multiplied = {
            let mut cur = starting_point;
            for _ in 0..C::ScalarField::BITS {
                cur = cur.double();
            }
            cur
        };

        let mut result = self.constant_affine_point(starting_point.to_affine());

        let precomputation = self.precompute_window(p);
        let zero = self.zero();

        let windows = self.split_nonnative_to_4_bit_limbs(n);
        for i in (0..windows.len()).rev() {
            result = self.curve_repeated_double(&result, WINDOW_SIZE);
            let window = windows[i];

            let to_add = self.random_access_curve_points(window, precomputation.clone());
            let is_zero = self.is_equal(window, zero);
            let should_add = self.not(is_zero);
            result = self.curve_conditional_add(&result, &to_add, should_add);
        }

        let to_subtract = self.constant_affine_point(starting_point_multiplied.to_affine());
        let to_add = self.curve_neg(&to_subtract);
        result = self.curve_add(&result, &to_add);

        result
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Neg;

    use anyhow::Result;
    use plonky2_field::field_types::Field;
    use plonky2_field::secp256k1_scalar::Secp256K1Scalar;
    use rand::Rng;

    use crate::curve::curve_types::{Curve, CurveScalar};
    use crate::curve::secp256k1::Secp256K1;
    use crate::iop::witness::PartialWitness;
    use crate::plonk::circuit_builder::CircuitBuilder;
    use crate::plonk::circuit_data::CircuitConfig;
    use crate::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use crate::plonk::verifier::verify;

    #[test]
    fn test_random_access_curve_points() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;

        let config = CircuitConfig::standard_ecc_config();

        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let num_points = 16;
        let points: Vec<_> = (0..num_points)
            .map(|_| {
                let g = (CurveScalar(Secp256K1Scalar::rand()) * Secp256K1::GENERATOR_PROJECTIVE)
                    .to_affine();
                builder.constant_affine_point(g)
            })
            .collect();

        let mut rng = rand::thread_rng();
        let access_index = rng.gen::<usize>() % num_points;

        let access_index_target = builder.constant(F::from_canonical_usize(access_index));
        let selected = builder.random_access_curve_points(access_index_target, points.clone());
        let expected = points[access_index].clone();
        builder.connect_affine_point(&selected, &expected);

        let data = builder.build::<C>();
        let proof = data.prove(pw).unwrap();

        verify(proof, &data.verifier_only, &data.common)
    }

    #[test]
    fn test_curve_windowed_mul() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;

        let config = CircuitConfig::standard_ecc_config();

        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let g =
            (CurveScalar(Secp256K1Scalar::rand()) * Secp256K1::GENERATOR_PROJECTIVE).to_affine();
        let five = Secp256K1Scalar::from_canonical_usize(5);
        let neg_five = five.neg();
        let neg_five_scalar = CurveScalar::<Secp256K1>(neg_five);
        let neg_five_g = (neg_five_scalar * g.to_projective()).to_affine();
        let neg_five_g_expected = builder.constant_affine_point(neg_five_g);
        builder.curve_assert_valid(&neg_five_g_expected);

        let g_target = builder.constant_affine_point(g);
        let neg_five_target = builder.constant_nonnative(neg_five);
        let neg_five_g_actual = builder.curve_scalar_mul_windowed(&g_target, &neg_five_target);
        builder.curve_assert_valid(&neg_five_g_actual);

        builder.connect_affine_point(&neg_five_g_expected, &neg_five_g_actual);

        let data = builder.build::<C>();
        let proof = data.prove(pw).unwrap();

        verify(proof, &data.verifier_only, &data.common)
    }
}
