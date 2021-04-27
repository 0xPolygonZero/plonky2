use crate::field::fft::ifft;
use crate::field::field::Field;
use crate::polynomial::polynomial::{PolynomialCoeffs, PolynomialValues};
use crate::util::log2_ceil;

/// Computes the unique degree < n interpolant of an arbitrary list of n (point, value) pairs.
///
/// Note that the implementation assumes that `F` is two-adic, in particular that
/// `2^{F::TWO_ADICITY} >= points.len()`. This leads to a simple FFT-based implementation.
pub(crate) fn interpolant<F: Field>(points: &[(F, F)]) -> PolynomialCoeffs<F> {
    let n = points.len();
    let n_log = log2_ceil(n);
    let n_padded = 1 << n_log;

    let g = F::primitive_root_of_unity(n_log);
    let subgroup = F::cyclic_subgroup_known_order(g, n_padded);
    let barycentric_weights = barycentric_weights(points);
    let subgroup_evals = subgroup
        .into_iter()
        .map(|x| interpolate(points, x, &barycentric_weights))
        .collect();

    let mut coeffs = ifft(PolynomialValues {
        values: subgroup_evals,
    });
    coeffs.trim();
    coeffs
}

/// Interpolate the polynomial defined by an arbitrary set of (point, value) pairs at the given
/// point `x`.
pub fn interpolate<F: Field>(points: &[(F, F)], x: F, barycentric_weights: &[F]) -> F {
    // If x is in the list of points, the Lagrange formula would divide by zero.
    for &(x_i, y_i) in points {
        if x_i == x {
            return y_i;
        }
    }

    let l_x: F = points.iter().map(|&(x_i, _y_i)| x - x_i).product();

    let sum = (0..points.len())
        .map(|i| {
            let x_i = points[i].0;
            let y_i = points[i].1;
            let w_i = barycentric_weights[i];
            w_i / (x - x_i) * y_i
        })
        .sum();

    l_x * sum
}

pub fn barycentric_weights<F: Field>(points: &[(F, F)]) -> Vec<F> {
    let n = points.len();
    F::batch_multiplicative_inverse(
        &(0..n)
            .map(|i| {
                (0..n)
                    .filter(|&j| j != i)
                    .map(|j| points[i].0 - points[j].0)
                    .product::<F>()
            })
            .collect::<Vec<_>>(),
    )
}

#[cfg(test)]
mod tests {
    use crate::field::crandall_field::CrandallField;
    use crate::field::field::Field;
    use crate::field::lagrange::interpolant;
    use crate::polynomial::polynomial::PolynomialCoeffs;

    #[test]
    fn interpolant_random() {
        type F = CrandallField;

        for deg in 0..10 {
            let domain = (0..deg).map(|_| F::rand()).collect::<Vec<_>>();
            let coeffs = (0..deg).map(|_| F::rand()).collect();
            let coeffs = PolynomialCoeffs { coeffs };

            let points = eval_naive(&coeffs, &domain);
            assert_eq!(interpolant(&points), coeffs);
        }
    }

    #[test]
    fn interpolant_random_roots_of_unity() {
        type F = CrandallField;

        for deg_log in 0..4 {
            let deg = 1 << deg_log;
            let g = F::primitive_root_of_unity(deg_log);
            let domain = F::cyclic_subgroup_known_order(g, deg);
            let coeffs = (0..deg).map(|_| F::rand()).collect();
            let coeffs = PolynomialCoeffs { coeffs };

            let points = eval_naive(&coeffs, &domain);
            assert_eq!(interpolant(&points), coeffs);
        }
    }

    #[test]
    fn interpolant_random_overspecified() {
        type F = CrandallField;

        for deg in 0..10 {
            let points = deg + 5;
            let domain = (0..points).map(|_| F::rand()).collect::<Vec<_>>();
            let coeffs = (0..deg).map(|_| F::rand()).collect();
            let coeffs = PolynomialCoeffs { coeffs };

            let points = eval_naive(&coeffs, &domain);
            assert_eq!(interpolant(&points), coeffs);
        }
    }

    fn eval_naive<F: Field>(coeffs: &PolynomialCoeffs<F>, domain: &[F]) -> Vec<(F, F)> {
        domain.iter().map(|&x| (x, coeffs.eval(x))).collect()
    }
}
