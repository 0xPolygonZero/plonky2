use std::iter::Sum;

use plonky2_field::ops::Square;
use plonky2_field::types::Field;

use crate::curve::curve_types::{AffinePoint, Curve, ProjectivePoint};

impl<C: Curve> Sum<AffinePoint<C>> for ProjectivePoint<C> {
    fn sum<I: Iterator<Item = AffinePoint<C>>>(iter: I) -> ProjectivePoint<C> {
        let points: Vec<_> = iter.collect();
        affine_summation_best(points)
    }
}

impl<C: Curve> Sum for ProjectivePoint<C> {
    fn sum<I: Iterator<Item = ProjectivePoint<C>>>(iter: I) -> ProjectivePoint<C> {
        iter.fold(ProjectivePoint::ZERO, |acc, x| acc + x)
    }
}

pub fn affine_summation_best<C: Curve>(summation: Vec<AffinePoint<C>>) -> ProjectivePoint<C> {
    let result = affine_multisummation_best(vec![summation]);
    debug_assert_eq!(result.len(), 1);
    result[0]
}

pub fn affine_multisummation_best<C: Curve>(
    summations: Vec<Vec<AffinePoint<C>>>,
) -> Vec<ProjectivePoint<C>> {
    let pairwise_sums: usize = summations.iter().map(|summation| summation.len() / 2).sum();

    // This threshold is chosen based on data from the summation benchmarks.
    if pairwise_sums < 70 {
        affine_multisummation_pairwise(summations)
    } else {
        affine_multisummation_batch_inversion(summations)
    }
}

/// Adds each pair of points using an affine + affine = projective formula, then adds up the
/// intermediate sums using a projective formula.
pub fn affine_multisummation_pairwise<C: Curve>(
    summations: Vec<Vec<AffinePoint<C>>>,
) -> Vec<ProjectivePoint<C>> {
    summations
        .into_iter()
        .map(affine_summation_pairwise)
        .collect()
}

/// Adds each pair of points using an affine + affine = projective formula, then adds up the
/// intermediate sums using a projective formula.
pub fn affine_summation_pairwise<C: Curve>(points: Vec<AffinePoint<C>>) -> ProjectivePoint<C> {
    let mut reduced_points: Vec<ProjectivePoint<C>> = Vec::new();
    for chunk in points.chunks(2) {
        match chunk.len() {
            1 => reduced_points.push(chunk[0].to_projective()),
            2 => reduced_points.push(chunk[0] + chunk[1]),
            _ => panic!(),
        }
    }
    // TODO: Avoid copying (deref)
    reduced_points
        .iter()
        .fold(ProjectivePoint::ZERO, |sum, x| sum + *x)
}

/// Computes several summations of affine points by applying an affine group law, except that the
/// divisions are batched via Montgomery's trick.
pub fn affine_summation_batch_inversion<C: Curve>(
    summation: Vec<AffinePoint<C>>,
) -> ProjectivePoint<C> {
    let result = affine_multisummation_batch_inversion(vec![summation]);
    debug_assert_eq!(result.len(), 1);
    result[0]
}

/// Computes several summations of affine points by applying an affine group law, except that the
/// divisions are batched via Montgomery's trick.
pub fn affine_multisummation_batch_inversion<C: Curve>(
    summations: Vec<Vec<AffinePoint<C>>>,
) -> Vec<ProjectivePoint<C>> {
    let mut elements_to_invert = Vec::new();

    // For each pair of points, (x1, y1) and (x2, y2), that we're going to add later, we want to
    // invert either y (if the points are equal) or x1 - x2 (otherwise). We will use these later.
    for summation in &summations {
        let n = summation.len();
        // The special case for n=0 is to avoid underflow.
        let range_end = if n == 0 { 0 } else { n - 1 };

        for i in (0..range_end).step_by(2) {
            let p1 = summation[i];
            let p2 = summation[i + 1];
            let AffinePoint {
                x: x1,
                y: y1,
                zero: zero1,
            } = p1;
            let AffinePoint {
                x: x2,
                y: _y2,
                zero: zero2,
            } = p2;

            if zero1 || zero2 || p1 == -p2 {
                // These are trivial cases where we won't need any inverse.
            } else if p1 == p2 {
                elements_to_invert.push(y1.double());
            } else {
                elements_to_invert.push(x1 - x2);
            }
        }
    }

    let inverses: Vec<C::BaseField> =
        C::BaseField::batch_multiplicative_inverse(&elements_to_invert);

    let mut all_reduced_points = Vec::with_capacity(summations.len());
    let mut inverse_index = 0;
    for summation in summations {
        let n = summation.len();
        let mut reduced_points = Vec::with_capacity((n + 1) / 2);

        // The special case for n=0 is to avoid underflow.
        let range_end = if n == 0 { 0 } else { n - 1 };

        for i in (0..range_end).step_by(2) {
            let p1 = summation[i];
            let p2 = summation[i + 1];
            let AffinePoint {
                x: x1,
                y: y1,
                zero: zero1,
            } = p1;
            let AffinePoint {
                x: x2,
                y: y2,
                zero: zero2,
            } = p2;

            let sum = if zero1 {
                p2
            } else if zero2 {
                p1
            } else if p1 == -p2 {
                AffinePoint::ZERO
            } else {
                // It's a non-trivial case where we need one of the inverses we computed earlier.
                let inverse = inverses[inverse_index];
                inverse_index += 1;

                if p1 == p2 {
                    // This is the doubling case.
                    let mut numerator = x1.square().triple();
                    if C::A.is_nonzero() {
                        numerator += C::A;
                    }
                    let quotient = numerator * inverse;
                    let x3 = quotient.square() - x1.double();
                    let y3 = quotient * (x1 - x3) - y1;
                    AffinePoint::nonzero(x3, y3)
                } else {
                    // This is the general case. We use the incomplete addition formulas 4.3 and 4.4.
                    let quotient = (y1 - y2) * inverse;
                    let x3 = quotient.square() - x1 - x2;
                    let y3 = quotient * (x1 - x3) - y1;
                    AffinePoint::nonzero(x3, y3)
                }
            };
            reduced_points.push(sum);
        }

        // If n is odd, the last point was not part of a pair.
        if n % 2 == 1 {
            reduced_points.push(summation[n - 1]);
        }

        all_reduced_points.push(reduced_points);
    }

    // We should have consumed all of the inverses from the batch computation.
    debug_assert_eq!(inverse_index, inverses.len());

    // Recurse with our smaller set of points.
    affine_multisummation_best(all_reduced_points)
}

#[cfg(test)]
mod tests {
    use crate::curve::curve_summation::{
        affine_summation_batch_inversion, affine_summation_pairwise,
    };
    use crate::curve::curve_types::{Curve, ProjectivePoint};
    use crate::curve::secp256k1::Secp256K1;

    #[test]
    fn test_pairwise_affine_summation() {
        let g_affine = Secp256K1::GENERATOR_AFFINE;
        let g2_affine = (g_affine + g_affine).to_affine();
        let g3_affine = (g_affine + g_affine + g_affine).to_affine();
        let g2_proj = g2_affine.to_projective();
        let g3_proj = g3_affine.to_projective();
        assert_eq!(
            affine_summation_pairwise::<Secp256K1>(vec![g_affine, g_affine]),
            g2_proj
        );
        assert_eq!(
            affine_summation_pairwise::<Secp256K1>(vec![g_affine, g2_affine]),
            g3_proj
        );
        assert_eq!(
            affine_summation_pairwise::<Secp256K1>(vec![g_affine, g_affine, g_affine]),
            g3_proj
        );
        assert_eq!(
            affine_summation_pairwise::<Secp256K1>(vec![]),
            ProjectivePoint::ZERO
        );
    }

    #[test]
    fn test_pairwise_affine_summation_batch_inversion() {
        let g = Secp256K1::GENERATOR_AFFINE;
        let g_proj = g.to_projective();
        assert_eq!(
            affine_summation_batch_inversion::<Secp256K1>(vec![g, g]),
            g_proj + g_proj
        );
        assert_eq!(
            affine_summation_batch_inversion::<Secp256K1>(vec![g, g, g]),
            g_proj + g_proj + g_proj
        );
        assert_eq!(
            affine_summation_batch_inversion::<Secp256K1>(vec![]),
            ProjectivePoint::ZERO
        );
    }
}
