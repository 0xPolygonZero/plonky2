use std::fmt::{self, Debug, Formatter};
use std::iter::{Product, Sum};
use std::mem::transmute_copy;
use std::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use crate::field::extension_field::Extendable;
use crate::field::extension_field::quadratic::QuadraticExtension;
use crate::field::packed_field::PackedField;

// The extension field consists of two scalars (a0, a1), which are interpreted as a polynomial
// a1 x + a0. We store them as two PackedFields (of prime FieldType).
//   In memory, extension fields are stored interleaved. E.g. a, b, c, and d would be stored as
// [a0, a1, b0, b1, c0, c1, d0, d1]. We need to be able to load PackedQuadraticExtension directly
// from memory, so store the interleaved form: self.0[0] has [a0, a1, b0, b1] and self.0[1] is
// [c0, c1, d0, d1]. This lets us cast a slice of QuadraticExtension directly to
// PackedQuadraticExtension.
//   Unfortunately, this representation is not very useful for calculations as multiplication has a
// different set of operations for a0 and a1, as well as requiring cross-terms. Before doing
// anything useful, we deinterleave the representation into two vectors, [a0, c0, b0, d0] and
// [a1, c1, b1, d1]. These need to be interleaved again before being stored in the struct. The new
// and get methods help with this. In a chain of operations, LLVM is smart enough to remove
// redundant swizzles.
#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct PackedQuadraticExtension<P: PackedField>
	(pub(crate) [P; 2]) where P::FieldType: Extendable<2>;

impl<P: PackedField> PackedQuadraticExtension<P> where P::FieldType: Extendable<2> {
	fn new(a0: P, a1: P) -> Self {
		let (s0, s1) = a0.interleave(a1, 0);
		Self([s0, s1])
	}
	fn get(&self) -> (P, P) {
		let Self([s0, s1]) = self;
		s0.interleave(*s1, 0)
	}
}

impl<P: PackedField> PackedField for PackedQuadraticExtension<P> where P::FieldType: Extendable<2> {
	type FieldType = QuadraticExtension::<P::FieldType>;
	type PackedPrimeField = P;
	const LOG2_WIDTH: usize = P::LOG2_WIDTH;

	fn broadcast(x: QuadraticExtension::<P::FieldType>) -> Self {
		let QuadraticExtension::<P::FieldType>([x0, x1]) = x;
		let v0 = P::broadcast(x0);
		let v1 = P::broadcast(x1);
		Self::new(v0, v1)
	}

    fn from_arr(arr: [Self::FieldType; Self::WIDTH]) -> Self {
    	unsafe { transmute_copy(&arr) }
    }
    fn to_arr(&self) -> [Self::FieldType; Self::WIDTH] {
    	unsafe { transmute_copy(self) }
    }

    fn from_slice(slice: &[Self::FieldType]) -> Self {
    	let base_slice = unsafe { std::slice::from_raw_parts(slice.as_ptr().cast(), slice.len() * 2) };
    	Self([
    		P::from_slice(&base_slice[..Self::WIDTH]),
    		P::from_slice(&base_slice[Self::WIDTH..]),
    	])
    }
    fn to_vec(&self) -> Vec<QuadraticExtension::<P::FieldType>> {
    	let Self([p0, p1]) = self;
    	let p0_ptr: *const P = p0;
    	let p1_ptr: *const P = p1;
    	let mut res = Vec::with_capacity(Self::WIDTH);
    	res.extend(unsafe { std::slice::from_raw_parts(p0_ptr.cast(), Self::WIDTH / 2) });
    	res.extend(unsafe { std::slice::from_raw_parts(p1_ptr.cast(), Self::WIDTH / 2) });
    	res
    }

    fn interleave(&self, other: Self, r: usize) -> (Self, Self) {
    	let (a0, a1) = self.get();
    	let (b0, b1) = other.get();
    	let (x0, y0) = a0.interleave(b0, r);
    	let (x1, y1) = a1.interleave(b1, r);
    	(Self::new(x0, x1), Self::new(y0, y1))
   	}
}

impl<P: PackedField> Add<Self> for PackedQuadraticExtension<P> where P::FieldType: Extendable<2> {
	type Output = Self;
	#[inline]
	fn add(self, rhs: Self) -> Self {
		let (a0, a1) = self.get();
		let (b0, b1) = rhs.get();
		Self::new(a0 + b0, a1 + b1)
	}
}

impl<P: PackedField> Add<QuadraticExtension::<P::FieldType>> for PackedQuadraticExtension<P> where P::FieldType: Extendable<2> {
	type Output = Self;
	#[inline]
	fn add(self, rhs: QuadraticExtension::<P::FieldType>) -> Self {
		self + Self::broadcast(rhs)
	}
}

impl<P: PackedField> AddAssign<Self> for PackedQuadraticExtension<P> where P::FieldType: Extendable<2> {
	#[inline]
	fn add_assign(&mut self, rhs: Self) {
		*self = *self + rhs;
	}
}

impl<P: PackedField> AddAssign<QuadraticExtension::<P::FieldType>> for PackedQuadraticExtension<P> where P::FieldType: Extendable<2> {
	#[inline]
	fn add_assign(&mut self, rhs: QuadraticExtension::<P::FieldType>) {
		*self = *self + rhs;
	}
}

impl<P: PackedField> Debug for PackedQuadraticExtension<P> where P::FieldType: Extendable<2> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    	panic!("Not implemented");
    }
}

impl<P: PackedField> Default for PackedQuadraticExtension<P> where P::FieldType: Extendable<2> {
    #[inline]
    fn default() -> Self {
        Self::zero()
    }
}

impl<P: PackedField> From<P> for PackedQuadraticExtension<P> where P::FieldType: Extendable<2> {
	fn from(p: P) -> Self {
		Self::new(p, P::zero())
	}
}

impl<P: PackedField> Mul<Self> for PackedQuadraticExtension<P> where P::FieldType: Extendable<2> {
	type Output = Self;
	#[inline]
	fn mul(self, rhs: Self) -> Self {
		panic!("Not implemented");
	}
}

impl<P: PackedField> Mul<QuadraticExtension::<P::FieldType>> for PackedQuadraticExtension<P> where P::FieldType: Extendable<2> {
	type Output = Self;
	#[inline]
	fn mul(self, rhs: QuadraticExtension::<P::FieldType>) -> Self {
		self * Self::broadcast(rhs)
	}
}

impl<P: PackedField> MulAssign<Self> for PackedQuadraticExtension<P> where P::FieldType: Extendable<2> {
	#[inline]
	fn mul_assign(&mut self, rhs: Self) {
		*self = *self * rhs;
	}
}

impl<P: PackedField> MulAssign<QuadraticExtension::<P::FieldType>> for PackedQuadraticExtension<P> where P::FieldType: Extendable<2> {
	#[inline]
	fn mul_assign(&mut self, rhs: QuadraticExtension::<P::FieldType>) {
		*self = *self * rhs;
	}
}

impl<P: PackedField> Neg for PackedQuadraticExtension<P> where P::FieldType: Extendable<2> {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        let (a0, a1) = self.get();
        Self::new(-a0, -a1)
    }
}

impl<P: PackedField> Product for PackedQuadraticExtension<P> where P::FieldType: Extendable<2> {
    #[inline]
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.reduce(|x, y| x * y).unwrap_or(Self::one())
    }
}

impl<P: PackedField> Sub<Self> for PackedQuadraticExtension<P> where P::FieldType: Extendable<2> {
	type Output = Self;
	#[inline]
	fn sub(self, rhs: Self) -> Self {
		let (a0, a1) = self.get();
		let (b0, b1) = rhs.get();
		Self::new(a0 - b0, a1 - b1)
	}
}

impl<P: PackedField> Sub<QuadraticExtension::<P::FieldType>> for PackedQuadraticExtension<P> where P::FieldType: Extendable<2> {
	type Output = Self;
	#[inline]
	fn sub(self, rhs: QuadraticExtension::<P::FieldType>) -> Self {
		self - Self::broadcast(rhs)
	}
}

impl<P: PackedField> SubAssign<Self> for PackedQuadraticExtension<P> where P::FieldType: Extendable<2> {
	#[inline]
	fn sub_assign(&mut self, rhs: Self) {
		*self = *self - rhs;
	}
}

impl<P: PackedField> SubAssign<QuadraticExtension::<P::FieldType>> for PackedQuadraticExtension<P> where P::FieldType: Extendable<2> {
	#[inline]
	fn sub_assign(&mut self, rhs: QuadraticExtension::<P::FieldType>) {
		*self = *self - rhs;
	}
}

impl<P: PackedField> Sum for PackedQuadraticExtension<P> where P::FieldType: Extendable<2> {
    #[inline]
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.reduce(|x, y| x + y).unwrap_or(Self::zero())
    }
}