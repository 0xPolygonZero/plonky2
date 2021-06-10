use crate::field::field::Field;
use crate::polynomial::polynomial::{PolynomialCoeffs, PolynomialValues};
use crate::util::{log2_ceil, log2_strict, reverse_index_bits, reverse_bits};

pub fn fft<F: Field>(poly: PolynomialCoeffs<F>) -> PolynomialValues<F> {
    fft_barretenberg(poly)
    //fft_classic(poly)
}

pub fn ifft<F: Field>(
    poly: PolynomialValues<F>
) -> PolynomialCoeffs<F> {
    let n = poly.len();
    let n_inv = F::from_canonical_usize(n).try_inverse().unwrap();

    let PolynomialValues { values } = poly;
    let PolynomialValues { values: mut result } =
        fft_barretenberg(PolynomialCoeffs { coeffs: values });
        //fft_classic(PolynomialCoeffs { coeffs: values });

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

/// FFT implementation based on Section 32.3 of "Introduction to
/// Algorithms" by Cormen et al.
pub(crate) fn fft_classic<F: Field>(
    poly: PolynomialCoeffs<F>
) -> PolynomialValues<F> {
    let PolynomialCoeffs { coeffs } = poly;
    let mut values = reverse_index_bits(coeffs);

    // TODO: First round is mult by 1, so should be done separately
    // TODO: Unroll later rounds.

    let n = values.len();
    let mut m = 2;
    let mut lg_m = 1;
    loop {
        if m > n {
            break;
        }

        // TODO: calculate incrementally
        let omega_m = F::primitive_root_of_unity(lg_m);
        for k in (0..n).step_by(m) {
            let mut omega = F::ONE;
            let half_m = m/2;
            for j in 0..half_m {
                let t = omega * values[k + half_m + j];
                let u = values[k + j];
                values[k + j] = u + t;
                values[k + half_m + j] = u - t;
                omega *= omega_m;
            }
        }
        m *= 2;
        lg_m += 1;
    }
    PolynomialValues { values }
}

/// FFT implementation inspired by Barretenberg's:
/// https://github.com/AztecProtocol/barretenberg/blob/master/barretenberg/src/aztec/polynomials/polynomial_arithmetic.cpp#L58
/// https://github.com/AztecProtocol/barretenberg/blob/master/barretenberg/src/aztec/polynomials/evaluation_domain.cpp#L30
pub(crate) fn fft_barretenberg<F: Field>(
    poly: PolynomialCoeffs<F>
) -> PolynomialValues<F> {
    let n = poly.len();
    let lg_n = poly.log_len();

    // Precompute a table of the roots of unity used in the main
    // loops.
    let rt = F::primitive_root_of_unity(lg_n);
    let mut root_table = Vec::with_capacity(lg_n);
    let mut m = 2;
    loop {
        if m >= n {
            break;
        }
        // TODO: calculate incrementally
        let round_root = rt.exp((n / (2 * m)) as u64);
        let mut round_roots = Vec::with_capacity(m);
        round_roots.push(F::ONE);
        for j in 1..m {
            round_roots.push(round_roots[j - 1] * round_root);
        }
        root_table.push(round_roots);
        m *= 2;
    }

    let PolynomialCoeffs { coeffs } = poly;
    let mut values = reverse_index_bits(coeffs);

    // m = 1
    for k in (0..n).step_by(2) {
        let t = values[k + 1];
        values[k + 1] = values[k] - t;
        values[k] += t;
    }

    // m = 2
    if n <= 2 {
        return PolynomialValues { values }
    }

    for k in (0..n).step_by(4) {
        // NB: Grouping statements as is done in the main loop below
        // does not seem to help here (worse by a few millis).
        let omega_0 = root_table[0][0];
        let tmp_0 = omega_0 * values[k + 2 + 0];
        values[k + 2 + 0] = values[k + 0] - tmp_0;
        values[k + 0] += tmp_0;

        let omega_1 = root_table[0][1];
        let tmp_1 = omega_1 * values[k + 2 + 1];
        values[k + 2 + 1] = values[k + 1] - tmp_1;
        values[k + 1] += tmp_1;
    }

    // m >= 4
    let mut m = 4;
    let mut lg_m = 2;
    loop {
        if m >= n {
            break;
        }
        for k in (0..n).step_by(2*m) {
            // Unrolled the commented loop by groups of 4 and
            // rearranged the lines. Improves runtime by about
            // 10%.
            /*
            for j in (0..m) {
                let omega = root_table[lg_m - 1][j];
                let tmp = omega * values[k + m + j];
                values[k + m + j] = values[k + j] - tmp;
                values[k + j] += tmp;
            }
            */
            for j in (0..m).step_by(4) {
                let off1 = k + j;
                let off2 = k + m + j;

                let omega_0 = root_table[lg_m - 1][j + 0];
                let omega_1 = root_table[lg_m - 1][j + 1];
                let omega_2 = root_table[lg_m - 1][j + 2];
                let omega_3 = root_table[lg_m - 1][j + 3];

                let tmp_0 = omega_0 * values[off2 + 0];
                let tmp_1 = omega_1 * values[off2 + 1];
                let tmp_2 = omega_2 * values[off2 + 2];
                let tmp_3 = omega_3 * values[off2 + 3];

                values[off2 + 0] = values[off1 + 0] - tmp_0;
                values[off2 + 1] = values[off1 + 1] - tmp_1;
                values[off2 + 2] = values[off1 + 2] - tmp_2;
                values[off2 + 3] = values[off1 + 3] - tmp_3;
                values[off1 + 0] += tmp_0;
                values[off1 + 1] += tmp_1;
                values[off1 + 2] += tmp_2;
                values[off1 + 3] += tmp_3;
            }
        }
        m *= 2;
        lg_m += 1;
    }
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
