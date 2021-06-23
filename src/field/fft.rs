use crate::field::field::Field;
use crate::polynomial::polynomial::{PolynomialCoeffs, PolynomialValues};
use crate::util::{log2_strict, reverse_index_bits};

/*
trait FftStrategy<F: Field> {
    fn fft_impl(input: Vec<F>) -> Vec<F>;
}
*/

enum FftStrategy { Classic, Barretenberg }

const FFT_STRATEGY: FftStrategy = FftStrategy::Classic;

type FftRootTable<F: Field> = Vec<Vec<F>>;

fn update_fft_root_table<F: Field>(root_table: &mut FftRootTable<F>, lg_n: usize) {
    // Precompute a table of the roots of unity used in the main
    // loops.

    // TODO: inline function for the condition on whether to modify the table.

    // TODO: If root_table already has s elements, only add the next n-s.

    // Suppose n is the size of the outer vector and g is a primitive nth
    // root of unity. Then the [lg(m) - 1][j] element of the table is
    // g^{ n/2m * j } for j = 0..m-1

    let rt = F::primitive_root_of_unity(lg_n);
    let n = 1 << lg_n;
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
}

/*
fn update_fft_classic_root_table<F: Field>() {
    // Pre-calculate the primitive 2^m-th roots of unity
    let mut roots_of_unity = Vec::with_capacity(lg_n);
    let mut base = F::primitive_root_of_unity(lg_n);
    roots_of_unity.push(base);
    for _ in 2..=lg_n {
        base = base.square();
        roots_of_unity.push(base);
    }
}
*/

#[inline]
fn fft_dispatch<F: Field>(input: Vec<F>) -> Vec<F> {
    match FFT_STRATEGY {
        FftStrategy::Classic
            => fft_classic(input),
        FftStrategy::Barretenberg
            => fft_barretenberg(input) //, fft_barretenberg_precomp(input.len()))
    }
}

pub fn fft<F: Field>(poly: PolynomialCoeffs<F>) -> PolynomialValues<F> {
    let PolynomialCoeffs { coeffs } = poly;
    PolynomialValues { values: fft_dispatch(coeffs) }
}

pub fn ifft<F: Field>(
    poly: PolynomialValues<F>
) -> PolynomialCoeffs<F> {
    let n = poly.len();
    let lg_n = log2_strict(n);
    let n_inv = F::inverse_2exp(lg_n);

    let PolynomialValues { values } = poly;
    let mut coeffs = fft_dispatch(values);

    // We reverse all values except the first, and divide each by n.
    coeffs[0] *= n_inv;
    coeffs[n / 2] *= n_inv;
    for i in 1..(n / 2) {
        let j = n - i;
        let coeffs_i = coeffs[j] * n_inv;
        let coeffs_j = coeffs[i] * n_inv;
        coeffs[i] = coeffs_i;
        coeffs[j] = coeffs_j;
    }
    PolynomialCoeffs { coeffs }
}

/// FFT implementation based on Section 32.3 of "Introduction to
/// Algorithms" by Cormen et al.
pub(crate) fn fft_classic<F: Field>(
    input: Vec<F>
) -> Vec<F> {
    let mut values = reverse_index_bits(input);

    // TODO: First round is mult by 1, so should be done separately
    // TODO: Unroll later rounds.

    let n = values.len();
    let lg_n = log2_strict(n);

    // Pre-calculate the primitive 2^m-th roots of unity
    let mut bases = Vec::with_capacity(lg_n);
    let mut base = F::primitive_root_of_unity(lg_n);
    bases.push(base);
    for _ in 2..=lg_n {
        base = base.square();
        bases.push(base);
    }

    let mut root_table = Vec::with_capacity(lg_n);
    for lg_m in 1..=lg_n {
        let half_m = 1 << (lg_m - 1);
        let mut root_row = Vec::with_capacity(half_m);
        let base = bases[lg_n - lg_m];
        let mut omega = base;
        root_row.push(F::ONE);
        root_row.push(omega);
        for j in 2..half_m {
            omega *= base;
            root_row.push(omega);
        }
        root_table.push(root_row);
    }

    let mut m = 2;
    for lg_m in 1..=lg_n {
        let half_m = m / 2;
        for k in (0..n).step_by(m) {
            for j in 0..half_m {
                let omega = root_table[lg_m - 1][j];
                let t = omega * values[k + half_m + j];
                let u = values[k + j];
                values[k + j] = u + t;
                values[k + half_m + j] = u - t;
            }
        }
        m *= 2;
    }
    values
}

pub fn fft_thing<F: Field>(poly: PolynomialCoeffs<F>, rate_bits: usize) -> PolynomialValues<F> {
    let PolynomialCoeffs { coeffs } = poly;
    PolynomialValues { values: fft_thingy(coeffs, rate_bits) }
}


/// FFT implementation based on Section 32.3 of "Introduction to
/// Algorithms" by Cormen et al.
pub fn fft_thingy<F: Field>(
    input: Vec<F>,
    r: usize
) -> Vec<F> {
    let mut values = reverse_index_bits(input);

    // TODO: Unroll later rounds.

    let n = values.len();
    let lg_n = log2_strict(n);

    // Pre-calculate the primitive 2^m-th roots of unity
    let mut bases = Vec::with_capacity(lg_n);
    let mut base = F::primitive_root_of_unity(lg_n);
    bases.push(base);
    for _ in 2..=lg_n {
        base = base.square();
        bases.push(base);
    }

    let mut root_table = Vec::with_capacity(lg_n);
    for lg_m in 1..=lg_n {
        let half_m = 1 << (lg_m - 1);
        let mut root_row = Vec::with_capacity(half_m);
        let base = bases[lg_n - lg_m];
        let mut omega = base;
        root_row.push(F::ONE);
        root_row.push(omega);
        for j in 2..half_m {
            omega *= base;
            root_row.push(omega);
        }
        root_table.push(root_row);
    }

    // After reverse_index_bits, the only non-zero elements of values
    // are at indices i*2^r for i = 0..n/2^r.  The loop below copies
    // the value at i*2^r to the positions [i*2^r + 1, i*2^r + 2, ...,
    // (i+1)*2^r - 1]; i.e. it replaces the 2^r - 1 zeros following
    // element i*2^r with the value at i*2^r.  This corresponds to the
    // first r rounds of the FFT when there are 2^r zeros at the end
    // of the original input.
    let mask = !((1 << r) - 1);
    for i in 0..n {
        values[i] = values[i & mask];
    }

    let mut m = 1 << (r + 1);
    for lg_m in (r+1)..=lg_n {
        let half_m = m / 2;
        for k in (0..n).step_by(m) {
            for j in 0..half_m {
                let omega = root_table[lg_m - 1][j];
                let t = omega * values[k + half_m + j];
                let u = values[k + j];
                values[k + j] = u + t;
                values[k + half_m + j] = u - t;
            }
        }
        m *= 2;
    }
    values
}

/// FFT implementation inspired by Barretenberg's:
/// https://github.com/AztecProtocol/barretenberg/blob/master/barretenberg/src/aztec/polynomials/polynomial_arithmetic.cpp#L58
/// https://github.com/AztecProtocol/barretenberg/blob/master/barretenberg/src/aztec/polynomials/evaluation_domain.cpp#L30
pub(crate) fn fft_barretenberg<F: Field>(
    input: Vec<F>
) -> Vec<F> {
    let n = input.len();
    let lg_n = log2_strict(input.len());

    let mut values = reverse_index_bits(input);

    // FFT of a constant polynomial (including zero) is itself.
    if n < 2 {
        return values
    }

    // Precompute a table of the roots of unity used in the main
    // loops.
    let mut root_table: FftRootTable<F> = Vec::with_capacity(lg_n);
    update_fft_root_table(&mut root_table, lg_n);

    // The 'm' corresponds to the specialisation from the 'm' in the
    // main loop (m >= 4) below.

    // m = 1
    for k in (0..n).step_by(2) {
        let t = values[k + 1];
        values[k + 1] = values[k] - t;
        values[k] += t;
    }

    if n == 2 {
        return values
    }

    // m = 2
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
    for lg_m in 2..lg_n {
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
    }
    values
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
