use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::packed::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::circuit_builder::CircuitBuilder;

use crate::config::StarkConfig;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::permutation::{
    eval_permutation_checks, eval_permutation_checks_circuit, PermutationCheckDataTarget,
    PermutationCheckVars,
};
use crate::stark::Stark;
use crate::vars::{StarkEvaluationTargets, StarkEvaluationVars};

pub(crate) fn eval_vanishing_poly<F, FE, P, S, const D: usize, const D2: usize>(
    stark: &S,
    config: &StarkConfig,
    vars: StarkEvaluationVars<FE, P, { S::COLUMNS }, { S::PUBLIC_INPUTS }>,
    permutation_data: Option<PermutationCheckVars<F, FE, P, D2>>,
    consumer: &mut ConstraintConsumer<P>,
) where
    F: RichField + Extendable<D>,
    FE: FieldExtension<D2, BaseField = F>,
    P: PackedField<Scalar = FE>,
    S: Stark<F, D>,
    [(); S::COLUMNS]:,
    [(); S::PUBLIC_INPUTS]:,
{
    stark.eval_packed_generic(vars, consumer);
    if let Some(permutation_data) = permutation_data {
        eval_permutation_checks::<F, FE, P, S, D, D2>(
            stark,
            config,
            vars,
            permutation_data,
            consumer,
        );
    }
}

pub(crate) fn eval_vanishing_poly_circuit<F, S, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    stark: &S,
    config: &StarkConfig,
    vars: StarkEvaluationTargets<D, { S::COLUMNS }, { S::PUBLIC_INPUTS }>,
    permutation_data: Option<PermutationCheckDataTarget<D>>,
    consumer: &mut RecursiveConstraintConsumer<F, D>,
) where
    F: RichField + Extendable<D>,
    S: Stark<F, D>,
    [(); S::COLUMNS]:,
    [(); S::PUBLIC_INPUTS]:,
{
    stark.eval_ext_circuit(builder, vars, consumer);
    if let Some(permutation_data) = permutation_data {
        eval_permutation_checks_circuit::<F, S, D>(
            builder,
            stark,
            config,
            vars,
            permutation_data,
            consumer,
        );
    }
}
