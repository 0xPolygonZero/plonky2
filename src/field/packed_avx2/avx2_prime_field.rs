use core::arch::x86_64::*;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::iter::{Product, Sum};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use crate::field::field_types::PrimeField;
use crate::field::packed_avx2::common::{
    add_no_canonicalize_64_64s_s, epsilon, field_order, shift, ReducibleAvx2,
};
use crate::field::packed_field::PackedField;

// Avx2PrimeField wraps an array of four u64s, with the new and get methods to convert that
// array to and from __m256i, which is the type we actually operate on. This indirection is a
// terrible trick to change Avx2PrimeField's alignment.
//   We'd like to be able to cast slices of PrimeField to slices of Avx2PrimeField. Rust
// aligns __m256i to 32 bytes but PrimeField has a lower alignment. That alignment extends to
// Avx2PrimeField and it appears that it cannot be lowered with #[repr(C, blah)]. It is
// important for Rust not to assume 32-byte alignment, so we cannot wrap __m256i directly.
//   There are two versions of vectorized load/store instructions on x86: aligned (vmovaps and
// friends) and unaligned (vmovups etc.). The difference between them is that aligned loads and
// stores are permitted to segfault on unaligned accesses. Historically, the aligned instructions
// were faster, and although this is no longer the case, compilers prefer the aligned versions if
// they know that the address is aligned. Using aligned instructions on unaligned addresses leads to
// bugs that can be frustrating to diagnose. Hence, we can't have Rust assuming alignment, and
// therefore Avx2PrimeField wraps [F; 4] and not __m256i.
#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct Avx2PrimeField<F: ReducibleAvx2>(pub [F; 4]);

impl<F: ReducibleAvx2> Avx2PrimeField<F> {
    #[inline]
    fn new(x: __m256i) -> Self {
        let mut obj = Self([F::ZERO; 4]);
        let ptr = (&mut obj.0).as_mut_ptr().cast::<__m256i>();
        unsafe {
            _mm256_storeu_si256(ptr, x);
        }
        obj
    }
    #[inline]
    fn get(&self) -> __m256i {
        let ptr = (&self.0).as_ptr().cast::<__m256i>();
        unsafe { _mm256_loadu_si256(ptr) }
    }
}

impl<F: ReducibleAvx2> Add<Self> for Avx2PrimeField<F> {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self::new(unsafe { add::<F>(self.get(), rhs.get()) })
    }
}
impl<F: ReducibleAvx2> Add<F> for Avx2PrimeField<F> {
    type Output = Self;
    #[inline]
    fn add(self, rhs: F) -> Self {
        self + <F as Into<Self>>::into(rhs)
    }
}
impl<F: ReducibleAvx2> Add<Avx2PrimeField<F>> for <Avx2PrimeField<F> as PackedField>::Scalar {
    type Output = Avx2PrimeField<F>;
    #[inline]
    fn add(self, rhs: Self::Output) -> Self::Output {
        <Self as Into<Self::Output>>::into(self) + rhs
    }
}
impl<F: ReducibleAvx2> AddAssign<Self> for Avx2PrimeField<F> {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}
impl<F: ReducibleAvx2> AddAssign<F> for Avx2PrimeField<F> {
    #[inline]
    fn add_assign(&mut self, rhs: F) {
        *self = *self + rhs;
    }
}

impl<F: ReducibleAvx2> Debug for Avx2PrimeField<F> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "({:?})", self.get())
    }
}

impl<F: ReducibleAvx2> Default for Avx2PrimeField<F> {
    #[inline]
    fn default() -> Self {
        Self::ZERO
    }
}

impl<F: ReducibleAvx2> Div<F> for Avx2PrimeField<F> {
    type Output = Self;
    #[inline]
    fn div(self, rhs: F) -> Self {
        self * rhs.inverse()
    }
}
impl<F: ReducibleAvx2> DivAssign<F> for Avx2PrimeField<F> {
    #[inline]
    fn div_assign(&mut self, rhs: F) {
        *self *= rhs.inverse();
    }
}

impl<F: ReducibleAvx2> From<F> for Avx2PrimeField<F> {
    fn from(x: F) -> Self {
        Self([x; 4])
    }
}

impl<F: ReducibleAvx2> Mul<Self> for Avx2PrimeField<F> {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        Self::new(unsafe { mul::<F>(self.get(), rhs.get()) })
    }
}
impl<F: ReducibleAvx2> Mul<F> for Avx2PrimeField<F> {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: F) -> Self {
        self * <F as Into<Self>>::into(rhs)
    }
}
impl<F: ReducibleAvx2> Mul<Avx2PrimeField<F>> for <Avx2PrimeField<F> as PackedField>::Scalar {
    type Output = Avx2PrimeField<F>;
    #[inline]
    fn mul(self, rhs: Avx2PrimeField<F>) -> Self::Output {
        <Self as Into<Self::Output>>::into(self) * rhs
    }
}
impl<F: ReducibleAvx2> MulAssign<Self> for Avx2PrimeField<F> {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}
impl<F: ReducibleAvx2> MulAssign<F> for Avx2PrimeField<F> {
    #[inline]
    fn mul_assign(&mut self, rhs: F) {
        *self = *self * rhs;
    }
}

impl<F: ReducibleAvx2> Neg for Avx2PrimeField<F> {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self::new(unsafe { neg::<F>(self.get()) })
    }
}

impl<F: ReducibleAvx2> Product for Avx2PrimeField<F> {
    #[inline]
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.reduce(|x, y| x * y).unwrap_or(Self::ONE)
    }
}

unsafe impl<F: ReducibleAvx2> PackedField for Avx2PrimeField<F> {
    const WIDTH: usize = 4;

    type Scalar = F;
    type PackedPrimeField = Avx2PrimeField<F>;

    const ZERO: Self = Self([F::ZERO; 4]);
    const ONE: Self = Self([F::ONE; 4]);

    #[inline]
    fn from_arr(arr: [Self::Scalar; Self::WIDTH]) -> Self {
        Self(arr)
    }

    #[inline]
    fn as_arr(&self) -> [Self::Scalar; Self::WIDTH] {
        self.0
    }

    #[inline]
    fn from_slice(slice: &[Self::Scalar]) -> &Self {
        assert_eq!(slice.len(), Self::WIDTH);
        unsafe { &*slice.as_ptr().cast() }
    }
    #[inline]
    fn from_slice_mut(slice: &mut [Self::Scalar]) -> &mut Self {
        assert_eq!(slice.len(), Self::WIDTH);
        unsafe { &mut *slice.as_mut_ptr().cast() }
    }
    #[inline]
    fn as_slice(&self) -> &[Self::Scalar] {
        &self.0[..]
    }
    #[inline]
    fn as_slice_mut(&mut self) -> &mut [Self::Scalar] {
        &mut self.0[..]
    }

    #[inline]
    fn interleave(&self, other: Self, block_len: usize) -> (Self, Self) {
        let (v0, v1) = (self.get(), other.get());
        let (res0, res1) = match r {
            1 => unsafe { interleave1(v0, v1) },
            2 => unsafe { interleave2(v0, v1) },
            4 => (v0, v1),
            _ => panic!("unsupported block_len"),
        };
        (Self::new(res0), Self::new(res1))
    }

    #[inline]
    fn square(&self) -> Self {
        Self::new(unsafe { square::<F>(self.get()) })
    }
}

impl<F: ReducibleAvx2> Sub<Self> for Avx2PrimeField<F> {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self::new(unsafe { sub::<F>(self.get(), rhs.get()) })
    }
}
impl<F: ReducibleAvx2> Sub<F> for Avx2PrimeField<F> {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: F) -> Self {
        self - <F as Into<Self>>::into(rhs)
    }
}
impl<F: ReducibleAvx2> Sub<Avx2PrimeField<F>> for <Avx2PrimeField<F> as PackedField>::Scalar {
    type Output = Avx2PrimeField<F>;
    #[inline]
    fn sub(self, rhs: Avx2PrimeField<F>) -> Self::Output {
        <Self as Into<Self::Output>>::into(self) - rhs
    }
}
impl<F: ReducibleAvx2> SubAssign<Self> for Avx2PrimeField<F> {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}
impl<F: ReducibleAvx2> SubAssign<F> for Avx2PrimeField<F> {
    #[inline]
    fn sub_assign(&mut self, rhs: F) {
        *self = *self - rhs;
    }
}

impl<F: ReducibleAvx2> Sum for Avx2PrimeField<F> {
    #[inline]
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.reduce(|x, y| x + y).unwrap_or(Self::ZERO)
    }
}

// Resources:
// 1. Intel Intrinsics Guide for explanation of each intrinsic:
//    https://software.intel.com/sites/landingpage/IntrinsicsGuide/
// 2. uops.info lists micro-ops for each instruction: https://uops.info/table.html
// 3. Intel optimization manual for introduction to x86 vector extensions and best practices:
//    https://software.intel.com/content/www/us/en/develop/download/intel-64-and-ia-32-architectures-optimization-reference-manual.html

// Preliminary knowledge:
// 1. Vector code usually avoids branching. Instead of branches, we can do input selection with
//    _mm256_blendv_epi8 or similar instruction. If all we're doing is conditionally zeroing a
//    vector element then _mm256_and_si256 or _mm256_andnot_si256 may be used and are cheaper.
//
// 2. AVX does not support addition with carry but 128-bit (2-word) addition can be easily
//    emulated. The method recognizes that for a + b overflowed iff (a + b) < a:
//        i. res_lo = a_lo + b_lo
//       ii. carry_mask = res_lo < a_lo
//      iii. res_hi = a_hi + b_hi - carry_mask
//    Notice that carry_mask is subtracted, not added. This is because AVX comparison instructions
//    return -1 (all bits 1) for true and 0 for false.
//
// 3. AVX does not have unsigned 64-bit comparisons. Those can be emulated with signed comparisons
//    by recognizing that a <u b iff a + (1 << 63) <s b + (1 << 63), where the addition wraps around
//    and the comparisons are unsigned and signed respectively. The shift function adds/subtracts
//    1 << 63 to enable this trick.
//      Example: addition with carry.
//        i. a_lo_s = shift(a_lo)
//       ii. res_lo_s = a_lo_s + b_lo
//      iii. carry_mask = res_lo_s <s a_lo_s
//       iv. res_lo = shift(res_lo_s)
//        v. res_hi = a_hi + b_hi - carry_mask
//    The suffix _s denotes a value that has been shifted by 1 << 63. The result of addition is
//    shifted if exactly one of the operands is shifted, as is the case on line ii. Line iii.
//    performs a signed comparison res_lo_s <s a_lo_s on shifted values to emulate unsigned
//    comparison res_lo <u a_lo on unshifted values. Finally, line iv. reverses the shift so the
//    result can be returned.
//      When performing a chain of calculations, we can often save instructions by letting the shift
//    propagate through and only undoing it when necessary. For example, to compute the addition of
//    three two-word (128-bit) numbers we can do:
//        i. a_lo_s = shift(a_lo)
//       ii. tmp_lo_s = a_lo_s + b_lo
//      iii. tmp_carry_mask = tmp_lo_s <s a_lo_s
//       iv. tmp_hi = a_hi + b_hi - tmp_carry_mask
//        v. res_lo_s = tmp_lo_s + c_lo
//       vi. res_carry_mask = res_lo_s <s tmp_lo_s
//      vii. res_lo = shift(res_lo_s)
//     viii. res_hi = tmp_hi + c_hi - res_carry_mask
//    Notice that the above 3-value addition still only requires two calls to shift, just like our
//    2-value addition.

/// Convert to canonical representation.
/// The argument is assumed to be shifted by 1 << 63 (i.e. x_s = x + 1<<63, where x is the field
///   value). The returned value is similarly shifted by 1 << 63 (i.e. we return y_s = y + (1<<63),
///   where 0 <= y < FIELD_ORDER).
#[inline]
unsafe fn canonicalize_s<F: PrimeField>(x_s: __m256i) -> __m256i {
    // If x >= FIELD_ORDER then corresponding mask bits are all 0; otherwise all 1.
    let mask = _mm256_cmpgt_epi64(shift(field_order::<F>()), x_s);
    // wrapback_amt is -FIELD_ORDER if mask is 0; otherwise 0.
    let wrapback_amt = _mm256_andnot_si256(mask, epsilon::<F>());
    _mm256_add_epi64(x_s, wrapback_amt)
}

#[inline]
unsafe fn add<F: PrimeField>(x: __m256i, y: __m256i) -> __m256i {
    let y_s = shift(y);
    let res_s = add_no_canonicalize_64_64s_s::<F>(x, canonicalize_s::<F>(y_s));
    shift(res_s)
}

#[inline]
unsafe fn sub<F: PrimeField>(x: __m256i, y: __m256i) -> __m256i {
    let mut y_s = shift(y);
    y_s = canonicalize_s::<F>(y_s);
    let x_s = shift(x);
    let mask = _mm256_cmpgt_epi64(y_s, x_s); // -1 if sub will underflow (y > x) else 0.
    let wrapback_amt = _mm256_and_si256(mask, epsilon::<F>()); // -FIELD_ORDER if underflow else 0.
    let res_wrapped = _mm256_sub_epi64(x_s, y_s);
    let res = _mm256_sub_epi64(res_wrapped, wrapback_amt);
    res
}

#[inline]
unsafe fn neg<F: PrimeField>(y: __m256i) -> __m256i {
    let y_s = shift(y);
    _mm256_sub_epi64(shift(field_order::<F>()), canonicalize_s::<F>(y_s))
}

/// Full 64-bit by 64-bit multiplication. This emulated multiplication is 1.33x slower than the
/// scalar instruction, but may be worth it if we want our data to live in vector registers.
#[inline]
unsafe fn mul64_64(x: __m256i, y: __m256i) -> (__m256i, __m256i) {
    // We want to move the high 32 bits to the low position. The multiplication instruction ignores
    // the high 32 bits, so it's ok to just duplicate it into the low position. This duplication can
    // be done on port 5; bitshifts run on ports 0 and 1, competing with multiplication.
    //   This instruction is only provided for 32-bit floats, not integers. Idk why Intel makes the
    // distinction; the casts are free and it guarantees that the exact bit pattern is preserved.
    // Using a swizzle instruction of the wrong domain (float vs int) does not increase latency
    // since Haswell.
    let x_hi = _mm256_castps_si256(_mm256_movehdup_ps(_mm256_castsi256_ps(x)));
    let y_hi = _mm256_castps_si256(_mm256_movehdup_ps(_mm256_castsi256_ps(y)));

    // All four pairwise multiplications
    let mul_ll = _mm256_mul_epu32(x, y);
    let mul_lh = _mm256_mul_epu32(x, y_hi);
    let mul_hl = _mm256_mul_epu32(x_hi, y);
    let mul_hh = _mm256_mul_epu32(x_hi, y_hi);

    // Bignum addition
    // Extract high 32 bits of mul_ll and add to mul_hl. This cannot overflow.
    let mul_ll_hi = _mm256_srli_epi64::<32>(mul_ll);
    let t0 = _mm256_add_epi64(mul_hl, mul_ll_hi);
    // Extract low 32 bits of t0 and add to mul_lh. Again, this cannot overflow.
    // Also, extract high 32 bits of t0 and add to mul_hh.
    let t0_lo = _mm256_and_si256(t0, _mm256_set1_epi64x(u32::MAX.into()));
    let t0_hi = _mm256_srli_epi64::<32>(t0);
    let t1 = _mm256_add_epi64(mul_lh, t0_lo);
    let t2 = _mm256_add_epi64(mul_hh, t0_hi);
    // Lastly, extract the high 32 bits of t1 and add to t2.
    let t1_hi = _mm256_srli_epi64::<32>(t1);
    let res_hi = _mm256_add_epi64(t2, t1_hi);

    // Form res_lo by combining the low half of mul_ll with the low half of t1 (shifted into high
    // position).
    let t1_lo = _mm256_castps_si256(_mm256_moveldup_ps(_mm256_castsi256_ps(t1)));
    let res_lo = _mm256_blend_epi32::<0xaa>(mul_ll, t1_lo);

    (res_hi, res_lo)
}

/// Full 64-bit squaring. This routine is 1.2x faster than the scalar instruction.
#[inline]
unsafe fn square64(x: __m256i) -> (__m256i, __m256i) {
    // Get high 32 bits of x. See comment in mul64_64_s.
    let x_hi = _mm256_castps_si256(_mm256_movehdup_ps(_mm256_castsi256_ps(x)));

    // All pairwise multiplications.
    let mul_ll = _mm256_mul_epu32(x, x);
    let mul_lh = _mm256_mul_epu32(x, x_hi);
    let mul_hh = _mm256_mul_epu32(x_hi, x_hi);

    // Bignum addition, but mul_lh is shifted by 33 bits (not 32).
    let mul_ll_hi = _mm256_srli_epi64::<33>(mul_ll);
    let t0 = _mm256_add_epi64(mul_lh, mul_ll_hi);
    let t0_hi = _mm256_srli_epi64::<31>(t0);
    let res_hi = _mm256_add_epi64(mul_hh, t0_hi);

    // Form low result by adding the mul_ll and the low 31 bits of mul_lh (shifted to the high
    // position).
    let mul_lh_lo = _mm256_slli_epi64::<33>(mul_lh);
    let res_lo = _mm256_add_epi64(mul_ll, mul_lh_lo);

    (res_hi, res_lo)
}

/// Multiply two integers modulo FIELD_ORDER.
#[inline]
unsafe fn mul<F: ReducibleAvx2>(x: __m256i, y: __m256i) -> __m256i {
    F::reduce128(mul64_64(x, y))
}

/// Square an integer modulo FIELD_ORDER.
#[inline]
unsafe fn square<F: ReducibleAvx2>(x: __m256i) -> __m256i {
    F::reduce128(square64(x))
}

#[inline]
unsafe fn interleave1(x: __m256i, y: __m256i) -> (__m256i, __m256i) {
    let a = _mm256_unpacklo_epi64(x, y);
    let b = _mm256_unpackhi_epi64(x, y);
    (a, b)
}

#[inline]
unsafe fn interleave2(x: __m256i, y: __m256i) -> (__m256i, __m256i) {
    let y_lo = _mm256_castsi256_si128(y); // This has 0 cost.

    // 1 places y_lo in the high half of x; 0 would place it in the lower half.
    let a = _mm256_inserti128_si256::<1>(x, y_lo);
    // NB: _mm256_permute2x128_si256 could be used here as well but _mm256_inserti128_si256 has
    // lower latency on Zen 3 processors.

    // Each nibble of the constant has the following semantics:
    // 0 => src1[low 128 bits]
    // 1 => src1[high 128 bits]
    // 2 => src2[low 128 bits]
    // 3 => src2[high 128 bits]
    // The low (resp. high) nibble chooses the low (resp. high) 128 bits of the result.
    let b = _mm256_permute2x128_si256::<0x31>(x, y);

    (a, b)
}
