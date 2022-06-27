use std::ops::Add;

use plonky2_field::ops::Square;
use plonky2_field::types::Field;

use crate::curve::curve_types::{AffinePoint, Curve, ProjectivePoint};

impl<C: Curve> Add<ProjectivePoint<C>> for ProjectivePoint<C> {
    type Output = ProjectivePoint<C>;

    fn add(self, rhs: ProjectivePoint<C>) -> Self::Output {
        let ProjectivePoint {
            x: x1,
            y: y1,
            z: z1,
        } = self;
        let ProjectivePoint {
            x: x2,
            y: y2,
            z: z2,
        } = rhs;

        if z1 == C::BaseField::ZERO {
            return rhs;
        }
        if z2 == C::BaseField::ZERO {
            return self;
        }

        let x1z2 = x1 * z2;
        let y1z2 = y1 * z2;
        let x2z1 = x2 * z1;
        let y2z1 = y2 * z1;

        // Check if we're doubling or adding inverses.
        if x1z2 == x2z1 {
            if y1z2 == y2z1 {
                // TODO: inline to avoid redundant muls.
                return self.double();
            }
            if y1z2 == -y2z1 {
                return ProjectivePoint::ZERO;
            }
        }

        // From https://www.hyperelliptic.org/EFD/g1p/data/shortw/projective/addition/add-1998-cmo-2
        let z1z2 = z1 * z2;
        let u = y2z1 - y1z2;
        let uu = u.square();
        let v = x2z1 - x1z2;
        let vv = v.square();
        let vvv = v * vv;
        let r = vv * x1z2;
        let a = uu * z1z2 - vvv - r.double();
        let x3 = v * a;
        let y3 = u * (r - a) - vvv * y1z2;
        let z3 = vvv * z1z2;
        ProjectivePoint::nonzero(x3, y3, z3)
    }
}

impl<C: Curve> Add<AffinePoint<C>> for ProjectivePoint<C> {
    type Output = ProjectivePoint<C>;

    fn add(self, rhs: AffinePoint<C>) -> Self::Output {
        let ProjectivePoint {
            x: x1,
            y: y1,
            z: z1,
        } = self;
        let AffinePoint {
            x: x2,
            y: y2,
            zero: zero2,
        } = rhs;

        if z1 == C::BaseField::ZERO {
            return rhs.to_projective();
        }
        if zero2 {
            return self;
        }

        let x2z1 = x2 * z1;
        let y2z1 = y2 * z1;

        // Check if we're doubling or adding inverses.
        if x1 == x2z1 {
            if y1 == y2z1 {
                // TODO: inline to avoid redundant muls.
                return self.double();
            }
            if y1 == -y2z1 {
                return ProjectivePoint::ZERO;
            }
        }

        // From https://www.hyperelliptic.org/EFD/g1p/data/shortw/projective/addition/madd-1998-cmo
        let u = y2z1 - y1;
        let uu = u.square();
        let v = x2z1 - x1;
        let vv = v.square();
        let vvv = v * vv;
        let r = vv * x1;
        let a = uu * z1 - vvv - r.double();
        let x3 = v * a;
        let y3 = u * (r - a) - vvv * y1;
        let z3 = vvv * z1;
        ProjectivePoint::nonzero(x3, y3, z3)
    }
}

impl<C: Curve> Add<AffinePoint<C>> for AffinePoint<C> {
    type Output = ProjectivePoint<C>;

    fn add(self, rhs: AffinePoint<C>) -> Self::Output {
        let AffinePoint {
            x: x1,
            y: y1,
            zero: zero1,
        } = self;
        let AffinePoint {
            x: x2,
            y: y2,
            zero: zero2,
        } = rhs;

        if zero1 {
            return rhs.to_projective();
        }
        if zero2 {
            return self.to_projective();
        }

        // Check if we're doubling or adding inverses.
        if x1 == x2 {
            if y1 == y2 {
                return self.to_projective().double();
            }
            if y1 == -y2 {
                return ProjectivePoint::ZERO;
            }
        }

        // From https://www.hyperelliptic.org/EFD/g1p/data/shortw/projective/addition/mmadd-1998-cmo
        let u = y2 - y1;
        let uu = u.square();
        let v = x2 - x1;
        let vv = v.square();
        let vvv = v * vv;
        let r = vv * x1;
        let a = uu - vvv - r.double();
        let x3 = v * a;
        let y3 = u * (r - a) - vvv * y1;
        let z3 = vvv;
        ProjectivePoint::nonzero(x3, y3, z3)
    }
}
