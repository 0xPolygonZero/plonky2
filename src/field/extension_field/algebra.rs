use crate::field::extension_field::OEF;
use std::fmt::{Debug, Display, Formatter};
use std::iter::{Product, Sum};
use std::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};

/// Let `F_D` be the optimal extension field `F[X]/(X^D-W)`. Then `ExtensionAlgebra<F_D>` is the quotient `F_D[X]/(X^D-W)`.
/// It's a `D`-dimensional algebra over `F_D` useful to lift the multiplication over `F_D` to a multiplication over `(F_D)^D`.
#[derive(Copy, Clone)]
pub struct ExtensionAlgebra<F: OEF<D>, const D: usize>([F; D]);

impl<F: OEF<D>, const D: usize> ExtensionAlgebra<F, D> {
    pub const ZERO: Self = Self([F::ZERO; D]);

    pub fn one() -> Self {
        F::ONE.into()
    }

    pub fn from_basefield_array(arr: [F; D]) -> Self {
        Self(arr)
    }

    pub fn to_basefield_array(self) -> [F; D] {
        self.0
    }
}

impl<F: OEF<D>, const D: usize> From<F> for ExtensionAlgebra<F, D> {
    fn from(x: F) -> Self {
        let mut arr = [F::ZERO; D];
        arr[0] = x;
        Self(arr)
    }
}

impl<F: OEF<D>, const D: usize> Display for ExtensionAlgebra<F, D> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}) + ", self.0[0])?;
        for i in 1..D - 1 {
            write!(f, "({})*b^{} + ", self.0[i], i)?;
        }
        write!(f, "({})*b^{}", self.0[D - 1], D - 1)
    }
}

impl<F: OEF<D>, const D: usize> Debug for ExtensionAlgebra<F, D> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl<F: OEF<D>, const D: usize> Neg for ExtensionAlgebra<F, D> {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        let mut arr = self.0;
        arr.iter_mut().for_each(|x| *x = -*x);
        Self(arr)
    }
}

impl<F: OEF<D>, const D: usize> Add for ExtensionAlgebra<F, D> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self {
        let mut arr = self.0;
        arr.iter_mut().zip(&rhs.0).for_each(|(x, &y)| *x += y);
        Self(arr)
    }
}

impl<F: OEF<D>, const D: usize> AddAssign for ExtensionAlgebra<F, D> {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl<F: OEF<D>, const D: usize> Sum for ExtensionAlgebra<F, D> {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ZERO, |acc, x| acc + x)
    }
}

impl<F: OEF<D>, const D: usize> Sub for ExtensionAlgebra<F, D> {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self {
        let mut arr = self.0;
        arr.iter_mut().zip(&rhs.0).for_each(|(x, &y)| *x -= y);
        Self(arr)
    }
}

impl<F: OEF<D>, const D: usize> SubAssign for ExtensionAlgebra<F, D> {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl<F: OEF<D>, const D: usize> Mul for ExtensionAlgebra<F, D> {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Self) -> Self {
        let mut res = [F::ZERO; D];
        let w = F::from_basefield(F::W);
        for i in 0..D {
            for j in 0..D {
                res[(i + j) % D] += if i + j < D {
                    self.0[i] * rhs.0[j]
                } else {
                    w * self.0[i] * rhs.0[j]
                }
            }
        }
        Self(res)
    }
}

impl<F: OEF<D>, const D: usize> MulAssign for ExtensionAlgebra<F, D> {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl<F: OEF<D>, const D: usize> Product for ExtensionAlgebra<F, D> {
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::one(), |acc, x| acc * x)
    }
}

/// A polynomial in coefficient form.
#[derive(Clone, Debug)]
pub struct PolynomialCoeffsAlgebra<F: OEF<D>, const D: usize> {
    pub(crate) coeffs: Vec<ExtensionAlgebra<F, D>>,
}

impl<F: OEF<D>, const D: usize> PolynomialCoeffsAlgebra<F, D> {
    pub fn new(coeffs: Vec<ExtensionAlgebra<F, D>>) -> Self {
        PolynomialCoeffsAlgebra { coeffs }
    }

    pub fn eval(&self, x: ExtensionAlgebra<F, D>) -> ExtensionAlgebra<F, D> {
        self.coeffs
            .iter()
            .rev()
            .fold(ExtensionAlgebra::ZERO, |acc, &c| acc * x + c)
    }
}
