use crate::field::fft::{fft, ifft};
use crate::field::field::Field;
use crate::util::{log2_ceil, log2_strict};
use crate::polynomial::polynomial::PolynomialCoeffs;

/// Takes a polynomial `a` in coefficient form, and divides it by `Z_H = X^n - 1`.
///
/// This assumes `Z_H | a`, otherwise result is meaningless.
pub(crate) fn divide_by_z_h<F: Field>(mut a: PolynomialCoeffs<F>, n: usize) -> PolynomialCoeffs<F> {
    // TODO: Is this special case needed?
    if a.coeffs.iter().all(|p| *p == F::ZERO) {
        return a.clone();
    }

    let g = F::MULTIPLICATIVE_SUBGROUP_GENERATOR;
    let mut g_pow = F::ONE;
    // Multiply the i-th coefficient of `a` by `g^i`. Then `new_a(w^j) = old_a(g.w^j)`.
    a.coeffs.iter_mut().for_each(|x| {
        *x = (*x) * g_pow;
        g_pow = g * g_pow;
    });

    let root = F::primitive_root_of_unity(log2_strict(a.len()));
    // Equals to the evaluation of `a` on `{g.w^i}`.
    let mut a_eval = fft(a);
    // Compute the denominators `1/(g^n.w^(n*i) - 1)` using batch inversion.
    let denominator_g = g.exp_usize(n);
    let root_n = root.exp_usize(n);
    let mut root_pow = F::ONE;
    let denominators = (0..a_eval.len())
        .map(|i| {
            if i != 0 {
                root_pow = root_pow * root_n;
            }
            denominator_g * root_pow - F::ONE
        })
        .collect::<Vec<_>>();
    let denominators_inv = F::batch_multiplicative_inverse(&denominators);
    // Divide every element of `a_eval` by the corresponding denominator.
    // Then, `a_eval` is the evaluation of `a/Z_H` on `{g.w^i}`.
    a_eval.values
        .iter_mut()
        .zip(denominators_inv.iter())
        .for_each(|(x, &d)| {
            *x = (*x) * d;
        });
    // `p` is the interpolating polynomial of `a_eval` on `{w^i}`.
    let mut p = ifft(a_eval);
    // We need to scale it by `g^(-i)` to get the interpolating polynomial of `a_eval` on `{g.w^i}`,
    // a.k.a `a/Z_H`.
    let g_inv = g.inverse();
    let mut g_inv_pow = F::ONE;
    p.coeffs.iter_mut().for_each(|x| {
        *x = (*x) * g_inv_pow;
        g_inv_pow = g_inv_pow * g_inv;
    });
    p
}

#[cfg(test)]
mod tests {
    #[test]
    fn division_by_z_h() {
        // TODO
    }
}
