use std::fmt::Debug;
use std::ops::{Add, Div, Mul, Neg, Sub};

use ethereum_types::{U256, U512};
use rand::distributions::{Distribution, Standard};
use rand::Rng;

pub trait FieldExt:
    Copy
    + std::fmt::Debug
    + std::cmp::PartialEq
    + std::ops::Add<Output = Self>
    + std::ops::Neg<Output = Self>
    + std::ops::Sub<Output = Self>
    + std::ops::Mul<Output = Self>
    + std::ops::Div<Output = Self>
{
    const ZERO: Self;
    const UNIT: Self;
    fn new(val: usize) -> Self;
    fn inv(self) -> Self;
}

pub(crate) const BN_BASE: U256 = U256([
    0x3c208c16d87cfd47,
    0x97816a916871ca8d,
    0xb85045b68181585d,
    0x30644e72e131a029,
]);

#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) struct BN254 {
    pub val: U256,
}

impl Distribution<BN254> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> BN254 {
        let xs = rng.gen::<[u64; 4]>();
        BN254 {
            val: U256(xs) % BN_BASE,
        }
    }
}

impl Add for BN254 {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        BN254 {
            val: (self.val + other.val) % BN_BASE,
        }
    }
}

impl Neg for BN254 {
    type Output = Self;

    fn neg(self) -> Self::Output {
        BN254 {
            val: (BN_BASE - self.val) % BN_BASE,
        }
    }
}

impl Sub for BN254 {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        BN254 {
            val: (BN_BASE + self.val - other.val) % BN_BASE,
        }
    }
}

#[allow(clippy::suspicious_arithmetic_impl)]
impl Mul for BN254 {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        BN254 {
            val: U256::try_from((self.val).full_mul(other.val) % BN_BASE).unwrap(),
        }
    }
}

impl FieldExt for BN254 {
    const ZERO: Self = BN254 { val: U256::zero() };
    const UNIT: Self = BN254 { val: U256::one() };
    fn new(val: usize) -> BN254 {
        BN254 {
            val: U256::from(val),
        }
    }
    fn inv(self) -> BN254 {
        let exp = BN_BASE - 2;
        let mut current = self;
        let mut product = BN254 { val: U256::one() };
        for j in 0..256 {
            if exp.bit(j) {
                product = product * current;
            }
            current = current * current;
        }
        product
    }
}

#[allow(clippy::suspicious_arithmetic_impl)]
impl Div for BN254 {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        self * rhs.inv()
    }
}

pub(crate) const BLS_BASE: U512 = U512([
    0xb9feffffffffaaab,
    0x1eabfffeb153ffff,
    0x6730d2a0f6b0f624,
    0x64774b84f38512bf,
    0x4b1ba7b6434bacd7,
    0x1a0111ea397fe69a,
    0x0,
    0x0,
]);

#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) struct BLS381 {
    pub val: U512,
}

impl BLS381 {
    pub(crate) fn lo(self) -> U256 {
        U256(self.val.0[..4].try_into().unwrap())
    }

    pub(crate) fn hi(self) -> U256 {
        U256(self.val.0[4..].try_into().unwrap())
    }
}

impl Distribution<BLS381> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> BLS381 {
        let xs = rng.gen::<[u64; 8]>();
        BLS381 {
            val: U512(xs) % BLS_BASE,
        }
    }
}

impl Add for BLS381 {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        BLS381 {
            val: (self.val + other.val) % BLS_BASE,
        }
    }
}

impl Neg for BLS381 {
    type Output = Self;

    fn neg(self) -> Self::Output {
        BLS381 {
            val: (BLS_BASE - self.val) % BLS_BASE,
        }
    }
}

impl Sub for BLS381 {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        BLS381 {
            val: (BLS_BASE + self.val - other.val) % BLS_BASE,
        }
    }
}

impl BLS381 {
    fn lsh_128(self) -> BLS381 {
        let b128: U512 = U512([0, 0, 1, 0, 0, 0, 0, 0]);
        // since BLS_BASE < 2^384, multiplying by 2^128 doesn't overflow the U512
        BLS381 {
            val: self.val.saturating_mul(b128) % BLS_BASE,
        }
    }

    fn lsh_256(self) -> BLS381 {
        self.lsh_128().lsh_128()
    }

    fn lsh_512(self) -> BLS381 {
        self.lsh_256().lsh_256()
    }
}

#[allow(clippy::suspicious_arithmetic_impl)]
impl Mul for BLS381 {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        // x1, y1 are at most ((q-1) // 2^256) < 2^125
        let x0 = U512::from(self.lo());
        let x1 = U512::from(self.hi());
        let y0 = U512::from(other.lo());
        let y1 = U512::from(other.hi());

        let z00 = BLS381 {
            val: x0.saturating_mul(y0) % BLS_BASE,
        };
        let z01 = BLS381 {
            val: x0.saturating_mul(y1),
        };
        let z10 = BLS381 {
            val: x1.saturating_mul(y0),
        };
        let z11 = BLS381 {
            val: x1.saturating_mul(y1),
        };

        z00 + (z01 + z10).lsh_256() + z11.lsh_512()
    }
}

impl FieldExt for BLS381 {
    const ZERO: Self = BLS381 { val: U512::zero() };
    const UNIT: Self = BLS381 { val: U512::one() };
    fn new(val: usize) -> BLS381 {
        BLS381 {
            val: U512::from(val),
        }
    }
    fn inv(self) -> BLS381 {
        let exp = BLS_BASE - 2;
        let mut current = self;
        let mut product = BLS381 { val: U512::one() };

        for j in 0..512 {
            if exp.bit(j) {
                product = product * current;
            }
            current = current * current;
        }
        product
    }
}

#[allow(clippy::suspicious_arithmetic_impl)]
impl Div for BLS381 {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        self * rhs.inv()
    }
}

/// The degree 2 field extension Fp2 is given by adjoining i, the square root of -1, to BN254
/// The arithmetic in this extension is standard complex arithmetic
#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) struct Fp2<T>
where
    T: FieldExt,
{
    pub re: T,
    pub im: T,
}

impl<T> Distribution<Fp2<T>> for Standard
where
    T: FieldExt,
    Standard: Distribution<T>,
{
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Fp2<T> {
        let (re, im) = rng.gen::<(T, T)>();
        Fp2 { re, im }
    }
}

impl<T: FieldExt> Add for Fp2<T> {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Fp2 {
            re: self.re + other.re,
            im: self.im + other.im,
        }
    }
}

impl<T: FieldExt> Neg for Fp2<T> {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Fp2 {
            re: -self.re,
            im: -self.im,
        }
    }
}

impl<T: FieldExt> Sub for Fp2<T> {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Fp2 {
            re: self.re - other.re,
            im: self.im - other.im,
        }
    }
}

impl<T: FieldExt> Mul for Fp2<T> {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        Fp2 {
            re: self.re * other.re - self.im * other.im,
            im: self.re * other.im + self.im * other.re,
        }
    }
}

/// This function scalar multiplies an Fp2 by an Fp
impl<T: FieldExt> Mul<T> for Fp2<T> {
    type Output = Fp2<T>;

    fn mul(self, other: T) -> Self {
        Fp2 {
            re: other * self.re,
            im: other * self.im,
        }
    }
}

impl<T: FieldExt> Fp2<T> {
    /// Return the complex conjugate z' of z: Fp2
    /// This also happens to be the frobenius map
    ///     z -> z^p
    /// since p == 3 mod 4 and hence
    ///     i^p = i^(4k) * i^3 = 1*(-i) = -i
    fn conj(self) -> Self {
        Fp2 {
            re: self.re,
            im: -self.im,
        }
    }

    // Return the magnitude squared of a complex number
    fn norm_sq(self) -> T {
        self.re * self.re + self.im * self.im
    }
}

impl<T: FieldExt> FieldExt for Fp2<T> {
    const ZERO: Fp2<T> = Fp2 {
        re: T::ZERO,
        im: T::ZERO,
    };

    const UNIT: Fp2<T> = Fp2 {
        re: T::UNIT,
        im: T::ZERO,
    };

    fn new(val: usize) -> Fp2<T> {
        Fp2 {
            re: T::new(val),
            im: T::ZERO,
        }
    }

    /// The inverse of z is given by z'/||z||^2 since ||z||^2 = zz'
    fn inv(self) -> Fp2<T> {
        let norm_sq = self.norm_sq();
        self.conj() * norm_sq.inv()
    }
}

#[allow(clippy::suspicious_arithmetic_impl)]
impl<T: FieldExt> Div for Fp2<T> {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        self * rhs.inv()
    }
}

/// This trait defines the method which multiplies
/// by the Fp2 element t^3 whose cube root we will
/// adjoin in the subsequent cubic extension.
/// For BN254 this is 9+i, and for BLS381 it is 1+i.
/// It also defines the relevant FROB constants,
/// given by t^(p^n) and t^(p^2n) for various n,
/// used to compute the frobenius operations.
pub trait Adj: Sized {
    fn mul_adj(self) -> Self;
    const FROB_T: [[Self; 6]; 2];
    const FROB_Z: [Self; 12];
}

impl Adj for Fp2<BN254> {
    fn mul_adj(self) -> Self {
        let nine = BN254::new(9);
        Fp2 {
            re: nine * self.re - self.im,
            im: self.re + nine * self.im,
        }
    }

    const FROB_T: [[Fp2<BN254>; 6]; 2] = [
        [
            Fp2 {
                re: BN254 { val: U256::one() },
                im: BN254 { val: U256::zero() },
            },
            Fp2 {
                re: BN254 {
                    val: U256([
                        0x99e39557176f553d,
                        0xb78cc310c2c3330c,
                        0x4c0bec3cf559b143,
                        0x2fb347984f7911f7,
                    ]),
                },
                im: BN254 {
                    val: U256([
                        0x1665d51c640fcba2,
                        0x32ae2a1d0b7c9dce,
                        0x4ba4cc8bd75a0794,
                        0x16c9e55061ebae20,
                    ]),
                },
            },
            Fp2 {
                re: BN254 {
                    val: U256([
                        0xe4bd44e5607cfd48,
                        0xc28f069fbb966e3d,
                        0x5e6dd9e7e0acccb0,
                        0x30644e72e131a029,
                    ]),
                },
                im: BN254 { val: U256::zero() },
            },
            Fp2 {
                re: BN254 {
                    val: U256([
                        0x7b746ee87bdcfb6d,
                        0x805ffd3d5d6942d3,
                        0xbaff1c77959f25ac,
                        0x0856e078b755ef0a,
                    ]),
                },
                im: BN254 {
                    val: U256([
                        0x380cab2baaa586de,
                        0x0fdf31bf98ff2631,
                        0xa9f30e6dec26094f,
                        0x04f1de41b3d1766f,
                    ]),
                },
            },
            Fp2 {
                re: BN254 {
                    val: U256([
                        0x5763473177fffffe,
                        0xd4f263f1acdb5c4f,
                        0x59e26bcea0d48bac,
                        0x0,
                    ]),
                },
                im: BN254 { val: U256::zero() },
            },
            Fp2 {
                re: BN254 {
                    val: U256([
                        0x62e913ee1dada9e4,
                        0xf71614d4b0b71f3a,
                        0x699582b87809d9ca,
                        0x28be74d4bb943f51,
                    ]),
                },
                im: BN254 {
                    val: U256([
                        0xedae0bcec9c7aac7,
                        0x54f40eb4c3f6068d,
                        0xc2b86abcbe01477a,
                        0x14a88ae0cb747b99,
                    ]),
                },
            },
        ],
        [
            Fp2 {
                re: BN254 { val: U256::one() },
                im: BN254 { val: U256::zero() },
            },
            Fp2 {
                re: {
                    BN254 {
                        val: U256([
                            0x848a1f55921ea762,
                            0xd33365f7be94ec72,
                            0x80f3c0b75a181e84,
                            0x05b54f5e64eea801,
                        ]),
                    }
                },
                im: {
                    BN254 {
                        val: U256([
                            0xc13b4711cd2b8126,
                            0x3685d2ea1bdec763,
                            0x9f3a80b03b0b1c92,
                            0x2c145edbe7fd8aee,
                        ]),
                    }
                },
            },
            Fp2 {
                re: {
                    BN254 {
                        val: U256([
                            0x5763473177fffffe,
                            0xd4f263f1acdb5c4f,
                            0x59e26bcea0d48bac,
                            0x0,
                        ]),
                    }
                },
                im: { BN254 { val: U256::zero() } },
            },
            Fp2 {
                re: {
                    BN254 {
                        val: U256([
                            0x0e1a92bc3ccbf066,
                            0xe633094575b06bcb,
                            0x19bee0f7b5b2444e,
                            0xbc58c6611c08dab,
                        ]),
                    }
                },
                im: {
                    BN254 {
                        val: U256([
                            0x5fe3ed9d730c239f,
                            0xa44a9e08737f96e5,
                            0xfeb0f6ef0cd21d04,
                            0x23d5e999e1910a12,
                        ]),
                    }
                },
            },
            Fp2 {
                re: {
                    BN254 {
                        val: U256([
                            0xe4bd44e5607cfd48,
                            0xc28f069fbb966e3d,
                            0x5e6dd9e7e0acccb0,
                            0x30644e72e131a029,
                        ]),
                    }
                },
                im: { BN254 { val: U256::zero() } },
            },
            Fp2 {
                re: {
                    BN254 {
                        val: U256([
                            0xa97bda050992657f,
                            0xde1afb54342c724f,
                            0x1d9da40771b6f589,
                            0x1ee972ae6a826a7d,
                        ]),
                    }
                },
                im: {
                    BN254 {
                        val: U256([
                            0x5721e37e70c255c9,
                            0x54326430418536d1,
                            0xd2b513cdbb257724,
                            0x10de546ff8d4ab51,
                        ]),
                    }
                },
            },
        ],
    ];

    const FROB_Z: [Fp2<BN254>; 12] = [
        Fp2 {
            re: { BN254 { val: U256::one() } },
            im: { BN254 { val: U256::zero() } },
        },
        Fp2 {
            re: {
                BN254 {
                    val: U256([
                        0xd60b35dadcc9e470,
                        0x5c521e08292f2176,
                        0xe8b99fdd76e68b60,
                        0x1284b71c2865a7df,
                    ]),
                }
            },
            im: {
                BN254 {
                    val: U256([
                        0xca5cf05f80f362ac,
                        0x747992778eeec7e5,
                        0xa6327cfe12150b8e,
                        0x246996f3b4fae7e6,
                    ]),
                }
            },
        },
        Fp2 {
            re: {
                BN254 {
                    val: U256([
                        0xe4bd44e5607cfd49,
                        0xc28f069fbb966e3d,
                        0x5e6dd9e7e0acccb0,
                        0x30644e72e131a029,
                    ]),
                }
            },
            im: { BN254 { val: U256::zero() } },
        },
        Fp2 {
            re: {
                BN254 {
                    val: U256([
                        0xe86f7d391ed4a67f,
                        0x894cb38dbe55d24a,
                        0xefe9608cd0acaa90,
                        0x19dc81cfcc82e4bb,
                    ]),
                }
            },
            im: {
                BN254 {
                    val: U256([
                        0x7694aa2bf4c0c101,
                        0x7f03a5e397d439ec,
                        0x06cbeee33576139d,
                        0xabf8b60be77d73,
                    ]),
                }
            },
        },
        Fp2 {
            re: {
                BN254 {
                    val: U256([
                        0xe4bd44e5607cfd48,
                        0xc28f069fbb966e3d,
                        0x5e6dd9e7e0acccb0,
                        0x30644e72e131a029,
                    ]),
                }
            },
            im: { BN254 { val: U256::zero() } },
        },
        Fp2 {
            re: {
                BN254 {
                    val: U256([
                        0x1264475e420ac20f,
                        0x2cfa95859526b0d4,
                        0x072fc0af59c61f30,
                        0x757cab3a41d3cdc,
                    ]),
                }
            },
            im: {
                BN254 {
                    val: U256([
                        0xe85845e34c4a5b9c,
                        0xa20b7dfd71573c93,
                        0x18e9b79ba4e2606c,
                        0xca6b035381e35b6,
                    ]),
                }
            },
        },
        Fp2 {
            re: {
                BN254 {
                    val: U256([
                        0x3c208c16d87cfd46,
                        0x97816a916871ca8d,
                        0xb85045b68181585d,
                        0x30644e72e131a029,
                    ]),
                }
            },
            im: { BN254 { val: U256::zero() } },
        },
        Fp2 {
            re: {
                BN254 {
                    val: U256([
                        0x6615563bfbb318d7,
                        0x3b2f4c893f42a916,
                        0xcf96a5d90a9accfd,
                        0x1ddf9756b8cbf849,
                    ]),
                }
            },
            im: {
                BN254 {
                    val: U256([
                        0x71c39bb757899a9b,
                        0x2307d819d98302a7,
                        0x121dc8b86f6c4ccf,
                        0x0bfab77f2c36b843,
                    ]),
                }
            },
        },
        Fp2 {
            re: {
                BN254 {
                    val: U256([
                        0x5763473177fffffe,
                        0xd4f263f1acdb5c4f,
                        0x59e26bcea0d48bac,
                        0x0,
                    ]),
                }
            },
            im: { BN254 { val: U256::zero() } },
        },
        Fp2 {
            re: {
                BN254 {
                    val: U256([
                        0x53b10eddb9a856c8,
                        0x0e34b703aa1bf842,
                        0xc866e529b0d4adcd,
                        0x1687cca314aebb6d,
                    ]),
                }
            },
            im: {
                BN254 {
                    val: U256([
                        0xc58be1eae3bc3c46,
                        0x187dc4add09d90a0,
                        0xb18456d34c0b44c0,
                        0x2fb855bcd54a22b6,
                    ]),
                }
            },
        },
        Fp2 {
            re: {
                BN254 {
                    val: U256([
                        0x5763473177ffffff,
                        0xd4f263f1acdb5c4f,
                        0x59e26bcea0d48bac,
                        0x0,
                    ]),
                }
            },
            im: { BN254 { val: U256::zero() } },
        },
        Fp2 {
            re: {
                BN254 {
                    val: U256([
                        0x29bc44b896723b38,
                        0x6a86d50bd34b19b9,
                        0xb120850727bb392d,
                        0x290c83bf3d14634d,
                    ]),
                }
            },
            im: {
                BN254 {
                    val: U256([
                        0x53c846338c32a1ab,
                        0xf575ec93f71a8df9,
                        0x9f668e1adc9ef7f0,
                        0x23bd9e3da9136a73,
                    ]),
                }
            },
        },
    ];
}

impl Adj for Fp2<BLS381> {
    fn mul_adj(self) -> Self {
        Fp2 {
            re: self.re - self.im,
            im: self.re + self.im,
        }
    }
    const FROB_T: [[Fp2<BLS381>; 6]; 2] = [[Fp2::<BLS381>::ZERO; 6]; 2];
    const FROB_Z: [Fp2<BLS381>; 12] = [Fp2::<BLS381>::ZERO; 12];
}

/// The degree 3 field extension Fp6 over Fp2 is given by adjoining t, where t^3 = 1 + i
/// Fp6 has basis 1, t, t^2 over Fp2
#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) struct Fp6<T>
where
    T: FieldExt,
    Fp2<T>: Adj,
{
    pub t0: Fp2<T>,
    pub t1: Fp2<T>,
    pub t2: Fp2<T>,
}

impl<T> Distribution<Fp6<T>> for Standard
where
    T: FieldExt,
    Fp2<T>: Adj,
    Standard: Distribution<T>,
{
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Fp6<T> {
        let (t0, t1, t2) = rng.gen::<(Fp2<T>, Fp2<T>, Fp2<T>)>();
        Fp6 { t0, t1, t2 }
    }
}

impl<T> Add for Fp6<T>
where
    T: FieldExt,
    Fp2<T>: Adj,
{
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Fp6 {
            t0: self.t0 + other.t0,
            t1: self.t1 + other.t1,
            t2: self.t2 + other.t2,
        }
    }
}

impl<T> Neg for Fp6<T>
where
    T: FieldExt,
    Fp2<T>: Adj,
{
    type Output = Self;

    fn neg(self) -> Self::Output {
        Fp6 {
            t0: -self.t0,
            t1: -self.t1,
            t2: -self.t2,
        }
    }
}

impl<T> Sub for Fp6<T>
where
    T: FieldExt,
    Fp2<T>: Adj,
{
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Fp6 {
            t0: self.t0 - other.t0,
            t1: self.t1 - other.t1,
            t2: self.t2 - other.t2,
        }
    }
}

impl<T> Mul for Fp6<T>
where
    T: FieldExt,
    Fp2<T>: Adj,
{
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        Fp6 {
            t0: self.t0 * other.t0 + (self.t1 * other.t2 + self.t2 * other.t1).mul_adj(),
            t1: self.t0 * other.t1 + self.t1 * other.t0 + (self.t2 * other.t2).mul_adj(),
            t2: self.t0 * other.t2 + self.t1 * other.t1 + self.t2 * other.t0,
        }
    }
}

/// This function scalar multiplies an Fp6 by an Fp2
impl<T> Mul<Fp2<T>> for Fp6<T>
where
    T: FieldExt,
    Fp2<T>: Adj,
{
    type Output = Fp6<T>;

    fn mul(self, other: Fp2<T>) -> Self {
        Fp6 {
            t0: other * self.t0,
            t1: other * self.t1,
            t2: other * self.t2,
        }
    }
}

impl<T> Fp6<T>
where
    T: FieldExt,
    Fp2<T>: Adj,
{
    /// This function multiplies an Fp6 element by t, and hence shifts the bases,
    /// where the t^2 coefficient picks up a factor of 1+i as the 1 coefficient of the output
    fn sh(self) -> Fp6<T> {
        Fp6 {
            t0: self.t2.mul_adj(),
            t1: self.t0,
            t2: self.t1,
        }
    }
}

impl<T> Fp6<T>
where
    T: FieldExt,
    Fp2<T>: Adj,
{
    /// The nth frobenius endomorphism of a p^q field is given by mapping
    ///     x to x^(p^n)
    /// which sends a + bt + ct^2: Fp6 to
    ///     a^(p^n) + b^(p^n) * t^(p^n) + c^(p^n) * t^(2p^n)
    /// The Fp2 coefficients are determined by the comment in the conj method,
    /// while the values of
    ///     t^(p^n) and t^(2p^n)
    /// are precomputed in the constant arrays FROB_T1 and FROB_T2
    pub(crate) fn frob(self, n: usize) -> Fp6<T> {
        let n = n % 6;
        let frob_t1 = Fp2::<T>::FROB_T[0][n];
        let frob_t2 = Fp2::<T>::FROB_T[1][n];

        if n % 2 != 0 {
            Fp6 {
                t0: self.t0.conj(),
                t1: frob_t1 * self.t1.conj(),
                t2: frob_t2 * self.t2.conj(),
            }
        } else {
            Fp6 {
                t0: self.t0,
                t1: frob_t1 * self.t1,
                t2: frob_t2 * self.t2,
            }
        }
    }
}

impl<T> FieldExt for Fp6<T>
where
    T: FieldExt,
    Fp2<T>: Adj,
{
    const ZERO: Fp6<T> = Fp6 {
        t0: Fp2::<T>::ZERO,
        t1: Fp2::<T>::ZERO,
        t2: Fp2::<T>::ZERO,
    };

    const UNIT: Fp6<T> = Fp6 {
        t0: Fp2::<T>::UNIT,
        t1: Fp2::<T>::ZERO,
        t2: Fp2::<T>::ZERO,
    };

    fn new(val: usize) -> Fp6<T> {
        Fp6 {
            t0: Fp2::<T>::new(val),
            t1: Fp2::<T>::ZERO,
            t2: Fp2::<T>::ZERO,
        }
    }

    /// Let x_n = x^(p^n) and note that
    ///     x_0 = x^(p^0) = x^1 = x
    ///     (x_n)_m = (x^(p^n))^(p^m) = x^(p^n * p^m) = x^(p^(n+m)) = x_{n+m}
    /// By Galois Theory, given x: Fp6, the product
    ///     phi = x_0 * x_1 * x_2 * x_3 * x_4 * x_5
    /// lands in BN254, and hence the inverse of x is given by
    ///     (x_1 * x_2 * x_3 * x_4 * x_5) / phi
    /// We can save compute by rearranging the numerator:
    ///     (x_1 * x_3) * x_5 * (x_1 * x_3)_1
    /// By Galois theory, the following are in Fp2 and are complex conjugates
    ///     x_1 * x_3 * x_5,  x_0 * x_2 * x_4
    /// and therefore
    ///     phi = ||x_1 * x_3 * x_5||^2
    /// and hence the inverse is given by
    ///     ([x_1 * x_3] * x_5) * [x_1 * x_3]_1 / ||[x_1 * x_3] * x_5||^2
    fn inv(self) -> Fp6<T> {
        let prod_13 = self.frob(1) * self.frob(3);
        let prod_135 = (prod_13 * self.frob(5)).t0;
        let phi = prod_135.norm_sq();
        let prod_odds_over_phi = prod_135 * phi.inv();
        let prod_24 = prod_13.frob(1);
        prod_24 * prod_odds_over_phi
    }
}

#[allow(clippy::suspicious_arithmetic_impl)]
impl<T> Div for Fp6<T>
where
    T: FieldExt,
    Fp2<T>: Adj,
{
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        self * rhs.inv()
    }
}

/// The degree 2 field extension Fp12 over Fp6 is given by
/// adjoining z, where z^2 = t. It thus has basis 1, z over Fp6
#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) struct Fp12<T>
where
    T: FieldExt,
    Fp2<T>: Adj,
{
    pub z0: Fp6<T>,
    pub z1: Fp6<T>,
}

impl<T> FieldExt for Fp12<T>
where
    T: FieldExt,
    Fp2<T>: Adj,
{
    const ZERO: Fp12<T> = Fp12 {
        z0: Fp6::<T>::ZERO,
        z1: Fp6::<T>::ZERO,
    };

    const UNIT: Fp12<T> = Fp12 {
        z0: Fp6::<T>::UNIT,
        z1: Fp6::<T>::ZERO,
    };

    fn new(val: usize) -> Fp12<T> {
        Fp12 {
            z0: Fp6::<T>::new(val),
            z1: Fp6::<T>::ZERO,
        }
    }

    /// By Galois Theory, given x: Fp12, the product
    ///     phi = Prod_{i=0}^11 x_i
    /// lands in BN254, and hence the inverse of x is given by
    ///     (Prod_{i=1}^11 x_i) / phi
    /// The 6th Frob map is nontrivial but leaves Fp6 fixed and hence must be the conjugate:
    ///     x_6 = (a + bz)_6 = a - bz = x.conj()
    /// Letting prod_17 = x_1 * x_7, the remaining factors in the numerator can be expressed as:
    ///     [(prod_17) * (prod_17)_2] * (prod_17)_4 * [(prod_17) * (prod_17)_2]_1
    /// By Galois theory, both the following are in Fp2 and are complex conjugates
    ///     prod_odds,  prod_evens
    /// Thus phi = ||prod_odds||^2, and hence the inverse is given by
    ///    prod_odds * prod_evens_except_six * x.conj() / ||prod_odds||^2
    fn inv(self) -> Fp12<T> {
        let prod_17 = (self.frob(1) * self.frob(7)).z0;
        let prod_1379 = prod_17 * prod_17.frob(2);
        let prod_odds = (prod_1379 * prod_17.frob(4)).t0;
        let phi = prod_odds.norm_sq();
        let prod_odds_over_phi = prod_odds * phi.inv();
        let prod_evens_except_six = prod_1379.frob(1);
        let prod_except_six = prod_evens_except_six * prod_odds_over_phi;
        self.conj() * prod_except_six
    }
}

impl<T> Distribution<Fp12<T>> for Standard
where
    T: FieldExt,
    Fp2<T>: Adj,
    Standard: Distribution<T>,
{
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Fp12<T> {
        let (z0, z1) = rng.gen::<(Fp6<T>, Fp6<T>)>();
        Fp12 { z0, z1 }
    }
}

impl<T> Add for Fp12<T>
where
    T: FieldExt,
    Fp2<T>: Adj,
{
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Fp12 {
            z0: self.z0 + other.z0,
            z1: self.z1 + other.z1,
        }
    }
}

impl<T> Neg for Fp12<T>
where
    T: FieldExt,
    Fp2<T>: Adj,
{
    type Output = Self;

    fn neg(self) -> Self::Output {
        Fp12 {
            z0: -self.z0,
            z1: -self.z1,
        }
    }
}

impl<T> Sub for Fp12<T>
where
    T: FieldExt,
    Fp2<T>: Adj,
{
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Fp12 {
            z0: self.z0 - other.z0,
            z1: self.z1 - other.z1,
        }
    }
}

impl<T> Mul for Fp12<T>
where
    T: FieldExt,
    Fp2<T>: Adj,
{
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        let h0 = self.z0 * other.z0;
        let h1 = self.z1 * other.z1;
        let h01 = (self.z0 + self.z1) * (other.z0 + other.z1);
        Fp12 {
            z0: h0 + h1.sh(),
            z1: h01 - (h0 + h1),
        }
    }
}

/// This function scalar multiplies an Fp12 by an Fp6
impl<T> Mul<Fp6<T>> for Fp12<T>
where
    T: FieldExt,
    Fp2<T>: Adj,
{
    type Output = Fp12<T>;

    fn mul(self, other: Fp6<T>) -> Self {
        Fp12 {
            z0: other * self.z0,
            z1: other * self.z1,
        }
    }
}

impl<T> Fp12<T>
where
    T: FieldExt,
    Fp2<T>: Adj,
{
    fn conj(self) -> Fp12<T> {
        Fp12 {
            z0: self.z0,
            z1: -self.z1,
        }
    }
}

impl<T> Fp12<T>
where
    T: FieldExt,
    Fp2<T>: Adj,
{
    /// The nth frobenius endomorphism of a p^q field is given by mapping
    ///     x to x^(p^n)
    /// which sends a + bz: Fp12 to
    ///     a^(p^n) + b^(p^n) * z^(p^n)
    /// where the values of z^(p^n) are precomputed in the constant array FROB_Z
    pub(crate) fn frob(self, n: usize) -> Fp12<T> {
        let n = n % 12;
        Fp12 {
            z0: self.z0.frob(n),
            z1: self.z1.frob(n) * (Fp2::<T>::FROB_Z[n]),
        }
    }
}

#[allow(clippy::suspicious_arithmetic_impl)]
impl<T> Div for Fp12<T>
where
    T: FieldExt,
    Fp2<T>: Adj,
{
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        self * rhs.inv()
    }
}

pub trait Stack {
    const SIZE: usize;

    fn to_stack(&self) -> Vec<U256>;

    fn from_stack(stack: &[U256]) -> Self;
}

impl Stack for BN254 {
    const SIZE: usize = 1;

    fn to_stack(&self) -> Vec<U256> {
        vec![self.val]
    }

    fn from_stack(stack: &[U256]) -> BN254 {
        BN254 { val: stack[0] }
    }
}

impl Stack for BLS381 {
    const SIZE: usize = 2;

    fn to_stack(&self) -> Vec<U256> {
        vec![self.lo(), self.hi()]
    }

    fn from_stack(stack: &[U256]) -> BLS381 {
        let mut val = [0u64; 8];
        val[..4].copy_from_slice(&stack[0].0);
        val[4..].copy_from_slice(&stack[1].0);
        BLS381 { val: U512(val) }
    }
}

impl<T: FieldExt + Stack> Stack for Fp2<T> {
    const SIZE: usize = 2 * T::SIZE;

    fn to_stack(&self) -> Vec<U256> {
        let mut stack = self.re.to_stack();
        stack.extend(self.im.to_stack());
        stack
    }

    fn from_stack(stack: &[U256]) -> Fp2<T> {
        let field_size = T::SIZE;
        let re = T::from_stack(&stack[0..field_size]);
        let im = T::from_stack(&stack[field_size..2 * field_size]);
        Fp2 { re, im }
    }
}

impl<T> Stack for Fp6<T>
where
    T: FieldExt,
    Fp2<T>: Adj,
    Fp2<T>: Stack,
{
    const SIZE: usize = 3 * Fp2::<T>::SIZE;

    fn to_stack(&self) -> Vec<U256> {
        let mut stack = self.t0.to_stack();
        stack.extend(self.t1.to_stack());
        stack.extend(self.t2.to_stack());
        stack
    }

    fn from_stack(stack: &[U256]) -> Self {
        let field_size = Fp2::<T>::SIZE;
        let t0 = Fp2::<T>::from_stack(&stack[0..field_size]);
        let t1 = Fp2::<T>::from_stack(&stack[field_size..2 * field_size]);
        let t2 = Fp2::<T>::from_stack(&stack[2 * field_size..3 * field_size]);
        Fp6 { t0, t1, t2 }
    }
}

impl<T> Stack for Fp12<T>
where
    T: FieldExt,
    Fp2<T>: Adj,
    Fp6<T>: Stack,
{
    const SIZE: usize = 2 * Fp6::<T>::SIZE;

    fn to_stack(&self) -> Vec<U256> {
        let mut stack = self.z0.to_stack();
        stack.extend(self.z1.to_stack());
        stack
    }

    fn from_stack(stack: &[U256]) -> Self {
        let field_size = Fp6::<T>::SIZE;
        let z0 = Fp6::<T>::from_stack(&stack[0..field_size]);
        let z1 = Fp6::<T>::from_stack(&stack[field_size..2 * field_size]);
        Fp12 { z0, z1 }
    }
}
