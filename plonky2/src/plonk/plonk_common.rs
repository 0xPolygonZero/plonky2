use plonky2_field::extension::Extendable;
use plonky2_field::packed::PackedField;
use plonky2_field::types::Field;

use crate::fri::oracle::SALT_SIZE;
use crate::fri::structure::FriOracleInfo;
use crate::gates::arithmetic_base::ArithmeticGate;
use crate::hash::hash_types::RichField;
use crate::iop::ext_target::ExtensionTarget;
use crate::iop::target::Target;
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::util::reducing::ReducingFactorTarget;

pub(crate) const FRI_ORACLES: [FriOracleInfo; 4] = [
    PlonkOracle::CONSTANTS_SIGMAS.as_fri_oracle(),
    PlonkOracle::WIRES.as_fri_oracle(),
    PlonkOracle::ZS_PARTIAL_PRODUCTS.as_fri_oracle(),
    PlonkOracle::QUOTIENT.as_fri_oracle(),
];

/// Holds the Merkle tree index and blinding flag of a set of polynomials used in FRI.
#[derive(Debug, Copy, Clone)]
pub struct PlonkOracle {
    pub(crate) index: usize,
    pub(crate) blinding: bool,
}

impl PlonkOracle {
    pub const CONSTANTS_SIGMAS: PlonkOracle = PlonkOracle {
        index: 0,
        blinding: false,
    };
    pub const WIRES: PlonkOracle = PlonkOracle {
        index: 1,
        blinding: true,
    };
    pub const ZS_PARTIAL_PRODUCTS: PlonkOracle = PlonkOracle {
        index: 2,
        blinding: true,
    };
    pub const QUOTIENT: PlonkOracle = PlonkOracle {
        index: 3,
        blinding: true,
    };

    pub(crate) const fn as_fri_oracle(&self) -> FriOracleInfo {
        FriOracleInfo {
            blinding: self.blinding,
        }
    }
}

pub fn salt_size(salted: bool) -> usize {
    if salted {
        SALT_SIZE
    } else {
        0
    }
}

/// Evaluate the polynomial which vanishes on any multiplicative subgroup of a given order `n`.
pub(crate) fn eval_zero_poly<F: Field>(n: usize, x: F) -> F {
    // Z(x) = x^n - 1
    x.exp_u64(n as u64) - F::ONE
}

/// Evaluate the Lagrange basis `L_0` with `L_0(1) = 1`, and `L_0(x) = 0` for other members of the
/// order `n` multiplicative subgroup.
pub(crate) fn eval_l_0<F: Field>(n: usize, x: F) -> F {
    if x.is_one() {
        // The code below would divide by zero, since we have (x - 1) in both the numerator and
        // denominator.
        return F::ONE;
    }

    // L_0(x) = (x^n - 1) / (n * (x - 1))
    //        = Z(x) / (n * (x - 1))
    eval_zero_poly(n, x) / (F::from_canonical_usize(n) * (x - F::ONE))
}

/// Evaluates the Lagrange basis L_0(x), which has L_0(1) = 1 and vanishes at all other points in
/// the order-`n` subgroup.
///
/// Assumes `x != 1`; if `x` could be 1 then this is unsound.
pub(crate) fn eval_l_0_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    n: usize,
    x: ExtensionTarget<D>,
    x_pow_n: ExtensionTarget<D>,
) -> ExtensionTarget<D> {
    // L_0(x) = (x^n - 1) / (n * (x - 1))
    //        = Z(x) / (n * (x - 1))
    let one = builder.one_extension();
    let neg_one = builder.neg_one();
    let neg_one = builder.convert_to_ext(neg_one);
    let eval_zero_poly = builder.sub_extension(x_pow_n, one);
    let denominator = builder.arithmetic_extension(
        F::from_canonical_usize(n),
        F::from_canonical_usize(n),
        x,
        one,
        neg_one,
    );
    builder.div_extension(eval_zero_poly, denominator)
}

/// For each alpha in alphas, compute a reduction of the given terms using powers of alpha. T can
/// be any type convertible to a double-ended iterator.
pub(crate) fn reduce_with_powers_multi<
    'a,
    F: Field,
    I: DoubleEndedIterator<Item = &'a F>,
    T: IntoIterator<IntoIter = I>,
>(
    terms: T,
    alphas: &[F],
) -> Vec<F> {
    let mut cumul = vec![F::ZERO; alphas.len()];
    for &term in terms.into_iter().rev() {
        cumul
            .iter_mut()
            .zip(alphas)
            .for_each(|(c, &alpha)| *c = term.multiply_accumulate(*c, alpha));
    }
    cumul
}

pub fn reduce_with_powers<'a, P: PackedField, T: IntoIterator<Item = &'a P>>(
    terms: T,
    alpha: P::Scalar,
) -> P
where
    T::IntoIter: DoubleEndedIterator,
{
    let mut sum = P::ZEROS;
    for &term in terms.into_iter().rev() {
        sum = sum * alpha + term;
    }
    sum
}

pub fn reduce_with_powers_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    terms: &[Target],
    alpha: Target,
) -> Target {
    if terms.len() <= ArithmeticGate::new_from_config(&builder.config).num_ops + 1 {
        terms
            .iter()
            .rev()
            .fold(builder.zero(), |acc, &t| builder.mul_add(alpha, acc, t))
    } else {
        let alpha = builder.convert_to_ext(alpha);
        let mut alpha = ReducingFactorTarget::new(alpha);
        alpha.reduce_base(terms, builder).0[0]
    }
}

pub fn reduce_with_powers_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    terms: &[ExtensionTarget<D>],
    alpha: Target,
) -> ExtensionTarget<D> {
    let alpha = builder.convert_to_ext(alpha);
    let mut alpha = ReducingFactorTarget::new(alpha);
    alpha.reduce(terms, builder)
}
