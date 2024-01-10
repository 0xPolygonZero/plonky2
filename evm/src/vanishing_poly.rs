use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::packed::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::circuit_builder::CircuitBuilder;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cross_table_lookup::{
    eval_cross_table_lookup_checks, eval_cross_table_lookup_checks_circuit, CtlCheckVars,
    CtlCheckVarsTarget,
};
use crate::lookup::{
    eval_ext_lookups_circuit, eval_packed_lookups_generic, Lookup, LookupCheckVars,
    LookupCheckVarsTarget,
};
use crate::stark::Stark;

/// Evaluates all constraint, permutation and cross-table lookup polynomials
/// of the current STARK at the local and next values.
pub(crate) fn eval_vanishing_poly<F, FE, P, S, const D: usize, const D2: usize>(
    stark: &S,
    vars: &S::EvaluationFrame<FE, P, D2>,
    lookups: &[Lookup<F>],
    lookup_vars: Option<LookupCheckVars<F, FE, P, D2>>,
    ctl_vars: &[CtlCheckVars<F, FE, P, D2>],
    consumer: &mut ConstraintConsumer<P>,
) where
    F: RichField + Extendable<D>,
    FE: FieldExtension<D2, BaseField = F>,
    P: PackedField<Scalar = FE>,
    S: Stark<F, D>,
{
    // Evaluate all of the STARK's table constraints.
    stark.eval_packed_generic(vars, consumer);
    if let Some(lookup_vars) = lookup_vars {
        // Evaluate the STARK constraints related to the permutation arguments.
        eval_packed_lookups_generic::<F, FE, P, S, D, D2>(
            stark,
            lookups,
            vars,
            lookup_vars,
            consumer,
        );
    }
    // Evaluate the STARK constraints related to the cross-table lookups.
    eval_cross_table_lookup_checks::<F, FE, P, S, D, D2>(
        vars,
        ctl_vars,
        consumer,
        stark.constraint_degree(),
    );
}

/// Circuit version of `eval_vanishing_poly`.
/// Evaluates all constraint, permutation and cross-table lookup polynomials
/// of the current STARK at the local and next values.
pub(crate) fn eval_vanishing_poly_circuit<F, S, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    stark: &S,
    vars: &S::EvaluationFrameTarget,
    lookup_vars: Option<LookupCheckVarsTarget<D>>,
    ctl_vars: &[CtlCheckVarsTarget<F, D>],
    consumer: &mut RecursiveConstraintConsumer<F, D>,
) where
    F: RichField + Extendable<D>,
    S: Stark<F, D>,
{
    // Evaluate all of the STARK's table constraints.
    stark.eval_ext_circuit(builder, vars, consumer);
    if let Some(lookup_vars) = lookup_vars {
        // Evaluate all of the STARK's constraints related to the permutation argument.
        eval_ext_lookups_circuit::<F, S, D>(builder, stark, vars, lookup_vars, consumer);
    }
    // Evaluate all of the STARK's constraints related to the cross-table lookups.
    eval_cross_table_lookup_checks_circuit::<S, F, D>(
        builder,
        vars,
        ctl_vars,
        consumer,
        stark.constraint_degree(),
    );
}
