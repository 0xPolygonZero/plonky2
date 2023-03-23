use std::ops::Add;

use rand::Rng;

use crate::extension_tower::{FieldExt, Fp12, Fp2, Fp6, BN254};

// The curve consists of pairs (x, y): (BN254, BN254) | y^2 = x^3 + 2
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Curve {
    pub x: BN254,
    pub y: BN254,
}

/// Standard addition formula for elliptic curves, restricted to the cases  
/// where neither inputs nor output would ever be the identity O. source:
/// https://en.wikipedia.org/wiki/Elliptic_curve#Algebraic_interpretation
impl Add for Curve {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let m = if self == other {
            BN254::new(3) * self.x * self.x / (BN254::new(2) * self.y)
        } else {
            (other.y - self.y) / (other.x - self.x)
        };
        let x = m * m - (self.x + other.x);
        Curve {
            x,
            y: m * (self.x - x) - self.y,
        }
    }
}

// The twisted curve consists of pairs (x, y): (Fp2, Fp2) | y^2 = x^3 + 3/(9 + i)
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct TwistedCurve {
    pub x: Fp2<BN254>,
    pub y: Fp2<BN254>,
}

// The tate pairing takes a point each from the curve and its twist and outputs an Fp12 element
pub fn tate(p: Curve, q: TwistedCurve) -> Fp12<BN254> {
    let miller_output = miller_loop(p, q);
    invariant_exponent(miller_output)
}

/// Standard code for miller loop, can be found on page 99 at this url:
/// https://static1.squarespace.com/static/5fdbb09f31d71c1227082339/t/5ff394720493bd28278889c6/1609798774687/PairingsForBeginners.pdf#page=107
/// where EXP is a hardcoding of the array of Booleans that the loop traverses
pub fn miller_loop(p: Curve, q: TwistedCurve) -> Fp12<BN254> {
    let mut r = p;
    let mut acc: Fp12<BN254> = Fp12::<BN254>::UNIT;
    let mut line: Fp12<BN254>;

    for i in EXP {
        line = tangent(r, q);
        r = r + r;
        acc = line * acc * acc;
        if i {
            line = cord(p, r, q);
            r = r + p;
            acc = line * acc;
        }
    }
    acc
}

/// The sloped line function for doubling a point
pub fn tangent(p: Curve, q: TwistedCurve) -> Fp12<BN254> {
    let cx = -BN254::new(3) * p.x * p.x;
    let cy = BN254::new(2) * p.y;
    sparse_embed(p.y * p.y - BN254::new(9), q.x * cx, q.y * cy)
}

/// The sloped line function for adding two points
pub fn cord(p1: Curve, p2: Curve, q: TwistedCurve) -> Fp12<BN254> {
    let cx = p2.y - p1.y;
    let cy = p1.x - p2.x;
    sparse_embed(p1.y * p2.x - p2.y * p1.x, q.x * cx, q.y * cy)
}

/// The tangent and cord functions output sparse Fp12 elements.
/// This map embeds the nonzero coefficients into an Fp12.
pub fn sparse_embed(g000: BN254, g01: Fp2<BN254>, g11: Fp2<BN254>) -> Fp12<BN254> {
    let g0 = Fp6 {
        t0: Fp2 {
            re: g000,
            im: BN254::ZERO,
        },
        t1: g01,
        t2: Fp2::<BN254>::ZERO,
    };

    let g1 = Fp6 {
        t0: Fp2::<BN254>::ZERO,
        t1: g11,
        t2: Fp2::<BN254>::ZERO,
    };

    Fp12 { z0: g0, z1: g1 }
}

pub fn gen_fp12_sparse<R: Rng + ?Sized>(rng: &mut R) -> Fp12<BN254> {
    sparse_embed(
        rng.gen::<BN254>(),
        rng.gen::<Fp2<BN254>>(),
        rng.gen::<Fp2<BN254>>(),
    )
}

/// The output y of the miller loop is not an invariant,
/// but one gets an invariant by raising y to the power
///     (p^12 - 1)/N = (p^6 - 1)(p^2 + 1)(p^4 - p^2 + 1)/N
/// where N is the cyclic group order of the curve.
/// To achieve this, we first exponentiate y by p^6 - 1 via
///     y = y_6 / y
/// and then exponentiate the result by p^2 + 1 via
///     y = y_2 * y
/// We then note that (p^4 - p^2 + 1)/N can be rewritten as
///     (p^4 - p^2 + 1)/N = p^3 + (a2)p^2 - (a1)p - a0
/// where 0 < a0, a1, a2 < p. Then the final power is given by
///     y = y_3 * (y^a2)_2 * (y^-a1)_1 * (y^-a0)
pub fn invariant_exponent(f: Fp12<BN254>) -> Fp12<BN254> {
    let mut y = f.frob(6) / f;
    y = y.frob(2) * y;
    let (y_a2, y_a1, y_a0) = get_custom_powers(y);
    y.frob(3) * y_a2.frob(2) * y_a1.frob(1) * y_a0
}

/// We first together (so as to avoid repeated steps) compute
///     y^a4, y^a2, y^a0
/// where a1 is given by
///     a1 = a4 + 2a2 - a0
/// we then invert y^a0 and return
///     y^a2, y^a1 = y^a4 * y^a2 * y^a2 * y^(-a0), y^(-a0)
///
/// Representing a4, a2, a0 in *little endian* binary, define
///     EXPS4 = [(a4[i], a2[i], a0[i]) for i in       0..len(a4)]
///     EXPS2 = [       (a2[i], a0[i]) for i in len(a4)..len(a2)]
///     EXPS0 = [               a0[i]  for i in len(a2)..len(a0)]
fn get_custom_powers(f: Fp12<BN254>) -> (Fp12<BN254>, Fp12<BN254>, Fp12<BN254>) {
    let mut sq: Fp12<BN254> = f;
    let mut y0: Fp12<BN254> = Fp12::<BN254>::UNIT;
    let mut y2: Fp12<BN254> = Fp12::<BN254>::UNIT;
    let mut y4: Fp12<BN254> = Fp12::<BN254>::UNIT;

    // proceed via standard squaring algorithm for exponentiation

    // must keep multiplying all three values: a4, a2, a0
    for (a, b, c) in EXPS4 {
        if a {
            y4 = y4 * sq;
        }
        if b {
            y2 = y2 * sq;
        }
        if c {
            y0 = y0 * sq;
        }
        sq = sq * sq;
    }
    // leading term of a4 is always 1
    y4 = y4 * sq;

    // must keep multiplying remaining two values: a2, a0
    for (a, b) in EXPS2 {
        if a {
            y2 = y2 * sq;
        }
        if b {
            y0 = y0 * sq;
        }
        sq = sq * sq;
    }
    // leading term of a2 is always 1
    y2 = y2 * sq;

    // must keep multiplying final remaining value: a0
    for a in EXPS0 {
        if a {
            y0 = y0 * sq;
        }
        sq = sq * sq;
    }
    // leading term of a0 is always 1
    y0 = y0 * sq;

    // invert y0 to compute y^(-a0)
    let y0_inv = y0.inv();

    // return y^a2 = y2, y^a1 = y4 * y2^2 * y^(-a0), y^(-a0)
    (y2, y4 * y2 * y2 * y0_inv, y0_inv)
}

const EXP: [bool; 253] = [
    true, false, false, false, false, false, true, true, false, false, true, false, false, false,
    true, false, false, true, true, true, false, false, true, true, true, false, false, true,
    false, true, true, true, false, false, false, false, true, false, false, true, true, false,
    false, false, true, true, false, true, false, false, false, false, false, false, false, true,
    false, true, false, false, true, true, false, true, true, true, false, false, false, false,
    true, false, true, false, false, false, false, false, true, false, false, false, true, false,
    true, true, false, true, true, false, true, true, false, true, false, false, false, false,
    false, false, true, true, false, false, false, false, false, false, true, false, true, false,
    true, true, false, false, false, false, true, false, true, true, true, false, true, false,
    false, true, false, true, false, false, false, false, false, true, true, false, false, true,
    true, true, true, true, false, true, false, false, false, false, true, false, false, true,
    false, false, false, false, true, true, true, true, false, false, true, true, false, true,
    true, true, false, false, true, false, true, true, true, false, false, false, false, true,
    false, false, true, false, false, false, true, false, true, false, false, false, false, true,
    true, true, true, true, false, false, false, false, true, true, true, true, true, false, true,
    false, true, true, false, false, true, false, false, true, true, true, true, true, true, false,
    false, false, false, false, false, false, false, false, false, false, false, false, false,
    false, false, false, false, false, false, false, false, false, false, false, false, false,
    false,
];

// The folowing constants are defined above get_custom_powers

const EXPS4: [(bool, bool, bool); 64] = [
    (true, true, false),
    (true, true, true),
    (true, true, true),
    (false, false, false),
    (false, false, true),
    (true, false, true),
    (false, true, false),
    (true, false, true),
    (true, true, false),
    (true, false, true),
    (false, true, false),
    (true, true, false),
    (true, true, false),
    (true, true, false),
    (false, true, false),
    (false, true, false),
    (false, false, true),
    (true, false, true),
    (true, true, false),
    (false, true, false),
    (true, true, false),
    (true, true, false),
    (true, true, false),
    (false, false, true),
    (false, false, true),
    (true, false, true),
    (true, false, true),
    (true, true, false),
    (true, false, false),
    (true, true, false),
    (false, true, false),
    (true, true, false),
    (true, false, false),
    (false, true, false),
    (false, false, false),
    (true, false, false),
    (true, false, false),
    (true, false, true),
    (false, false, true),
    (false, true, true),
    (false, false, true),
    (false, true, true),
    (false, true, true),
    (false, false, false),
    (true, true, true),
    (true, false, true),
    (true, false, true),
    (false, true, true),
    (true, false, true),
    (false, true, true),
    (false, true, true),
    (true, true, false),
    (true, true, false),
    (true, true, false),
    (true, false, false),
    (false, false, true),
    (true, false, false),
    (false, false, true),
    (true, false, true),
    (true, true, false),
    (true, true, true),
    (false, true, true),
    (false, true, false),
    (true, true, true),
];

const EXPS2: [(bool, bool); 62] = [
    (true, false),
    (true, true),
    (false, false),
    (true, false),
    (true, false),
    (true, true),
    (true, false),
    (true, true),
    (true, false),
    (false, true),
    (false, true),
    (true, true),
    (true, true),
    (false, false),
    (true, true),
    (false, false),
    (false, false),
    (false, true),
    (false, true),
    (true, true),
    (true, true),
    (true, true),
    (false, true),
    (true, true),
    (false, false),
    (true, true),
    (true, false),
    (true, true),
    (false, false),
    (true, true),
    (true, true),
    (true, false),
    (false, false),
    (false, true),
    (false, false),
    (true, true),
    (false, true),
    (false, false),
    (true, false),
    (false, true),
    (false, true),
    (true, false),
    (false, true),
    (false, false),
    (false, false),
    (false, false),
    (false, true),
    (true, false),
    (true, true),
    (false, true),
    (true, true),
    (true, false),
    (false, true),
    (false, false),
    (true, false),
    (false, true),
    (true, false),
    (true, true),
    (true, false),
    (true, true),
    (false, true),
    (true, true),
];

const EXPS0: [bool; 65] = [
    false, false, true, false, false, true, true, false, true, false, true, true, true, false,
    true, false, false, false, true, false, false, true, false, true, false, true, true, false,
    false, false, false, false, true, false, true, false, true, true, true, false, false, true,
    true, true, true, false, true, false, true, true, false, false, true, false, false, false,
    true, true, true, true, false, false, true, true, false,
];
