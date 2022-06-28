use std::ops::Mul;

use plonky2_field::types::Field;
use plonky2_field::types::PrimeField;

use crate::curve::curve_types::{Curve, CurveScalar, ProjectivePoint};

const WINDOW_BITS: usize = 4;
const BASE: usize = 1 << WINDOW_BITS;

fn digits_per_scalar<C: Curve>() -> usize {
    (C::ScalarField::BITS + WINDOW_BITS - 1) / WINDOW_BITS
}

/// Precomputed state used for scalar x ProjectivePoint multiplications,
/// specific to a particular generator.
#[derive(Clone)]
pub struct MultiplicationPrecomputation<C: Curve> {
    /// [(2^w)^i] g for each i < digits_per_scalar.
    powers: Vec<ProjectivePoint<C>>,
}

impl<C: Curve> ProjectivePoint<C> {
    pub fn mul_precompute(&self) -> MultiplicationPrecomputation<C> {
        let num_digits = digits_per_scalar::<C>();
        let mut powers = Vec::with_capacity(num_digits);
        powers.push(*self);
        for i in 1..num_digits {
            let mut power_i = powers[i - 1];
            for _j in 0..WINDOW_BITS {
                power_i = power_i.double();
            }
            powers.push(power_i);
        }

        MultiplicationPrecomputation { powers }
    }

    #[must_use]
    pub fn mul_with_precomputation(
        &self,
        scalar: C::ScalarField,
        precomputation: MultiplicationPrecomputation<C>,
    ) -> Self {
        // Yao's method; see https://koclab.cs.ucsb.edu/teaching/ecc/eccPapers/Doche-ch09.pdf
        let precomputed_powers = precomputation.powers;

        let digits = to_digits::<C>(&scalar);

        let mut y = ProjectivePoint::ZERO;
        let mut u = ProjectivePoint::ZERO;
        let mut all_summands = Vec::new();
        for j in (1..BASE).rev() {
            let mut u_summands = Vec::new();
            for (i, &digit) in digits.iter().enumerate() {
                if digit == j as u64 {
                    u_summands.push(precomputed_powers[i]);
                }
            }
            all_summands.push(u_summands);
        }

        let all_sums: Vec<ProjectivePoint<C>> = all_summands
            .iter()
            .cloned()
            .map(|vec| vec.iter().fold(ProjectivePoint::ZERO, |a, &b| a + b))
            .collect();
        for i in 0..all_sums.len() {
            u = u + all_sums[i];
            y = y + u;
        }
        y
    }
}

impl<C: Curve> Mul<ProjectivePoint<C>> for CurveScalar<C> {
    type Output = ProjectivePoint<C>;

    fn mul(self, rhs: ProjectivePoint<C>) -> Self::Output {
        let precomputation = rhs.mul_precompute();
        rhs.mul_with_precomputation(self.0, precomputation)
    }
}

#[allow(clippy::assertions_on_constants)]
fn to_digits<C: Curve>(x: &C::ScalarField) -> Vec<u64> {
    debug_assert!(
        64 % WINDOW_BITS == 0,
        "For simplicity, only power-of-two window sizes are handled for now"
    );
    let digits_per_u64 = 64 / WINDOW_BITS;
    let mut digits = Vec::with_capacity(digits_per_scalar::<C>());
    for limb in x.to_canonical_biguint().to_u64_digits() {
        for j in 0..digits_per_u64 {
            digits.push((limb >> (j * WINDOW_BITS) as u64) % BASE as u64);
        }
    }

    digits
}
