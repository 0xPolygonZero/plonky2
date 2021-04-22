use crate::field::fft::{fft, ifft};
use crate::field::field::Field;
use crate::util::log2_strict;

/// A polynomial in point-value form. The number of values must be a power of two.
///
/// The points are implicitly `g^i`, where `g` generates the subgroup whose size equals the number
/// of points.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolynomialValues<F: Field> {
    pub values: Vec<F>,
}

impl<F: Field> PolynomialValues<F> {
    pub fn new(values: Vec<F>) -> Self {
        assert!(values.len().is_power_of_two());
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
        let mut coeffs = ifft(self).lde(rate_bits);
        fft(coeffs)
    }
}

/// A polynomial in coefficient form. The number of coefficients must be a power of two.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolynomialCoeffs<F: Field> {
    pub(crate) coeffs: Vec<F>,
}

impl<F: Field> PolynomialCoeffs<F> {
    pub fn new(coeffs: Vec<F>) -> Self {
        assert!(coeffs.len().is_power_of_two());
        PolynomialCoeffs { coeffs }
    }

    pub(crate) fn pad(mut coeffs: Vec<F>) -> Self {
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
        assert!(chunk_size.is_power_of_two());
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

    pub(crate) fn lde(mut self, rate_bits: usize) -> Self {
        let original_size = self.len();
        let lde_size = original_size << rate_bits;
        let Self { mut coeffs } = self;
        for _ in 0..(lde_size - original_size) {
            coeffs.push(F::ZERO);
        }
        Self { coeffs }
    }
}
