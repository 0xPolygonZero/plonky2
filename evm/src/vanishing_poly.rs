use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::packed::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::config::GenericConfig;

use crate::config::StarkConfig;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cross_table_lookup::{
    eval_cross_table_lookup_checks, eval_cross_table_lookup_checks_circuit, CtlCheckVars,
    CtlCheckVarsTarget,
};
use crate::permutation::{
    eval_permutation_checks, eval_permutation_checks_circuit, PermutationCheckDataTarget,
    PermutationCheckVars,
};
use crate::stark::Stark;
use crate::vars::{StarkEvaluationTargets, StarkEvaluationVars};

pub(crate) fn eval_vanishing_poly<F, FE, P, C, S, const D: usize, const D2: usize>(
    stark: &S,
    config: &StarkConfig,
    vars: StarkEvaluationVars<FE, P, { S::COLUMNS }>,
    permutation_vars: Option<PermutationCheckVars<F, FE, P, D2>>,
    ctl_vars: &[CtlCheckVars<F, FE, P, D2>],
    consumer: &mut ConstraintConsumer<P>,
) where
    F: RichField + Extendable<D>,
    FE: FieldExtension<D2, BaseField = F>,
    P: PackedField<Scalar = FE>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
{
    stark.eval_packed_generic(vars, consumer);
    if let Some(permutation_vars) = permutation_vars {
        eval_permutation_checks::<F, FE, P, C, S, D, D2>(
            stark,
            config,
            vars,
            permutation_vars,
            consumer,
        );
    }
    eval_cross_table_lookup_checks::<F, FE, P, C, S, D, D2>(vars, ctl_vars, consumer);
}

pub(crate) fn eval_vanishing_poly_circuit<F, C, S, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    stark: &S,
    config: &StarkConfig,
    vars: StarkEvaluationTargets<D, { S::COLUMNS }>,
    permutation_data: Option<PermutationCheckDataTarget<D>>,
    ctl_vars: &[CtlCheckVarsTarget<F, D>],
    consumer: &mut RecursiveConstraintConsumer<F, D>,
) where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
    [(); S::COLUMNS]:,
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
    eval_cross_table_lookup_checks_circuit::<S, F, D>(builder, vars, ctl_vars, consumer);
}
