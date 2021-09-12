use std::fmt;
use std::fmt::{Debug, Formatter};
use std::iter::{Product, Sum};
use std::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use crate::field::field_types::Field;

pub trait PackedField:
    'static
    + Add<Self, Output = Self>
    + Add<Self::FieldType, Output = Self>
    + AddAssign<Self>
    + AddAssign<Self::FieldType>
    + Copy
    + Debug
    + Default
    // TODO: Implementing Div sounds like a pain so it's a worry for later.
    + Mul<Self, Output = Self>
    + Mul<Self::FieldType, Output = Self>
    + MulAssign<Self>
    + MulAssign<Self::FieldType>
    + Neg<Output = Self>
    + Product
    + Send
    + Sub<Self, Output = Self>
    + Sub<Self::FieldType, Output = Self>
    + SubAssign<Self>
    + SubAssign<Self::FieldType>
    + Sum
    + Sync
{
    type FieldType: Field;

    const LOG2_WIDTH: usize;
    const WIDTH: usize = 1 << Self::LOG2_WIDTH;

    fn square(&self) -> Self {
        *self * *self
    }

    fn zero() -> Self {
        Self::broadcast(Self::FieldType::ZERO)
    }
    fn one() -> Self {
        Self::broadcast(Self::FieldType::ONE)
    }

    fn broadcast(x: Self::FieldType) -> Self;

    fn from_arr(arr: [Self::FieldType; Self::WIDTH]) -> Self;
    fn to_arr(&self) -> [Self::FieldType; Self::WIDTH];

    /// Take interpret two vectors as chunks of (1 << r) elements. Unpack and interleave those
    /// chunks. This is best seen with an example. If we have:
    ///     A = [x0, y0, x1, y1],
    ///     B = [x2, y2, x3, y3],
    /// then
    ///     interleave(A, B, 0) = ([x0, x2, x1, x3], [y0, y2, y1, y3]).
    /// Pairs that were adjacent in the input are at corresponding positions in the output.
    ///   r lets us set the size of chunks we're interleaving. If we set r = 1, then for
    ///     A = [x0, x1, y0, y1],
    ///     B = [x2, x3, y2, y3],
    /// we obtain
    ///     interleave(A, B, r) = ([x0, x1, x2, x3], [y0, y1, y2, y3]).
    ///   We can also think about this as stacking the vectors, dividing them into 2x2 matrices, and
    /// transposing those matrices.
    ///   When r = LOG2_WIDTH, this operation is a no-op. Values of r > LOG2_WIDTH are not
    /// permitted.
    fn interleave(&self, other: Self, r: usize) -> (Self, Self);

    fn pack_slice(buf: &[Self::FieldType]) -> &[Self] {
        assert!(
            buf.len() % Self::WIDTH == 0,
            "Slice length (got {}) must be a multiple of packed field width ({}).",
            buf.len(),
            Self::WIDTH
        );
        let buf_ptr = buf.as_ptr().cast::<Self>();
        let n = buf.len() / Self::WIDTH;
        unsafe { std::slice::from_raw_parts(buf_ptr, n) }
    }
    fn pack_slice_mut(buf: &mut [Self::FieldType]) -> &mut [Self] {
        assert!(
            buf.len() % Self::WIDTH == 0,
            "Slice length (got {}) must be a multiple of packed field width ({}).",
            buf.len(),
            Self::WIDTH
        );
        let buf_ptr = buf.as_mut_ptr().cast::<Self>();
        let n = buf.len() / Self::WIDTH;
        unsafe { std::slice::from_raw_parts_mut(buf_ptr, n) }
    }
}

#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct Singleton<F: Field>(pub F);

impl<F: Field> Add<Self> for Singleton<F> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}
impl<F: Field> Add<F> for Singleton<F> {
    type Output = Self;
    fn add(self, rhs: F) -> Self {
        self + Self::broadcast(rhs)
    }
}
impl<F: Field> AddAssign<Self> for Singleton<F> {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}
impl<F: Field> AddAssign<F> for Singleton<F> {
    fn add_assign(&mut self, rhs: F) {
        *self = *self + rhs;
    }
}

impl<F: Field> Debug for Singleton<F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "({:?})", self.0)
    }
}

impl<F: Field> Default for Singleton<F> {
    fn default() -> Self {
        Self::zero()
    }
}

impl<F: Field> Mul<Self> for Singleton<F> {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        Self(self.0 * rhs.0)
    }
}
impl<F: Field> Mul<F> for Singleton<F> {
    type Output = Self;
    fn mul(self, rhs: F) -> Self {
        self * Self::broadcast(rhs)
    }
}
impl<F: Field> MulAssign<Self> for Singleton<F> {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}
impl<F: Field> MulAssign<F> for Singleton<F> {
    fn mul_assign(&mut self, rhs: F) {
        *self = *self * rhs;
    }
}

impl<F: Field> Neg for Singleton<F> {
    type Output = Self;
    fn neg(self) -> Self {
        Self(-self.0)
    }
}

impl<F: Field> Product for Singleton<F> {
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        Self(iter.map(|x| x.0).product())
    }
}

impl<F: Field> PackedField for Singleton<F> {
    const LOG2_WIDTH: usize = 0;
    type FieldType = F;

    fn broadcast(x: F) -> Self {
        Self(x)
    }

    fn from_arr(arr: [Self::FieldType; Self::WIDTH]) -> Self {
        Self(arr[0])
    }

    fn to_arr(&self) -> [Self::FieldType; Self::WIDTH] {
        [self.0]
    }

    fn interleave(&self, other: Self, r: usize) -> (Self, Self) {
        match r {
            0 => (*self, other), // This is a no-op whenever r == LOG2_WIDTH.
            _ => panic!("r cannot be more than LOG2_WIDTH"),
        }
    }

    fn square(&self) -> Self {
        Self(self.0.square())
    }
}

impl<F: Field> Sub<Self> for Singleton<F> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0)
    }
}
impl<F: Field> Sub<F> for Singleton<F> {
    type Output = Self;
    fn sub(self, rhs: F) -> Self {
        self - Self::broadcast(rhs)
    }
}
impl<F: Field> SubAssign<Self> for Singleton<F> {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}
impl<F: Field> SubAssign<F> for Singleton<F> {
    fn sub_assign(&mut self, rhs: F) {
        *self = *self - rhs;
    }
}

impl<F: Field> Sum for Singleton<F> {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Self(iter.map(|x| x.0).sum())
    }
}
