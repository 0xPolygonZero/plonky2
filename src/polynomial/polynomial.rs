use std::cmp::max;
use std::ops::{Add, Mul, Sub};

use crate::field::fft::{fft, ifft};
use crate::field::field::Field;
use crate::polynomial::old_polynomial::Polynomial;
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

    pub fn ifft(self) -> PolynomialCoeffs<F> {
        ifft(self)
    }
    pub fn lde_multiple(polys: Vec<Self>, rate_bits: usize) -> Vec<Self> {
        polys.into_iter().map(|p| p.lde(rate_bits)).collect()
    }

    pub fn lde(self, rate_bits: usize) -> Self {
        let coeffs = ifft(self).lde(rate_bits);
        fft(coeffs)
    }
}

impl<F: Field> From<Vec<F>> for PolynomialValues<F> {
    fn from(values: Vec<F>) -> Self {
        Self::new(values)
    }
}

/// A polynomial in coefficient form.
#[derive(Clone, Debug, Eq, PartialEq)]
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
    pub(crate) fn len(&self) -> usize {
        self.coeffs.len()
    }

    pub(crate) fn log_len(&self) -> usize {
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

    pub fn lde_multiple(polys: Vec<Self>, rate_bits: usize) -> Vec<Self> {
        polys.into_iter().map(|p| p.lde(rate_bits)).collect()
    }

    pub(crate) fn lde(&self, rate_bits: usize) -> Self {
        self.padded(self.len() << rate_bits)
    }

    pub(crate) fn padded(&self, new_len: usize) -> Self {
        let mut coeffs = self.coeffs.clone();
        coeffs.resize(new_len, F::ZERO);
        Self { coeffs }
    }

    /// Removes leading zero coefficients.
    pub fn trim(&mut self) {
        self.coeffs.drain(self.degree_plus_one()..);
    }

    /// Degree of the polynomial + 1.
    pub(crate) fn degree_plus_one(&self) -> usize {
        (0usize..self.len())
            .rev()
            .find(|&i| self.coeffs[i].is_nonzero())
            .map_or(0, |i| i + 1)
    }

    pub(crate) fn div_rem(&self, rhs: &Self) -> (Self, Self) {
        let lhs = Polynomial::from(self.clone());
        let rhs = Polynomial::from(rhs.clone());
        let (q, r) = lhs.polynomial_division(&rhs);
        (q.into(), r.into())
    }

    pub fn fft(self) -> PolynomialValues<F> {
        fft(self)
    }

    pub fn coset_fft(self, shift: F) -> PolynomialValues<F> {
        let modified_poly: Self = shift
            .powers()
            .zip(self.coeffs)
            .map(|(r, c)| r * c)
            .collect::<Vec<_>>()
            .into();
        modified_poly.fft()
    }
}

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

impl<F: Field> Sub for &PolynomialCoeffs<F> {
    type Output = PolynomialCoeffs<F>;

    fn sub(self, rhs: Self) -> Self::Output {
        let len = max(self.len(), rhs.len());
        let mut coeffs = self.coeffs.clone();
        coeffs.resize(len, F::ZERO);
        for (i, &c) in rhs.coeffs.iter().enumerate() {
            coeffs[i] -= c;
        }
        PolynomialCoeffs::new(coeffs)
    }
}

impl<F: Field> Mul<F> for &PolynomialCoeffs<F> {
    type Output = PolynomialCoeffs<F>;

    fn mul(self, rhs: F) -> Self::Output {
        let coeffs = self.coeffs.iter().map(|&x| rhs * x).collect();
        PolynomialCoeffs::new(coeffs)
    }
}

impl<F: Field> Mul for &PolynomialCoeffs<F> {
    type Output = PolynomialCoeffs<F>;

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
        ifft(mul_evals.into())
    }
}

#[cfg(test)]
mod tests {
    use crate::field::crandall_field::CrandallField;

    use super::*;

    #[test]
    fn test_coset_fft() {
        type F = CrandallField;

        let k = 8;
        let n = 1 << k;
        let poly = PolynomialCoeffs::new(F::rand_vec(n));
        let shift = F::rand();
        let coset_evals = poly.clone().coset_fft(shift).values;

        let generator = F::primitive_root_of_unity(k);
        let naive_coset_evals = F::cyclic_subgroup_coset_known_order(generator, shift, n)
            .into_iter()
            .map(|x| poly.eval(x))
            .collect::<Vec<_>>();

        assert_eq!(coset_evals, naive_coset_evals);
    }
}
