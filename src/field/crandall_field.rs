use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::iter::{Product, Sum};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use num::bigint::BigUint;
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::field::extension_field::quadratic::QuadraticExtension;
use crate::field::extension_field::quartic::QuarticExtension;
use crate::field::extension_field::{Extendable, Frobenius};
use crate::field::field_types::{Field, PrimeField, RichField};
use crate::field::inversion::try_inverse_u64;

/// EPSILON = 9 * 2**28 - 1
const EPSILON: u64 = 2415919103;

/// A field designed for use with the Crandall reduction algorithm.
///
/// Its order is
/// ```ignore
/// P = 2**64 - EPSILON
///   = 2**64 - 9 * 2**28 + 1
///   = 2**28 * (2**36 - 9) + 1
/// ```
#[derive(Copy, Clone, Serialize, Deserialize)]
#[repr(transparent)] // Must be compatible with PackedCrandallAVX2
pub struct CrandallField(pub u64);

impl Default for CrandallField {
    fn default() -> Self {
        Self::ZERO
    }
}

impl PartialEq for CrandallField {
    fn eq(&self, other: &Self) -> bool {
        self.to_canonical_u64() == other.to_canonical_u64()
    }
}

impl Eq for CrandallField {}

impl Hash for CrandallField {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.to_canonical_u64())
    }
}

impl Display for CrandallField {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.to_canonical_u64(), f)
    }
}

impl Debug for CrandallField {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.to_canonical_u64(), f)
    }
}

/// BINARY_INVERSES[k] = 2^-k (mod FIELD_ORDER)
const BINARY_INVERSES: [u64; 257] = [ 1, 9223372035646816257,
13835058053470224385, 16140901062381928449, 17293822566837780481,
17870283319065706497, 18158513695179669505, 18302628883236651009,
18374686477265141761, 18410715274279387137, 18428729672786509825,
18437736872040071169, 18442240471666851841, 18444492271480242177,
18445618171386937345, 18446181121340284929, 18446462596316958721,
18446603333805295617, 18446673702549464065, 18446708886921548289,
18446726479107590401, 18446735275200611457, 18446739673247121985,
18446741872270377249, 18446742971782004881, 18446743521537818697,
18446743796415725605, 18446743933854679059, 18446744002574155786,
9223372001287077893, 13835058036290355203, 16140901053791993858,
8070450526895996929, 13258597299094814721, 15852670685194223617,
17149707378243928065, 17798225724768780289, 18122484898031206401,
18284614484662419457, 18365679277978025985, 18406211674635829249,
18426477872964730881, 18436610972129181697, 18441677521711407105,
18444210796502519809, 18445477433898076161, 18446110752595854337,
18446427411944743425, 18446585741619187969, 18446664906456410241,
18446704488875021377, 18446724280084326945, 18446734175688979729,
18446739123491306121, 18446741597392469317, 18446742834343050915,
18446743452818341714, 9223371726409170857, 13835057898851401685,
16140900985072517099, 17293822528183074806, 8646911264091537403,
13546827667692584958, 6773413833846292479, 12610078952569962496,
6305039476284981248, 3152519738142490624, 1576259869071245312,
788129934535622656, 394064967267811328, 197032483633905664,
98516241816952832, 49258120908476416, 24629060454238208,
12314530227119104, 6157265113559552, 3078632556779776,
1539316278389888, 769658139194944, 384829069597472, 192414534798736,
96207267399368, 48103633699684, 24051816849842, 12025908424921,
9223378048601028717, 13835061059947330615, 16140902565620481564,
8070451282810240782, 4035225641405120391, 11240984856349376452,
5620492428174688226, 2810246214087344113, 10628495142690488313,
14537619606992060413, 16492181839142846463, 17469462955218239488,
8734731477609119744, 4367365738804559872, 2183682869402279936,
1091841434701139968, 545920717350569984, 272960358675284992,
136480179337642496, 68240089668821248, 34120044834410624,
17060022417205312, 8530011208602656, 4265005604301328,
2132502802150664, 1066251401075332, 533125700537666, 266562850268833,
9223505317071950673, 13835124694182791593, 16140934382738212053,
17293839227015922283, 17870291649154777398, 8935145824577388699,
13690944947935510606, 6845472473967755303, 12646108272630693908,
6323054136315346954, 3161527068157673477, 10804135569725652995,
14625439820509642754, 7312719910254821377, 12879731990774226945,
15663238031033929729, 17054991051163781121, 17750867561228706817,
18098805816261169665, 18272774943777401089, 18359759507535516801,
18403251789414574657, 18424997930354103585, 18435871000823868049,
18441307536058750281, 18444025803676191397, 18445384937484911955,
18446064504389272234, 9223032252194636117, 13834888161744134315,
16140816116518883414, 8070408058259441707, 13258576064776537110,
6629288032388268555, 12538016051840950534, 6269008025920475267,
12357876048607053890, 6178938024303526945, 12312841047798579729,
15379792559546106121, 16913268315419869317, 17680006193356750915,
18063375132325191714, 9031687566162595857, 13739215818728114185,
16092979945010873349, 17269862008152252931, 17858303039722942722,
8929151519861471361, 13687947795577551937, 16067345933435592225,
17257045002364612369, 17851894536829122441, 18149319304061377477,
18298031687677504995, 18372387879485568754, 9186193939742784377,
13816469005518208445, 16131606538405920479, 17289175304849776496,
8644587652424888248, 4322293826212444124, 2161146913106222062,
1080573456553111031, 9763658763923371772, 4881829381961685886,
2440914690980842943, 10443829381137237728, 5221914690568618864,
2610957345284309432, 1305478672642154716, 652739336321077358,
326369668160538679, 9386556869727085596, 4693278434863542798,
2346639217431771399, 10396691644362701956, 5198345822181350978,
2599172911090675489, 10522958491192154001, 14484851281242893257,
16465797676268262885, 17456270873780947699, 17951507472537290106,
8975753736268645053, 13711248903781138783, 16078996487537385648,
8039498243768692824, 4019749121884346412, 2009874560942173206,
1004937280471086603, 9725840675882359558, 4862920337941179779,
11654832204617406146, 5827416102308703073, 12137080086801167793,
15291912079047400153, 16869328075170516333, 17658036073232074423,
18052390072262853468, 9026195036131426734, 4513097518065713367,
11479920794679672940, 5739960397339836470, 2869980198669918235,
10658362134981775374, 5329181067490887687, 11887962569392260100,
5943981284696130050, 2971990642348065025, 10709367356820848769,
14578055714057240641, 16512399892675436577, 17479571981984534545,
17963158026639083529, 18204951048966358021, 18325847560129995267,
18386295815711813890, 9193147907855906945, 13819945989574769729,
16133345030434201121, 17290044550863916817, 17868394311078774665,
18157569191186203589, 18302156631239918051, 18374450351266775282,
9187225175633387641, 13816984623463510077, 16131864347378571295,
17289304209336101904, 8644652104668050952, 4322326052334025476,
2161163026167012738, 1080581513083506369, 9763662792188569441,
14105203431741100977, 16275973751517366745, 17361358911405499629,
17904051491349566071, 18175397781321599292, 9087698890660799646,
4543849445330399823, 11495296758312016168, 5747648379156008084];

impl Field for CrandallField {
    type PrimeField = Self;

    const ZERO: Self = Self(0);
    const ONE: Self = Self(1);
    const TWO: Self = Self(2);
    const NEG_ONE: Self = Self(Self::ORDER - 1);

    const CHARACTERISTIC: u64 = Self::ORDER;
    const TWO_ADICITY: usize = 28;

    const MULTIPLICATIVE_GROUP_GENERATOR: Self = Self(5);
    const POWER_OF_TWO_GENERATOR: Self = Self(10281950781551402419);

    fn order() -> BigUint {
        Self::ORDER.into()
    }

    #[allow(clippy::many_single_char_names)]
    fn try_inverse(&self) -> Option<Self> {
        // The algorithm below is the "plus-minus-inversion" method
        // with an "almost Montgomery inverse" flair. See Handbook of
        // Elliptic and Hyperelliptic Cryptography, Algorithms 11.6
        // and 11.12.

        let mut f = self.0;
        let mut g = Self::ORDER;
        // NB: These two are very rarely such that their absolute
        // value exceeds (p-1)/2; we are paying the price of i128 for
        // the whole calculation, just for the times they do though.
        let mut c = 1i64 as i128;
        let mut d = 0i64 as i128;

        if f == 0 {
            return None;
        }

        let mut k = f.trailing_zeros();
        f >>= k;

        if f < g {
            (f, g) = (g, f);
            (c, d) = (d, c);
        }
        if f & 3 == g & 3 {
            // f - g = 0 (mod 4)
            f -= g;
            c -= d;

            // kk >= 2
            let kk = f.trailing_zeros();
            f >>= kk;
            d <<= kk;
            k += kk;
        } else {
            // f + g = 0 (mod 4)
            c += d;
            f = (f >> 2) + (g >> 2) + 1u64;
            let kk = f.trailing_zeros();
            f >>= kk;
            d <<= kk + 2;
            k += kk + 2;
        }

        if f == 1 {
            if c < 0 {
                c += Self::ORDER as i128;
            }
            return Some(Self(c as u64) * Self(BINARY_INVERSES[k as usize]));
        }

        if f < g {
            (f, g) = (g, f);
            (c, d) = (d, c);
        }
        if f & 3 == g & 3 {
            // f - g = 0 (mod 4)
            f -= g;
            c -= d;

            // kk >= 2
            let kk = f.trailing_zeros();
            f >>= kk;
            d <<= kk;
            k += kk;
        } else {
            // f + g = 0 (mod 4)
            c += d;
            f = (f >> 2) + (g >> 2) + 1u64;
            let kk = f.trailing_zeros();
            f >>= kk;
            d <<= kk + 2;
            k += kk + 2;
        }


        //println!("\n\ninput = {}", self.0);
        loop {
            /*
            let f_bits = crate::util::bits_u64(f);
            let g_bits = crate::util::bits_u64(g);
            let c_bits = c.unsigned_abs().next_power_of_two().trailing_zeros() as usize;
            let d_bits = d.unsigned_abs().next_power_of_two().trailing_zeros() as usize;

            if self.0 == 2147483642u64 {
                println!("{:2}  {:2}  {}{:3}  {}{:3}  {:3}",
                         f_bits, g_bits,
                         if c < 0 { "-" } else { "+" }, c_bits,
                         if c < 0 { "-" } else { "+" }, d_bits,
                         k);
            }
            */

            if f == 1 {
                break;
            }

            if f < g {
                (f, g) = (g, f);
                (c, d) = (d, c);
            }
            if f & 3 == g & 3 {
                // f - g = 0 (mod 4)
                f -= g;
                c -= d;
            } else {
                // f + g = 0 (mod 4)
                f += g;
                c += d;
            }

            // kk >= 2
            let kk = f.trailing_zeros();
            f >>= kk;
            d <<= kk;
            k += kk;
        }

        // TODO: document maximum number of iterations (it's at least 2)
        while c < 0 {
            c += Self::ORDER as i128;
        }

        // TODO: c can be greater than ORDER; document maximum number
        // of iterations (it's at least 1).
        while c >= Self::ORDER as i128 {
            c -= Self::ORDER as i128;
        }

        Some(Self(c as u64) * Self(BINARY_INVERSES[k as usize]))
    }

    #[inline]
    fn from_canonical_u64(n: u64) -> Self {
        Self(n)
    }

    #[inline]
    fn from_noncanonical_u128(n: u128) -> Self {
        reduce128(n)
    }

    #[inline]
    fn from_noncanonical_u96(n: (u64, u32)) -> Self {
        reduce96(n)
    }

    fn rand_from_rng<R: Rng>(rng: &mut R) -> Self {
        Self::from_canonical_u64(rng.gen_range(0..Self::ORDER))
    }

    fn cube_root(&self) -> Self {
        let x0 = *self;
        let x1 = x0.square();
        let x2 = x1.square();
        let x3 = x2 * x0;
        let x4 = x3.square();
        let x5 = x4.square();
        let x7 = x5.square();
        let x8 = x7.square();
        let x9 = x8.square();
        let x10 = x9.square();
        let x11 = x10 * x5;
        let x12 = x11.square();
        let x13 = x12.square();
        let x14 = x13.square();
        let x16 = x14.square();
        let x17 = x16.square();
        let x18 = x17.square();
        let x19 = x18.square();
        let x20 = x19.square();
        let x21 = x20 * x11;
        let x22 = x21.square();
        let x23 = x22.square();
        let x24 = x23.square();
        let x25 = x24.square();
        let x26 = x25.square();
        let x27 = x26.square();
        let x28 = x27.square();
        let x29 = x28.square();
        let x30 = x29.square();
        let x31 = x30.square();
        let x32 = x31.square();
        let x33 = x32 * x14;
        let x34 = x33 * x3;
        let x35 = x34.square();
        let x36 = x35 * x34;
        let x37 = x36 * x5;
        let x38 = x37 * x34;
        let x39 = x38 * x37;
        let x40 = x39.square();
        let x41 = x40.square();
        let x42 = x41 * x38;
        let x43 = x42.square();
        let x44 = x43.square();
        let x45 = x44.square();
        let x46 = x45.square();
        let x47 = x46.square();
        let x48 = x47.square();
        let x49 = x48.square();
        let x50 = x49.square();
        let x51 = x50.square();
        let x52 = x51.square();
        let x53 = x52.square();
        let x54 = x53.square();
        let x55 = x54.square();
        let x56 = x55.square();
        let x57 = x56.square();
        let x58 = x57.square();
        let x59 = x58.square();
        let x60 = x59.square();
        let x61 = x60.square();
        let x62 = x61.square();
        let x63 = x62.square();
        let x64 = x63.square();
        let x65 = x64.square();
        let x66 = x65.square();
        let x67 = x66.square();
        let x68 = x67.square();
        let x69 = x68.square();
        let x70 = x69.square();
        let x71 = x70.square();
        let x72 = x71.square();
        let x73 = x72.square();
        let x74 = x73 * x39;
        x74
    }

    #[inline]
    fn multiply_accumulate(&self, x: Self, y: Self) -> Self {
        // u64 + u64 * u64 cannot overflow.
        reduce128((self.0 as u128) + (x.0 as u128) * (y.0 as u128))
    }
}

impl PrimeField for CrandallField {
    const ORDER: u64 = 18446744071293632513;

    #[inline]
    fn to_canonical_u64(&self) -> u64 {
        let mut c = self.0;
        // We only need one condition subtraction, since 2 * ORDER would not fit in a u64.
        if c >= Self::ORDER {
            c -= Self::ORDER;
        }
        c
    }

    #[inline]
    fn to_noncanonical_u64(&self) -> u64 {
        self.0
    }

    #[inline]
    fn from_noncanonical_u64(n: u64) -> Self {
        Self(n)
    }

    #[inline]
    /// Faster addition for when we know that rhs <= ORDER. If this is the case, then the
    /// .to_canonical_u64() that addition usually performs is unnecessary. Omitting it saves three
    /// instructions. This function is marked unsafe because it may yield incorrect results if the
    /// condition is not satisfied.
    unsafe fn add_canonical_u64(&self, rhs: u64) -> Self {
        let (sum, over) = self.0.overflowing_add(rhs);
        Self(sum.wrapping_sub((over as u64) * Self::ORDER))
    }
}

impl Neg for CrandallField {
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

impl Add for CrandallField {
    type Output = Self;

    #[inline]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn add(self, rhs: Self) -> Self {
        let rhs_canonical = rhs.to_canonical_u64();
        // rhs_canonical is definitely canonical, so below is safe.
        unsafe { self.add_canonical_u64(rhs_canonical) }
    }
}

impl AddAssign for CrandallField {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sum for CrandallField {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ZERO, |acc, x| acc + x)
    }
}

impl Sub for CrandallField {
    type Output = Self;

    #[inline]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn sub(self, rhs: Self) -> Self {
        let (diff, under) = self.0.overflowing_sub(rhs.to_canonical_u64());
        Self(diff.wrapping_add((under as u64) * Self::ORDER))
    }
}

impl SubAssign for CrandallField {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul for CrandallField {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Self) -> Self {
        reduce128((self.0 as u128) * (rhs.0 as u128))
    }
}

impl MulAssign for CrandallField {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl Product for CrandallField {
    #[inline]
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.reduce(|acc, x| acc * x).unwrap_or(Self::ONE)
    }
}

impl Div for CrandallField {
    type Output = Self;

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn div(self, rhs: Self) -> Self::Output {
        self * rhs.inverse()
    }
}

impl DivAssign for CrandallField {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

impl Extendable<2> for CrandallField {
    type Extension = QuadraticExtension<Self>;

    // Verifiable in Sage with
    // `R.<x> = GF(p)[]; assert (x^2 - 3).is_irreducible()`.
    const W: Self = Self(3);

    const EXT_MULTIPLICATIVE_GROUP_GENERATOR: [Self; 2] =
        [Self(6483724566312148654), Self(12194665049945415126)];

    const EXT_POWER_OF_TWO_GENERATOR: [Self; 2] = [Self(0), Self(14420468973723774561)];
}

impl Extendable<4> for CrandallField {
    type Extension = QuarticExtension<Self>;

    const W: Self = Self(3);

    const EXT_MULTIPLICATIVE_GROUP_GENERATOR: [Self; 4] = [
        Self(12476589904174392631),
        Self(896937834427772243),
        Self(7795248119019507390),
        Self(9005769437373554825),
    ];

    const EXT_POWER_OF_TWO_GENERATOR: [Self; 4] =
        [Self(0), Self(0), Self(0), Self(15170983443234254033)];
}

impl RichField for CrandallField {}

/// Reduces to a 64-bit value. The result might not be in canonical form; it could be in between the
/// field order and `2^64`.
#[inline]
fn reduce128(x: u128) -> CrandallField {
    // This is Crandall's algorithm. When we have some high-order bits (i.e. with a weight of 2^64),
    // we convert them to low-order bits by multiplying by EPSILON (the logic is a simple
    // generalization of Mersenne prime reduction). The first time we do this, the product will take
    // ~96 bits, so we still have some high-order bits. But when we repeat this another time, the
    // product will fit in 64 bits.
    let (lo_1, hi_1) = split(x);
    let (lo_2, hi_2) = split((EPSILON as u128) * (hi_1 as u128) + (lo_1 as u128));
    reduce96((lo_2, hi_2 as u32)) // hi_2 will always fit in a u32.
}

/// Reduces to a 64-bit value. The result might not be in canonical form; it could be in between the
/// field order and `2^64`.
#[inline]
fn reduce96((x_lo, x_hi): (u64, u32)) -> CrandallField {
    // This is Crandall's algorithm. See reduce128.
    let t = (x_hi as u64) * EPSILON;

    unsafe {
        // This is safe to do because x_lo + t < 2^64 + Self::ORDER. Notice that x_hi <= 2^32 - 1.
        // Then t = x_hi * EPSILON <= (2^32 - 1) * EPSILON < Self::ORDER.
        // Use of standard addition here would make multiplication 20% more expensive.
        CrandallField(x_lo).add_canonical_u64(t)
    }
}

#[inline]
fn split(x: u128) -> (u64, u64) {
    (x as u64, (x >> 64) as u64)
}

impl Frobenius<1> for CrandallField {}

#[cfg(test)]
mod tests {
    use crate::{test_field_arithmetic, test_prime_field_arithmetic};

    test_prime_field_arithmetic!(crate::field::crandall_field::CrandallField);
    test_field_arithmetic!(crate::field::crandall_field::CrandallField);
}
