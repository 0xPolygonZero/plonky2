use std::cmp::min;
use std::option::Option;

use plonky2_util::{log2_strict, reverse_index_bits_in_place};

use crate::packable::Packable;
use crate::packed::PackedField;
use crate::polynomial::{PolynomialCoeffs, PolynomialValues};
use crate::types::Field;

pub type FftRootTable<F> = Vec<F>;

pub fn fft_root_table<F: Field>(n: usize) -> FftRootTable<F> {
    let lg_n = log2_strict(n);

    if lg_n <= 1 {
        vec![F::ONE; 1]
    } else {
        let base = F::primitive_root_of_unity(lg_n);
        let half_n = 1 << (lg_n - 1);
        let mut root_table = vec![F::ZERO; half_n];
        // store roots of unity in "reverse bits" order
        // faster than calling: reverse_index_bits_in_place(&mut root_table[..])
        for (i, b) in base.powers().take(half_n).enumerate() {
            let j = i.reverse_bits() >> (64 - lg_n + 1);
            root_table[j] = b;
        }
        root_table
    }
}

#[inline]
fn fft_dispatch<F: Field>(
    input: &mut [F],
    zero_factor: Option<usize>,
    root_table: Option<&FftRootTable<F>>,
) {
    let computed_root_table = if root_table.is_some() {
        None
    } else {
        Some(fft_root_table(input.len()))
    };
    let used_root_table = root_table.or(computed_root_table.as_ref()).unwrap();

    fft_bowers(input, zero_factor.unwrap_or(0), used_root_table);
}

#[inline]
pub fn fft<F: Field>(poly: PolynomialCoeffs<F>) -> PolynomialValues<F> {
    fft_with_options(poly, None, None)
}

#[inline]
pub fn fft_with_options<F: Field>(
    poly: PolynomialCoeffs<F>,
    zero_factor: Option<usize>,
    root_table: Option<&FftRootTable<F>>,
) -> PolynomialValues<F> {
    let PolynomialCoeffs { coeffs: mut buffer } = poly;
    fft_dispatch(&mut buffer, zero_factor, root_table);
    PolynomialValues::new(buffer)
}

#[inline]
pub fn ifft<F: Field>(poly: PolynomialValues<F>) -> PolynomialCoeffs<F> {
    ifft_with_options(poly, None, None)
}

pub fn ifft_with_options<F: Field>(
    poly: PolynomialValues<F>,
    zero_factor: Option<usize>,
    root_table: Option<&FftRootTable<F>>,
) -> PolynomialCoeffs<F> {
    let n = poly.len();
    let lg_n = log2_strict(n);
    let n_inv = F::inverse_2exp(lg_n);

    let PolynomialValues { values: mut buffer } = poly;
    fft_dispatch(&mut buffer, zero_factor, root_table);

    // We reverse all values except the first, and divide each by n.
    buffer[0] *= n_inv;
    buffer[n / 2] *= n_inv;
    for i in 1..(n / 2) {
        let j = n - i;
        let coeffs_i = buffer[j] * n_inv;
        let coeffs_j = buffer[i] * n_inv;
        buffer[i] = coeffs_i;
        buffer[j] = coeffs_j;
    }
    PolynomialCoeffs { coeffs: buffer }
}

/// Form omega for the case where `half_m` is smaller than `P::WIDTH`.
///
/// `half_m` is the number of contiguous values that are multiplied by the same twiddle factor.
/// When `half_m >= P::WIDTH`, all packed values in the loop iteration are multiplied by the same
/// twiddle factor, so the `omega` vector is just one value repeated `P::WIDTH` times. However, when
/// `half_m < P::WIDTH`, the `omega` vector is composed of different `P::WIDTH / half_m` twiddle
/// factors, and we construct it here.
///
/// We return the twiddle factors for `P::WIDTH / half_m` values (`half_m` doubles with every
/// iteration). More concretely, the return value is read from
/// `root_table[(k / 2) * P::WIDTH / half_m][..P::WIDTH / half_m]`, with each twiddle factor
/// repeated `half_m` times.
///
/// The twiddle factors are permuted in a seemingly strange way, but this is needed for consistency
/// with `PackedField::interleave`. To minimize the movement of data, `u.interleave(v, half_m)`
/// exchanges `u[half_m..2 * half_m]` with `v[0..half_m]`, `u[3 * half_m..4 * half_m]` with
/// `v[2 * half_m..3 * half_m]`, and so on. Therefore, the arrangement we want is:
///  - For `roots[..roots.len() / 2]` (the first half of the twiddle factors we read), `roots[i]`
///    gets written to `omega[2 * half_m * i..2 * half_m * i + half_m]`. Note that this leaves gaps
///    of `half_m`.
///  - We fill those gaps with `roots[roots.len() / 2..]`. `roots[roots.len() / 2 + i]` gets written
///    to `omega[2 * half_m * i + half_m..2 * half_m * i + 2 * half_m]`.
/// As an example, suppose we have a packing of width 8. Then:
///  - For `half_m == 1`, we return
///    `[roots[0], roots[4], roots[1], roots[5], roots[2], roots[6], roots[3], roots[7]]`.
///  - For `half_m == 2`, we return
///    `[roots[0], roots[0], roots[2], roots[2], roots[1], roots[1], roots[3], roots[3]]`.
///  - For `half_m == 4`, we return
///    `[roots[0], roots[0], roots[0], roots[0], roots[1], roots[1], roots[1], roots[1]]`.
fn form_small_m_omega<P: PackedField>(
    k: usize,
    lg_half_m: usize,
    root_table: &FftRootTable<P::Scalar>,
) -> P {
    let half_k = k / 2;
    let lg_m = lg_half_m + 1;
    let half_m = 1 << lg_half_m;

    // Ideally, we'd like the compiler to optimize the permutation below and just emit vector
    // instructions. Thankfully, LLVM seems to do just that: https://godbolt.org/z/MG11W8Wj6

    let roots = &root_table[half_k * (P::WIDTH >> lg_half_m)..][..P::WIDTH >> lg_half_m];

    let mut omega = P::default();
    let omega_slice = omega.as_slice_mut();
    for i in 0..P::WIDTH >> lg_m {
        omega_slice[i << lg_m..][..half_m].fill(roots[i]);
        omega_slice[(i << lg_m) + half_m..][..half_m].fill(roots[i + (P::WIDTH >> lg_m)]);
    }
    omega
}

/// FFT implementation that works with both scalar and packed inputs.
/// Bowers et al., Improved Twiddle Access for Fast Fourier Transforms
/// https://doi.org/10.1109/TSP.2009.2035984
/// In short, Bowers et al. rearrange the computation so that
/// the *twiddle is the same* within the inner-most loop.
/// Surprisingly, this ends up looking like a decimation in time loop,
/// but with a decimation in frequency butterfly!
/// In our experiments this is 10%+ faster than a classic DIT.
fn fft_bowers_simd<P: PackedField>(
    values: &mut [P::Scalar],
    _r: usize,
    lg_n: usize,
    root_table: &FftRootTable<P::Scalar>,
) {
    let lg_packed_width = log2_strict(P::WIDTH); // 0 when P is a scalar.
    let packed_values = P::pack_slice_mut(values);
    let packed_n = packed_values.len();
    debug_assert!(packed_n == 1 << (lg_n - lg_packed_width));

    // Handle `half_m < P::WIDTH`; this is when `omega` is composed of multiple values and must be
    // specially constructed.
    // Want the below for loop to unroll, hence the need for a literal.
    // This loop will not run when P is a scalar.
    assert!(lg_packed_width <= 4);
    for lg_half_m in 0..4 {
        if (0..min(lg_n, lg_packed_width)).contains(&lg_half_m) {
            let half_m = 1 << lg_half_m;
            for k in (0..packed_n).step_by(2) {
                let omega: P = form_small_m_omega(k, lg_half_m, root_table);
                let (u, v) = packed_values[k].interleave(packed_values[k + 1], half_m);
                (packed_values[k], packed_values[k + 1]) =
                    (u + v).interleave((u - v) * omega, half_m);
            }
        }
    }

    // decimation in time loop
    for lg_half_m in lg_packed_width..lg_n {
        let lg_m = lg_half_m + 1;
        let m = 1 << lg_m; // Subarray size (in field elements).
        let packed_m = m >> lg_packed_width; // Subarray size (in vectors).
        let half_packed_m = packed_m / 2;
        debug_assert!(half_packed_m != 0);

        // k = 0 unrolled: w^0 = 1, save the mul
        for j in 0..half_packed_m {
            let u = packed_values[j];
            let v = packed_values[j + half_packed_m];
            packed_values[j] = u + v;
            packed_values[half_packed_m + j] = u - v;
        }

        let mut omega_idx = 1;
        for k in (packed_m..packed_n).step_by(packed_m) {
            // use the same omega for the whole inner loop!
            let omega = root_table[omega_idx];
            for j in 0..half_packed_m {
                // decimation in frequency butterlfy
                let u = packed_values[k + j];
                let v = packed_values[k + j + half_packed_m];
                packed_values[k + j] = u + v;
                packed_values[k + half_packed_m + j] = (u - v) * omega;
            }
            omega_idx += 1;
        }
    }
}

/// FFT implementation based on Section 32.3 of "Introduction to
/// Algorithms" by Cormen et al.
///
/// The parameter r signifies that the first 1/2^r of the entries of
/// input may be non-zero, but the last 1 - 1/2^r entries are
/// definitely zero.
pub(crate) fn fft_bowers<F: Field>(values: &mut [F], r: usize, root_table: &FftRootTable<F>) {
    reverse_index_bits_in_place(values);

    let n = values.len();
    let lg_n = log2_strict(n);

    let lg_packed_width = log2_strict(<F as Packable>::Packing::WIDTH);
    if lg_n <= lg_packed_width {
        // Need the slice to be at least the width of two packed vectors for the vectorized version
        // to work. Do this tiny problem in scalar.
        fft_bowers_simd::<F>(values, r, lg_n, root_table);
    } else {
        fft_bowers_simd::<<F as Packable>::Packing>(values, r, lg_n, root_table);
    }
}

#[cfg(test)]
mod tests {
    use plonky2_util::{log2_ceil, log2_strict};

    use crate::fft::{fft, fft_with_options, ifft};
    use crate::goldilocks_field::GoldilocksField;
    use crate::polynomial::{PolynomialCoeffs, PolynomialValues};
    use crate::types::Field;

    #[test]
    fn fft_and_ifft() {
        type F = GoldilocksField;
        let degree = 200usize;
        let degree_padded = degree.next_power_of_two();

        // Create a vector of coeffs; the first degree of them are
        // "random", the last degree_padded-degree of them are zero.
        let coeffs = (0..degree)
            .map(|i| F::from_canonical_usize(i * 1337 % 100))
            .chain(std::iter::repeat(F::ZERO).take(degree_padded - degree))
            .collect::<Vec<_>>();
        assert_eq!(coeffs.len(), degree_padded);
        let coefficients = PolynomialCoeffs { coeffs };

        let points = fft(coefficients.clone());
        assert_eq!(points, evaluate_naive(&coefficients));

        let interpolated_coefficients = ifft(points);
        for i in 0..degree {
            assert_eq!(interpolated_coefficients.coeffs[i], coefficients.coeffs[i]);
        }
        for i in degree..degree_padded {
            assert_eq!(interpolated_coefficients.coeffs[i], F::ZERO);
        }

        for r in 0..4 {
            // expand coefficients by factor 2^r by filling with zeros
            let zero_tail = coefficients.lde(r);
            assert_eq!(
                fft(zero_tail.clone()),
                fft_with_options(zero_tail, Some(r), None)
            );
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
            .map(|x| evaluate_at_naive(coefficients, x))
            .collect();
        PolynomialValues::new(values)
    }

    fn evaluate_at_naive<F: Field>(coefficients: &PolynomialCoeffs<F>, point: F) -> F {
        let mut sum = F::ZERO;
        let mut point_power = F::ONE;
        for &c in &coefficients.coeffs {
            sum += c * point_power;
            point_power *= point;
        }
        sum
    }
}
