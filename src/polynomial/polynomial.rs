use std::cmp::max;
use std::iter::Sum;
use std::ops::{Add, AddAssign, Mul, MulAssign, Sub, SubAssign};

use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};

use crate::field::extension_field::{Extendable, FieldExtension};
use crate::field::fft::{fft, fft_with_options, ifft};
use crate::field::fft::{FftRootTable, FftStrategy, DEFAULT_STRATEGY};
use crate::field::field_types::Field;
use crate::util::log2_strict;

/// A polynomial in point-value form.
///
/// The points are implicitly `g^i`, where `g` generates the subgroup whose size equals the number
/// of points.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolynomialValues<F: Field> {
    pub values: Vec<F>,
}

impl<F: Field> PolynomialValues<F> {
    pub fn new(values: Vec<F>) -> Self {
        PolynomialValues { values }
    }

    pub(crate) fn zero(len: usize) -> Self {
        Self::new(vec![F::ZERO; len])
    }

    /// The number of values stored.
    pub(crate) fn len(&self) -> usize {
        self.values.len()
    }

    pub fn ifft(&self) -> PolynomialCoeffs<F> {
        ifft(self)
    }

    /// Returns the polynomial whose evaluation on the coset `shift*H` is `self`.
    pub fn coset_ifft(&self, shift: F) -> PolynomialCoeffs<F> {
        let mut shifted_coeffs = self.ifft();
        shifted_coeffs
            .coeffs
            .iter_mut()
            .zip(shift.inverse().powers())
            .for_each(|(c, r)| {
                *c *= r;
            });
        shifted_coeffs
    }

    pub fn lde_multiple(polys: Vec<Self>, rate_bits: usize) -> Vec<Self> {
        polys.into_iter().map(|p| p.lde(rate_bits)).collect()
    }

    pub fn lde(&self, rate_bits: usize) -> Self {
        let coeffs = ifft(self).lde(rate_bits);
        fft_with_options(&coeffs, DEFAULT_STRATEGY, Some(rate_bits), None)
    }

    pub fn degree(&self) -> usize {
        self.degree_plus_one()
            .checked_sub(1)
            .expect("deg(0) is undefined")
    }

    pub fn degree_plus_one(&self) -> usize {
        self.ifft().degree_plus_one()
    }
}

impl<F: Field> From<Vec<F>> for PolynomialValues<F> {
    fn from(values: Vec<F>) -> Self {
        Self::new(values)
    }
}

/// A polynomial in coefficient form.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct PolynomialCoeffs<F: Field> {
    pub(crate) coeffs: Vec<F>,
}

impl<F: Field> PolynomialCoeffs<F> {
    pub fn new(coeffs: Vec<F>) -> Self {
        PolynomialCoeffs { coeffs }
    }

    /// Create a new polynomial with its coefficient list padded to the next power of two.
    pub(crate) fn new_padded(mut coeffs: Vec<F>) -> Self {
        while !coeffs.len().is_power_of_two() {
            coeffs.push(F::ZERO);
        }
        PolynomialCoeffs { coeffs }
    }

    pub(crate) fn empty() -> Self {
        Self::new(Vec::new())
    }

    pub(crate) fn zero(len: usize) -> Self {
        Self::new(vec![F::ZERO; len])
    }

    pub(crate) fn one() -> Self {
        Self::new(vec![F::ONE])
    }

    pub(crate) fn is_zero(&self) -> bool {
        self.coeffs.iter().all(|x| x.is_zero())
    }

    /// The number of coefficients. This does not filter out any zero coefficients, so it is not
    /// necessarily related to the degree.
    pub fn len(&self) -> usize {
        self.coeffs.len()
    }

    pub fn log_len(&self) -> usize {
        log2_strict(self.len())
    }

    pub(crate) fn chunks(&self, chunk_size: usize) -> Vec<Self> {
        self.coeffs
            .chunks(chunk_size)
            .map(|chunk| PolynomialCoeffs::new(chunk.to_vec()))
            .collect()
    }

    pub fn eval(&self, x: F) -> F {
        self.coeffs
            .iter()
            .rev()
            .fold(F::ZERO, |acc, &c| acc * x + c)
    }

    pub fn eval_base<const D: usize>(&self, x: F::BaseField) -> F
    where
        F: FieldExtension<D>,
    {
        self.coeffs
            .iter()
            .rev()
            .fold(F::ZERO, |acc, &c| acc.scalar_mul(x) + c)
    }

    pub fn lde_multiple(polys: Vec<&Self>, rate_bits: usize) -> Vec<Self> {
        polys.into_iter().map(|p| p.lde(rate_bits)).collect()
    }

    pub fn lde(&self, rate_bits: usize) -> Self {
        self.padded(self.len() << rate_bits)
    }

    pub(crate) fn pad(&mut self, new_len: usize) -> Result<()> {
        ensure!(
            new_len >= self.len(),
            "Trying to pad a polynomial of length {} to a length of {}.",
            self.len(),
            new_len
        );
        self.coeffs.resize(new_len, F::ZERO);
        Ok(())
    }

    pub(crate) fn padded(&self, new_len: usize) -> Self {
        let mut poly = self.clone();
        poly.pad(new_len).unwrap();
        poly
    }

    /// Removes leading zero coefficients.
    pub fn trim(&mut self) {
        self.coeffs.truncate(self.degree_plus_one());
    }

    /// Removes leading zero coefficients.
    pub fn trimmed(&self) -> Self {
        let coeffs = self.coeffs[..self.degree_plus_one()].to_vec();
        Self { coeffs }
    }

    /// Degree of the polynomial + 1, or 0 for a polynomial with no non-zero coefficients.
    pub(crate) fn degree_plus_one(&self) -> usize {
        (0usize..self.len())
            .rev()
            .find(|&i| self.coeffs[i].is_nonzero())
            .map_or(0, |i| i + 1)
    }

    /// Leading coefficient.
    pub fn lead(&self) -> F {
        self.coeffs
            .iter()
            .rev()
            .find(|x| x.is_nonzero())
            .map_or(F::ZERO, |x| *x)
    }

    /// Reverse the order of the coefficients, not taking into account the leading zero coefficients.
    pub(crate) fn rev(&self) -> Self {
        Self::new(self.trimmed().coeffs.into_iter().rev().collect())
    }

    pub fn fft(&self) -> PolynomialValues<F> {
        fft(self)
    }

    pub fn fft_with_options(
        &self,
        strategy: FftStrategy,
        zero_factor: Option<usize>,
        root_table: Option<FftRootTable<F>>,
    ) -> PolynomialValues<F> {
        fft_with_options(self, strategy, zero_factor, root_table)
    }

    /// Returns the evaluation of the polynomial on the coset `shift*H`.
    pub fn coset_fft(&self, shift: F) -> PolynomialValues<F> {
        self.coset_fft_with_options(shift, DEFAULT_STRATEGY, None, None)
    }

    /// Returns the evaluation of the polynomial on the coset `shift*H`.
    pub fn coset_fft_with_options(
        &self,
        shift: F,
        strategy: FftStrategy,
        zero_factor: Option<usize>,
        root_table: Option<FftRootTable<F>>,
    ) -> PolynomialValues<F> {
        let modified_poly: Self = shift
            .powers()
            .zip(&self.coeffs)
            .map(|(r, &c)| r * c)
            .collect::<Vec<_>>()
            .into();
        modified_poly.fft_with_options(strategy, zero_factor, root_table)
    }

    pub fn to_extension<const D: usize>(&self) -> PolynomialCoeffs<F::Extension>
    where
        F: Extendable<D>,
    {
        PolynomialCoeffs::new(self.coeffs.iter().map(|&c| c.into()).collect())
    }

    pub fn mul_extension<const D: usize>(&self, rhs: F::Extension) -> PolynomialCoeffs<F::Extension>
    where
        F: Extendable<D>,
    {
        PolynomialCoeffs::new(self.coeffs.iter().map(|&c| rhs.scalar_mul(c)).collect())
    }
}

impl<F: Field> PartialEq for PolynomialCoeffs<F> {
    fn eq(&self, other: &Self) -> bool {
        let max_terms = self.coeffs.len().max(other.coeffs.len());
        for i in 0..max_terms {
            let self_i = self.coeffs.get(i).cloned().unwrap_or(F::ZERO);
            let other_i = other.coeffs.get(i).cloned().unwrap_or(F::ZERO);
            if self_i != other_i {
                return false;
            }
        }
        true
    }
}

impl<F: Field> Eq for PolynomialCoeffs<F> {}

impl<F: Field> From<Vec<F>> for PolynomialCoeffs<F> {
    fn from(coeffs: Vec<F>) -> Self {
        Self::new(coeffs)
    }
}

impl<F: Field> Add for &PolynomialCoeffs<F> {
    type Output = PolynomialCoeffs<F>;

    fn add(self, rhs: Self) -> Self::Output {
        let len = max(self.len(), rhs.len());
        let a = self.padded(len).coeffs;
        let b = rhs.padded(len).coeffs;
        let coeffs = a.into_iter().zip(b).map(|(x, y)| x + y).collect();
        PolynomialCoeffs::new(coeffs)
    }
}

impl<F: Field> Sum for PolynomialCoeffs<F> {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::empty(), |acc, p| &acc + &p)
    }
}

impl<F: Field> Sub for &PolynomialCoeffs<F> {
    type Output = PolynomialCoeffs<F>;

    fn sub(self, rhs: Self) -> Self::Output {
        let len = max(self.len(), rhs.len());
        let mut coeffs = self.padded(len).coeffs;
        for (i, &c) in rhs.coeffs.iter().enumerate() {
            coeffs[i] -= c;
        }
        PolynomialCoeffs::new(coeffs)
    }
}

impl<F: Field> AddAssign for PolynomialCoeffs<F> {
    fn add_assign(&mut self, rhs: Self) {
        let len = max(self.len(), rhs.len());
        self.coeffs.resize(len, F::ZERO);
        for (l, r) in self.coeffs.iter_mut().zip(rhs.coeffs) {
            *l += r;
        }
    }
}

impl<F: Field> AddAssign<&Self> for PolynomialCoeffs<F> {
    fn add_assign(&mut self, rhs: &Self) {
        let len = max(self.len(), rhs.len());
        self.coeffs.resize(len, F::ZERO);
        for (l, &r) in self.coeffs.iter_mut().zip(&rhs.coeffs) {
            *l += r;
        }
    }
}

impl<F: Field> SubAssign for PolynomialCoeffs<F> {
    fn sub_assign(&mut self, rhs: Self) {
        let len = max(self.len(), rhs.len());
        self.coeffs.resize(len, F::ZERO);
        for (l, r) in self.coeffs.iter_mut().zip(rhs.coeffs) {
            *l -= r;
        }
    }
}

impl<F: Field> SubAssign<&Self> for PolynomialCoeffs<F> {
    fn sub_assign(&mut self, rhs: &Self) {
        let len = max(self.len(), rhs.len());
        self.coeffs.resize(len, F::ZERO);
        for (l, &r) in self.coeffs.iter_mut().zip(&rhs.coeffs) {
            *l -= r;
        }
    }
}

impl<F: Field> Mul<F> for &PolynomialCoeffs<F> {
    type Output = PolynomialCoeffs<F>;

    fn mul(self, rhs: F) -> Self::Output {
        let coeffs = self.coeffs.iter().map(|&x| rhs * x).collect();
        PolynomialCoeffs::new(coeffs)
    }
}

impl<F: Field> MulAssign<F> for PolynomialCoeffs<F> {
    fn mul_assign(&mut self, rhs: F) {
        self.coeffs.iter_mut().for_each(|x| *x *= rhs);
    }
}

impl<F: Field> Mul for &PolynomialCoeffs<F> {
    type Output = PolynomialCoeffs<F>;

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn mul(self, rhs: Self) -> Self::Output {
        let new_len = (self.len() + rhs.len()).next_power_of_two();
        let a = self.padded(new_len);
        let b = rhs.padded(new_len);
        let a_evals = a.fft();
        let b_evals = b.fft();

        let mul_evals: Vec<F> = a_evals
            .values
            .into_iter()
            .zip(b_evals.values)
            .map(|(pa, pb)| pa * pb)
            .collect();
        ifft(&mul_evals.into())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use rand::{thread_rng, Rng};

    use super::*;
    use crate::field::crandall_field::CrandallField;

    #[test]
    fn test_trimmed() {
        type F = CrandallField;

        assert_eq!(
            PolynomialCoeffs::<F> { coeffs: vec![] }.trimmed(),
            PolynomialCoeffs::<F> { coeffs: vec![] }
        );
        assert_eq!(
            PolynomialCoeffs::<F> {
                coeffs: vec![F::ZERO]
            }
            .trimmed(),
            PolynomialCoeffs::<F> { coeffs: vec![] }
        );
        assert_eq!(
            PolynomialCoeffs::<F> {
                coeffs: vec![F::ONE, F::TWO, F::ZERO, F::ZERO]
            }
            .trimmed(),
            PolynomialCoeffs::<F> {
                coeffs: vec![F::ONE, F::TWO]
            }
        );
    }

    #[test]
    fn test_coset_fft() {
        type F = CrandallField;

        let k = 8;
        let n = 1 << k;
        let poly = PolynomialCoeffs::new(F::rand_vec(n));
        let shift = F::rand();
        let coset_evals = poly.coset_fft(shift).values;

        let generator = F::primitive_root_of_unity(k);
        let naive_coset_evals = F::cyclic_subgroup_coset_known_order(generator, shift, n)
            .into_iter()
            .map(|x| poly.eval(x))
            .collect::<Vec<_>>();
        assert_eq!(coset_evals, naive_coset_evals);

        let ifft_coeffs = PolynomialValues::new(coset_evals).coset_ifft(shift);
        assert_eq!(poly, ifft_coeffs.into());
    }

    #[test]
    fn test_coset_ifft() {
        type F = CrandallField;

        let k = 8;
        let n = 1 << k;
        let evals = PolynomialValues::new(F::rand_vec(n));
        let shift = F::rand();
        let coeffs = evals.coset_ifft(shift);

        let generator = F::primitive_root_of_unity(k);
        let naive_coset_evals = F::cyclic_subgroup_coset_known_order(generator, shift, n)
            .into_iter()
            .map(|x| coeffs.eval(x))
            .collect::<Vec<_>>();
        assert_eq!(evals, naive_coset_evals.into());

        let fft_evals = coeffs.coset_fft(shift);
        assert_eq!(evals, fft_evals);
    }

    #[test]
    fn test_polynomial_multiplication() {
        type F = CrandallField;
        let mut rng = thread_rng();
        let (a_deg, b_deg) = (rng.gen_range(1..10_000), rng.gen_range(1..10_000));
        let a = PolynomialCoeffs::new(F::rand_vec(a_deg));
        let b = PolynomialCoeffs::new(F::rand_vec(b_deg));
        let m1 = &a * &b;
        let m2 = &a * &b;
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
        let a_deg = rng.gen_range(1..1_000);
        let n = rng.gen_range(1..1_000);
        let a = PolynomialCoeffs::new(F::rand_vec(a_deg));
        let b = a.inv_mod_xn(n);
        let mut m = &a * &b;
        m.coeffs.drain(n..);
        m.trim();
        assert_eq!(
            m,
            PolynomialCoeffs::new(vec![F::ONE]),
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
        let (a_deg, b_deg) = (rng.gen_range(1..10_000), rng.gen_range(1..10_000));
        let a = PolynomialCoeffs::new(F::rand_vec(a_deg));
        let b = PolynomialCoeffs::new(F::rand_vec(b_deg));
        let (q, r) = a.div_rem_long_division(&b);
        for _ in 0..1000 {
            let x = F::rand();
            assert_eq!(a.eval(x), b.eval(x) * q.eval(x) + r.eval(x));
        }
    }

    #[test]
    fn test_polynomial_division() {
        type F = CrandallField;
        let mut rng = thread_rng();
        let (a_deg, b_deg) = (rng.gen_range(1..10_000), rng.gen_range(1..10_000));
        let a = PolynomialCoeffs::new(F::rand_vec(a_deg));
        let b = PolynomialCoeffs::new(F::rand_vec(b_deg));
        let (q, r) = a.div_rem(&b);
        for _ in 0..1000 {
            let x = F::rand();
            assert_eq!(a.eval(x), b.eval(x) * q.eval(x) + r.eval(x));
        }
    }

    #[test]
    fn test_polynomial_division_by_constant() {
        type F = CrandallField;
        let mut rng = thread_rng();
        let a_deg = rng.gen_range(1..10_000);
        let a = PolynomialCoeffs::new(F::rand_vec(a_deg));
        let b = PolynomialCoeffs::from(vec![F::rand()]);
        let (q, r) = a.div_rem(&b);
        for _ in 0..1000 {
            let x = F::rand();
            assert_eq!(a.eval(x), b.eval(x) * q.eval(x) + r.eval(x));
        }
    }

    #[test]
    fn test_division_by_z_h() {
        type F = CrandallField;
        let mut rng = thread_rng();
        let a_deg = rng.gen_range(1..10_000);
        let n = rng.gen_range(1..a_deg);
        let mut a = PolynomialCoeffs::new(F::rand_vec(a_deg));
        a.trim();
        let z_h = {
            let mut z_h_vec = vec![F::ZERO; n + 1];
            z_h_vec[n] = F::ONE;
            z_h_vec[0] = F::NEG_ONE;
            PolynomialCoeffs::new(z_h_vec)
        };
        let m = &a * &z_h;
        let now = Instant::now();
        let mut a_test = m.divide_by_z_h(n);
        a_test.trim();
        println!("Division time: {:?}", now.elapsed());
        assert_eq!(a, a_test);
    }

    #[test]
    fn divide_zero_poly_by_z_h() {
        let zero_poly = PolynomialCoeffs::<CrandallField>::empty();
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
            PolynomialCoeffs::new(xn_min_one_vec)
        };

        let a = g.exp_u64(rng.gen_range(0..(n as u64)));
        let denom = PolynomialCoeffs::new(vec![-a, F::ONE]);
        let now = Instant::now();
        xn_minus_one.div_rem(&denom);
        println!("Division time: {:?}", now.elapsed());
        let now = Instant::now();
        xn_minus_one.div_rem_long_division(&denom);
        println!("Division time: {:?}", now.elapsed());
    }

    #[test]
    fn eq() {
        type F = CrandallField;
        assert_eq!(
            PolynomialCoeffs::<F>::new(vec![]),
            PolynomialCoeffs::new(vec![])
        );
        assert_eq!(
            PolynomialCoeffs::<F>::new(vec![F::ZERO]),
            PolynomialCoeffs::new(vec![F::ZERO])
        );
        assert_eq!(
            PolynomialCoeffs::<F>::new(vec![]),
            PolynomialCoeffs::new(vec![F::ZERO])
        );
        assert_eq!(
            PolynomialCoeffs::<F>::new(vec![F::ZERO]),
            PolynomialCoeffs::new(vec![])
        );
        assert_eq!(
            PolynomialCoeffs::<F>::new(vec![F::ZERO]),
            PolynomialCoeffs::new(vec![F::ZERO, F::ZERO])
        );
        assert_eq!(
            PolynomialCoeffs::<F>::new(vec![F::ONE]),
            PolynomialCoeffs::new(vec![F::ONE, F::ZERO])
        );
        assert_ne!(
            PolynomialCoeffs::<F>::new(vec![]),
            PolynomialCoeffs::new(vec![F::ONE])
        );
        assert_ne!(
            PolynomialCoeffs::<F>::new(vec![F::ZERO]),
            PolynomialCoeffs::new(vec![F::ZERO, F::ONE])
        );
        assert_ne!(
            PolynomialCoeffs::<F>::new(vec![F::ZERO]),
            PolynomialCoeffs::new(vec![F::ONE, F::ZERO])
        );
    }
}
