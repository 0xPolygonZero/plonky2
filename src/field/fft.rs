use std::cmp::max;
use std::option::Option;

use unroll::unroll_for_loops;

use crate::field::field_types::Field;
use crate::field::packable::Packable;
use crate::field::packed_field::{PackedField, Singleton};
use crate::polynomial::polynomial::{PolynomialCoeffs, PolynomialValues};
use crate::util::{log2_strict, reverse_index_bits};

// TODO: Should really do some "dynamic" dispatch to handle the
// different FFT algos rather than C-style enum dispatch.
#[derive(Copy, Clone, Debug)]
pub enum FftStrategy {
    Classic,
    Unrolled,
}

pub(crate) const DEFAULT_STRATEGY: FftStrategy = FftStrategy::Classic;

pub(crate) type FftRootTable<F> = Vec<Vec<F>>;

fn fft_classic_root_table<F: Field>(n: usize) -> FftRootTable<F> {
    let lg_n = log2_strict(n);
    // bases[i] = g^2^i, for i = 0, ..., lg_n - 1
    let mut bases = Vec::with_capacity(lg_n);
    let mut base = F::primitive_root_of_unity(lg_n);
    bases.push(base);
    for _ in 1..lg_n {
        base = base.square(); // base = g^2^_
        bases.push(base);
    }

    let mut root_table = Vec::with_capacity(lg_n);
    for lg_m in 1..=lg_n {
        let half_m = 1 << (lg_m - 1);
        let base = bases[lg_n - lg_m];
        let root_row = base.powers().take(half_m.max(2)).collect();
        root_table.push(root_row);
    }
    root_table
}

fn fft_unrolled_root_table<F: Field>(n: usize) -> FftRootTable<F> {
    // Precompute a table of the roots of unity used in the main
    // loops.

    // Suppose n is the size of the outer vector and g is a primitive nth
    // root of unity. Then the [lg(m) - 1][j] element of the table is
    // g^{ n/2m * j } for j = 0..m-1

    let lg_n = log2_strict(n);
    // bases[i] = g^2^i, for i = 0, ..., lg_n - 2
    let mut bases = Vec::with_capacity(lg_n);
    let mut base = F::primitive_root_of_unity(lg_n);
    bases.push(base);
    // NB: If n = 1, then lg_n is zero, so we can't do 1..(lg_n-1) here
    for _ in 2..lg_n {
        base = base.square(); // base = g^2^(_-1)
        bases.push(base);
    }

    let mut root_table = Vec::with_capacity(lg_n);
    for lg_m in 1..lg_n {
        let m = 1 << lg_m;
        let base = bases[lg_n - lg_m - 1];
        let root_row = base.powers().take(m.max(2)).collect();
        root_table.push(root_row);
    }
    root_table
}

#[inline]
fn fft_dispatch<F: Field>(
    input: &[F],
    strategy: FftStrategy,
    zero_factor: Option<usize>,
    root_table: Option<FftRootTable<F>>,
) -> Vec<F> {
    let n = input.len();
    match strategy {
        FftStrategy::Classic => fft_classic(
            input,
            zero_factor.unwrap_or(0),
            root_table.unwrap_or_else(|| fft_classic_root_table(n)),
        ),
        FftStrategy::Unrolled => fft_unrolled(
            input,
            zero_factor.unwrap_or(0),
            root_table.unwrap_or_else(|| fft_unrolled_root_table(n)),
        ),
    }
}

#[inline]
pub fn fft<F: Field>(poly: &PolynomialCoeffs<F>) -> PolynomialValues<F> {
    fft_with_options(poly, DEFAULT_STRATEGY, None, None)
}

#[inline]
pub fn fft_with_options<F: Field>(
    poly: &PolynomialCoeffs<F>,
    strategy: FftStrategy,
    zero_factor: Option<usize>,
    root_table: Option<FftRootTable<F>>,
) -> PolynomialValues<F> {
    let PolynomialCoeffs { coeffs } = poly;
    PolynomialValues {
        values: fft_dispatch(coeffs, strategy, zero_factor, root_table),
    }
}

#[inline]
pub fn ifft<F: Field>(poly: &PolynomialValues<F>) -> PolynomialCoeffs<F> {
    ifft_with_options(poly, DEFAULT_STRATEGY, None, None)
}

pub fn ifft_with_options<F: Field>(
    poly: &PolynomialValues<F>,
    strategy: FftStrategy,
    zero_factor: Option<usize>,
    root_table: Option<FftRootTable<F>>,
) -> PolynomialCoeffs<F> {
    let n = poly.len();
    let lg_n = log2_strict(n);
    let n_inv = F::inverse_2exp(lg_n);

    let PolynomialValues { values } = poly;
    let mut coeffs = fft_dispatch(values, strategy, zero_factor, root_table);

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

/// Generic FFT implementation that works with both scalar and packed inputs.
#[unroll_for_loops]
fn fft_classic_simd<P: PackedField>(
    values: &mut [P::FieldType],
    r: usize,
    lg_n: usize,
    root_table: &FftRootTable<P::FieldType>,
) {
    let log_packed_width = P::LOG2_WIDTH; // 0 when P is a scalar.
    let packed_values = P::pack_slice_mut(values);
    let packed_n = packed_values.len();
    debug_assert!(packed_n == 1 << (lg_n - log_packed_width));

    // Want the below for loop to unroll, hence the need for a literal.
    // This loop will not run when P is a scalar.
    assert!(log_packed_width <= 4);
    for i in 0..4 {
        if i < log_packed_width && i >= r && i + 1 >= lg_n {
            // Intuitively, we split values into m slices: subarr[0], ..., subarr[m - 1]. Each of
            // those slices is split into two halves: subarr[j].left, subarr[j].right. We do
            // (subarr[j].left[k], subarr[j].right[k])
            //   := f(subarr[j].left[k], subarr[j].right[k], omega[k]),
            // where f(u, v, omega) = (u + omega * v, u - omega * v).
            let half_m = 1 << i;

            // Set omega to root_table[i][0..half_m] but repeated
            let mut omega_arr = P::zero().to_vec();
            for j in 0..omega_arr.len() {
                omega_arr[j] = root_table[i][j % half_m];
            }
            let omega = P::new_from_slice(&omega_arr);

            for k in (0..packed_n).step_by(2) {
                // We have two vectors and want to do math on pairs of adjacent elements (or for
                // i > 0, pairs of adjacent blocks of elements). .interleave does the appropriate
                // shuffling and is its own transpose.
                let (u, v) = packed_values[k].interleave(packed_values[k + 1], i);
                let t = omega * v;
                (packed_values[k], packed_values[k + 1]) = (u + t).interleave(u - t, i);
            }
        }
    }

    // We've already done the first log_packed_width (if they were required) iterations.
    let s = max(r, log_packed_width) + 1;

    for lg_m in s..=lg_n {
        let m = 1 << lg_m; // Subarray size (in field elements).
        let packed_m = m >> log_packed_width; // Subarray size (in vectors).
        let half_packed_m = packed_m / 2;
        debug_assert!(half_packed_m != 0);

        // omega values for this iteration, as slice of vectors
        let omega_table = P::pack_slice(&root_table[lg_m - 1][..]);
        for k in (0..packed_n).step_by(packed_m) {
            for j in 0..half_packed_m {
                let omega = omega_table[j];
                let t = omega * packed_values[k + half_packed_m + j];
                let u = packed_values[k + j];
                packed_values[k + j] = u + t;
                packed_values[k + half_packed_m + j] = u - t;
            }
        }
    }
}

/// FFT implementation based on Section 32.3 of "Introduction to
/// Algorithms" by Cormen et al.
///
/// The parameter r signifies that the first 1/2^r of the entries of
/// input may be non-zero, but the last 1 - 1/2^r entries are
/// definitely zero.
pub(crate) fn fft_classic<F: Field>(input: &[F], r: usize, root_table: FftRootTable<F>) -> Vec<F> {
    let mut values = reverse_index_bits(input);

    let n = values.len();
    let lg_n = log2_strict(n);

    if root_table.len() != lg_n {
        panic!(
            "Expected root table of length {}, but it was {}.",
            lg_n,
            root_table.len()
        );
    }

    // After reverse_index_bits, the only non-zero elements of values
    // are at indices i*2^r for i = 0..n/2^r.  The loop below copies
    // the value at i*2^r to the positions [i*2^r + 1, i*2^r + 2, ...,
    // (i+1)*2^r - 1]; i.e. it replaces the 2^r - 1 zeros following
    // element i*2^r with the value at i*2^r.  This corresponds to the
    // first r rounds of the FFT when there are 2^r zeros at the end
    // of the original input.
    if r > 0 {
        // if r == 0 then this loop is a noop.
        let mask = !((1 << r) - 1);
        for i in 0..n {
            values[i] = values[i & mask];
        }
    }

    let log_packed_width = <F as Packable>::PackedType::LOG2_WIDTH;
    if lg_n <= log_packed_width {
        // Need the slice to be at least the width of two packed vectors for the vectorized version
        // to work. Do this tiny problem in scalar.
        fft_classic_simd::<Singleton<F>>(&mut values[..], r, lg_n, &root_table);
    } else {
        fft_classic_simd::<<F as Packable>::PackedType>(&mut values[..], r, lg_n, &root_table);
    }
    values
}

/// FFT implementation inspired by Barretenberg's (but with extra unrolling):
/// https://github.com/AztecProtocol/barretenberg/blob/master/barretenberg/src/aztec/polynomials/polynomial_arithmetic.cpp#L58
/// https://github.com/AztecProtocol/barretenberg/blob/master/barretenberg/src/aztec/polynomials/evaluation_domain.cpp#L30
///
/// The parameter r signifies that the first 1/2^r of the entries of
/// input may be non-zero, but the last 1 - 1/2^r entries are
/// definitely zero.
fn fft_unrolled<F: Field>(input: &[F], r_orig: usize, root_table: FftRootTable<F>) -> Vec<F> {
    let n = input.len();
    let lg_n = log2_strict(input.len());

    let mut values = reverse_index_bits(input);

    // FFT of a constant polynomial (including zero) is itself.
    if n < 2 {
        return values;
    }

    // The 'm' corresponds to the specialisation from the 'm' in the
    // main loop (m >= 4) below.

    // (See comment in fft_classic near same code.)
    let mut r = r_orig;
    let mut m = 1 << r;
    if r > 0 {
        // if r == 0 then this loop is a noop.
        let mask = !((1 << r) - 1);
        for i in 0..n {
            values[i] = values[i & mask];
        }
    }

    // m = 1
    if m == 1 {
        for k in (0..n).step_by(2) {
            let t = values[k + 1];
            values[k + 1] = values[k] - t;
            values[k] += t;
        }
        r += 1;
        m *= 2;
    }

    if n == 2 {
        return values;
    }

    if root_table.len() != (lg_n - 1) {
        panic!(
            "Expected root table of length {}, but it was {}.",
            lg_n,
            root_table.len()
        );
    }

    // m = 2
    if m <= 2 {
        for k in (0..n).step_by(4) {
            // NB: Grouping statements as is done in the main loop below
            // does not seem to help here (worse by a few millis).
            let omega_0 = root_table[0][0];
            let tmp_0 = omega_0 * values[k + 2];
            values[k + 2] = values[k] - tmp_0;
            values[k] += tmp_0;

            let omega_1 = root_table[0][1];
            let tmp_1 = omega_1 * values[k + 2 + 1];
            values[k + 2 + 1] = values[k + 1] - tmp_1;
            values[k + 1] += tmp_1;
        }
        r += 1;
        m *= 2;
    }

    // m >= 4
    for lg_m in r..lg_n {
        for k in (0..n).step_by(2 * m) {
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

                let omega_0 = root_table[lg_m - 1][j];
                let omega_1 = root_table[lg_m - 1][j + 1];
                let omega_2 = root_table[lg_m - 1][j + 2];
                let omega_3 = root_table[lg_m - 1][j + 3];

                let tmp_0 = omega_0 * values[off2];
                let tmp_1 = omega_1 * values[off2 + 1];
                let tmp_2 = omega_2 * values[off2 + 2];
                let tmp_3 = omega_3 * values[off2 + 3];

                values[off2] = values[off1] - tmp_0;
                values[off2 + 1] = values[off1 + 1] - tmp_1;
                values[off2 + 2] = values[off1 + 2] - tmp_2;
                values[off2 + 3] = values[off1 + 3] - tmp_3;
                values[off1] += tmp_0;
                values[off1 + 1] += tmp_1;
                values[off1 + 2] += tmp_2;
                values[off1 + 3] += tmp_3;
            }
        }
        m *= 2;
    }
    values
}

#[cfg(test)]
mod tests {
    use crate::field::crandall_field::CrandallField;
    use crate::field::fft::{fft, fft_with_options, ifft, FftStrategy};
    use crate::field::field_types::Field;
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

        let points = fft(&coefficients);
        assert_eq!(points, evaluate_naive(&coefficients));

        let interpolated_coefficients = ifft(&points);
        for i in 0..degree {
            assert_eq!(interpolated_coefficients.coeffs[i], coefficients.coeffs[i]);
        }
        for i in degree..degree_padded {
            assert_eq!(interpolated_coefficients.coeffs[i], F::ZERO);
        }

        for strategy in [FftStrategy::Classic, FftStrategy::Unrolled] {
            for r in 0..4 {
                // expand coefficients by factor 2^r by filling with zeros
                let zero_tail = coefficients.lde(r);
                assert_eq!(
                    fft(&zero_tail),
                    fft_with_options(&zero_tail, strategy, Some(r), None)
                );
            }
        }
    }

    fn evaluate_naive<F: Field>(coefficients: &PolynomialCoeffs<F>) -> PolynomialValues<F> {
        let degree = coefficients.len();
        let degree_padded = 1 << log2_ceil(degree);

        let coefficients_padded = coefficients.padded(degree_padded);
        evaluate_naive_power_of_2(&coefficients_padded)
    }

    fn evaluate_naive_power_of_2<F: Field>(
        coefficients: &PolynomialCoeffs<F>,
    ) -> PolynomialValues<F> {
        let degree = coefficients.len();
        let degree_log = log2_strict(degree);

        let subgroup = F::two_adic_subgroup(degree_log);

        let values = subgroup
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
