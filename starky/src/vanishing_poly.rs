#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::iop::target::Target;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::with_context;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cross_table_lookup::{
    eval_cross_table_lookup_checks, eval_cross_table_lookup_checks_circuit, CtlCheckVars,
    CtlCheckVarsTarget,
};
use crate::evaluation_frame::StarkEvaluationFrame;
use crate::lookup::{
    eval_ext_lookups_circuit, eval_packed_lookups_generic, Lookup, LookupCheckVars,
    LookupCheckVarsTarget,
};
use crate::proof::{StarkOpeningSet, StarkOpeningSetTarget};
use crate::stark::Stark;

/// Evaluates all constraint, permutation and cross-table lookup polynomials
/// of the current STARK at the local and next values.
pub(crate) fn eval_vanishing_poly<F, FE, P, S, const D: usize, const D2: usize>(
    stark: &S,
    vars: &S::EvaluationFrame<FE, P, D2>,
    lookups: &[Lookup<F>],
    lookup_vars: Option<LookupCheckVars<F, FE, P, D2>>,
    ctl_vars: Option<&[CtlCheckVars<F, FE, P, D2>]>,
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
    if let Some(ctl_vars) = ctl_vars {
        // Evaluate the STARK constraints related to the CTLs.
        eval_cross_table_lookup_checks::<F, FE, P, S, D, D2>(
            vars,
            ctl_vars,
            consumer,
            stark.constraint_degree(),
        );
    }
}

/// Circuit version of `eval_vanishing_poly`.
/// Evaluates all constraint, permutation and cross-table lookup polynomials
/// of the current STARK at the local and next values.
pub(crate) fn eval_vanishing_poly_circuit<F, S, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    stark: &S,
    vars: &S::EvaluationFrameTarget,
    lookup_vars: Option<LookupCheckVarsTarget<D>>,
    ctl_vars: Option<&[CtlCheckVarsTarget<F, D>]>,
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
    if let Some(ctl_vars) = ctl_vars {
        // Evaluate all of the STARK's constraints related to the CTLs.
        eval_cross_table_lookup_checks_circuit::<S, F, D>(
            builder,
            vars,
            ctl_vars,
            consumer,
            stark.constraint_degree(),
        );
    }
}

/// Evaluate the Lagrange polynomials `L_0` and `L_(n-1)` at a point `x`.
/// `L_0(x) = (x^n - 1)/(n * (x - 1))`
/// `L_(n-1)(x) = (x^n - 1)/(n * (g * x - 1))`, with `g` the first element of the subgroup.
pub(crate) fn eval_l_0_and_l_last<F: Field>(log_n: usize, x: F) -> (F, F) {
    let n = F::from_canonical_usize(1 << log_n);
    let g = F::primitive_root_of_unity(log_n);
    let z_x = x.exp_power_of_2(log_n) - F::ONE;
    let invs = F::batch_multiplicative_inverse(&[n * (x - F::ONE), n * (g * x - F::ONE)]);

    (z_x * invs[0], z_x * invs[1])
}

/// Evaluates the constraints at a random extension point. It is used to bind the constraints.
pub(crate) fn compute_eval_vanishing_poly<F, S, const D: usize>(
    stark: &S,
    stark_opening_set: &StarkOpeningSet<F, D>,
    ctl_vars: Option<&[CtlCheckVars<F, F::Extension, F::Extension, D>]>,
    lookup_challenges: Option<&Vec<F>>,
    lookups: &[Lookup<F>],
    public_inputs: &[F],
    alphas: Vec<F>,
    zeta: F::Extension,
    degree_bits: usize,
    num_lookup_columns: usize,
) -> Vec<F::Extension>
where
    F: RichField + Extendable<D>,
    S: Stark<F, D>,
{
    let StarkOpeningSet {
        local_values,
        next_values,
        auxiliary_polys,
        auxiliary_polys_next,
        ctl_zs_first: _,
        quotient_polys: _,
    } = &stark_opening_set;

    let (l_0, l_last) = eval_l_0_and_l_last(degree_bits, zeta);
    let last = F::primitive_root_of_unity(degree_bits).inverse();
    let z_last = zeta - last.into();

    let mut consumer = ConstraintConsumer::<F::Extension>::new(
        alphas
            .iter()
            .map(|&alpha| F::Extension::from_basefield(alpha))
            .collect::<Vec<_>>(),
        z_last,
        l_0,
        l_last,
    );

    let vars = S::EvaluationFrame::from_values(
        local_values,
        next_values,
        &public_inputs
            .iter()
            .copied()
            .map(F::Extension::from_basefield)
            .collect::<Vec<_>>(),
    );

    let lookup_vars = lookup_challenges.map(|l_c| LookupCheckVars {
        local_values: auxiliary_polys.as_ref().unwrap()[..num_lookup_columns].to_vec(),
        next_values: auxiliary_polys_next.as_ref().unwrap()[..num_lookup_columns].to_vec(),
        challenges: l_c.to_vec(),
    });

    eval_vanishing_poly::<F, F::Extension, F::Extension, S, D, D>(
        stark,
        &vars,
        lookups,
        lookup_vars,
        ctl_vars,
        &mut consumer,
    );
    consumer.accumulators()
}

pub(crate) fn eval_l_0_and_l_last_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    n: ExtensionTarget<D>,
    g: ExtensionTarget<D>,
    x: ExtensionTarget<D>,
    z_x: ExtensionTarget<D>,
) -> (ExtensionTarget<D>, ExtensionTarget<D>) {
    let one = builder.one_extension();
    let l_0_deno = builder.mul_sub_extension(n, x, n);
    let l_last_deno = builder.mul_sub_extension(g, x, one);
    let l_last_deno = builder.mul_extension(n, l_last_deno);

    (
        builder.div_extension(z_x, l_0_deno),
        builder.div_extension(z_x, l_last_deno),
    )
}

/// Evaluates the constraints at a random extension point. It is used to bind the constraints.
pub(crate) fn compute_eval_vanishing_poly_circuit<F, S, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    stark: &S,
    stark_opening_set: &StarkOpeningSetTarget<D>,
    ctl_vars: Option<&[CtlCheckVarsTarget<F, D>]>,
    lookup_challenges: Option<&Vec<Target>>,
    public_inputs: &[Target],
    alphas: Vec<Target>,
    zeta: ExtensionTarget<D>,
    degree_bits: usize,
    degree_bits_target: Target,
    num_lookup_columns: usize,
) -> Vec<ExtensionTarget<D>>
where
    F: RichField + Extendable<D>,
    S: Stark<F, D>,
{
    let one = builder.one_extension();
    let two = builder.two();

    let StarkOpeningSetTarget {
        local_values,
        next_values,
        auxiliary_polys,
        auxiliary_polys_next,
        ctl_zs_first: _,
        quotient_polys: _,
    } = stark_opening_set;

    let max_num_of_bits_in_degree = degree_bits + 1;
    let degree = builder.exp(two, degree_bits_target, max_num_of_bits_in_degree);
    let degree_bits_vec = builder.split_le(degree, max_num_of_bits_in_degree);
    let zeta_pow_deg = builder.exp_extension_from_bits(zeta, &degree_bits_vec);
    let z_h_zeta = builder.sub_extension(zeta_pow_deg, one);
    let degree_ext = builder.convert_to_ext(degree);

    // Calculate primitive_root_of_unity(degree_bits)
    let two_adicity = builder.constant(F::from_canonical_usize(F::TWO_ADICITY));
    let two_adicity_sub_degree_bits = builder.sub(two_adicity, degree_bits_target);
    let two_exp_two_adicity_sub_degree_bits =
        builder.exp(two, two_adicity_sub_degree_bits, F::TWO_ADICITY);
    let base = builder.constant(F::POWER_OF_TWO_GENERATOR);
    let g = builder.exp(base, two_exp_two_adicity_sub_degree_bits, F::TWO_ADICITY);
    let g_ext = builder.convert_to_ext(g);

    let (l_0, l_last) = eval_l_0_and_l_last_circuit(builder, degree_ext, g_ext, zeta, z_h_zeta);
    let last = builder.inverse_extension(g_ext);
    let z_last = builder.sub_extension(zeta, last);

    let mut consumer = RecursiveConstraintConsumer::<F, D>::new(
        builder.zero_extension(),
        alphas,
        z_last,
        l_0,
        l_last,
    );

    let vars = S::EvaluationFrameTarget::from_values(
        local_values,
        next_values,
        &public_inputs
            .iter()
            .map(|&t| builder.convert_to_ext(t))
            .collect::<Vec<_>>(),
    );

    let lookup_vars = stark.uses_lookups().then(|| LookupCheckVarsTarget {
        local_values: auxiliary_polys.as_ref().unwrap()[..num_lookup_columns].to_vec(),
        next_values: auxiliary_polys_next.as_ref().unwrap()[..num_lookup_columns].to_vec(),
        challenges: lookup_challenges.unwrap().to_vec(),
    });

    with_context!(
        builder,
        "evaluate extra vanishing polynomial",
        eval_vanishing_poly_circuit::<F, S, D>(
            builder,
            stark,
            &vars,
            lookup_vars,
            ctl_vars,
            &mut consumer
        )
    );
    consumer.accumulators()
}
