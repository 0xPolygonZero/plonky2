use plonky2::field::extension_field::Extendable;
use plonky2::field::packed_field::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::config::GenericConfig;
use rayon::prelude::*;

use crate::config::StarkConfig;
use crate::constraint_consumer::ConstraintConsumer;
use crate::permutation::{get_permutation_batches, PermutationChallenge};
use crate::stark::Stark;
use crate::vars::StarkEvaluationVars;

pub(crate) fn eval_vanishing_poly<F, C, S, const D: usize>(
    stark: S,
    config: &StarkConfig,
    vars: StarkEvaluationVars<F, F, S::COLUMNS, S::PUBLIC_INPUTS>,
    local_zs: &[F::Extension],
    next_zs: &[F::Extension],
    mut consumer: ConstraintConsumer<F>,
    permutation_challenge_sets: &[PermutationChallenge<F>],
) where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
    [(); S::COLUMNS]:,
    [(); S::PUBLIC_INPUTS]:,
{
    stark.eval_packed_base(vars, &mut consumer);
}

fn eval_permutation_checks<F, C, S, const D: usize>(
    stark: S,
    config: &StarkConfig,
    vars: StarkEvaluationVars<F::Extension, F::Extension, S::COLUMNS, S::PUBLIC_INPUTS>,
    local_zs: &[F::Extension],
    next_zs: &[F::Extension],
    mut consumer: ConstraintConsumer<F>,
    permutation_challenge_sets: &[PermutationChallenge<F>],
) where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
    [(); S::COLUMNS]:,
    [(); S::PUBLIC_INPUTS]:,
{
    let permutation_pairs = stark.permutation_pairs();

    let permutation_batches = get_permutation_batches(
        &permutation_pairs,
        &permutation_challenge_sets,
        config.num_challenges,
        stark.permutation_batch_size(),
    );

    // Each zs value corresponds to a permutation batch.
    permutation_batches
        .into_par_iter()
        .map(|instances| compute_permutation_z_poly(&instances, trace_poly_values))
        .collect()
}
