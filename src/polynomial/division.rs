use crate::field::fft::{fft, ifft};
use crate::field::field::Field;
use crate::polynomial::polynomial::PolynomialCoeffs;
use crate::util::{log2_strict, log2_ceil};

impl<F: Field> PolynomialCoeffs<F> {
    /// Polynomial division.
    /// Returns `(q, r)`, the quotient and remainder of the polynomial division of `a` by `b`.
    pub(crate) fn div_rem(&self, b: &Self) -> (Self, Self) {
        let (a_degree_plug_1, b_degree_plus_1) = (self.degree_plus_one(), b.degree_plus_one());
        if a_degree_plug_1 == 0 {
            (Self::zero(1), Self::empty())
        } else if b_degree_plus_1 == 0 {
            panic!("Division by zero polynomial");
        } else if a_degree_plug_1 < b_degree_plus_1 {
            (Self::zero(1), self.clone())
        } else if b_degree_plus_1 == 1 {
            (self * b.coeffs[0].inverse(), Self::empty())
        } else {
            let rev_b = b.rev();
            let rev_b_inv = rev_b.inv_mod_xn(a_degree_plug_1 - b_degree_plus_1 + 1);
            let rhs: Self = self.rev().coeffs[..=a_degree_plug_1 - b_degree_plus_1]
                .to_vec()
                .into();
            let rev_q: Self = (&rev_b_inv * &rhs).coeffs[..=a_degree_plug_1 - b_degree_plus_1]
                .to_vec()
                .into();
            let mut q = rev_q.rev();
            let mut qb = &q * b;
            let mut r = self - &qb;
            q.trim();
            r.trim();
            (q, r)
        }
    }

    /// Polynomial long division.
    /// Returns `(q, r)`, the quotient and remainder of the polynomial division of `a` by `b`.
    /// Generally slower that the equivalent function `Polynomial::polynomial_division`.
    pub fn div_rem_long_division(&self, b: &Self) -> (Self, Self) {
        let b = b.trimmed();

        let (a_degree_plus_1, b_degree_plus_1) = (self.degree_plus_one(), b.degree_plus_one());
        if a_degree_plus_1 == 0 {
            (Self::zero(1), Self::empty())
        } else if b_degree_plus_1 == 0 {
            panic!("Division by zero polynomial");
        } else if a_degree_plus_1 < b_degree_plus_1 {
            (Self::zero(1), self.clone())
        } else {
            // Now we know that self.degree() >= divisor.degree();
            let mut quotient = Self::zero(a_degree_plus_1 - b_degree_plus_1 + 1);
            let mut remainder = self.clone();
            // Can unwrap here because we know self is not zero.
            let divisor_leading_inv = b.lead().inverse();
            while !remainder.is_zero() && remainder.degree_plus_one() >= b_degree_plus_1 {
                let cur_q_coeff = remainder.lead() * divisor_leading_inv;
                let cur_q_degree = remainder.degree_plus_one() - b_degree_plus_1;
                quotient.coeffs[cur_q_degree] = cur_q_coeff;

                for (i, &div_coeff) in b.coeffs.iter().enumerate() {
                    remainder.coeffs[cur_q_degree + i] =
                        remainder.coeffs[cur_q_degree + i] - (cur_q_coeff * div_coeff);
                }
                remainder.trim();
            }
            (quotient, remainder)
        }
    }

    /// Takes a polynomial `a` in coefficient form, and divides it by `Z_H = X^n - 1`.
    ///
    /// This assumes `Z_H | a`, otherwise result is meaningless.
    pub(crate) fn divide_by_z_h(&self, n: usize) -> PolynomialCoeffs<F> {
        let mut a = self.clone();

        // TODO: Is this special case needed?
        if a.coeffs.iter().all(|p| *p == F::ZERO) {
            return a;
        }

        let g = F::MULTIPLICATIVE_GROUP_GENERATOR;
        let mut g_pow = F::ONE;
        // Multiply the i-th coefficient of `a` by `g^i`. Then `new_a(w^j) = old_a(g.w^j)`.
        a.coeffs.iter_mut().for_each(|x| {
            *x *= g_pow;
            g_pow *= g;
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
        a_eval
            .values
            .iter_mut()
            .zip(denominators_inv.iter())
            .for_each(|(x, &d)| {
                *x *= d;
            });
        // `p` is the interpolating polynomial of `a_eval` on `{w^i}`.
        let mut p = ifft(a_eval);
        // We need to scale it by `g^(-i)` to get the interpolating polynomial of `a_eval` on `{g.w^i}`,
        // a.k.a `a/Z_H`.
        let g_inv = g.inverse();
        let mut g_inv_pow = F::ONE;
        p.coeffs.iter_mut().for_each(|x| {
            *x *= g_inv_pow;
            g_inv_pow *= g_inv;
        });
        p
    }

    /// Computes the inverse of `self` modulo `x^n`.
    pub(crate) fn inv_mod_xn(&self, n: usize) -> Self {
        assert!(self.coeffs[0].is_nonzero(), "Inverse doesn't exist.");
        let mut h = self.padded(n);
        let mut a = Self::empty();
        a.coeffs.push(h.coeffs[0].inverse());
        for i in 0..log2_ceil(n) {
            let l = 1 << i;
            let h0 = h.coeffs[..l].to_vec().into();
            let mut h1: Self = h.coeffs[l..].to_vec().into();
            let mut c = &a * &h0;
            if l == c.len() {
                c = Self::zero(1);
            } else {
                c.coeffs.drain(0..l);
            }
            h1.trim();
            let mut tmp = &a * &h1;
            tmp = &tmp + &c;
            tmp.coeffs.iter_mut().for_each(|x| *x = -(*x));
            tmp.trim();
            let mut b = &a * &tmp;
            b.trim();
            if b.len() > l {
                b.coeffs.drain(l..);
            }
            a.coeffs.extend_from_slice(&b.coeffs);
        }
        a.coeffs.drain(n..);
        a
    }
}

#[cfg(test)]
mod tests {
    use crate::field::crandall_field::CrandallField;
    use crate::field::field::Field;
    use crate::polynomial::polynomial::PolynomialCoeffs;

    #[test]
    fn zero_div_z_h() {
        type F = CrandallField;
        let zero = PolynomialCoeffs::<F>::zero(16);
        let quotient = zero.divide_by_z_h(4);
        assert_eq!(quotient, zero);
    }

    #[test]
    fn division_by_z_h() {
        type F = CrandallField;
        let zero = F::ZERO;
        let one = F::ONE;
        let two = F::TWO;
        let three = F::from_canonical_u64(3);
        let four = F::from_canonical_u64(4);
        let five = F::from_canonical_u64(5);
        let six = F::from_canonical_u64(6);

        // a(x) = Z_4(x) q(x), where
        // a(x) = 3 x^7 + 4 x^6 + 5 x^5 + 6 x^4 - 3 x^3 - 4 x^2 - 5 x - 6
        // Z_4(x) = x^4 - 1
        // q(x) = 3 x^3 + 4 x^2 + 5 x + 6
        let a = PolynomialCoeffs::new(vec![-six, -five, -four, -three, six, five, four, three]);
        let q = PolynomialCoeffs::new(vec![six, five, four, three, zero, zero, zero, zero]);

        let computed_q = a.divide_by_z_h(4);
        assert_eq!(computed_q, q);
    }
}
