use crate::field::fft::{fft, ifft};
use crate::field::field::Field;
use crate::util::log2_strict;
use std::slice::Iter;

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

    pub(crate) fn lde(self, rate_bits: usize) -> Self {
        let original_size = self.len();
        let lde_size = original_size << rate_bits;
        let Self { mut coeffs } = self;
        for _ in 0..(lde_size - original_size) {
            coeffs.push(F::ZERO);
        }
        Self { coeffs }
    }

    /// Removes leading zero coefficients.
    pub fn trim(&mut self) {
        self.coeffs.drain(self.degree_plus_one()..);
    }

    /// Degree of the polynomial + 1.
    fn degree_plus_one(&self) -> usize {
        (0usize..self.len())
            .rev()
            .find(|&i| self.coeffs[i].is_nonzero())
            .map_or(0, |i| i + 1)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::crandall_field::CrandallField;

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
