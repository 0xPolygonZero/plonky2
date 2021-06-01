use crate::field::field::Field;
use crate::polynomial::polynomial::{PolynomialCoeffs, PolynomialValues};
use crate::util::{log2_ceil, log2_strict, reverse_index_bits, reverse_bits};


pub(crate) struct FftPrecomputation<F: Field> {
    /// Store powers of a primitive root of unity of F. Specifically,
    /// the ith element of subgroups_rev contains g**reverse_bits(i),
    /// where g is the root.
    subgroups_rev: Vec<F>,
}

impl<F: Field> FftPrecomputation<F> {
    pub fn size(&self) -> usize {
        self.subgroups_rev.len()
    }
}

pub(crate) fn fft_precompute<F: Field>(degree: usize) -> FftPrecomputation<F> {
    let degree_log = log2_ceil(degree);
    let g = F::primitive_root_of_unity(degree_log);
    let subgroup = F::cyclic_subgroup_known_order(g, 1 << degree_log);
    let subgroups_rev = reverse_index_bits(subgroup);

    FftPrecomputation { subgroups_rev }
}

pub fn fft<F: Field>(poly: PolynomialCoeffs<F>) -> PolynomialValues<F> {
    let precomputation = fft_precompute(poly.len());
    fft_with_precomputation_power_of_2(poly, &precomputation)
}

pub(crate) fn ifft_with_precomputation_power_of_2<F: Field>(
    poly: PolynomialValues<F>,
    precomputation: &FftPrecomputation<F>,
) -> PolynomialCoeffs<F> {
    let n = poly.len();
    let n_inv = F::from_canonical_usize(n).try_inverse().unwrap();

    let PolynomialValues { values } = poly;
    let PolynomialValues { values: mut result } =
        fft_with_precomputation_power_of_2(PolynomialCoeffs { coeffs: values }, precomputation);

    // We reverse all values except the first, and divide each by n.
    result[0] *= n_inv;
    result[n / 2] *= n_inv;
    for i in 1..(n / 2) {
        let j = n - i;
        let result_i = result[j] * n_inv;
        let result_j = result[i] * n_inv;
        result[i] = result_i;
        result[j] = result_j;
    }
    PolynomialCoeffs { coeffs: result }
}

pub(crate) fn fft_with_precomputation_power_of_2<F: Field>(
    poly: PolynomialCoeffs<F>,
    precomputation: &FftPrecomputation<F>,
) -> PolynomialValues<F> {
    debug_assert_eq!(
        poly.len(), precomputation.size(),
        "Number of coefficients does not match size of subgroup in precomputation"
    );

    let half_degree = poly.len() >> 1;
    let degree_log = poly.log_len();

    // In the base layer, we're just evaluating "degree 0 polynomials", i.e. the coefficients
    // themselves.
    let PolynomialCoeffs { coeffs } = poly;
    let mut evaluations = reverse_index_bits(coeffs);

    for i in 1..=degree_log {
        // In layer i, we're evaluating a series of polynomials, each at 2^i points. In practice
        // we evaluate a pair of points together, so we have 2^(i - 1) pairs.
        let points_per_poly = 1 << i;
        let pairs_per_poly = 1 << (i - 1);

        let mut new_evaluations = Vec::new();
        for pair_index in 0..half_degree {
            let poly_index = pair_index / pairs_per_poly;
            let pair_index_within_poly = pair_index % pairs_per_poly;

            let child_index_0 = poly_index * points_per_poly + pair_index_within_poly;
            let child_index_1 = child_index_0 + pairs_per_poly;

            let even = evaluations[child_index_0];
            let odd = evaluations[child_index_1];

            let point_0 = precomputation.subgroups_rev[pair_index_within_poly * 2];
            let product = point_0 * odd;
            new_evaluations.push(even + product);
            new_evaluations.push(even - product);
        }
        evaluations = new_evaluations;
    }

    // Reorder so that evaluations' indices correspond to (g_0, g_1, g_2, ...)
    let values = reverse_index_bits(evaluations);
    PolynomialValues { values }
}

pub(crate) fn coset_fft<F: Field>(poly: PolynomialCoeffs<F>, shift: F) -> PolynomialValues<F> {
    let mut points = fft(poly);
    let mut shift_exp_i = F::ONE;
    for p in points.values.iter_mut() {
        *p *= shift_exp_i;
        shift_exp_i *= shift;
    }
    points
}

pub(crate) fn ifft<F: Field>(poly: PolynomialValues<F>) -> PolynomialCoeffs<F> {
    let precomputation = fft_precompute(poly.len());
    ifft_with_precomputation_power_of_2(poly, &precomputation)
}

pub(crate) fn coset_ifft<F: Field>(poly: PolynomialValues<F>, shift: F) -> PolynomialCoeffs<F> {
    let shift_inv = shift.inverse();
    let mut shift_inv_exp_i = F::ONE;
    let mut coeffs = ifft(poly);
    for c in coeffs.coeffs.iter_mut() {
        *c *= shift_inv_exp_i;
        shift_inv_exp_i *= shift_inv;
    }
    coeffs
}

#[cfg(test)]
mod tests {
    use crate::field::crandall_field::CrandallField;
    use crate::field::fft::{fft, ifft};
    use crate::field::field::Field;
    use crate::polynomial::polynomial::{PolynomialCoeffs, PolynomialValues};
    use crate::util::{log2_ceil, log2_strict};

    #[test]
    fn fft_and_ifft() {
        type F = CrandallField;
        let degree = 200;
        let degree_padded = log2_ceil(degree);
        let mut coefficients = Vec::new();
        for i in 0..degree {
            coefficients.push(F::from_canonical_usize(i * 1337 % 100));
        }
        let coefficients = PolynomialCoeffs::new_padded(coefficients);

        let points = fft(coefficients.clone());
        assert_eq!(points, evaluate_naive(&coefficients));

        let interpolated_coefficients = ifft(points);
        for i in 0..degree {
            assert_eq!(interpolated_coefficients.coeffs[i], coefficients.coeffs[i]);
        }
        for i in degree..degree_padded {
            assert_eq!(interpolated_coefficients.coeffs[i], F::ZERO);
        }
    }

    fn evaluate_naive<F: Field>(coefficients: &PolynomialCoeffs<F>) -> PolynomialValues<F> {
        let degree = coefficients.len();
        let degree_padded = 1 << log2_ceil(degree);

        let mut coefficients_padded = coefficients.clone();
        for _i in degree..degree_padded {
            coefficients_padded.coeffs.push(F::ZERO);
        }
        evaluate_naive_power_of_2(&coefficients_padded)
    }

    fn evaluate_naive_power_of_2<F: Field>(
        coefficients: &PolynomialCoeffs<F>,
    ) -> PolynomialValues<F> {
        let degree = coefficients.len();
        let degree_log = log2_strict(degree);

        let g = F::primitive_root_of_unity(degree_log);
        let powers_of_g = F::cyclic_subgroup_known_order(g, degree);

        let values = powers_of_g
            .into_iter()
            .map(|x| evaluate_at_naive(&coefficients, x))
            .collect();
        PolynomialValues::new(values)
    }

    fn evaluate_at_naive<F: Field>(coefficients: &PolynomialCoeffs<F>, point: F) -> F {
        let mut sum = F::ZERO;
        let mut point_power = F::ONE;
        for &c in &coefficients.coeffs {
            sum = sum + c * point_power;
            point_power = point_power * point;
        }
        sum
    }
}
