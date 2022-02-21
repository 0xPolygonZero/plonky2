use plonky2::field::extension_field::{Extendable, FieldExtension};
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::config::GenericConfig;

use crate::config::StarkConfig;
use crate::constraint_consumer::ConstraintConsumer;
use crate::permutation::{eval_permutation_checks, PermutationCheckData};
use crate::stark::Stark;
use crate::vars::StarkEvaluationVars;

pub(crate) fn eval_vanishing_poly<F, FE, C, S, const D: usize, const D2: usize>(
    stark: &S,
    config: &StarkConfig,
    vars: StarkEvaluationVars<FE, FE, { S::COLUMNS }, { S::PUBLIC_INPUTS }>,
    permutation_data: Option<PermutationCheckData<F, FE, D2>>,
    consumer: &mut ConstraintConsumer<FE>,
) where
    F: RichField + Extendable<D>,
    FE: FieldExtension<D2, BaseField = F>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
    [(); S::COLUMNS]:,
    [(); S::PUBLIC_INPUTS]:,
{
    stark.eval_packed_generic(vars, consumer);
    if let Some(PermutationCheckData {
        local_zs,
        next_zs,
        permutation_challenge_sets,
    }) = permutation_data
    {
        eval_permutation_checks::<F, FE, C, S, D, D2>(
            stark,
            config,
            vars,
            &local_zs,
            &next_zs,
            consumer,
            &permutation_challenge_sets,
        );
    }
}
