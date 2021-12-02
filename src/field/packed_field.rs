use std::slice;
use std::fmt::Debug;
use std::iter::{Product, Sum};
use std::ops::{Add, AddAssign, Div, Mul, MulAssign, Neg, Sub, SubAssign};

use crate::field::field_types::Field;

pub unsafe trait PackedField:
    'static
    + Add<Self, Output = Self>
    + Add<Self::Field, Output = Self>
    + AddAssign<Self>
    + AddAssign<Self::Field>
    + Copy
    + Debug
    + Default
    + From<Self::Field>
    // TODO: Implement packed / packed division
    + Div<Self::Field, Output = Self>
    + Mul<Self, Output = Self>
    + Mul<Self::Field, Output = Self>
    + MulAssign<Self>
    + MulAssign<Self::Field>
    + Neg<Output = Self>
    + Product
    + Send
    + Sub<Self, Output = Self>
    + Sub<Self::Field, Output = Self>
    + SubAssign<Self>
    + SubAssign<Self::Field>
    + Sum
    + Sync
where
    Self::Field: Add<Self, Output = Self>,
    Self::Field: Mul<Self, Output = Self>,
    Self::Field: Sub<Self, Output = Self>,
{
    type Field: Field;
    type PrimePackedField: PackedField<Field = <Self::Field as Field>::PrimeField>;

    const WIDTH: usize;
    const ZERO: Self;
    const ONE: Self;

    fn square(&self) -> Self {
        *self * *self
    }

    fn from_arr(arr: [Self::Field; Self::WIDTH]) -> Self;
    fn as_arr(&self) -> [Self::Field; Self::WIDTH];

    fn from_slice(slice: &[Self::Field]) -> &Self;
    fn from_slice_mut(slice: &mut [Self::Field]) -> &mut Self;
    fn as_slice(&self) -> &[Self::Field];
    fn as_slice_mut(&mut self) -> &mut [Self::Field];

    /// Take interpret two vectors as chunks of block_len elements. Unpack and interleave those
    /// chunks. This is best seen with an example. If we have:
    ///     A = [x0, y0, x1, y1],
    ///     B = [x2, y2, x3, y3],
    /// then
    ///     interleave(A, B, 1) = ([x0, x2, x1, x3], [y0, y2, y1, y3]).
    /// Pairs that were adjacent in the input are at corresponding positions in the output.
    ///   r lets us set the size of chunks we're interleaving. If we set block_len = 2, then for
    ///     A = [x0, x1, y0, y1],
    ///     B = [x2, x3, y2, y3],
    /// we obtain
    ///     interleave(A, B, block_len) = ([x0, x1, x2, x3], [y0, y1, y2, y3]).
    ///   We can also think about this as stacking the vectors, dividing them into 2x2 matrices, and
    /// transposing those matrices.
    ///   When block_len = WIDTH, this operation is a no-op. block_len must divide WIDTH. Since
    /// WIDTH is specified to be a power of 2, block_len must also be a power of 2. It cannot be 0
    /// and it cannot be > WIDTH.
    fn interleave(&self, other: Self, block_len: usize) -> (Self, Self);

    fn pack_slice(buf: &[Self::Field]) -> &[Self] {
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
    fn pack_slice_mut(buf: &mut [Self::Field]) -> &mut [Self] {
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

unsafe impl<F: Field> PackedField for F {
    type Field = Self;
    type PrimePackedField = F::PrimeField;

    const WIDTH: usize = 1;
    const ZERO: Self = <F as Field>::ZERO;
    const ONE: Self = <F as Field>::ONE;

    fn square(&self) -> Self {
        <Self as Field>::square(self)
    }

    fn from_arr(arr: [Self::Field; Self::WIDTH]) -> Self {
        arr[0]
    }
    fn as_arr(&self) -> [Self::Field; Self::WIDTH] {
        [*self]
    }

    fn from_slice(slice: &[Self::Field]) -> &Self {
        &slice[0]
    }
    fn from_slice_mut(slice: &mut [Self::Field]) -> &mut Self {
        &mut slice[0]
    }
    fn as_slice(&self) -> &[Self::Field] {
        slice::from_ref(self)
    }
    fn as_slice_mut(&mut self) -> &mut [Self::Field] {
        slice::from_mut(self)
    }

    fn interleave(&self, other: Self, block_len: usize) -> (Self, Self) {
        match block_len {
            1 => (*self, other),
            _ => panic!("unsupported block length"),
        }
    }
}
