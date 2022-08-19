use std::fmt::Debug;
use std::hash::Hash;
use std::ops::Neg;

use plonky2_field::ops::Square;
use plonky2_field::types::{Field, PrimeField};
use serde::{Deserialize, Serialize};

// To avoid implementation conflicts from associated types,
// see https://github.com/rust-lang/rust/issues/20400
pub struct CurveScalar<C: Curve>(pub <C as Curve>::ScalarField);

/// A short Weierstrass curve.
pub trait Curve: 'static + Sync + Sized + Copy + Debug {
    type BaseField: PrimeField;
    type ScalarField: PrimeField;

    const A: Self::BaseField;
    const B: Self::BaseField;

    const GENERATOR_AFFINE: AffinePoint<Self>;

    const GENERATOR_PROJECTIVE: ProjectivePoint<Self> = ProjectivePoint {
        x: Self::GENERATOR_AFFINE.x,
        y: Self::GENERATOR_AFFINE.y,
        z: Self::BaseField::ONE,
    };

    fn convert(x: Self::ScalarField) -> CurveScalar<Self> {
        CurveScalar(x)
    }

    fn is_safe_curve() -> bool {
        // Added additional check to prevent using vulnerabilties in case a discriminant is equal to 0.
        (Self::A.cube().double().double() + Self::B.square().triple().triple().triple())
            .is_nonzero()
    }
}

/// A point on a short Weierstrass curve, represented in affine coordinates.
#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
pub struct AffinePoint<C: Curve> {
    pub x: C::BaseField,
    pub y: C::BaseField,
    pub zero: bool,
}

impl<C: Curve> AffinePoint<C> {
    pub const ZERO: Self = Self {
        x: C::BaseField::ZERO,
        y: C::BaseField::ZERO,
        zero: true,
    };

    pub fn nonzero(x: C::BaseField, y: C::BaseField) -> Self {
        let point = Self { x, y, zero: false };
        debug_assert!(point.is_valid());
        point
    }

    pub fn is_valid(&self) -> bool {
        let Self { x, y, zero } = *self;
        zero || y.square() == x.cube() + C::A * x + C::B
    }

    pub fn to_projective(&self) -> ProjectivePoint<C> {
        let Self { x, y, zero } = *self;
        let z = if zero {
            C::BaseField::ZERO
        } else {
            C::BaseField::ONE
        };

        ProjectivePoint { x, y, z }
    }

    pub fn batch_to_projective(affine_points: &[Self]) -> Vec<ProjectivePoint<C>> {
        affine_points.iter().map(Self::to_projective).collect()
    }

    #[must_use]
    pub fn double(&self) -> Self {
        let AffinePoint { x: x1, y: y1, zero } = *self;

        if zero {
            return AffinePoint::ZERO;
        }

        let double_y = y1.double();
        let inv_double_y = double_y.inverse(); // (2y)^(-1)
        let triple_xx = x1.square().triple(); // 3x^2
        let lambda = (triple_xx + C::A) * inv_double_y;
        let x3 = lambda.square() - self.x.double();
        let y3 = lambda * (x1 - x3) - y1;

        Self {
            x: x3,
            y: y3,
            zero: false,
        }
    }
}

impl<C: Curve> PartialEq for AffinePoint<C> {
    fn eq(&self, other: &Self) -> bool {
        let AffinePoint {
            x: x1,
            y: y1,
            zero: zero1,
        } = *self;
        let AffinePoint {
            x: x2,
            y: y2,
            zero: zero2,
        } = *other;
        if zero1 || zero2 {
            return zero1 == zero2;
        }
        x1 == x2 && y1 == y2
    }
}

impl<C: Curve> Eq for AffinePoint<C> {}

impl<C: Curve> Hash for AffinePoint<C> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        if self.zero {
            self.zero.hash(state);
        } else {
            self.x.hash(state);
            self.y.hash(state);
        }
    }
}

/// A point on a short Weierstrass curve, represented in projective coordinates.
#[derive(Copy, Clone, Debug)]
pub struct ProjectivePoint<C: Curve> {
    pub x: C::BaseField,
    pub y: C::BaseField,
    pub z: C::BaseField,
}

impl<C: Curve> ProjectivePoint<C> {
    pub const ZERO: Self = Self {
        x: C::BaseField::ZERO,
        y: C::BaseField::ONE,
        z: C::BaseField::ZERO,
    };

    pub fn nonzero(x: C::BaseField, y: C::BaseField, z: C::BaseField) -> Self {
        let point = Self { x, y, z };
        debug_assert!(point.is_valid());
        point
    }

    pub fn is_valid(&self) -> bool {
        let Self { x, y, z } = *self;
        z.is_zero() || y.square() * z == x.cube() + C::A * x * z.square() + C::B * z.cube()
    }

    pub fn to_affine(&self) -> AffinePoint<C> {
        let Self { x, y, z } = *self;
        if z == C::BaseField::ZERO {
            AffinePoint::ZERO
        } else {
            let z_inv = z.inverse();
            AffinePoint::nonzero(x * z_inv, y * z_inv)
        }
    }

    pub fn batch_to_affine(proj_points: &[Self]) -> Vec<AffinePoint<C>> {
        let n = proj_points.len();
        let zs: Vec<C::BaseField> = proj_points.iter().map(|pp| pp.z).collect();
        let z_invs = C::BaseField::batch_multiplicative_inverse(&zs);

        let mut result = Vec::with_capacity(n);
        for i in 0..n {
            let Self { x, y, z } = proj_points[i];
            result.push(if z == C::BaseField::ZERO {
                AffinePoint::ZERO
            } else {
                let z_inv = z_invs[i];
                AffinePoint::nonzero(x * z_inv, y * z_inv)
            });
        }
        result
    }

    // From https://www.hyperelliptic.org/EFD/g1p/data/shortw/projective/doubling/dbl-2007-bl
    #[must_use]
    pub fn double(&self) -> Self {
        let Self { x, y, z } = *self;
        if z == C::BaseField::ZERO {
            return ProjectivePoint::ZERO;
        }

        let xx = x.square();
        let zz = z.square();
        let mut w = xx.triple();
        if C::A.is_nonzero() {
            w += C::A * zz;
        }
        let s = y.double() * z;
        let r = y * s;
        let rr = r.square();
        let b = (x + r).square() - (xx + rr);
        let h = w.square() - b.double();
        let x3 = h * s;
        let y3 = w * (b - h) - rr.double();
        let z3 = s.cube();
        Self {
            x: x3,
            y: y3,
            z: z3,
        }
    }

    pub fn add_slices(a: &[Self], b: &[Self]) -> Vec<Self> {
        assert_eq!(a.len(), b.len());
        a.iter()
            .zip(b.iter())
            .map(|(&a_i, &b_i)| a_i + b_i)
            .collect()
    }

    #[must_use]
    pub fn neg(&self) -> Self {
        Self {
            x: self.x,
            y: -self.y,
            z: self.z,
        }
    }
}

impl<C: Curve> PartialEq for ProjectivePoint<C> {
    fn eq(&self, other: &Self) -> bool {
        let ProjectivePoint {
            x: x1,
            y: y1,
            z: z1,
        } = *self;
        let ProjectivePoint {
            x: x2,
            y: y2,
            z: z2,
        } = *other;
        if z1 == C::BaseField::ZERO || z2 == C::BaseField::ZERO {
            return z1 == z2;
        }

        // We want to compare (x1/z1, y1/z1) == (x2/z2, y2/z2).
        // But to avoid field division, it is better to compare (x1*z2, y1*z2) == (x2*z1, y2*z1).
        x1 * z2 == x2 * z1 && y1 * z2 == y2 * z1
    }
}

impl<C: Curve> Eq for ProjectivePoint<C> {}

impl<C: Curve> Neg for AffinePoint<C> {
    type Output = AffinePoint<C>;

    fn neg(self) -> Self::Output {
        let AffinePoint { x, y, zero } = self;
        AffinePoint { x, y: -y, zero }
    }
}

impl<C: Curve> Neg for ProjectivePoint<C> {
    type Output = ProjectivePoint<C>;

    fn neg(self) -> Self::Output {
        let ProjectivePoint { x, y, z } = self;
        ProjectivePoint { x, y: -y, z }
    }
}

pub fn base_to_scalar<C: Curve>(x: C::BaseField) -> C::ScalarField {
    C::ScalarField::from_noncanonical_biguint(x.to_canonical_biguint())
}

pub fn scalar_to_base<C: Curve>(x: C::ScalarField) -> C::BaseField {
    C::BaseField::from_noncanonical_biguint(x.to_canonical_biguint())
}
