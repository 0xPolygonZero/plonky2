use itertools::Itertools;
use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::packed::PackedField;
use plonky2::field::polynomial::PolynomialValues;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::iop::target::Target;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::config::GenericConfig;
use plonky2_util::ceil_div_usize;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::stark::Stark;
use crate::vars::{StarkEvaluationTargets, StarkEvaluationVars};

pub struct Lookup {
    pub(crate) columns: Vec<usize>,
    pub(crate) table_column: usize,
    pub(crate) frequencies_column: usize,
}

impl Lookup {
    pub(crate) fn num_helper_columns(&self, constraint_degree: usize) -> usize {
        // Split the lookup columns in batches of size `constraint_degree-1`,
        // then 1 column of inverse of `table + challenge` and one for the `Z` polynomial.
        ceil_div_usize(self.columns.len(), constraint_degree - 1) + 2
    }
}

/// Compute the helper columns for the lookup argument.
/// Given columns `f0,...,fk` and a column `t`, such that `∪fi ⊆ t`, and challenges `x`,
/// we compute the helper columns `h_i = 1/(x+f_2i) + 1/(x+f_2i+1)` and `g = 1/(x+t)`.
pub(crate) fn lookup_helper_columns<F: Field>(
    lookup: &Lookup,
    trace_poly_values: &[PolynomialValues<F>],
    challenge: F,
    constraint_degree: usize,
) -> Vec<PolynomialValues<F>> {
    assert_eq!(
        constraint_degree, 3,
        "TODO: Allow other constraint degrees."
    );
    let num_helper_columns = lookup.num_helper_columns(constraint_degree);
    let mut helper_columns: Vec<PolynomialValues<F>> = Vec::with_capacity(num_helper_columns);

    // TODO: This does one batch inversion per column. It would also be possible to do one batch inversion
    // for every column, but that would require building a big vector of all the columns concatenated.
    // Not sure which approach is better.
    // TODO: The clone could probably be avoided by using a modified version of `batch_multiplicative_inverse`
    // taking `challenge` as an additional argument.
    for mut col_inds in &lookup.columns.iter().chunks(constraint_degree - 1) {
        let first = *col_inds.next().unwrap();
        let mut column = trace_poly_values[first].values.clone();
        for x in column.iter_mut() {
            *x = challenge + *x;
        }
        let mut acc = F::batch_multiplicative_inverse(&column);
        for &ind in col_inds {
            let mut column = trace_poly_values[ind].values.clone();
            for x in column.iter_mut() {
                *x = challenge + *x;
            }
            column = F::batch_multiplicative_inverse(&column);
            for (x, y) in acc.iter_mut().zip(column) {
                *x += y;
            }
        }
        helper_columns.push(acc.into());
    }

    let mut table = trace_poly_values[lookup.table_column].values.clone();
    for x in table.iter_mut() {
        *x = challenge + *x;
    }
    helper_columns.push(F::batch_multiplicative_inverse(&table).into());

    let frequencies = &trace_poly_values[lookup.frequencies_column].values;
    let mut z = Vec::with_capacity(frequencies.len());
    z.push(F::ZERO);
    for i in 0..frequencies.len() - 1 {
        let x = helper_columns[..num_helper_columns - 2]
            .iter()
            .map(|col| col.values[i])
            .sum::<F>()
            - frequencies[i] * helper_columns[num_helper_columns - 2].values[i];
        z.push(z[i] + x);
    }
    helper_columns.push(z.into());

    // Constraints
    for i in 0..frequencies.len() {
        for (j, chunk) in lookup.columns.chunks(constraint_degree - 1).enumerate() {
            let mut x = helper_columns[j].values[i];
            let mut y = F::ZERO;
            let fs: Vec<_> = chunk
                .iter()
                .map(|&k| trace_poly_values[k].values[i])
                .collect();
            for &f in &fs {
                x *= challenge + f;
                y += challenge + f;
            }
            match chunk.len() {
                2 => assert_eq!(
                    x, y,
                    "{} {} {:?} {:?} {} {} {} {}",
                    i, j, chunk, fs, challenge, x, y, helper_columns[j].values[i]
                ),
                1 => assert_eq!(x, F::ONE),
                _ => todo!("Allow other constraint degrees."),
            }
        }
        let x = helper_columns[num_helper_columns - 2].values[i];
        let x = x * (challenge + trace_poly_values[lookup.table_column].values[i]);
        assert!(x.is_one());

        let z = helper_columns[num_helper_columns - 1].values[i];
        let next_z = helper_columns[num_helper_columns - 1].values[(i + 1) % frequencies.len()];
        let y = helper_columns[..num_helper_columns - 2]
            .iter()
            .map(|col| col.values[i])
            .sum::<F>()
            - trace_poly_values[lookup.frequencies_column].values[i]
                * helper_columns[num_helper_columns - 2].values[i];
        assert_eq!(
            next_z - z,
            y,
            "{} {} {} {} {:?}",
            i,
            z,
            y,
            next_z,
            helper_columns
                .iter()
                .map(|col| col.values[i])
                .collect::<Vec<_>>()
        );
    }

    helper_columns
}

pub struct LookupCheckVars<F, FE, P, const D2: usize>
where
    F: Field,
    FE: FieldExtension<D2, BaseField = F>,
    P: PackedField<Scalar = FE>,
{
    pub(crate) local_values: Vec<P>,
    pub(crate) next_values: Vec<P>,
    pub(crate) challenges: Vec<F>,
}

pub(crate) fn eval_lookups_checks<F, FE, P, C, S, const D: usize, const D2: usize>(
    stark: &S,
    lookups: &[Lookup],
    vars: StarkEvaluationVars<FE, P, { S::COLUMNS }>,
    lookup_vars: LookupCheckVars<F, FE, P, D2>,
    yield_constr: &mut ConstraintConsumer<P>,
) where
    F: RichField + Extendable<D>,
    FE: FieldExtension<D2, BaseField = F>,
    P: PackedField<Scalar = FE>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
{
    let degree = stark.constraint_degree();
    assert_eq!(degree, 3, "TODO: Allow other constraint degrees.");
    let mut start = 0;
    for lookup in lookups {
        let num_helper_columns = lookup.num_helper_columns(degree);
        for &challenge in &lookup_vars.challenges {
            let challenge = FE::from_basefield(challenge);
            for (j, chunk) in lookup.columns.chunks(degree - 1).enumerate() {
                let mut x = lookup_vars.local_values[start + j];
                let mut y = P::ZEROS;
                let fs = chunk.iter().map(|&k| vars.local_values[k]);
                for f in fs {
                    x *= f + challenge;
                    y += f + challenge;
                }
                match chunk.len() {
                    2 => yield_constr.constraint(x - y),
                    1 => yield_constr.constraint(x - P::ONES),
                    _ => todo!("Allow other constraint degrees."),
                }
            }
            let x = lookup_vars.local_values[start + num_helper_columns - 2];
            let x = x * (vars.local_values[lookup.table_column] + challenge);
            yield_constr.constraint(x - P::ONES);

            let z = lookup_vars.local_values[start + num_helper_columns - 1];
            let next_z = lookup_vars.next_values[start + num_helper_columns - 1];
            let y = lookup_vars.local_values[start..start + num_helper_columns - 2]
                .iter()
                .fold(P::ZEROS, |acc, x| acc + *x)
                - vars.local_values[lookup.frequencies_column]
                    * lookup_vars.local_values[start + num_helper_columns - 2];
            yield_constr.constraint(next_z - z - y);
            start += num_helper_columns;
        }
    }
}

pub struct LookupCheckVarsTarget<const D: usize> {
    pub(crate) local_values: Vec<ExtensionTarget<D>>,
    pub(crate) next_values: Vec<ExtensionTarget<D>>,
    pub(crate) challenges: Vec<Target>,
}

pub(crate) fn eval_lookups_checks_circuit<
    F: RichField + Extendable<D>,
    S: Stark<F, D>,
    const D: usize,
>(
    builder: &mut CircuitBuilder<F, D>,
    stark: &S,
    vars: StarkEvaluationTargets<D, { S::COLUMNS }>,
    lookup_vars: LookupCheckVarsTarget<D>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let one = builder.one_extension();
    let degree = stark.constraint_degree();
    let lookups = stark.lookups();
    assert_eq!(degree, 3, "TODO: Allow other constraint degrees.");
    let mut start = 0;
    for lookup in lookups {
        let num_helper_columns = lookup.num_helper_columns(degree);
        for &challenge in &lookup_vars.challenges {
            let challenge = builder.convert_to_ext(challenge);
            for (j, chunk) in lookup.columns.chunks(degree - 1).enumerate() {
                let mut x = lookup_vars.local_values[start + j];
                let mut y = builder.zero_extension();
                let fs = chunk.iter().map(|&k| vars.local_values[k]);
                for f in fs {
                    let tmp = builder.add_extension(f, challenge);
                    x = builder.mul_extension(x, tmp);
                    y = builder.add_extension(y, tmp);
                }
                match chunk.len() {
                    2 => {
                        let tmp = builder.sub_extension(x, y);
                        yield_constr.constraint(builder, tmp)
                    }
                    1 => {
                        let tmp = builder.sub_extension(x, one);
                        yield_constr.constraint(builder, tmp)
                    }
                    _ => todo!("Allow other constraint degrees."),
                }
            }
            let x = lookup_vars.local_values[start + num_helper_columns - 2];
            let tmp = builder.add_extension(vars.local_values[lookup.table_column], challenge);
            let x = builder.mul_sub_extension(x, tmp, one);
            yield_constr.constraint(builder, x);

            let z = lookup_vars.local_values[start + num_helper_columns - 1];
            let next_z = lookup_vars.next_values[start + num_helper_columns - 1];
            let y = builder.add_many_extension(
                &lookup_vars.local_values[start..start + num_helper_columns - 2],
            );
            let tmp = builder.mul_extension(
                vars.local_values[lookup.frequencies_column],
                lookup_vars.local_values[start + num_helper_columns - 2],
            );
            let y = builder.sub_extension(y, tmp);
            let constraint = builder.sub_extension(next_z, z);
            let constraint = builder.sub_extension(constraint, y);
            yield_constr.constraint(builder, constraint);
            start += num_helper_columns;
        }
    }
}
