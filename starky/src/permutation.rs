//! Permutation arguments.

use alloc::vec;
use alloc::vec::Vec;

use itertools::Itertools;
use plonky2::field::batch_util::batch_multiply_inplace;
use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::packed::PackedField;
use plonky2::field::polynomial::PolynomialValues;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::challenger::{Challenger, RecursiveChallenger};
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::iop::target::Target;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::config::{AlgebraicHasher, Hasher};
use plonky2::util::reducing::{ReducingFactor, ReducingFactorTarget};
use plonky2_maybe_rayon::*;

use crate::config::StarkConfig;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::stark::Stark;
use crate::vars::{StarkEvaluationTargets, StarkEvaluationVars};

/// A pair of lists of columns, `lhs` and `rhs`, that should be permutations of one another.
/// In particular, there should exist some permutation `pi` such that for any `i`,
/// `trace[lhs[i]] = pi(trace[rhs[i]])`. Here `trace` denotes the trace in column-major form, so
/// `trace[col]` is a column vector.
pub struct PermutationPair {
    /// Each entry contains two column indices, representing two columns which should be
    /// permutations of one another.
    pub column_pairs: Vec<(usize, usize)>,
}

impl PermutationPair {
    pub fn singletons(lhs: usize, rhs: usize) -> Self {
        Self {
            column_pairs: vec![(lhs, rhs)],
        }
    }
}

/// A single instance of a permutation check protocol.
pub(crate) struct PermutationInstance<'a, T: Copy> {
    pub(crate) pair: &'a PermutationPair,
    pub(crate) challenge: PermutationChallenge<T>,
}

/// Randomness for a single instance of a permutation check protocol.
#[derive(Copy, Clone)]
pub(crate) struct PermutationChallenge<T: Copy> {
    /// Randomness used to combine multiple columns into one.
    pub(crate) beta: T,
    /// Random offset that's added to the beta-reduced column values.
    pub(crate) gamma: T,
}

/// Like `PermutationChallenge`, but with `num_challenges` copies to boost soundness.
#[derive(Clone)]
pub(crate) struct PermutationChallengeSet<T: Copy> {
    pub(crate) challenges: Vec<PermutationChallenge<T>>,
}

/// Compute all Z polynomials (for permutation arguments).
pub(crate) fn compute_permutation_z_polys<F, S, const D: usize>(
    stark: &S,
    config: &StarkConfig,
    trace_poly_values: &[PolynomialValues<F>],
    permutation_challenge_sets: &[PermutationChallengeSet<F>],
) -> Vec<PolynomialValues<F>>
where
    F: RichField + Extendable<D>,
    S: Stark<F, D>,
{
    let permutation_pairs = stark.permutation_pairs();
    let permutation_batches = get_permutation_batches(
        &permutation_pairs,
        permutation_challenge_sets,
        config.num_challenges,
        stark.permutation_batch_size(),
    );

    permutation_batches
        .into_par_iter()
        .map(|instances| compute_permutation_z_poly(&instances, trace_poly_values))
        .collect()
}

/// Compute a single Z polynomial.
fn compute_permutation_z_poly<F: Field>(
    instances: &[PermutationInstance<F>],
    trace_poly_values: &[PolynomialValues<F>],
) -> PolynomialValues<F> {
    let degree = trace_poly_values[0].len();
    let (reduced_lhs_polys, reduced_rhs_polys): (Vec<_>, Vec<_>) = instances
        .iter()
        .map(|instance| permutation_reduced_polys(instance, trace_poly_values, degree))
        .unzip();

    let numerator = poly_product_elementwise(reduced_lhs_polys.into_iter());
    let denominator = poly_product_elementwise(reduced_rhs_polys.into_iter());

    // Compute the quotients.
    let denominator_inverses = F::batch_multiplicative_inverse(&denominator.values);
    let mut quotients = numerator.values;
    batch_multiply_inplace(&mut quotients, &denominator_inverses);

    // Compute Z, which contains partial products of the quotients.
    let mut partial_products = Vec::with_capacity(degree);
    let mut acc = F::ONE;
    for q in quotients {
        partial_products.push(acc);
        acc *= q;
    }
    PolynomialValues::new(partial_products)
}

/// Computes the reduced polynomial, `\sum beta^i f_i(x) + gamma`, for both the "left" and "right"
/// sides of a given `PermutationPair`.
fn permutation_reduced_polys<F: Field>(
    instance: &PermutationInstance<F>,
    trace_poly_values: &[PolynomialValues<F>],
    degree: usize,
) -> (PolynomialValues<F>, PolynomialValues<F>) {
    let PermutationInstance {
        pair: PermutationPair { column_pairs },
        challenge: PermutationChallenge { beta, gamma },
    } = instance;

    let mut reduced_lhs = PolynomialValues::constant(*gamma, degree);
    let mut reduced_rhs = PolynomialValues::constant(*gamma, degree);
    for ((lhs, rhs), weight) in column_pairs.iter().zip(beta.powers()) {
        reduced_lhs.add_assign_scaled(&trace_poly_values[*lhs], weight);
        reduced_rhs.add_assign_scaled(&trace_poly_values[*rhs], weight);
    }
    (reduced_lhs, reduced_rhs)
}

/// Computes the elementwise product of a set of polynomials. Assumes that the set is non-empty and
/// that each polynomial has the same length.
fn poly_product_elementwise<F: Field>(
    mut polys: impl Iterator<Item = PolynomialValues<F>>,
) -> PolynomialValues<F> {
    let mut product = polys.next().expect("Expected at least one polynomial");
    for poly in polys {
        batch_multiply_inplace(&mut product.values, &poly.values)
    }
    product
}

fn get_permutation_challenge<F: RichField, H: Hasher<F>>(
    challenger: &mut Challenger<F, H>,
) -> PermutationChallenge<F> {
    let beta = challenger.get_challenge();
    let gamma = challenger.get_challenge();
    PermutationChallenge { beta, gamma }
}

fn get_permutation_challenge_set<F: RichField, H: Hasher<F>>(
    challenger: &mut Challenger<F, H>,
    num_challenges: usize,
) -> PermutationChallengeSet<F> {
    let challenges = (0..num_challenges)
        .map(|_| get_permutation_challenge(challenger))
        .collect();
    PermutationChallengeSet { challenges }
}

pub(crate) fn get_n_permutation_challenge_sets<F: RichField, H: Hasher<F>>(
    challenger: &mut Challenger<F, H>,
    num_challenges: usize,
    num_sets: usize,
) -> Vec<PermutationChallengeSet<F>> {
    (0..num_sets)
        .map(|_| get_permutation_challenge_set(challenger, num_challenges))
        .collect()
}

fn get_permutation_challenge_target<
    F: RichField + Extendable<D>,
    H: AlgebraicHasher<F>,
    const D: usize,
>(
    builder: &mut CircuitBuilder<F, D>,
    challenger: &mut RecursiveChallenger<F, H, D>,
) -> PermutationChallenge<Target> {
    let beta = challenger.get_challenge(builder);
    let gamma = challenger.get_challenge(builder);
    PermutationChallenge { beta, gamma }
}

fn get_permutation_challenge_set_target<
    F: RichField + Extendable<D>,
    H: AlgebraicHasher<F>,
    const D: usize,
>(
    builder: &mut CircuitBuilder<F, D>,
    challenger: &mut RecursiveChallenger<F, H, D>,
    num_challenges: usize,
) -> PermutationChallengeSet<Target> {
    let challenges = (0..num_challenges)
        .map(|_| get_permutation_challenge_target(builder, challenger))
        .collect();
    PermutationChallengeSet { challenges }
}

pub(crate) fn get_n_permutation_challenge_sets_target<
    F: RichField + Extendable<D>,
    H: AlgebraicHasher<F>,
    const D: usize,
>(
    builder: &mut CircuitBuilder<F, D>,
    challenger: &mut RecursiveChallenger<F, H, D>,
    num_challenges: usize,
    num_sets: usize,
) -> Vec<PermutationChallengeSet<Target>> {
    (0..num_sets)
        .map(|_| get_permutation_challenge_set_target(builder, challenger, num_challenges))
        .collect()
}

/// Get a list of instances of our batch-permutation argument. These are permutation arguments
/// where the same `Z(x)` polynomial is used to check more than one permutation.
/// Before batching, each permutation pair leads to `num_challenges` permutation arguments, so we
/// start with the cartesian product of `permutation_pairs` and `0..num_challenges`. Then we
/// chunk these arguments based on our batch size.
pub(crate) fn get_permutation_batches<'a, T: Copy>(
    permutation_pairs: &'a [PermutationPair],
    permutation_challenge_sets: &[PermutationChallengeSet<T>],
    num_challenges: usize,
    batch_size: usize,
) -> Vec<Vec<PermutationInstance<'a, T>>> {
    permutation_pairs
        .iter()
        .cartesian_product(0..num_challenges)
        .chunks(batch_size)
        .into_iter()
        .map(|batch| {
            batch
                .enumerate()
                .map(|(i, (pair, chal))| {
                    let challenge = permutation_challenge_sets[i].challenges[chal];
                    PermutationInstance { pair, challenge }
                })
                .collect_vec()
        })
        .collect()
}

pub struct PermutationCheckVars<F, FE, P, const D2: usize>
where
    F: Field,
    FE: FieldExtension<D2, BaseField = F>,
    P: PackedField<Scalar = FE>,
{
    pub(crate) local_zs: Vec<P>,
    pub(crate) next_zs: Vec<P>,
    pub(crate) permutation_challenge_sets: Vec<PermutationChallengeSet<F>>,
}

pub(crate) fn eval_permutation_checks<F, FE, P, S, const D: usize, const D2: usize>(
    stark: &S,
    config: &StarkConfig,
    vars: StarkEvaluationVars<FE, P, { S::COLUMNS }, { S::PUBLIC_INPUTS }>,
    permutation_data: PermutationCheckVars<F, FE, P, D2>,
    consumer: &mut ConstraintConsumer<P>,
) where
    F: RichField + Extendable<D>,
    FE: FieldExtension<D2, BaseField = F>,
    P: PackedField<Scalar = FE>,
    S: Stark<F, D>,
    [(); S::COLUMNS]:,
    [(); S::PUBLIC_INPUTS]:,
{
    let PermutationCheckVars {
        local_zs,
        next_zs,
        permutation_challenge_sets,
    } = permutation_data;

    // Check that Z(1) = 1;
    for &z in &local_zs {
        consumer.constraint_first_row(z - FE::ONE);
    }

    let permutation_pairs = stark.permutation_pairs();

    let permutation_batches = get_permutation_batches(
        &permutation_pairs,
        &permutation_challenge_sets,
        config.num_challenges,
        stark.permutation_batch_size(),
    );

    // Each zs value corresponds to a permutation batch.
    for (i, instances) in permutation_batches.iter().enumerate() {
        // Z(gx) * down = Z x  * up
        let (reduced_lhs, reduced_rhs): (Vec<P>, Vec<P>) = instances
            .iter()
            .map(|instance| {
                let PermutationInstance {
                    pair: PermutationPair { column_pairs },
                    challenge: PermutationChallenge { beta, gamma },
                } = instance;
                let mut factor = ReducingFactor::new(*beta);
                let (lhs, rhs): (Vec<_>, Vec<_>) = column_pairs
                    .iter()
                    .map(|&(i, j)| (vars.local_values[i], vars.local_values[j]))
                    .unzip();
                (
                    factor.reduce_ext(lhs.into_iter()) + FE::from_basefield(*gamma),
                    factor.reduce_ext(rhs.into_iter()) + FE::from_basefield(*gamma),
                )
            })
            .unzip();
        let constraint = next_zs[i] * reduced_rhs.into_iter().product::<P>()
            - local_zs[i] * reduced_lhs.into_iter().product::<P>();
        consumer.constraint(constraint);
    }
}

pub struct PermutationCheckDataTarget<const D: usize> {
    pub(crate) local_zs: Vec<ExtensionTarget<D>>,
    pub(crate) next_zs: Vec<ExtensionTarget<D>>,
    pub(crate) permutation_challenge_sets: Vec<PermutationChallengeSet<Target>>,
}

pub(crate) fn eval_permutation_checks_circuit<F, S, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    stark: &S,
    config: &StarkConfig,
    vars: StarkEvaluationTargets<D, { S::COLUMNS }, { S::PUBLIC_INPUTS }>,
    permutation_data: PermutationCheckDataTarget<D>,
    consumer: &mut RecursiveConstraintConsumer<F, D>,
) where
    F: RichField + Extendable<D>,
    S: Stark<F, D>,
    [(); S::COLUMNS]:,
    [(); S::PUBLIC_INPUTS]:,
{
    let PermutationCheckDataTarget {
        local_zs,
        next_zs,
        permutation_challenge_sets,
    } = permutation_data;

    let one = builder.one_extension();
    // Check that Z(1) = 1;
    for &z in &local_zs {
        let z_1 = builder.sub_extension(z, one);
        consumer.constraint_first_row(builder, z_1);
    }

    let permutation_pairs = stark.permutation_pairs();

    let permutation_batches = get_permutation_batches(
        &permutation_pairs,
        &permutation_challenge_sets,
        config.num_challenges,
        stark.permutation_batch_size(),
    );

    // Each zs value corresponds to a permutation batch.
    for (i, instances) in permutation_batches.iter().enumerate() {
        let (reduced_lhs, reduced_rhs): (Vec<ExtensionTarget<D>>, Vec<ExtensionTarget<D>>) =
            instances
                .iter()
                .map(|instance| {
                    let PermutationInstance {
                        pair: PermutationPair { column_pairs },
                        challenge: PermutationChallenge { beta, gamma },
                    } = instance;
                    let beta_ext = builder.convert_to_ext(*beta);
                    let gamma_ext = builder.convert_to_ext(*gamma);
                    let mut factor = ReducingFactorTarget::new(beta_ext);
                    let (lhs, rhs): (Vec<_>, Vec<_>) = column_pairs
                        .iter()
                        .map(|&(i, j)| (vars.local_values[i], vars.local_values[j]))
                        .unzip();
                    let reduced_lhs = factor.reduce(&lhs, builder);
                    let reduced_rhs = factor.reduce(&rhs, builder);
                    (
                        builder.add_extension(reduced_lhs, gamma_ext),
                        builder.add_extension(reduced_rhs, gamma_ext),
                    )
                })
                .unzip();
        let reduced_lhs_product = builder.mul_many_extension(reduced_lhs);
        let reduced_rhs_product = builder.mul_many_extension(reduced_rhs);
        // constraint = next_zs[i] * reduced_rhs_product - local_zs[i] * reduced_lhs_product
        let constraint = {
            let tmp = builder.mul_extension(local_zs[i], reduced_lhs_product);
            builder.mul_sub_extension(next_zs[i], reduced_rhs_product, tmp)
        };
        consumer.constraint(builder, constraint)
    }
}
