use itertools::Itertools;
use plonky2::field::batch_util::batch_add_inplace;
use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::packed::PackedField;
use plonky2::field::polynomial::PolynomialValues;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::iop::target::Target;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2_util::ceil_div_usize;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::stark::Stark;
use crate::vars::{StarkEvaluationTargets, StarkEvaluationVars};

pub struct Lookup {
    /// Columns whose values should be contained in the lookup table.
    /// These are the f_i(x) polynomials in the logUp paper.
    pub(crate) columns: Vec<usize>,
    /// Column containing the lookup table.
    /// This is the t(x) polynomial in the paper.
    pub(crate) table_column: usize,
    /// Column containing the frequencies of `columns` in `table_column`.
    /// This is the m(x) polynomial in the paper.
    pub(crate) frequencies_column: usize,
}

impl Lookup {
    pub(crate) fn num_helper_columns(&self, constraint_degree: usize) -> usize {
        // One helper column for each column batch of size `constraint_degree-1`,
        // then one column for the inverse of `table + challenge` and one for the `Z` polynomial.
        ceil_div_usize(self.columns.len(), constraint_degree - 1) + 2
    }
}

/// logUp protocol from https://ia.cr/2022/1530 (TODO link to newer version?)
/// Compute the helper columns for the lookup argument.
/// Given columns `f0,...,fk` and a column `t`, such that `∪fi ⊆ t`, and challenges `x`,
/// this computes the helper columns `h_i = 1/(x+f_2i) + 1/(x+f_2i+1)`, `g = 1/(x+t)`,
/// and `Z(gx) = Z(x) + sum h_i(x) - m(x)g(x)` where `m` is the frequencies column.
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

    // For each batch of `constraint_degree-1` columns `fi`, compute `sum 1/(f_i+challenge)` and
    // add it to the helper columns.
    // TODO: This does one batch inversion per column. It would also be possible to do one batch inversion
    // for every column, but that would require building a big vector of all the columns concatenated.
    // Not sure which approach is better.
    // Note: these are the h_k(x) polynomials in the paper, with a few differences:
    //       * Here, the first ratio m_0(x)/phi_0(x) is not included with the columns batched up to create the 
    //         h_k polynomials; instead there's a separate helper column for it (see below).
    //       * Here, we use 1 instead of -1 as the numerator (and subtract later).
    //       * Here, for now, the batch size (l) is always constraint_degree - 1 = 2.
    for mut col_inds in &lookup.columns.iter().chunks(constraint_degree - 1) {
        let first = *col_inds.next().unwrap();
        // TODO: The clone could probably be avoided by using a modified version of `batch_multiplicative_inverse`
        // taking `challenge` as an additional argument.
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
            batch_add_inplace(&mut acc, &column);
        }
        helper_columns.push(acc.into());
    }

    // Add `1/(table+challenge)` to the helper columns.
    // This is 1/phi_0(x) = 1/(x + t(x)) from the paper.
    // Here, we don't include m(x) in the numerator, instead multiplying it with this column later.
    let mut table = trace_poly_values[lookup.table_column].values.clone();
    for x in table.iter_mut() {
        *x = challenge + *x;
    }
    helper_columns.push(F::batch_multiplicative_inverse(&table).into());

    // Compute the `Z` polynomial with `Z(1)=0` and `Z(gx) = Z(x) + sum h_i(x) - frequencies(x)g(x)`.
    // This enforces the check from the paper, that the sum of the h_k(x) polynomials is 0 over H.
    // In the paper, that sum includes m(x)/(x + t(x)) = frequencies(x)/g(x), because that was bundled
    // into the h_k(x) polynomials.
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

/// Constraints for the logUp lookup argument.
pub(crate) fn eval_lookups_checks<F, FE, P, S, const D: usize, const D2: usize>(
    stark: &S,
    lookups: &[Lookup],
    vars: StarkEvaluationVars<FE, P, { S::COLUMNS }>,
    lookup_vars: LookupCheckVars<F, FE, P, D2>,
    yield_constr: &mut ConstraintConsumer<P>,
) where
    F: RichField + Extendable<D>,
    FE: FieldExtension<D2, BaseField = F>,
    P: PackedField<Scalar = FE>,
    S: Stark<F, D>,
{
    let degree = stark.constraint_degree();
    assert_eq!(degree, 3, "TODO: Allow other constraint degrees.");
    let mut start = 0;
    for lookup in lookups {
        let num_helper_columns = lookup.num_helper_columns(degree);
        for &challenge in &lookup_vars.challenges {
            let challenge = FE::from_basefield(challenge);
            // For each chunk, check that `h_i (x+f_2i) (x+f_2i+1) = (x+f_2i) + (x+f_2i+1)` if the chunk has length 2
            // or if it has length 1, check that `h_i * (x+f_2i) = 1`, where x is the challenge
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
            // Check that the penultimate helper column contains `1/(table+challenge)`.
            let x = lookup_vars.local_values[start + num_helper_columns - 2];
            let x = x * (vars.local_values[lookup.table_column] + challenge);
            yield_constr.constraint(x - P::ONES);

            // Check the `Z` polynomial.
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
