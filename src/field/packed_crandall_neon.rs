use core::arch::aarch64::*;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::iter::{Product, Sum};
use std::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use crate::field::crandall_field::CrandallField;
use crate::field::packed_field::PackedField;

/// PackedCrandallNeon wraps to ensure that Rust does not assume 16-byte alignment. Similar to
/// AVX2's PackedPrimeField. I don't think it matters as much on ARM but incorrectly-aligned
/// pointers are undefined behavior in Rust, so let's avoid them.
#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct PackedCrandallNeon(pub [CrandallField; 2]);

impl PackedCrandallNeon {
    #[inline]
    fn new(x: uint64x2_t) -> Self {
        let mut obj = Self([CrandallField::ZERO, CrandallField::ZERO]);
        let ptr = (&mut obj.0).as_mut_ptr().cast::<u64>();
        unsafe {
            vst1q_u64(ptr, x);
        }
        obj
    }
    #[inline]
    fn get(&self) -> uint64x2_t {
        let ptr = (&self.0).as_ptr().cast::<u64>();
        unsafe { vld1q_u64(ptr) }
    }

    /// Addition that assumes x + y < 2^64 + F::ORDER. May return incorrect results if this
    /// condition is not met, hence it is marked unsafe.
    #[inline]
    pub unsafe fn add_canonical_u64(&self, rhs: __m256i) -> Self {
        Self::new(add_canonical_u64::<F>(self.get(), rhs))
    }
}

impl Add<Self> for PackedCrandallNeon {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self::new(unsafe { add(self.get(), rhs.get()) })
    }
}
impl Add<CrandallField> for PackedCrandallNeon {
    type Output = Self;
    #[inline]
    fn add(self, rhs: CrandallField) -> Self {
        self + Self::broadcast(rhs)
    }
}
impl AddAssign<Self> for PackedCrandallNeon {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}
impl AddAssign<CrandallField> for PackedCrandallNeon {
    #[inline]
    fn add_assign(&mut self, rhs: CrandallField) {
        *self = *self + rhs;
    }
}

impl Debug for PackedCrandallNeon {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "({:?})", self.get())
    }
}

impl Default for PackedCrandallNeon {
    #[inline]
    fn default() -> Self {
        Self::zero()
    }
}

impl Mul<Self> for PackedCrandallNeon {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        // TODO: Implement.
        // Do this in scalar for now.
        Self([self.0[0] * rhs[0], self.0[1] * rhs[1]])
    }
}
impl Mul<CrandallField> for PackedCrandallNeon {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: CrandallField) -> Self {
        self * Self::broadcast(rhs)
    }
}
impl MulAssign<Self> for PackedCrandallNeon {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}
impl MulAssign<CrandallField> for PackedCrandallNeon {
    #[inline]
    fn mul_assign(&mut self, rhs: CrandallField) {
        *self = *self * rhs;
    }
}

impl Neg for PackedCrandallNeon {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self::new(unsafe { neg(self.get()) })
    }
}

impl Product for PackedCrandallNeon {
    #[inline]
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.reduce(|x, y| x * y).unwrap_or(Self::one())
    }
}

impl PackedField for PackedCrandallNeon {
    const LOG2_WIDTH: usize = 1;

    type FieldType = CrandallField;

    #[inline]
    fn broadcast(x: CrandallField) -> Self {
        Self[x; 2]
    }

    #[inline]
    fn from_arr(arr: [Self::FieldType; Self::WIDTH]) -> Self {
        Self(arr)
    }

    #[inline]
    fn to_arr(&self) -> [Self::FieldType; Self::WIDTH] {
        self.0
    }

    #[inline]
    fn from_slice(slice: &[F]) -> Self {
        Self(slice.try_into().unwrap())
    }

    #[inline]
    fn to_vec(&self) -> Vec<F> {
        self.0.into()
    }

    #[inline]
    fn interleave(&self, other: Self, r: usize) -> (Self, Self) {
        let (v0, v1) = (self.get(), other.get());
        let (res0, res1) = match r {
            0 => unsafe { interleave0(v0, v1) },
            1 => (v0, v1),
            _ => panic!("r cannot be more than LOG2_WIDTH"),
        };
        (Self::new(res0), Self::new(res1))
    }
}

impl Sub<Self> for PackedCrandallNeon {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self::new(unsafe { sub(self.get(), rhs.get()) })
    }
}
impl Sub<CrandallField> for PackedCrandallNeon {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: CrandallField) -> Self {
        self - Self::broadcast(rhs)
    }
}
impl SubAssign<Self> for PackedCrandallNeon {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}
impl SubAssign<CrandallField> for PackedCrandallNeon {
    #[inline]
    fn sub_assign(&mut self, rhs: CrandallField) {
        *self = *self - rhs;
    }
}

impl Sum for PackedCrandallNeon {
    #[inline]
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.reduce(|x, y| x + y).unwrap_or(Self::zero())
    }
}

const FIELD_ORDER: u64 = CrandallField::ORDER;
const EPSILON: u64 = 0u64.wrapping_sub(FIELD_ORDER);

#[inline]
unsafe fn field_order() -> uint64x2_t {
    vmovq_n_u64(FIELD_ORDER)
}

#[inline]
unsafe fn epsilon() -> uint64x2_t {
    vmovq_n_u64(EPSILON)
}

#[inline]
unsafe fn canonicalize(x: uint64x2_t) -> uint64x2_t {
    let mask = vcgeq_u64(x, field_order()); // Mask is -1 if x >= FIELD_ORDER.
    let x_maybe_unwrapped = vsubq_u64(x, field_order());
    vbslq_u64(mask, x_maybe_unwrapped, x) // Bitwise select
}

#[inline]
unsafe fn add_no_canonicalize_64_64(x: uint64x2_t, y: uint64x2_t) -> uint64x2_t {
    let res_wrapped = vaddq_u64(x, y);
    let mask = vcgtq_u64(y, res_wrapped); // Mask is -1 if overflow.
    let res_maybe_unwrapped = vsubq_u64(res_wrapped, field_order());
    vbslq_u64(mask, res_maybe_unwrapped, res_wrapped) // Bitwise select
}

#[inline]
unsafe fn add_canonical_u64(x: uint64x2_t, y: uint64x2_t) -> uint64x2_t {
    add_no_canonicalize_64_64(x, y)
}

#[inline]
unsafe fn add(x: uint64x2_t, y: uint64x2_t) -> uint64x2_t {
    add_no_canonicalize_64_64(x, canonicalize(y))
}

#[inline]
unsafe fn sub(x: uint64x2_t, y: uint64x2_t) -> uint64x2_t {
    let y = canonicalize(y);
    let mask = vcgtq_u64(y, x); // Mask is -1 if overflow.
    let res_wrapped = vsubq_u64(x, y);
    let res_maybe_unwrapped = vaddq_u64(res_wrapped, field_order());
    vbslq_u64(mask, res_maybe_unwrapped, res_wrapped) // Bitwise select
}

#[inline]
unsafe fn neg(y: uint64x2_t) -> uint64x2_t {
    vsubq_u64(field_order(), canonicalize(y))
}

#[inline]
unsafe fn interleave0(x: uint64x2_t, y: uint64x2_t) -> (uint64x2_t, uint64x2_t) {
    (vtrn1q_u64(x, y), vtrn2q_u64(x, y))
}
