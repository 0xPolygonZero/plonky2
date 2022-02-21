use plonky2::field::extension_field::{Extendable, FieldExtension};
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::config::GenericConfig;

use crate::config::StarkConfig;
use crate::constraint_consumer::ConstraintConsumer;
use crate::permutation::{
    get_permutation_batches, PermutationChallenge, PermutationChallengeSet, PermutationInstance,
    PermutationPair,
};
use crate::stark::Stark;
use crate::vars::StarkEvaluationVars;

pub(crate) fn eval_vanishing_poly<F, C, S, const D: usize>(
    stark: S,
    config: &StarkConfig,
    vars: StarkEvaluationVars<F::Extension, F::Extension, { S::COLUMNS }, { S::PUBLIC_INPUTS }>,
    local_zs: &[F::Extension],
    next_zs: &[F::Extension],
    mut consumer: ConstraintConsumer<F::Extension>,
    permutation_challenge_sets: &[PermutationChallengeSet<F>],
) where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
    [(); S::COLUMNS]:,
    [(); S::PUBLIC_INPUTS]:,
{
    stark.eval_packed_generic(vars, &mut consumer);
}

fn eval_permutation_checks<F, C, S, const D: usize>(
    stark: S,
    config: &StarkConfig,
    vars: StarkEvaluationVars<F::Extension, F::Extension, { S::COLUMNS }, { S::PUBLIC_INPUTS }>,
    local_zs: &[F::Extension],
    next_zs: &[F::Extension],
    consumer: &mut ConstraintConsumer<F::Extension>,
    permutation_challenge_sets: &[PermutationChallengeSet<F>],
) where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
    [(); S::COLUMNS]:,
    [(); S::PUBLIC_INPUTS]:,
{
    // TODO: Z_1 check.
    let permutation_pairs = stark.permutation_pairs();

    let permutation_batches = get_permutation_batches(
        &permutation_pairs,
        permutation_challenge_sets,
        config.num_challenges,
        stark.permutation_batch_size(),
    );

    // Each zs value corresponds to a permutation batch.
    for (i, instances) in permutation_batches.iter().enumerate() {
        // Z(gx) * down = Z x  * up
        let (reduced_lhs, reduced_rhs): (Vec<F::Extension>, Vec<F::Extension>) = instances
            .iter()
            .map(|instance| {
                let PermutationInstance {
                    pair: PermutationPair { column_pairs },
                    challenge: PermutationChallenge { beta, gamma },
                } = instance;
                column_pairs.iter().rev().fold(
                    (
                        F::Extension::from_basefield(*gamma),
                        F::Extension::from_basefield(*gamma),
                    ),
                    |(lhs, rhs), &(i, j)| {
                        (
                            lhs.scalar_mul(*beta) + vars.local_values[i],
                            rhs.scalar_mul(*beta) + vars.local_values[j],
                        )
                    },
                )
            })
            .unzip();
        let constraint = next_zs[i] * reduced_rhs.into_iter().product()
            - local_zs[i] * reduced_lhs.into_iter().product();
        consumer.constraint(constraint);
    }
}
