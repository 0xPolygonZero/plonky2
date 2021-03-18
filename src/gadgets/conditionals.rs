use crate::constraint_polynomial::ConstraintPolynomial;
use crate::field::field::Field;

/// Evaluates to `x` if `c == 0`, or `x * y` if `c == 1`.
pub fn conditional_multiply_poly<F: Field>(
    x: &ConstraintPolynomial<F>,
    y: &ConstraintPolynomial<F>,
    c: &ConstraintPolynomial<F>,
) -> ConstraintPolynomial<F> {
    let product = x * y;
    let delta = product - x;
    x + c * delta
}
