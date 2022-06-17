use plonky2_field::extension_field::Extendable;
use plonky2_field::field_types::Field;

use crate::curve::curve_types::{AffinePoint, Curve, CurveScalar};
use crate::gadgets::curve::AffinePointTarget;
use crate::gadgets::nonnative::NonNativeTarget;
use crate::hash::hash_types::RichField;
use crate::plonk::circuit_builder::CircuitBuilder;

const DIGITS_PER_CHUNK: usize = 80;

const WINDOW_SIZE: usize = 4;

pub struct MsmPrecomputationTarget<C: Curve> {
    /// For each generator (in the order they were passed to `msm_precompute`), contains a vector
    /// of powers, i.e. [(2^w)^i] for i < DIGITS.
    powers_per_generator: Vec<Vec<AffinePointTarget<C>>>,
}

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    pub fn precompute_single_generator<C: Curve>(
        &mut self,
        g: AffinePointTarget<C>,
    ) -> Vec<AffinePointTarget<C>> {
        let digits = (C::ScalarField::BITS + WINDOW_SIZE - 1) / WINDOW_SIZE;
        let mut powers: Vec<AffinePointTarget<C>> = Vec::with_capacity(digits);
        powers.push(g);
        for i in 1..digits {
            let mut power_i_proj = powers[i - 1].clone();
            for _j in 0..WINDOW_SIZE {
                power_i_proj = self.curve_double(&power_i_proj);
            }
            powers.push(power_i_proj);
        }
        powers
    }

    pub fn msm_precompute<C: Curve>(
        &mut self,
        generators: &[AffinePointTarget<C>],
    ) -> MsmPrecomputationTarget<C> {
        MsmPrecomputationTarget {
            powers_per_generator: generators
                .into_iter()
                .map(|g| self.precompute_single_generator(g.clone()))
                .collect(),
            w,
        }
    }

    pub fn msm_execute<C: Curve>(
        &mut self,
        precomputation: &MsmPrecomputationTarget<C>,
        scalars: &[NonNativeTarget<C::ScalarField>],
    ) -> AffinePointTarget<C> {
        debug_assert_eq!(precomputation.powers_per_generator.len(), scalars.len());

        let digits = (C::ScalarField::BITS + WINDOW_SIZE - 1) / WINDOW_SIZE;
        let base = 1 << WINDOW_SIZE;

        for (i, scalar) in scalars.iter().enumerate() {
            let digits = self.split_nonnative_to_4_bit_limbs(scalar);
        }

        let digits: Vec<_> = (0..base).map(|i| self.constant(F::from_canonical_usize(i))).collect();
        let mut digit_acc: Vec<ProjectivePoint<C>> = Vec::new();
        for i in 0..base {
            
        }
    }
}
