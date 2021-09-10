use core::arch::aarch64::*;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::iter::{Product, Sum};
use std::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use crate::field::crandall_field::CrandallField;
use crate::field::packed_field::PackedField;

#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct PackedCrandallNeon(pub [u64; 2]);

impl PackedCrandallNeon {
    #[inline]
    fn new(x: uint64x2_t) -> Self {
        let mut obj = Self([0, 0]);
        let ptr = (&mut obj.0).as_mut_ptr();
        unsafe {
            vst1q_u64(ptr, x);
        }
        obj
    }
    #[inline]
    fn get(&self) -> uint64x2_t {
        let ptr = (&self.0).as_ptr();
        unsafe { vld1q_u64(ptr) }
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
        Self::new(unsafe { mul(self.get(), rhs.get()) })
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
        Self::new(unsafe { vmovq_n_u64(x.0) })
    }

    #[inline]
    fn from_arr(arr: [Self::FieldType; Self::WIDTH]) -> Self {
        Self([arr[0].0, arr[1].0])
    }

    #[inline]
    fn to_arr(&self) -> [Self::FieldType; Self::WIDTH] {
        [
            CrandallField(self.0[0]),
            CrandallField(self.0[1]),
        ]
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

const EPSILON: u64 = (1 << 31) + (1 << 28) - 1;
const FIELD_ORDER: u64 = 0u64.overflowing_sub(EPSILON).0;
const SIGN_BIT: u64 = 1 << 63;

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
    let mask = vcgeq_u64(x, field_order());
    let x_maybe_unwrapped = vsubq_u64(x, field_order());
    vbslq_u64(mask, x_maybe_unwrapped, x)
}

#[inline]
unsafe fn add_no_canonicalize_64_64(x: uint64x2_t, y: uint64x2_t) -> uint64x2_t {
    let res_wrapped = vaddq_u64(x, y);
    let mask = vcgtq_u64(y, res_wrapped);
    let res_maybe_unwrapped = vsubq_u64(res_wrapped, field_order());
    vbslq_u64(mask, res_maybe_unwrapped, res_wrapped)
}

#[inline]
unsafe fn add(x: uint64x2_t, y: uint64x2_t) -> uint64x2_t {
    add_no_canonicalize_64_64(x, canonicalize(y))
}

#[inline]
unsafe fn sub(x: uint64x2_t, y: uint64x2_t) -> uint64x2_t {
    let y = canonicalize(y);
    let mask = vcgtq_u64(y, x);
    let res_wrapped = vsubq_u64(x, y);
    let res_maybe_unwrapped = vaddq_u64(res_wrapped, field_order());
    vbslq_u64(mask, res_maybe_unwrapped, res_wrapped)
}

#[inline]
unsafe fn neg(y: uint64x2_t) -> uint64x2_t {
    vsubq_u64(field_order(), canonicalize(y))
}

#[inline]
unsafe fn mul64_64(x: uint64x2_t, y: uint64x2_t) -> (uint64x2_t, uint64x2_t) {
    let x_lo = blah; // TODO
    let y_lo = blah; // TODO
    let x_hi = blah; // TODO
    let y_hi = blah; // TODO

    let mul_ll = vmull_u32(x_lo, y_lo);
    let mul_lh = vmull_u32(x_lo, y_hi);
    let mul_hl = vmull_u32(x_hi, y_lo);
    let mul_hh = vmull_u32(x_hi, y_hi);

    let res_lo0 = mul_ll;
    let res_lo1 = vaddq_u64(res_lo0, vshlq_n_u64(mul_lh, 32));
    let res_lo2 = vaddq_u64(res_lo1, vshlq_n_u64(mul_hl, 32));

    let carry0 = vcgtq_u64(res_lo0, res_lo1);
    let carry1 = vcgtq_u64(res_lo1, res_lo2);

    let res_hi0 = mul_hh;
    let res_hi1 = vsraq_n_u64(res_hi0, mul_lh, 32);
    let res_hi2 = vsraq_n_u64(res_hi1, mul_hl, 32));
    let res_hi3 = vsubq_u64(res_hi2, carry0);
    let res_hi4 = vsubq_u64(res_hi3, carry1);

    (res_hi4, res_lo2)
}

#[inline]
unsafe fn add_with_carry_hi_lo_lo(
    hi: uint64x2_t,
    lo0: uint64x2_t,
    lo1: uint64x2_t,
) -> (uint64x2_t, uint64x2_t) {
    let res_lo = vaddq_u64(lo0, lo1);
    let carry = vcgtq_u64(res_lo, lo1);
    let res_hi = vsubq_u64(hi, carry);
    (res_hi, res_lo)
}

#[inline] // TODO
unsafe fn fmadd_64_32_64(x: uint64x2_t, y: uint64x2_t, z: uint64x2_t) -> (uint64x2_t, uint64x2_t) {
    let x_hi = _mm256_srli_epi64(x, 32);
    let mul_lo = _mm256_mul_epu32(x, y);
    let mul_hi = _mm256_mul_epu32(x_hi, y);
    let (tmp_hi, tmp_lo) = add_with_carry_hi_lo_lo(_mm256_srli_epi64(mul_hi, 32), mul_lo, z);
    add_with_carry_hi_lo_lo(tmp_hi, _mm256_slli_epi64(mul_hi, 32), tmp_lo)
}

#[inline] // TODO
unsafe fn reduce128(x: (uint64x2_t, uint64x2_t)) -> uint64x2_t {
    let (hi0, lo0) = x;
    let (hi1, lo1) = fmadd_64_32_64(hi0, epsilon(), lo0);
    let lo2 = _mm256_mul_epu32(hi1, epsilon());
    add_no_canonicalize_64_64(lo2, lo1)
}

#[inline]
unsafe fn mul(x: uint64x2_t, y: uint64x2_t) -> uint64x2_t {
    reduce128(mul64_64(x, y))
}

#[inline]
unsafe fn interleave0(x: uint64x2_t, y: uint64x2_t) -> (uint64x2_t, uint64x2_t) {
    (vtrn1q_u64(x, y), vtrn2q_u64(x, y))
}
