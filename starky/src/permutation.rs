//! Permutation arguments.

use itertools::Itertools;
use plonky2::field::batch_util::batch_multiply_inplace;
use plonky2::field::extension_field::Extendable;
use plonky2::field::field_types::Field;
use plonky2::field::polynomial::PolynomialValues;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::challenger::Challenger;
use plonky2::plonk::config::{GenericConfig, Hasher};
use rayon::prelude::*;

use crate::config::StarkConfig;
use crate::stark::Stark;

/// A pair of lists of columns, `lhs` and `rhs`, that should be permutations of one another.
/// In particular, there should exist some permutation `pi` such that for any `i`,
/// `trace[lhs[i]] = pi(trace[rhs[i]])`. Here `trace` denotes the trace in column-major form, so
/// `trace[col]` is a column vector.
pub struct PermutationPair {
    /// Each entry contains two column indices, representing two columns which should be
    /// permutations of one another.
    pub column_pairs: Vec<(usize, usize)>,
}

/// A single instance of a permutation check protocol.
pub(crate) struct PermutationInstance<'a, F: Field> {
    pub(crate) pair: &'a PermutationPair,
    pub(crate) challenge: PermutationChallenge<F>,
}

/// Randomness for a single instance of a permutation check protocol.
#[derive(Copy, Clone)]
pub(crate) struct PermutationChallenge<F: Field> {
    /// Randomness used to combine multiple columns into one.
    pub(crate) beta: F,
    /// Random offset that's added to the beta-reduced column values.
    pub(crate) gamma: F,
}

/// Like `PermutationChallenge`, but with `num_challenges` copies to boost soundness.
pub(crate) struct PermutationChallengeSet<F: Field> {
    pub(crate) challenges: Vec<PermutationChallenge<F>>,
}

/// Compute all Z polynomials (for permutation arguments).
pub(crate) fn compute_permutation_z_polys<F, C, S, const D: usize>(
    stark: &S,
    config: &StarkConfig,
    challenger: &mut Challenger<F, C::Hasher>,
    trace_poly_values: &[PolynomialValues<F>],
) -> Vec<PolynomialValues<F>>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
{
    let permutation_pairs = stark.permutation_pairs();
    let permutation_challenge_sets = get_n_permutation_challenge_sets(
        challenger,
        config.num_challenges,
        stark.permutation_batch_size(),
    );

    // Get a list of instances of our batch-permutation argument. These are permutation arguments
    // where the same `Z(x)` polynomial is used to check more than one permutation.
    // Before batching, each permutation pair leads to `num_challenges` permutation arguments, so we
    // start with the cartesian product of `permutation_pairs` and `0..num_challenges`. Then we
    // chunk these arguments based on our batch size.
    let permutation_instances = permutation_pairs
        .iter()
        .cartesian_product(0..config.num_challenges)
        .chunks(stark.permutation_batch_size())
        .into_iter()
        .flat_map(|batch| {
            batch.enumerate().map(|(i, (pair, chal))| {
                let challenge = permutation_challenge_sets[i].challenges[chal];
                PermutationInstance { pair, challenge }
            })
        })
        .collect_vec();

    permutation_instances
        .into_par_iter()
        .map(|instance| compute_permutation_z_poly(instance, trace_poly_values))
        .collect()
}

/// Compute a single Z polynomial.
// TODO: Change this to handle a batch of `PermutationInstance`s.
fn compute_permutation_z_poly<F: Field>(
    instance: PermutationInstance<F>,
    trace_poly_values: &[PolynomialValues<F>],
) -> PolynomialValues<F> {
    let PermutationInstance { pair, challenge } = instance;
    let PermutationPair { column_pairs } = pair;
    let PermutationChallenge { beta, gamma } = challenge;

    let degree = trace_poly_values[0].len();
    let mut reduced_lhs = PolynomialValues::constant(gamma, degree);
    let mut reduced_rhs = PolynomialValues::constant(gamma, degree);

    for ((lhs, rhs), weight) in column_pairs.iter().zip(beta.powers()) {
        reduced_lhs.add_assign_scaled(&trace_poly_values[*lhs], weight);
        reduced_rhs.add_assign_scaled(&trace_poly_values[*rhs], weight);
    }

    // Compute the quotients.
    let reduced_rhs_inverses = F::batch_multiplicative_inverse(&reduced_rhs.values);
    let mut quotients = reduced_lhs.values;
    batch_multiply_inplace(&mut quotients, &reduced_rhs_inverses);

    // Compute Z, which contains partial products of the quotients.
    let mut partial_products = Vec::with_capacity(degree);
    let mut acc = F::ONE;
    for q in quotients {
        partial_products.push(acc);
        acc *= q;
    }
    PolynomialValues::new(partial_products)
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
