#![allow(clippy::many_single_char_names)]
use crate::field::fft::{
    fft_precompute, fft_with_precomputation_power_of_2, ifft_with_precomputation_power_of_2,
    FftPrecomputation,
};
use crate::field::field::Field;
use crate::polynomial::polynomial::PolynomialCoeffs;
use crate::util::log2_ceil;
use std::cmp::Ordering;
use std::ops::{Index, IndexMut, RangeBounds};
use std::slice::{Iter, IterMut, SliceIndex};

/// Polynomial struct holding a polynomial in coefficient form.
#[derive(Debug, Clone)]
pub struct Polynomial<F: Field>(Vec<F>);

impl<F: Field> PartialEq for Polynomial<F> {
    fn eq(&self, other: &Self) -> bool {
        let max_terms = self.0.len().max(other.0.len());
        for i in 0..max_terms {
            let self_i = self.0.get(i).cloned().unwrap_or(F::ZERO);
            let other_i = other.0.get(i).cloned().unwrap_or(F::ZERO);
            if self_i != other_i {
                return false;
            }
        }
        true
    }
}

impl<F: Field> Eq for Polynomial<F> {}

impl<F: Field> From<Vec<F>> for Polynomial<F> {
    /// Takes a vector of coefficients and returns the corresponding polynomial.
    fn from(coeffs: Vec<F>) -> Self {
        Self(coeffs)
    }
}

impl<F: Field> From<PolynomialCoeffs<F>> for Polynomial<F> {
    fn from(coeffs: PolynomialCoeffs<F>) -> Self {
        Self(coeffs.coeffs)
    }
}
impl<F: Field> From<Polynomial<F>> for PolynomialCoeffs<F> {
    fn from(poly: Polynomial<F>) -> Self {
        Self::new(poly.0)
    }
}

impl<F, I> Index<I> for Polynomial<F>
where
    F: Field,
    I: SliceIndex<[F]>,
{
    type Output = I::Output;

    /// Indexing on the coefficients.
    fn index(&self, index: I) -> &Self::Output {
        &self.0[index]
    }
}

impl<F, I> IndexMut<I> for Polynomial<F>
where
    F: Field,
    I: SliceIndex<[F]>,
{
    fn index_mut(&mut self, index: I) -> &mut <Self as Index<I>>::Output {
        &mut self.0[index]
    }
}

impl<F: Field> Polynomial<F> {
    /// Takes a slice of coefficients and returns the corresponding polynomial.
    pub fn from_coeffs(coeffs: &[F]) -> Self {
        Self(coeffs.to_vec())
    }

    /// Returns the coefficient vector.
    pub fn coeffs(&self) -> &[F] {
        &self.0
    }

    /// Empty polynomial;
    pub fn empty() -> Self {
        Self(Vec::new())
    }

    /// Zero polynomial with length `len`.
    /// `len = 1` is the standard representation, but sometimes it's useful to set `len > 1`
    /// to have polynomials with uniform length.
    pub fn zero(len: usize) -> Self {
        Self(vec![F::ZERO; len])
    }

    pub fn iter(&self) -> Iter<F> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<F> {
        self.0.iter_mut()
    }

    pub fn is_zero(&self) -> bool {
        self.0.iter().all(|x| x.is_zero())
    }

    /// Number of coefficients held by the polynomial. Is NOT equal to the degree in general.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Degree of the polynomial.
    /// Panics on zero polynomial.
    pub fn degree(&self) -> usize {
        (0usize..self.len())
            .rev()
            .find(|&i| self[i].is_nonzero())
            .expect("Zero polynomial")
    }

    /// Degree of the polynomial + 1.
    fn degree_plus_one(&self) -> usize {
        (0usize..self.len())
            .rev()
            .find(|&i| self[i].is_nonzero())
            .map_or(0, |i| i + 1)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Removes the coefficients in the range `range`.
    fn drain<R: RangeBounds<usize>>(&mut self, range: R) {
        self.0.drain(range);
    }

    /// Evaluates the polynomial at a point `x`.
    pub fn eval(&self, x: F) -> F {
        self.iter().rev().fold(F::ZERO, |acc, &c| acc * x + c)
    }

    /// Evaluates the polynomial at a point `x`, given the list of powers of `x`.
    /// Assumes that `self.len() == x_pow.len()`.
    pub fn eval_from_power(&self, x_pow: &[F]) -> F {
        self.iter()
            .zip(x_pow)
            .fold(F::ZERO, |acc, (&c, &p)| acc + c * p)
    }

    /// Evaluates the polynomial on subgroup of `F^*` with a given FFT precomputation.
    pub(crate) fn eval_domain(&self, fft_precomputation: &FftPrecomputation<F>) -> Vec<F> {
        let domain_size = fft_precomputation.size();
        if self.len() < domain_size {
            // Need to pad the polynomial to have the same length as the domain.
            fft_with_precomputation_power_of_2(
                self.padded(domain_size).coeffs().to_vec().into(),
                fft_precomputation,
            )
            .values
        } else {
            fft_with_precomputation_power_of_2(self.coeffs().to_vec().into(), fft_precomputation)
                .values
        }
    }

    /// Computes the interpolating polynomial of a list of `values` on a subgroup of `F^*`.
    pub(crate) fn from_evaluations(
        values: &[F],
        fft_precomputation: &FftPrecomputation<F>,
    ) -> Self {
        Self(ifft_with_precomputation_power_of_2(values.to_vec().into(), fft_precomputation).coeffs)
    }

    /// Leading coefficient.
    pub fn lead(&self) -> F {
        self.iter()
            .rev()
            .find(|x| x.is_nonzero())
            .map_or(F::ZERO, |x| *x)
    }

    /// Reverse the order of the coefficients, not taking into account the leading zero coefficients.
    fn rev(&self) -> Self {
        let d = self.degree();
        Self(self.0[..=d].iter().rev().copied().collect())
    }

    /// Negates the polynomial's coefficients.
    pub fn neg(&self) -> Self {
        Self(self.iter().map(|&x| -x).collect())
    }

    /// Multiply the polynomial's coefficients by a scalar.
    pub(crate) fn scalar_mul(&self, c: F) -> Self {
        Self(self.iter().map(|&x| c * x).collect())
    }

    /// Removes leading zero coefficients.
    pub fn trim(&mut self) {
        self.0.drain(self.degree_plus_one()..);
    }

    /// Polynomial addition.
    pub fn add(&self, other: &Self) -> Self {
        let (mut a, mut b) = (self.clone(), other.clone());
        match a.len().cmp(&b.len()) {
            Ordering::Less => a.pad(b.len()),
            Ordering::Greater => b.pad(a.len()),
            _ => (),
        }
        Self(a.iter().zip(b.iter()).map(|(&x, &y)| x + y).collect())
    }

    /// Zero-pad the coefficients to have a given length.
    pub fn pad(&mut self, len: usize) {
        self.trim();
        assert!(self.len() <= len);
        self.0.extend((self.len()..len).map(|_| F::ZERO));
    }

    /// Returns the zero-padded polynomial.
    pub fn padded(&self, len: usize) -> Self {
        let mut a = self.clone();
        a.pad(len);
        a
    }

    /// Polynomial multiplication.
    pub fn mul(&self, b: &Self) -> Self {
        if self.is_zero() || b.is_zero() {
            return Self::zero(1);
        }
        let a_deg = self.degree();
        let b_deg = b.degree();
        let new_deg = (a_deg + b_deg + 1).next_power_of_two();
        let a_pad = self.padded(new_deg);
        let b_pad = b.padded(new_deg);

        let precomputation = fft_precompute(new_deg);
        let a_evals = fft_with_precomputation_power_of_2(a_pad.0.to_vec().into(), &precomputation);
        let b_evals = fft_with_precomputation_power_of_2(b_pad.0.to_vec().into(), &precomputation);

        let mul_evals: Vec<F> = a_evals
            .values
            .iter()
            .zip(b_evals.values.iter())
            .map(|(&pa, &pb)| pa * pb)
            .collect();
        ifft_with_precomputation_power_of_2(mul_evals.to_vec().into(), &precomputation)
            .coeffs
            .into()
    }

    /// Polynomial long division.
    /// Returns `(q,r)` the quotient and remainder of the polynomial division of `a` by `b`.
    /// Generally slower that the equivalent function `Polynomial::polynomial_division`.
    pub fn polynomial_long_division(&self, b: &Self) -> (Self, Self) {
        let (a_degree, b_degree) = (self.degree(), b.degree());
        if self.is_zero() {
            (Self::zero(1), Self::empty())
        } else if b.is_zero() {
            panic!("Division by zero polynomial");
        } else if a_degree < b_degree {
            (Self::zero(1), self.clone())
        } else {
            // Now we know that self.degree() >= divisor.degree();
            let mut quotient = Self::zero(a_degree - b_degree + 1);
            let mut remainder = self.clone();
            // Can unwrap here because we know self is not zero.
            let divisor_leading_inv = b.lead().inverse();
            while !remainder.is_zero() && remainder.degree() >= b_degree {
                let cur_q_coeff = remainder.lead() * divisor_leading_inv;
                let cur_q_degree = remainder.degree() - b_degree;
                quotient[cur_q_degree] = cur_q_coeff;

                for (i, &div_coeff) in b.iter().enumerate() {
                    remainder[cur_q_degree + i] =
                        remainder[cur_q_degree + i] - (cur_q_coeff * div_coeff);
                }
                remainder.trim();
            }
            (quotient, remainder)
        }
    }

    /// Computes the inverse of `self` modulo `x^n`.
    fn inv_mod_xn(&self, n: usize) -> Self {
        assert!(self[0].is_nonzero(), "Inverse doesn't exist.");
        let mut h = self.clone();
        if h.len() < n {
            h.pad(n);
        }
        let mut a = Self::empty();
        a.0.push(h[0].inverse());
        for i in 0..log2_ceil(n) {
            let l = 1 << i;
            let h0 = h[..l].to_vec().into();
            let mut h1: Polynomial<F> = h[l..].to_vec().into();
            let mut c = a.mul(&h0);
            if l == c.len() {
                c = Self::zero(1);
            } else {
                c.drain(0..l);
            }
            h1.trim();
            let mut tmp = a.mul(&h1);
            tmp = tmp.add(&c);
            tmp.iter_mut().for_each(|x| *x = -(*x));
            tmp.trim();
            let mut b = a.mul(&tmp);
            b.trim();
            if b.len() > l {
                b.drain(l..);
            }
            a.0.extend_from_slice(&b[..]);
        }
        a.drain(n..);
        a
    }

    /// Polynomial division.
    /// Returns `(q,r)` the quotient and remainder of the polynomial division of `a` by `b`.
    /// Algorithm from http://people.csail.mit.edu/madhu/ST12/scribe/lect06.pdf
    pub fn polynomial_division(&self, b: &Self) -> (Self, Self) {
        let (a_degree, b_degree) = (self.degree(), b.degree());
        if self.is_zero() {
            (Self::zero(1), Self::empty())
        } else if b.is_zero() {
            panic!("Division by zero polynomial");
        } else if a_degree < b_degree {
            (Self::zero(1), self.clone())
        } else if b_degree == 0 {
            (self.scalar_mul(b[0].inverse()), Self::empty())
        } else {
            let rev_b = b.rev();
            let rev_b_inv = rev_b.inv_mod_xn(a_degree - b_degree + 1);
            let rev_q: Polynomial<F> = rev_b_inv
                .mul(&self.rev()[..=a_degree - b_degree].to_vec().into())[..=a_degree - b_degree]
                .to_vec()
                .into();
            let mut q = rev_q.rev();
            let mut qb = q.mul(b);
            qb.pad(self.len());
            let mut r = self.add(&qb.neg());
            q.trim();
            r.trim();
            (q, r)
        }
    }

    // Divides a polynomial `a` by `Z_H = X^n - 1`. Assumes `Z_H | a`, otherwise result is meaningless.
    pub fn divide_by_z_h(&self, n: usize) -> Self {
        if self.is_zero() {
            return self.clone();
        }
        let mut a_trim = self.clone();
        a_trim.trim();
        let g = F::MULTIPLICATIVE_GROUP_GENERATOR;
        let mut g_pow = F::ONE;
        // Multiply the i-th coefficient of `a` by `g^i`. Then `new_a(w^j) = old_a(g.w^j)`.
        a_trim.iter_mut().for_each(|x| {
            *x = (*x) * g_pow;
            g_pow = g * g_pow;
        });
        let d = a_trim.degree();
        let root = F::primitive_root_of_unity(log2_ceil(d + 1));
        let precomputation = fft_precompute(d + 1);
        // Equals to the evaluation of `a` on `{g.w^i}`.
        let mut a_eval = a_trim.eval_domain(&precomputation);
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
            .iter_mut()
            .zip(denominators_inv.iter())
            .for_each(|(x, &d)| {
                *x = (*x) * d;
            });
        // `p` is the interpolating polynomial of `a_eval` on `{w^i}`.
        let mut p = Self::from_evaluations(&a_eval, &precomputation);
        // We need to scale it by `g^(-i)` to get the interpolating polynomial of `a_eval` on `{g.w^i}`,
        // a.k.a `a/Z_H`.
        let g_inv = g.inverse();
        let mut g_inv_pow = F::ONE;
        p.iter_mut().for_each(|x| {
            *x = (*x) * g_inv_pow;
            g_inv_pow = g_inv_pow * g_inv;
        });
        p
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::field::crandall_field::CrandallField;
    use rand::{thread_rng, Rng};
    use std::time::Instant;

    #[test]
    fn test_polynomial_multiplication() {
        type F = CrandallField;
        let mut rng = thread_rng();
        let (a_deg, b_deg) = (rng.gen_range(1, 10_000), rng.gen_range(1, 10_000));
        let a = Polynomial(F::rand_vec(a_deg));
        let b = Polynomial(F::rand_vec(b_deg));
        let m1 = a.mul(&b);
        let m2 = a.mul(&b);
        for _ in 0..1000 {
            let x = F::rand();
            assert_eq!(m1.eval(x), a.eval(x) * b.eval(x));
            assert_eq!(m2.eval(x), a.eval(x) * b.eval(x));
        }
    }

    #[test]
    fn test_inv_mod_xn() {
        type F = CrandallField;
        let mut rng = thread_rng();
        let a_deg = rng.gen_range(1, 1_000);
        let n = rng.gen_range(1, 1_000);
        let a = Polynomial(F::rand_vec(a_deg));
        let b = a.inv_mod_xn(n);
        let mut m = a.mul(&b);
        m.drain(n..);
        m.trim();
        assert_eq!(
            m,
            Polynomial(vec![F::ONE]),
            "a: {:#?}, b:{:#?}, n:{:#?}, m:{:#?}",
            a,
            b,
            n,
            m
        );
    }

    #[test]
    fn test_polynomial_long_division() {
        type F = CrandallField;
        let mut rng = thread_rng();
        let (a_deg, b_deg) = (rng.gen_range(1, 10_000), rng.gen_range(1, 10_000));
        let a = Polynomial(F::rand_vec(a_deg));
        let b = Polynomial(F::rand_vec(b_deg));
        let (q, r) = a.polynomial_long_division(&b);
        for _ in 0..1000 {
            let x = F::rand();
            assert_eq!(a.eval(x), b.eval(x) * q.eval(x) + r.eval(x));
        }
    }

    #[test]
    fn test_polynomial_division() {
        type F = CrandallField;
        let mut rng = thread_rng();
        let (a_deg, b_deg) = (rng.gen_range(1, 10_000), rng.gen_range(1, 10_000));
        let a = Polynomial(F::rand_vec(a_deg));
        let b = Polynomial(F::rand_vec(b_deg));
        let (q, r) = a.polynomial_division(&b);
        for _ in 0..1000 {
            let x = F::rand();
            assert_eq!(a.eval(x), b.eval(x) * q.eval(x) + r.eval(x));
        }
    }

    #[test]
    fn test_polynomial_division_by_constant() {
        type F = CrandallField;
        let mut rng = thread_rng();
        let a_deg = rng.gen_range(1, 10_000);
        let a = Polynomial(F::rand_vec(a_deg));
        let b = Polynomial::from(vec![F::rand()]);
        let (q, r) = a.polynomial_division(&b);
        for _ in 0..1000 {
            let x = F::rand();
            assert_eq!(a.eval(x), b.eval(x) * q.eval(x) + r.eval(x));
        }
    }

    #[test]
    fn test_division_by_z_h() {
        type F = CrandallField;
        let mut rng = thread_rng();
        let a_deg = rng.gen_range(1, 10_000);
        let n = rng.gen_range(1, a_deg);
        let mut a = Polynomial(F::rand_vec(a_deg));
        a.trim();
        let z_h = {
            let mut z_h_vec = vec![F::ZERO; n + 1];
            z_h_vec[n] = F::ONE;
            z_h_vec[0] = F::NEG_ONE;
            Polynomial(z_h_vec)
        };
        let m = a.mul(&z_h);
        let now = Instant::now();
        let mut a_test = m.divide_by_z_h(n);
        a_test.trim();
        println!("Division time: {:?}", now.elapsed());
        assert_eq!(a, a_test);
    }

    #[test]
    fn divide_zero_poly_by_z_h() {
        let zero_poly = Polynomial::<CrandallField>::empty();
        zero_poly.divide_by_z_h(16);
    }

    // Test to see which polynomial division method is faster for divisions of the type
    // `(X^n - 1)/(X - a)
    #[test]
    fn test_division_linear() {
        type F = CrandallField;
        let mut rng = thread_rng();
        let l = 14;
        let n = 1 << l;
        let g = F::primitive_root_of_unity(l);
        let xn_minus_one = {
            let mut xn_min_one_vec = vec![F::ZERO; n + 1];
            xn_min_one_vec[n] = F::ONE;
            xn_min_one_vec[0] = F::NEG_ONE;
            Polynomial(xn_min_one_vec)
        };

        let a = g.exp_usize(rng.gen_range(0, n));
        let denom = Polynomial(vec![-a, F::ONE]);
        let now = Instant::now();
        xn_minus_one.polynomial_division(&denom);
        println!("Division time: {:?}", now.elapsed());
        let now = Instant::now();
        xn_minus_one.polynomial_long_division(&denom);
        println!("Division time: {:?}", now.elapsed());
    }

    #[test]
    fn eq() {
        type F = CrandallField;
        assert_eq!(Polynomial::<F>(vec![]), Polynomial(vec![]));
        assert_eq!(Polynomial::<F>(vec![F::ZERO]), Polynomial(vec![F::ZERO]));
        assert_eq!(Polynomial::<F>(vec![]), Polynomial(vec![F::ZERO]));
        assert_eq!(Polynomial::<F>(vec![F::ZERO]), Polynomial(vec![]));
        assert_eq!(
            Polynomial::<F>(vec![F::ZERO]),
            Polynomial(vec![F::ZERO, F::ZERO])
        );
        assert_eq!(
            Polynomial::<F>(vec![F::ONE]),
            Polynomial(vec![F::ONE, F::ZERO])
        );
        assert_ne!(Polynomial::<F>(vec![]), Polynomial(vec![F::ONE]));
        assert_ne!(
            Polynomial::<F>(vec![F::ZERO]),
            Polynomial(vec![F::ZERO, F::ONE])
        );
        assert_ne!(
            Polynomial::<F>(vec![F::ZERO]),
            Polynomial(vec![F::ONE, F::ZERO])
        );
    }
}
