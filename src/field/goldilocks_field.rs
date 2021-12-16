use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::iter::{Product, Sum};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use num::{BigUint, Integer};
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::field::extension_field::quadratic::QuadraticExtension;
use crate::field::extension_field::quartic::QuarticExtension;
use crate::field::extension_field::{Extendable, Frobenius};
use crate::field::field_types::{Field, Powers, PrimeField, RichField};
use crate::field::inversion::try_inverse_u64;
use crate::util::{assume, branch_hint};

const EPSILON: u64 = (1 << 32) - 1;

/// A field selected to have fast reduction.
///
/// Its order is 2^64 - 2^32 + 1.
/// ```ignore
/// P = 2**64 - EPSILON
///   = 2**64 - 2**32 + 1
///   = 2**32 * (2**32 - 1) + 1
/// ```
#[derive(Copy, Clone, Serialize, Deserialize)]
#[repr(transparent)]
pub struct GoldilocksField(pub u64);

impl GoldilocksField {
    const INVS2: [Self; 139] = [
        Self(1),
        Self(9223372034707292161),
        Self(13835058052060938241),
        Self(16140901060737761281),
        Self(17293822565076172801),
        Self(17870283317245378561),
        Self(18158513693329981441),
        Self(18302628881372282881),
        Self(18374686475393433601),
        Self(18410715272404008961),
        Self(18428729670909296641),
        Self(18437736870161940481),
        Self(18442240469788262401),
        Self(18444492269601423361),
        Self(18445618169508003841),
        Self(18446181119461294081),
        Self(18446462594437939201),
        Self(18446603331926261761),
        Self(18446673700670423041),
        Self(18446708885042503681),
        Self(18446726477228544001),
        Self(18446735273321564161),
        Self(18446739671368074241),
        Self(18446741870391329281),
        Self(18446742969902956801),
        Self(18446743519658770561),
        Self(18446743794536677441),
        Self(18446743931975630881),
        Self(18446744000695107601),
        Self(18446744035054845961),
        Self(18446744052234715141),
        Self(18446744060824649731),
        Self(18446744065119617026),
        Self(9223372032559808513),
        Self(13835058050987196417),
        Self(16140901060200890369),
        Self(17293822564807737345),
        Self(17870283317111160833),
        Self(18158513693262872577),
        Self(18302628881338728449),
        Self(18374686475376656385),
        Self(18410715272395620353),
        Self(18428729670905102337),
        Self(18437736870159843329),
        Self(18442240469787213825),
        Self(18444492269600899073),
        Self(18445618169507741697),
        Self(18446181119461163009),
        Self(18446462594437873665),
        Self(18446603331926228993),
        Self(18446673700670406657),
        Self(18446708885042495489),
        Self(18446726477228539905),
        Self(18446735273321562113),
        Self(18446739671368073217),
        Self(18446741870391328769),
        Self(18446742969902956545),
        Self(18446743519658770433),
        Self(18446743794536677377),
        Self(18446743931975630849),
        Self(18446744000695107585),
        Self(18446744035054845953),
        Self(18446744052234715137),
        Self(18446744060824649729),
        Self(18446744065119617025),
        Self(18446744067267100673),
        Self(18446744068340842497),
        Self(18446744068877713409),
        Self(18446744069146148865),
        Self(18446744069280366593),
        Self(18446744069347475457),
        Self(18446744069381029889),
        Self(18446744069397807105),
        Self(18446744069406195713),
        Self(18446744069410390017),
        Self(18446744069412487169),
        Self(18446744069413535745),
        Self(18446744069414060033),
        Self(18446744069414322177),
        Self(18446744069414453249),
        Self(18446744069414518785),
        Self(18446744069414551553),
        Self(18446744069414567937),
        Self(18446744069414576129),
        Self(18446744069414580225),
        Self(18446744069414582273),
        Self(18446744069414583297),
        Self(18446744069414583809),
        Self(18446744069414584065),
        Self(18446744069414584193),
        Self(18446744069414584257),
        Self(18446744069414584289),
        Self(18446744069414584305),
        Self(18446744069414584313),
        Self(18446744069414584317),
        Self(18446744069414584319),
        Self(18446744069414584320),
        Self(9223372034707292160),
        Self(4611686017353646080),
        Self(2305843008676823040),
        Self(1152921504338411520),
        Self(576460752169205760),
        Self(288230376084602880),
        Self(144115188042301440),
        Self(72057594021150720),
        Self(36028797010575360),
        Self(18014398505287680),
        Self(9007199252643840),
        Self(4503599626321920),
        Self(2251799813160960),
        Self(1125899906580480),
        Self(562949953290240),
        Self(281474976645120),
        Self(140737488322560),
        Self(70368744161280),
        Self(35184372080640),
        Self(17592186040320),
        Self(8796093020160),
        Self(4398046510080),
        Self(2199023255040),
        Self(1099511627520),
        Self(549755813760),
        Self(274877906880),
        Self(137438953440),
        Self(68719476720),
        Self(34359738360),
        Self(17179869180),
        Self(8589934590),
        Self(4294967295),
        Self(9223372036854775808),
        Self(4611686018427387904),
        Self(2305843009213693952),
        Self(1152921504606846976),
        Self(576460752303423488),
        Self(288230376151711744),
        Self(144115188075855872),
        Self(72057594037927936),
        Self(36028797018963968),
        Self(18014398509481984),
    ];
}

impl Default for GoldilocksField {
    fn default() -> Self {
        Self::ZERO
    }
}

impl PartialEq for GoldilocksField {
    fn eq(&self, other: &Self) -> bool {
        self.to_canonical_u64() == other.to_canonical_u64()
    }
}

impl Eq for GoldilocksField {}

impl Hash for GoldilocksField {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.to_canonical_u64())
    }
}

impl Display for GoldilocksField {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Debug for GoldilocksField {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl Field for GoldilocksField {
    const ZERO: Self = Self(0);
    const ONE: Self = Self(1);
    const TWO: Self = Self(2);
    const NEG_ONE: Self = Self(Self::ORDER - 1);

    const TWO_ADICITY: usize = 32;
    const CHARACTERISTIC_TWO_ADICITY: usize = Self::TWO_ADICITY;

    // Sage: `g = GF(p).multiplicative_generator()`
    const MULTIPLICATIVE_GROUP_GENERATOR: Self = Self(7);

    // Sage:
    // ```
    // g_2 = g^((p - 1) / 2^32)
    // g_2.multiplicative_order().factor()
    // ```
    const POWER_OF_TWO_GENERATOR: Self = Self(1753635133440165772);

    const BITS: usize = 64;

    fn order() -> BigUint {
        Self::ORDER.into()
    }
    fn characteristic() -> BigUint {
        Self::order()
    }

    fn inverse_2exp(exp: usize) -> Self {
        Self::INVS2[exp]
    }

    #[inline(always)]
    fn try_inverse(&self) -> Option<Self> {
        try_inverse_u64(self)
    }

    fn from_biguint(n: BigUint) -> Self {
        Self(n.mod_floor(&Self::order()).to_u64_digits()[0])
    }

    fn to_biguint(&self) -> BigUint {
        self.to_canonical_u64().into()
    }

    #[inline]
    fn from_canonical_u64(n: u64) -> Self {
        debug_assert!(n < Self::ORDER);
        Self(n)
    }

    fn from_noncanonical_u128(n: u128) -> Self {
        reduce128(n)
    }

    fn rand_from_rng<R: Rng>(rng: &mut R) -> Self {
        Self::from_canonical_u64(rng.gen_range(0..Self::ORDER))
    }

    #[inline]
    fn multiply_accumulate(&self, x: Self, y: Self) -> Self {
        // u64 + u64 * u64 cannot overflow.
        reduce128((self.0 as u128) + (x.0 as u128) * (y.0 as u128))
    }
}

impl PrimeField for GoldilocksField {
    const ORDER: u64 = 0xFFFFFFFF00000001;

    #[inline]
    fn to_canonical_u64(&self) -> u64 {
        let mut c = self.0;
        // We only need one condition subtraction, since 2 * ORDER would not fit in a u64.
        if c >= Self::ORDER {
            c -= Self::ORDER;
        }
        c
    }

    fn to_noncanonical_u64(&self) -> u64 {
        self.0
    }

    #[inline]
    fn from_noncanonical_u64(n: u64) -> Self {
        Self(n)
    }

    #[inline]
    unsafe fn add_canonical_u64(&self, rhs: u64) -> Self {
        let (res_wrapped, carry) = self.0.overflowing_add(rhs);
        // Add EPSILON * carry cannot overflow unless rhs is not in canonical form.
        Self(res_wrapped + EPSILON * (carry as u64))
    }

    #[inline]
    unsafe fn sub_canonical_u64(&self, rhs: u64) -> Self {
        let (res_wrapped, borrow) = self.0.overflowing_sub(rhs);
        // Sub EPSILON * carry cannot underflow unless rhs is not in canonical form.
        Self(res_wrapped - EPSILON * (borrow as u64))
    }
}

impl Neg for GoldilocksField {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        if self.is_zero() {
            Self::ZERO
        } else {
            Self(Self::ORDER - self.to_canonical_u64())
        }
    }
}

impl Add for GoldilocksField {
    type Output = Self;

    #[inline]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn add(self, rhs: Self) -> Self {
        let (sum, over) = self.0.overflowing_add(rhs.0);
        let (mut sum, over) = sum.overflowing_add((over as u64) * EPSILON);
        if over {
            // NB: self.0 > Self::ORDER && rhs.0 > Self::ORDER is necessary but not sufficient for
            // double-overflow.
            // This assume does two things:
            //  1. If compiler knows that either self.0 or rhs.0 <= ORDER, then it can skip this
            //     check.
            //  2. Hints to the compiler how rare this double-overflow is (thus handled better with
            //     a branch).
            assume(self.0 > Self::ORDER && rhs.0 > Self::ORDER);
            branch_hint();
            sum += EPSILON; // Cannot overflow.
        }
        Self(sum)
    }
}

impl AddAssign for GoldilocksField {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sum for GoldilocksField {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ZERO, |acc, x| acc + x)
    }
}

impl Sub for GoldilocksField {
    type Output = Self;

    #[inline]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn sub(self, rhs: Self) -> Self {
        let (diff, under) = self.0.overflowing_sub(rhs.0);
        let (mut diff, under) = diff.overflowing_sub((under as u64) * EPSILON);
        if under {
            // NB: self.0 < EPSILON - 1 && rhs.0 > Self::ORDER is necessary but not sufficient for
            // double-underflow.
            // This assume does two things:
            //  1. If compiler knows that either self.0 >= EPSILON - 1 or rhs.0 <= ORDER, then it
            //     can skip this check.
            //  2. Hints to the compiler how rare this double-underflow is (thus handled better
            //     with a branch).
            assume(self.0 < EPSILON - 1 && rhs.0 > Self::ORDER);
            branch_hint();
            diff -= EPSILON; // Cannot underflow.
        }
        Self(diff)
    }
}

impl SubAssign for GoldilocksField {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul for GoldilocksField {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Self) -> Self {
        reduce128((self.0 as u128) * (rhs.0 as u128))
    }
}

impl MulAssign for GoldilocksField {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl Product for GoldilocksField {
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ONE, |acc, x| acc * x)
    }
}

impl Div for GoldilocksField {
    type Output = Self;

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn div(self, rhs: Self) -> Self::Output {
        self * rhs.inverse()
    }
}

impl DivAssign for GoldilocksField {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

impl Extendable<2> for GoldilocksField {
    type Extension = QuadraticExtension<Self>;

    // Verifiable in Sage with
    // `R.<x> = GF(p)[]; assert (x^2 - 7).is_irreducible()`.
    const W: Self = Self(7);

    // DTH_ROOT = W^((ORDER - 1)/2)
    const DTH_ROOT: Self = Self(18446744069414584320);

    const EXT_MULTIPLICATIVE_GROUP_GENERATOR: [Self; 2] =
        [Self(18081566051660590251), Self(16121475356294670766)];

    const EXT_POWER_OF_TWO_GENERATOR: [Self; 2] = [Self(0), Self(15659105665374529263)];
}

impl Extendable<4> for GoldilocksField {
    type Extension = QuarticExtension<Self>;

    const W: Self = Self(7);

    // DTH_ROOT = W^((ORDER - 1)/4)
    const DTH_ROOT: Self = Self(281474976710656);

    const EXT_MULTIPLICATIVE_GROUP_GENERATOR: [Self; 4] = [
        Self(5024755240244648895),
        Self(13227474371289740625),
        Self(3912887029498544536),
        Self(3900057112666848848),
    ];

    const EXT_POWER_OF_TWO_GENERATOR: [Self; 4] =
        [Self(0), Self(0), Self(0), Self(12587610116473453104)];
}

impl RichField for GoldilocksField {}

/// Fast addition modulo ORDER for x86-64.
/// This function is marked unsafe for the following reasons:
///   - It is only correct if x + y < 2**64 + ORDER = 0x1ffffffff00000001.
///   - It is only faster in some circumstances. In particular, on x86 it overwrites both inputs in
///     the registers, so its use is not recommended when either input will be used again.
#[inline(always)]
#[cfg(target_arch = "x86_64")]
unsafe fn add_no_canonicalize_trashing_input(x: u64, y: u64) -> u64 {
    use std::arch::asm;
    let res_wrapped: u64;
    let adjustment: u64;
    asm!(
        "add {0}, {1}",
        // Trick. The carry flag is set iff the addition overflowed.
        // sbb x, y does x := x - y - CF. In our case, x and y are both {1:e}, so it simply does
        // {1:e} := 0xffffffff on overflow and {1:e} := 0 otherwise. {1:e} is the low 32 bits of
        // {1}; the high 32-bits are zeroed on write. In the end, we end up with 0xffffffff in {1}
        // on overflow; this happens be EPSILON.
        // Note that the CPU does not realize that the result of sbb x, x does not actually depend
        // on x. We must write the result to a register that we know to be ready. We have a
        // dependency on {1} anyway, so let's use it.
        "sbb {1:e}, {1:e}",
        inlateout(reg) x => res_wrapped,
        inlateout(reg) y => adjustment,
        options(pure, nomem, nostack),
    );
    assume(x != 0 || (res_wrapped == y && adjustment == 0));
    assume(y != 0 || (res_wrapped == x && adjustment == 0));
    // Add EPSILON == subtract ORDER.
    // Cannot overflow unless the assumption if x + y < 2**64 + ORDER is incorrect.
    res_wrapped + adjustment
}

#[inline(always)]
#[cfg(not(target_arch = "x86_64"))]
unsafe fn add_no_canonicalize_trashing_input(x: u64, y: u64) -> u64 {
    let (res_wrapped, carry) = x.overflowing_add(y);
    // Below cannot overflow unless the assumption if x + y < 2**64 + ORDER is incorrect.
    res_wrapped + EPSILON * (carry as u64)
}

/// Fast subtraction modulo ORDER for x86-64.
/// This function is marked unsafe for the following reasons:
///   - It is only correct if x - y >= -ORDER.
///   - It is only faster in some circumstances. In particular, on x86 it overwrites both inputs in
///     the registers, so its use is not recommended when either input will be used again.
#[inline(always)]
#[cfg(target_arch = "x86_64")]
unsafe fn sub_no_canonicalize_trashing_input(x: u64, y: u64) -> u64 {
    use std::arch::asm;
    let res_wrapped: u64;
    let adjustment: u64;
    asm!(
        "sub {0}, {1}",
        "sbb {1:e}, {1:e}", // See add_no_canonicalize_trashing_input.
        inlateout(reg) x => res_wrapped,
        inlateout(reg) y => adjustment,
        options(pure, nomem, nostack),
    );
    assume(y != 0 || (res_wrapped == x && adjustment == 0));
    // Subtract EPSILON == add ORDER.
    // Cannot underflow unless the assumption x - y >= -ORDER is incorrect.
    res_wrapped - adjustment
}

#[inline(always)]
#[cfg(not(target_arch = "x86_64"))]
unsafe fn sub_no_canonicalize_trashing_input(x: u64, y: u64) -> u64 {
    let (res_wrapped, borrow) = x.overflowing_sub(y);
    // Below cannot underflow unless the assumption x - y >= -ORDER is incorrect.
    res_wrapped - EPSILON * (borrow as u64)
}

/// Reduces to a 64-bit value. The result might not be in canonical form; it could be in between the
/// field order and `2^64`.
#[inline]
fn reduce128(x: u128) -> GoldilocksField {
    let (x_lo, x_hi) = split(x); // This is a no-op
    let x_hi_hi = x_hi >> 32;
    let x_hi_lo = x_hi & EPSILON;

    let t0 = unsafe { sub_no_canonicalize_trashing_input(x_lo, x_hi_hi) };
    let t1 = x_hi_lo * EPSILON;
    let t2 = unsafe { add_no_canonicalize_trashing_input(t0, t1) };
    GoldilocksField(t2)
}

#[inline]
fn split(x: u128) -> (u64, u64) {
    (x as u64, (x >> 64) as u64)
}

impl Frobenius<1> for GoldilocksField {}

#[cfg(test)]
mod tests {
    use crate::{test_field_arithmetic, test_prime_field_arithmetic};

    test_prime_field_arithmetic!(crate::field::goldilocks_field::GoldilocksField);
    test_field_arithmetic!(crate::field::goldilocks_field::GoldilocksField);
}
