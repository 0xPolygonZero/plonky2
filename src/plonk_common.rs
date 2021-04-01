use crate::circuit_builder::CircuitBuilder;
use crate::field::field::Field;
use crate::target::Target;

/// Evaluate the polynomial which vanishes on any multiplicative subgroup of a given order `n`.
pub(crate) fn eval_zero_poly<F: Field>(n: usize, x: F) -> F {
    // Z(x) = x^n - 1
    x.exp_usize(n) - F::ONE
}

/// Evaluate the Lagrange basis `L_1` with `L_1(1) = 1`, and `L_1(x) = 0` for other members of an
/// order `n` multiplicative subgroup.
pub(crate) fn eval_l_1<F: Field>(n: usize, x: F) -> F {
    if x.is_one() {
        // The code below would divide by zero, since we have (x - 1) in both the numerator and
        // denominator.
        return F::ONE;
    }

    // L_1(x) = (x^n - 1) / (n * (x - 1))
    //        = Z(x) / (n * (x - 1))
    eval_zero_poly(n, x) / (F::from_canonical_usize(n) * (x - F::ONE))
}

/// For each alpha in alphas, compute a reduction of the given terms using powers of alpha.
pub(crate) fn reduce_with_powers_multi<F: Field>(terms: &[F], alphas: &[F]) -> Vec<F> {
    alphas.iter()
        .map(|&alpha| reduce_with_powers(terms, alpha))
        .collect()
}

pub(crate) fn reduce_with_powers<F: Field>(terms: &[F], alpha: F) -> F {
    let mut sum = F::ZERO;
    for &term in terms.iter().rev() {
        sum = sum * alpha + term;
    }
    sum
}

pub(crate) fn reduce_with_powers_recursive<F: Field>(
    builder: &mut CircuitBuilder<F>,
    terms: Vec<Target>,
    alpha: Target,
) -> Target {
    todo!()
}
