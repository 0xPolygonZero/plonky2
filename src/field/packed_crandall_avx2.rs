use core::arch::x86_64::*;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::iter::{Product, Sum};
use std::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use crate::field::crandall_field::CrandallField;
use crate::field::packed_field::PackedField;

// PackedCrandallAVX2 wraps an array of four u64s, with the new and get methods to convert that
// array to and from __m256i, which is the type we actually operate on. This indirection is a
// terrible trick to change PackedCrandallAVX2's alignment.
//   We'd like to be able to cast slices of CrandallField to slices of PackedCrandallAVX2. Rust
// aligns __m256i to 32 bytes but CrandallField has a lower alignment. That alignment extends to
// PackedCrandallAVX2 and it appears that it cannot be lowered with #[repr(C, blah)]. It is
// important for Rust not to assume 32-byte alignment, so we cannot wrap __m256i directly.
//   There are two versions of vectorized load/store instructions on x86: aligned (vmovaps and
// friends) and unaligned (vmovups etc.). The difference between them is that aligned loads and
// stores are permitted to segfault on unaligned accesses. Historically, the aligned instructions
// were faster, and although this is no longer the case, compilers prefer the aligned versions if
// they know that the address is aligned. Using aligned instructions on unaligned addresses leads to
// bugs that can be frustrating to diagnose. Hence, we can't have Rust assuming alignment, and
// therefore PackedCrandallAVX2 wraps [u64; 4] and not __m256i.
#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct PackedCrandallAVX2(pub [u64; 4]);

impl PackedCrandallAVX2 {
    #[inline]
    fn new(x: __m256i) -> Self {
        let mut obj = Self([0, 0, 0, 0]);
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

impl Add<Self> for PackedCrandallAVX2 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self::new(unsafe { add(self.get(), rhs.get()) })
    }
}
impl Add<CrandallField> for PackedCrandallAVX2 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: CrandallField) -> Self {
        self + Self::broadcast(rhs)
    }
}
impl AddAssign<Self> for PackedCrandallAVX2 {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}
impl AddAssign<CrandallField> for PackedCrandallAVX2 {
    #[inline]
    fn add_assign(&mut self, rhs: CrandallField) {
        *self = *self + rhs;
    }
}

impl Debug for PackedCrandallAVX2 {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "({:?})", self.get())
    }
}

impl Default for PackedCrandallAVX2 {
    #[inline]
    fn default() -> Self {
        Self::zero()
    }
}

impl Mul<Self> for PackedCrandallAVX2 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        Self::new(unsafe { mul(self.get(), rhs.get()) })
    }
}
impl Mul<CrandallField> for PackedCrandallAVX2 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: CrandallField) -> Self {
        self * Self::broadcast(rhs)
    }
}
impl MulAssign<Self> for PackedCrandallAVX2 {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}
impl MulAssign<CrandallField> for PackedCrandallAVX2 {
    #[inline]
    fn mul_assign(&mut self, rhs: CrandallField) {
        *self = *self * rhs;
    }
}

impl Neg for PackedCrandallAVX2 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self::new(unsafe { neg(self.get()) })
    }
}

impl Product for PackedCrandallAVX2 {
    #[inline]
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.reduce(|x, y| x * y).unwrap_or(Self::one())
    }
}

impl PackedField for PackedCrandallAVX2 {
    const LOG2_WIDTH: usize = 2;

    type FieldType = CrandallField;

    #[inline]
    fn broadcast(x: CrandallField) -> Self {
        Self::new(unsafe { _mm256_set1_epi64x(x.0 as i64) })
    }

    #[inline]
    fn from_arr(arr: [Self::FieldType; Self::WIDTH]) -> Self {
        Self([arr[0].0, arr[1].0, arr[2].0, arr[3].0])
    }

    #[inline]
    fn to_arr(&self) -> [Self::FieldType; Self::WIDTH] {
        [CrandallField(self.0[0]), CrandallField(self.0[1]), CrandallField(self.0[2]), CrandallField(self.0[3])]
    }

    #[inline]
    fn interleave(&self, other: Self, r: usize) -> (Self, Self) {
        let (v0, v1) = (self.get(), other.get());
        let (res0, res1) = match r {
            0 => unsafe { interleave0(v0, v1) },
            1 => unsafe { interleave1(v0, v1) },
            2 => (v0, v1),
            _ => panic!("r cannot be more than LOG2_WIDTH"),
        };
        (Self::new(res0), Self::new(res1))
    }
}

impl Sub<Self> for PackedCrandallAVX2 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self::new(unsafe { sub(self.get(), rhs.get()) })
    }
}
impl Sub<CrandallField> for PackedCrandallAVX2 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: CrandallField) -> Self {
        self - Self::broadcast(rhs)
    }
}
impl SubAssign<Self> for PackedCrandallAVX2 {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}
impl SubAssign<CrandallField> for PackedCrandallAVX2 {
    #[inline]
    fn sub_assign(&mut self, rhs: CrandallField) {
        *self = *self - rhs;
    }
}

impl Sum for PackedCrandallAVX2 {
    #[inline]
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.reduce(|x, y| x + y).unwrap_or(Self::zero())
    }
}

const EPSILON: u64 = (1 << 31) + (1 << 28) - 1;
const FIELD_ORDER: u64 = 0u64.overflowing_sub(EPSILON).0;
const SIGN_BIT: u64 = 1 << 63;

#[inline]
unsafe fn field_order() -> __m256i {
    _mm256_set1_epi64x(FIELD_ORDER as i64)
}

#[inline]
unsafe fn epsilon() -> __m256i {
    _mm256_set1_epi64x(EPSILON as i64)
}

#[inline]
unsafe fn sign_bit() -> __m256i {
    _mm256_set1_epi64x(SIGN_BIT as i64)
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

/// Add 2^63 with overflow. Needed to emulate unsigned comparisons (see point 3. above).
#[inline]
unsafe fn shift(x: __m256i) -> __m256i {
    _mm256_xor_si256(x, sign_bit())
}

/// Convert to canonical representation.
/// The argument is assumed to be shifted by 1 << 63 (i.e. x_s = x + 1<<63, where x is the
///   Crandall field value). The returned value is similarly shifted by 1 << 63 (i.e. we return y`_s
///   = y + 1<<63, where 0 <= y < FIELD_ORDER).
#[inline]
unsafe fn canonicalize_s(x_s: __m256i) -> __m256i {
    // If x >= FIELD_ORDER then corresponding mask bits are all 0; otherwise all 1.
    let mask = _mm256_cmpgt_epi64(shift(field_order()), x_s);
    // wrapback_amt is -FIELD_ORDER if mask is 0; otherwise 0.
    let wrapback_amt = _mm256_andnot_si256(mask, epsilon());
    _mm256_add_epi64(x_s, wrapback_amt)
}

/// Addition u64 + u64 -> u64. Assumes that x + y < 2^64 + FIELD_ORDER. The second argument is
/// pre-shifted by 1 << 63. The result is similarly shifted.
#[inline]
unsafe fn add_no_canonicalize_64_64s_s(x: __m256i, y_s: __m256i) -> __m256i {
    let res_wrapped_s = _mm256_add_epi64(x, y_s);
    let mask = _mm256_cmpgt_epi64(y_s, res_wrapped_s); // -1 if overflowed else 0.
    let wrapback_amt = _mm256_and_si256(mask, epsilon()); // -FIELD_ORDER if overflowed else 0.
    let res_s = _mm256_add_epi64(res_wrapped_s, wrapback_amt);
    res_s
}

#[inline]
unsafe fn add(x: __m256i, y: __m256i) -> __m256i {
    let y_s = shift(y);
    let res_s = add_no_canonicalize_64_64s_s(x, canonicalize_s(y_s));
    shift(res_s)
}

#[inline]
unsafe fn sub(x: __m256i, y: __m256i) -> __m256i {
    let mut y_s = shift(y);
    y_s = canonicalize_s(y_s);
    let x_s = shift(x);
    let mask = _mm256_cmpgt_epi64(y_s, x_s); // -1 if sub will underflow (y > x) else 0.
    let wrapback_amt = _mm256_and_si256(mask, epsilon()); // -FIELD_ORDER if underflow else 0.
    let res_wrapped = _mm256_sub_epi64(x_s, y_s);
    let res = _mm256_sub_epi64(res_wrapped, wrapback_amt);
    res
}

#[inline]
unsafe fn neg(y: __m256i) -> __m256i {
    let y_s = shift(y);
    _mm256_sub_epi64(shift(field_order()), canonicalize_s(y_s))
}

/// Full 64-bit by 64-bit multiplication. This emulated multiplication is 1.5x slower than the
/// scalar instruction, but may be worth it if we want our data to live in vector registers.
#[inline]
unsafe fn mul64_64_s(x: __m256i, y: __m256i) -> (__m256i, __m256i) {
    let x_hi = _mm256_srli_epi64(x, 32);
    let y_hi = _mm256_srli_epi64(y, 32);
    let mul_ll = _mm256_mul_epu32(x, y);
    let mul_lh = _mm256_mul_epu32(x, y_hi);
    let mul_hl = _mm256_mul_epu32(x_hi, y);
    let mul_hh = _mm256_mul_epu32(x_hi, y_hi);

    let res_lo0_s = shift(mul_ll);
    let res_lo1_s = _mm256_add_epi32(res_lo0_s, _mm256_slli_epi64(mul_lh, 32));
    let res_lo2_s = _mm256_add_epi32(res_lo1_s, _mm256_slli_epi64(mul_hl, 32));

    // cmpgt returns -1 on true and 0 on false. Hence, the carry values below are set to -1 on
    // overflow and must be subtracted, not added.
    let carry0 = _mm256_cmpgt_epi64(res_lo0_s, res_lo1_s);
    let carry1 = _mm256_cmpgt_epi64(res_lo1_s, res_lo2_s);

    let res_hi0 = mul_hh;
    let res_hi1 = _mm256_add_epi64(res_hi0, _mm256_srli_epi64(mul_lh, 32));
    let res_hi2 = _mm256_add_epi64(res_hi1, _mm256_srli_epi64(mul_hl, 32));
    let res_hi3 = _mm256_sub_epi64(res_hi2, carry0);
    let res_hi4 = _mm256_sub_epi64(res_hi3, carry1);

    (res_hi4, res_lo2_s)
}

/// (u64 << 64) + u64 + u64 -> u128 addition with carry. The third argument is pre-shifted by 2^63.
/// The result is also shifted.
#[inline]
unsafe fn add_with_carry_hi_lo_los_s(
    hi: __m256i,
    lo0: __m256i,
    lo1_s: __m256i,
) -> (__m256i, __m256i) {
    let res_lo_s = _mm256_add_epi64(lo0, lo1_s);
    // carry is -1 if overflow (res_lo < lo1) because cmpgt returns -1 on true and 0 on false.
    let carry = _mm256_cmpgt_epi64(lo1_s, res_lo_s);
    let res_hi = _mm256_sub_epi64(hi, carry);
    (res_hi, res_lo_s)
}

/// u64 * u32 + u64 fused multiply-add. The result is given as a tuple (u64, u64). The third
/// argument is assumed to be pre-shifted by 2^63. The result is similarly shifted.
#[inline]
unsafe fn fmadd_64_32_64s_s(x: __m256i, y: __m256i, z_s: __m256i) -> (__m256i, __m256i) {
    let x_hi = _mm256_srli_epi64(x, 32);
    let mul_lo = _mm256_mul_epu32(x, y);
    let mul_hi = _mm256_mul_epu32(x_hi, y);
    let (tmp_hi, tmp_lo_s) = add_with_carry_hi_lo_los_s(_mm256_srli_epi64(mul_hi, 32), mul_lo, z_s);
    add_with_carry_hi_lo_los_s(tmp_hi, _mm256_slli_epi64(mul_hi, 32), tmp_lo_s)
}

/// Reduce a u128 modulo FIELD_ORDER. The input is (u64, u64), pre-shifted by 2^63. The result is
/// similarly shifted.
#[inline]
unsafe fn reduce128s_s(x_s: (__m256i, __m256i)) -> __m256i {
    let (hi0, lo0_s) = x_s;
    let (hi1, lo1_s) = fmadd_64_32_64s_s(hi0, epsilon(), lo0_s);
    let lo2 = _mm256_mul_epu32(hi1, epsilon());
    add_no_canonicalize_64_64s_s(lo2, lo1_s)
}

/// Multiply two integers modulo FIELD_ORDER.
#[inline]
unsafe fn mul(x: __m256i, y: __m256i) -> __m256i {
    shift(reduce128s_s(mul64_64_s(x, y)))
}

#[inline]
unsafe fn interleave0(x: __m256i, y: __m256i) -> (__m256i, __m256i) {
    let a = _mm256_unpacklo_epi64(x, y);
    let b = _mm256_unpackhi_epi64(x, y);
    (a, b)
}

#[inline]
unsafe fn interleave1(x: __m256i, y: __m256i) -> (__m256i, __m256i) {
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

#[cfg(test)]
mod tests {
    use crate::field::crandall_field::CrandallField;
    use crate::field::packed_crandall_avx2::*;

    const TEST_VALS_A: [CrandallField; 4] = [
        CrandallField(14479013849828404771),
        CrandallField(9087029921428221768),
        CrandallField(2441288194761790662),
        CrandallField(5646033492608483824),
    ];
    const TEST_VALS_B: [CrandallField; 4] = [
        CrandallField(17891926589593242302),
        CrandallField(11009798273260028228),
        CrandallField(2028722748960791447),
        CrandallField(7929433601095175579),
    ];

    #[test]
    fn test_add() {
        let packed_a = PackedCrandallAVX2::from_arr(TEST_VALS_A);
        let packed_b = PackedCrandallAVX2::from_arr(TEST_VALS_B);
        let packed_res = packed_a + packed_b;
        let arr_res = packed_res.to_arr();

        let expected = TEST_VALS_A
            .iter()
            .zip(TEST_VALS_B)
            .map(|(&a, b)| a + b);
        for (exp, res) in expected.zip(arr_res) {
            assert_eq!(res, exp);
        }
    }

    #[test]
    fn test_mul() {
        let packed_a = PackedCrandallAVX2::from_arr(TEST_VALS_A);
        let packed_b = PackedCrandallAVX2::from_arr(TEST_VALS_B);
        let packed_res = packed_a * packed_b;
        let arr_res = packed_res.to_arr();

        let expected = TEST_VALS_A
            .iter()
            .zip(TEST_VALS_B)
            .map(|(&a, b)| a * b);
        for (exp, res) in expected.zip(arr_res) {
            assert_eq!(res, exp);
        }
    }

    #[test]
    fn test_neg() {
        let packed_a = PackedCrandallAVX2::from_arr(TEST_VALS_A);
        let packed_res = -packed_a;
        let arr_res = packed_res.to_arr();

        let expected = TEST_VALS_A.iter().map(|&a| -a);
        for (exp, res) in expected.zip(arr_res) {
            assert_eq!(res, exp);
        }
    }

    #[test]
    fn test_sub() {
        let packed_a = PackedCrandallAVX2::from_arr(TEST_VALS_A);
        let packed_b = PackedCrandallAVX2::from_arr(TEST_VALS_B);
        let packed_res = packed_a - packed_b;
        let arr_res = packed_res.to_arr();

        let expected = TEST_VALS_A.iter().zip(TEST_VALS_B).map(|(&a, b)| a - b);
        for (exp, res) in expected.zip(arr_res) {
            assert_eq!(res, exp);
        }
    }

    #[test]
    fn test_interleave_is_involution() {
        let packed_a = PackedCrandallAVX2::from_arr(TEST_VALS_A);
        let packed_b = PackedCrandallAVX2::from_arr(TEST_VALS_B);
        {
            // Interleave, then deinterleave.
            let (x, y) = packed_a.interleave(packed_b, 0);
            let (res_a, res_b) = x.interleave(y, 0);
            assert_eq!(res_a.to_arr(), TEST_VALS_A);
            assert_eq!(res_b.to_arr(), TEST_VALS_B);
        }
        {
            let (x, y) = packed_a.interleave(packed_b, 1);
            let (res_a, res_b) = x.interleave(y, 1);
            assert_eq!(res_a.to_arr(), TEST_VALS_A);
            assert_eq!(res_b.to_arr(), TEST_VALS_B);
        }
    }

    #[test]
    fn test_interleave() {
        let in_a: [CrandallField; 4] = [
            CrandallField(00),
            CrandallField(01),
            CrandallField(02),
            CrandallField(03),
        ];
        let in_b: [CrandallField; 4] = [
            CrandallField(10),
            CrandallField(11),
            CrandallField(12),
            CrandallField(13),
        ];
        let int0_a: [CrandallField; 4] = [
            CrandallField(00),
            CrandallField(10),
            CrandallField(02),
            CrandallField(12),
        ];
        let int0_b: [CrandallField; 4] = [
            CrandallField(01),
            CrandallField(11),
            CrandallField(03),
            CrandallField(13),
        ];
        let int1_a: [CrandallField; 4] = [
            CrandallField(00),
            CrandallField(01),
            CrandallField(10),
            CrandallField(11),
        ];
        let int1_b: [CrandallField; 4] = [
            CrandallField(02),
            CrandallField(03),
            CrandallField(12),
            CrandallField(13),
        ];

        let packed_a = PackedCrandallAVX2::from_arr(in_a);
        let packed_b = PackedCrandallAVX2::from_arr(in_b);
        {
            let (x0, y0) = packed_a.interleave(packed_b, 0);
            assert_eq!(x0.to_arr(), int0_a);
            assert_eq!(y0.to_arr(), int0_b);
        }
        {
            let (x1, y1) = packed_a.interleave(packed_b, 1);
            assert_eq!(x1.to_arr(), int1_a);
            assert_eq!(y1.to_arr(), int1_b);
        }
    }
}
