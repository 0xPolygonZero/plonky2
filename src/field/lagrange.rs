use crate::field::fft::ifft;
use crate::field::field::Field;
use crate::polynomial::polynomial::{PolynomialCoeffs, PolynomialValues};
use crate::util::log2_ceil;

/// Computes the interpolant of an arbitrary list of (point, value) pairs.
///
/// Note that the implementation assumes that `F` is two-adic, in particular that
/// `2^{F::TWO_ADICITY} >= points.len()`. This leads to a simple FFT-based implementation.
pub(crate) fn interpolant<F: Field>(points: &[(F, F)]) -> PolynomialCoeffs<F> {
    let n = points.len();
    let n_log = log2_ceil(n);
    let n_padded = 1 << n_log;

    let g = F::primitive_root_of_unity(n_log);
    let subgroup = F::cyclic_subgroup_known_order(g, n_padded);
    let subgroup_evals = subgroup
        .into_iter()
        .map(|x| interpolate(points, x))
        .collect();

    let mut coeffs = ifft(PolynomialValues {
        values: subgroup_evals,
    });
    coeffs.trim();
    coeffs
}

/// Interpolate the polynomial defined by an arbitrary set of (point, value) pairs at the given
/// point `x`.
fn interpolate<F: Field>(points: &[(F, F)], x: F) -> F {
    (0..points.len())
        .map(|i| {
            let y_i = points[i].1;
            let l_i_x = eval_basis(points, i, x);
            y_i * l_i_x
        })
        .sum()
}

/// Evaluate the `i`th Lagrange basis, i.e. the one that vanishes except on the `i`th point.
fn eval_basis<F: Field>(points: &[(F, F)], i: usize, x: F) -> F {
    let n = points.len();
    let x_i = points[i].0;
    let mut numerator = F::ONE;
    let mut denominator_parts = Vec::with_capacity(n - 1);

    for j in 0..n {
        if i != j {
            let x_j = points[j].0;
            numerator *= x - x_j;
            denominator_parts.push(x_i - x_j);
        }
    }

    let denominator_inv = F::batch_multiplicative_inverse(&denominator_parts)
        .into_iter()
        .product();
    numerator * denominator_inv
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
        domain
            .iter()
            .map(|&x| {
                let eval = x
                    .powers()
                    .zip(&coeffs.coeffs)
                    .map(|(x_power, &coeff)| coeff * x_power)
                    .sum();
                (x, eval)
            })
            .collect()
    }
}
