use std::borrow::Borrow;

use crate::circuit_builder::CircuitBuilder;
use crate::circuit_data::CommonCircuitData;
use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::field::field::Field;
use crate::gates::gate::{GateRef, PrefixedGate};
use crate::polynomial::commitment::SALT_SIZE;
use crate::polynomial::polynomial::PolynomialCoeffs;
use crate::target::Target;
use crate::util::partial_products::partial_products;
use crate::vars::{EvaluationTargets, EvaluationVars, EvaluationVarsBase};

/// Holds the Merkle tree index and blinding flag of a set of polynomials used in FRI.
#[derive(Debug, Copy, Clone)]
pub struct PolynomialsIndexBlinding {
    pub(crate) index: usize,
    pub(crate) blinding: bool,
}
impl PolynomialsIndexBlinding {
    pub fn salt_size(&self) -> usize {
        if self.blinding {
            SALT_SIZE
        } else {
            0
        }
    }
}
/// Holds the indices and blinding flags of the Plonk polynomials.
pub struct PlonkPolynomials;
impl PlonkPolynomials {
    pub const CONSTANTS_SIGMAS: PolynomialsIndexBlinding = PolynomialsIndexBlinding {
        index: 0,
        blinding: false,
    };
    pub const WIRES: PolynomialsIndexBlinding = PolynomialsIndexBlinding {
        index: 1,
        blinding: true,
    };
    pub const ZS: PolynomialsIndexBlinding = PolynomialsIndexBlinding {
        index: 2,
        blinding: true,
    };
    pub const QUOTIENT: PolynomialsIndexBlinding = PolynomialsIndexBlinding {
        index: 3,
        blinding: true,
    };

    pub fn polynomials(i: usize) -> PolynomialsIndexBlinding {
        match i {
            0 => Self::CONSTANTS_SIGMAS,
            1 => Self::WIRES,
            2 => Self::ZS,
            3 => Self::QUOTIENT,
            _ => panic!("There are only 4 sets of polynomials in Plonk."),
        }
    }
}

/// Evaluate the vanishing polynomial at `x`. In this context, the vanishing polynomial is a random
/// linear combination of gate constraints, plus some other terms relating to the permutation
/// argument. All such terms should vanish on `H`.
pub(crate) fn eval_vanishing_poly<F: Extendable<D>, const D: usize>(
    common_data: &CommonCircuitData<F, D>,
    x: F::Extension,
    vars: EvaluationVars<F, D>,
    local_plonk_zs: &[F::Extension],
    next_plonk_zs: &[F::Extension],
    s_sigmas: &[F::Extension],
    betas: &[F],
    gammas: &[F],
    alphas: &[F],
) -> Vec<F::Extension> {
    let constraint_terms =
        evaluate_gate_constraints(&common_data.gates, common_data.num_gate_constraints, vars);

    // The L_1(x) (Z(x) - 1) vanishing terms.
    let mut vanishing_z_1_terms = Vec::new();
    // The Z(x) f'(x) - g'(x) Z(g x) terms.
    let mut vanishing_v_shift_terms = Vec::new();

    for i in 0..common_data.config.num_challenges {
        let z_x = local_plonk_zs[i];
        let z_gz = next_plonk_zs[i];
        vanishing_z_1_terms.push(eval_l_1(common_data.degree(), x) * (z_x - F::Extension::ONE));

        let mut f_prime = F::Extension::ONE;
        let mut g_prime = F::Extension::ONE;
        for j in 0..common_data.config.num_routed_wires {
            let wire_value = vars.local_wires[j];
            let k_i = common_data.k_is[j];
            let s_id = x * k_i.into();
            let s_sigma = s_sigmas[j];
            f_prime *= wire_value + s_id * betas[i].into() + gammas[i].into();
            g_prime *= wire_value + s_sigma * betas[i].into() + gammas[i].into();
        }
        vanishing_v_shift_terms.push(f_prime * z_x - g_prime * z_gz);
    }

    let vanishing_terms = [
        vanishing_z_1_terms,
        vanishing_v_shift_terms,
        constraint_terms,
    ]
    .concat();

    let alphas = &alphas.iter().map(|&a| a.into()).collect::<Vec<_>>();
    reduce_with_powers_multi(&vanishing_terms, alphas)
}

/// Like `eval_vanishing_poly`, but specialized for base field points.
pub(crate) fn eval_vanishing_poly_base<F: Extendable<D>, const D: usize>(
    common_data: &CommonCircuitData<F, D>,
    index: usize,
    x: F,
    vars: EvaluationVarsBase<F>,
    local_zs: &[F],
    next_zs: &[F],
    local_partial_products: &[F],
    s_sigmas: &[F],
    betas: &[F],
    gammas: &[F],
    alphas: &[F],
    z_h_on_coset: &ZeroPolyOnCoset<F>,
) -> Vec<F> {
    let constraint_terms =
        evaluate_gate_constraints_base(&common_data.gates, common_data.num_gate_constraints, vars);

    // The L_1(x) (Z(x) - 1) vanishing terms.
    let mut vanishing_z_1_terms = Vec::new();
    // The terms checking the partial products.
    let mut vanishing_partial_products_terms = Vec::new();
    // The Z(x) f'(x) - g'(x) Z(g x) terms.
    let mut vanishing_v_shift_terms = Vec::new();

    for i in 0..common_data.config.num_challenges {
        let z_x = local_zs[i];
        let z_gz = next_zs[i];
        vanishing_z_1_terms.push(z_h_on_coset.eval_l1(index, x) * (z_x - F::ONE));

        let numerator_values = (0..common_data.config.num_routed_wires)
            .map(|j| {
                let wire_value = vars.local_wires[j];
                let k_i = common_data.k_is[j];
                let s_id = k_i * x;
                wire_value + betas[i] * s_id + gammas[i]
            })
            .collect::<Vec<_>>();
        let denominator_values = (0..common_data.config.num_routed_wires)
            .map(|j| {
                let wire_value = vars.local_wires[j];
                let s_sigma = s_sigmas[j];
                wire_value + betas[i] * s_sigma + gammas[i]
            })
            .collect::<Vec<_>>();
        let numerator_partial_products =
            partial_products(numerator_values, common_data.max_filtered_constraint_degree);
        let denominator_partial_products = partial_products(
            denominator_values,
            common_data.max_filtered_constraint_degree,
        );

        dbg!(numerator_partial_products
            .clone()
            .0
            .into_iter()
            .chain(denominator_partial_products.clone().0)
            .zip(local_partial_products)
            .map(|(a, &b)| a - b)
            .collect::<Vec<_>>(),);
        vanishing_partial_products_terms.append(
            &mut numerator_partial_products
                .0
                .into_iter()
                .chain(denominator_partial_products.0)
                .zip(local_partial_products)
                .map(|(a, &b)| a - b)
                .collect::<Vec<_>>(),
        );
        dbg!(&numerator_partial_products.1);
        dbg!(&denominator_partial_products.1);
        dbg!(common_data.max_filtered_constraint_degree);
        let f_prime: F = numerator_partial_products.1.into_iter().product();
        let g_prime: F = denominator_partial_products.1.into_iter().product();
        // vanishing_v_shift_terms.push(f_prime * z_x - g_prime * z_gz);
    }

    let vanishing_terms = [
        vanishing_z_1_terms,
        vanishing_partial_products_terms,
        vanishing_v_shift_terms,
        constraint_terms,
    ]
    .concat();

    reduce_with_powers_multi(&vanishing_terms, alphas)
}

/// Evaluates all gate constraints.
///
/// `num_gate_constraints` is the largest number of constraints imposed by any gate. It is not
/// strictly necessary, but it helps performance by ensuring that we allocate a vector with exactly
/// the capacity that we need.
pub fn evaluate_gate_constraints<F: Extendable<D>, const D: usize>(
    gates: &[PrefixedGate<F, D>],
    num_gate_constraints: usize,
    vars: EvaluationVars<F, D>,
) -> Vec<F::Extension> {
    let mut constraints = vec![F::Extension::ZERO; num_gate_constraints];
    for gate in gates {
        let gate_constraints = gate.gate.0.eval_filtered(vars, &gate.prefix);
        for (i, c) in gate_constraints.into_iter().enumerate() {
            debug_assert!(
                i < num_gate_constraints,
                "num_constraints() gave too low of a number"
            );
            constraints[i] += c;
        }
    }
    constraints
}

pub fn evaluate_gate_constraints_base<F: Extendable<D>, const D: usize>(
    gates: &[PrefixedGate<F, D>],
    num_gate_constraints: usize,
    vars: EvaluationVarsBase<F>,
) -> Vec<F> {
    let mut constraints = vec![F::ZERO; num_gate_constraints];
    for gate in gates {
        let gate_constraints = gate.gate.0.eval_filtered_base(vars, &gate.prefix);
        for (i, c) in gate_constraints.into_iter().enumerate() {
            debug_assert!(
                i < num_gate_constraints,
                "num_constraints() gave too low of a number"
            );
            constraints[i] += c;
        }
    }
    constraints
}

pub fn evaluate_gate_constraints_recursively<F: Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    gates: &[GateRef<F, D>],
    num_gate_constraints: usize,
    vars: EvaluationTargets<D>,
) -> Vec<ExtensionTarget<D>> {
    let mut constraints = vec![builder.zero_extension(); num_gate_constraints];
    for gate in gates {
        let gate_constraints = gate.0.eval_filtered_recursively(builder, vars);
        for (i, c) in gate_constraints.into_iter().enumerate() {
            constraints[i] = builder.add_extension(constraints[i], c);
        }
    }
    constraints
}

/// Evaluate the polynomial which vanishes on any multiplicative subgroup of a given order `n`.
pub(crate) fn eval_zero_poly<F: Field>(n: usize, x: F) -> F {
    // Z(x) = x^n - 1
    x.exp(n as u64) - F::ONE
}

/// Precomputations of the evaluation of `Z_H(X) = X^n - 1` on a coset `gK` with `H <= K`.
pub(crate) struct ZeroPolyOnCoset<F: Field> {
    /// `n = |H|`.
    n: F,
    /// `rate = |K|/|H|`.
    rate: usize,
    /// Holds `g^n * (w^n)^i - 1 = g^n * v^i - 1` for `i in 0..rate`, with `w` a generator of `K` and `v` a
    /// `rate`-primitive root of unity.
    evals: Vec<F>,
    /// Holds the multiplicative inverses of `evals`.
    inverses: Vec<F>,
}
impl<F: Field> ZeroPolyOnCoset<F> {
    pub fn new(n_log: usize, rate_bits: usize) -> Self {
        let g_pow_n = F::coset_shift().exp_power_of_2(n_log);
        let evals = F::two_adic_subgroup(rate_bits)
            .into_iter()
            .map(|x| g_pow_n * x - F::ONE)
            .collect::<Vec<_>>();
        let inverses = F::batch_multiplicative_inverse(&evals);
        Self {
            n: F::from_canonical_usize(1 << n_log),
            rate: 1 << rate_bits,
            evals,
            inverses,
        }
    }

    /// Returns `Z_H(g * w^i)`.
    pub fn eval(&self, i: usize) -> F {
        self.evals[i % self.rate]
    }

    /// Returns `1 / Z_H(g * w^i)`.
    pub fn eval_inverse(&self, i: usize) -> F {
        self.inverses[i % self.rate]
    }

    /// Returns `L_1(x) = Z_H(x)/(n * (x - 1))` with `x = w^i`.
    pub fn eval_l1(&self, i: usize, x: F) -> F {
        // Could also precompute the inverses using Montgomery.
        self.eval(i) * (self.n * (x - F::ONE)).inverse()
    }
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
    alphas
        .iter()
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

pub(crate) fn reduce_with_powers_recursive<F: Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    terms: &[ExtensionTarget<D>],
    alpha: Target,
) -> ExtensionTarget<D> {
    let mut sum = builder.zero_extension();
    for &term in terms.iter().rev() {
        sum = builder.scalar_mul_ext(alpha, sum);
        sum = builder.add_extension(sum, term);
    }
    sum
}

/// Reduce a sequence of field elements by the given coefficients.
pub(crate) fn reduce_with_iter<F: Field>(
    terms: impl IntoIterator<Item = impl Borrow<F>>,
    coeffs: impl IntoIterator<Item = impl Borrow<F>>,
) -> F {
    terms
        .into_iter()
        .zip(coeffs)
        .map(|(t, c)| *t.borrow() * *c.borrow())
        .sum()
}

/// Reduce a sequence of polynomials by the given coefficients.
pub(crate) fn reduce_polys_with_iter<F: Field>(
    polys: impl IntoIterator<Item = impl Borrow<PolynomialCoeffs<F>>>,
    coeffs: impl IntoIterator<Item = impl Borrow<F>>,
) -> PolynomialCoeffs<F> {
    polys
        .into_iter()
        .zip(coeffs)
        .map(|(p, c)| p.borrow() * *c.borrow())
        .sum()
}
