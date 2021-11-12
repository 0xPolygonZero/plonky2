use crate::field::field_types::Field;
use crate::polynomial::polynomial::PolynomialCoeffs;
use crate::util::log2_ceil;

impl<F: Field> PolynomialCoeffs<F> {
    /// Polynomial division.
    /// Returns `(q, r)`, the quotient and remainder of the polynomial division of `a` by `b`.
    pub fn div_rem(&self, b: &Self) -> (Self, Self) {
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
            let qb = &q * b;
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
                    remainder.coeffs[cur_q_degree + i] -= cur_q_coeff * div_coeff;
                }
                remainder.trim();
            }
            (quotient, remainder)
        }
    }

    /// Let `self=p(X)`, this returns `(p(X)-p(z))/(X-z)` and `p(z)`.
    /// See https://en.wikipedia.org/wiki/Horner%27s_method
    pub(crate) fn divide_by_linear(&self, z: F) -> (PolynomialCoeffs<F>, F) {
        let mut bs = self
            .coeffs
            .iter()
            .rev()
            .scan(F::ZERO, |acc, &c| {
                *acc = *acc * z + c;
                Some(*acc)
            })
            .collect::<Vec<_>>();
        let ev = bs.pop().unwrap_or(F::ZERO);
        bs.reverse();
        (Self { coeffs: bs }, ev)
    }

    /// Computes the inverse of `self` modulo `x^n`.
    pub fn inv_mod_xn(&self, n: usize) -> Self {
        assert!(self.coeffs[0].is_nonzero(), "Inverse doesn't exist.");

        let h = if self.len() < n {
            self.padded(n)
        } else {
            self.clone()
        };

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
    use std::time::Instant;

    use crate::field::extension_field::quartic::QuarticExtension;
    use crate::field::field_types::Field;
    use crate::field::goldilocks_field::GoldilocksField;
    use crate::polynomial::polynomial::PolynomialCoeffs;

    #[test]
    #[ignore]
    fn test_division_by_linear() {
        type F = QuarticExtension<GoldilocksField>;
        let n = 1_000_000;
        let poly = PolynomialCoeffs::new(F::rand_vec(n));
        let z = F::rand();
        let ev = poly.eval(z);

        let timer = Instant::now();
        let (_quotient, ev2) = poly.div_rem(&PolynomialCoeffs::new(vec![-z, F::ONE]));
        println!("{:.3}s for usual", timer.elapsed().as_secs_f32());
        assert_eq!(ev2.trimmed().coeffs, vec![ev]);

        let timer = Instant::now();
        let (quotient, ev3) = poly.div_rem_long_division(&PolynomialCoeffs::new(vec![-z, F::ONE]));
        println!("{:.3}s for long division", timer.elapsed().as_secs_f32());
        assert_eq!(ev3.trimmed().coeffs, vec![ev]);

        let timer = Instant::now();
        let horn = poly.divide_by_linear(z);
        println!("{:.3}s for Horner", timer.elapsed().as_secs_f32());
        assert_eq!((quotient, ev), horn);
    }

    #[test]
    #[ignore]
    fn test_division_by_quadratic() {
        type F = QuarticExtension<GoldilocksField>;
        let n = 1_000_000;
        let poly = PolynomialCoeffs::new(F::rand_vec(n));
        let quad = PolynomialCoeffs::new(F::rand_vec(2));

        let timer = Instant::now();
        let (quotient0, rem0) = poly.div_rem(&quad);
        println!("{:.3}s for usual", timer.elapsed().as_secs_f32());

        let timer = Instant::now();
        let (quotient1, rem1) = poly.div_rem_long_division(&quad);
        println!("{:.3}s for long division", timer.elapsed().as_secs_f32());

        assert_eq!(quotient0.trimmed(), quotient1.trimmed());
        assert_eq!(rem0.trimmed(), rem1.trimmed());
    }
}
