use itertools::Itertools;
use num_bigint::BigUint;
use plonky2::field::batch_util::{batch_add_inplace, batch_multiply_inplace};
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
use crate::cross_table_lookup::{Column, Filter};
use crate::evaluation_frame::StarkEvaluationFrame;
use crate::stark::Stark;

pub struct Lookup<F: Field> {
    /// Columns whose values should be contained in the lookup table.
    /// These are the f_i(x) polynomials in the logUp paper.
    pub(crate) columns: Vec<Column<F>>,
    /// Column containing the lookup table.
    /// This is the t(x) polynomial in the paper.
    pub(crate) table_column: Column<F>,
    /// Column containing the frequencies of `columns` in `table_column`.
    /// This is the m(x) polynomial in the paper.
    pub(crate) frequencies_column: Column<F>,

    /// Columns to filter some elements. There is at most one filter
    /// column per column to range-check.
    pub(crate) filter_columns: Vec<Option<Filter<F>>>,
}

impl<F: Field> Lookup<F> {
    pub(crate) fn num_helper_columns(&self, constraint_degree: usize) -> usize {
        // One helper column for each column batch of size `constraint_degree-1`,
        // then one column for the inverse of `table + challenge` and one for the `Z` polynomial.
        ceil_div_usize(self.columns.len(), constraint_degree - 1) + 1
    }
}

/// logUp protocol from <https://ia.cr/2022/1530>
/// Compute the helper columns for the lookup argument.
/// Given columns `f0,...,fk` and a column `t`, such that `∪fi ⊆ t`, and challenges `x`,
/// this computes the helper columns `h_i = 1/(x+f_2i) + 1/(x+f_2i+1)`, `g = 1/(x+t)`,
/// and `Z(gx) = Z(x) + sum h_i(x) - m(x)g(x)` where `m` is the frequencies column.
pub(crate) fn lookup_helper_columns<F: Field>(
    lookup: &Lookup<F>,
    trace_poly_values: &[PolynomialValues<F>],
    challenge: F,
    constraint_degree: usize,
) -> Vec<PolynomialValues<F>> {
    assert_eq!(
        constraint_degree, 3,
        "TODO: Allow other constraint degrees."
    );

    assert_eq!(lookup.columns.len(), lookup.filter_columns.len());

    let num_total_logup_entries = trace_poly_values[0].values.len() * lookup.columns.len();
    assert!(BigUint::from(num_total_logup_entries) < F::characteristic());

    let num_helper_columns = lookup.num_helper_columns(constraint_degree);
    let mut helper_columns: Vec<PolynomialValues<F>> = Vec::with_capacity(num_helper_columns);

    // For each batch of `constraint_degree-1` columns `fi`, compute `sum 1/(f_i+challenge)` and
    // add it to the helper columns.
    // TODO: This does one batch inversion per column. It would also be possible to do one batch inversion
    // for every group of columns, but that would require building a big vector of all the columns concatenated.
    // Not sure which approach is better.
    // Note: these are the h_k(x) polynomials in the paper, with a few differences:
    //       * Here, the first ratio m_0(x)/phi_0(x) is not included with the columns batched up to create the
    //         h_k polynomials; instead there's a separate helper column for it (see below).
    //       * Here, we use 1 instead of -1 as the numerator (and subtract later).
    //       * Here, for now, the batch size (l) is always constraint_degree - 1 = 2.
    for (i, mut col_inds) in (&lookup.columns.iter().chunks(constraint_degree - 1))
        .into_iter()
        .enumerate()
    {
        let first = col_inds.next().unwrap();

        let mut column = first.eval_all_rows(trace_poly_values);
        let length = column.len();

        for x in column.iter_mut() {
            *x = challenge + *x;
        }
        let mut acc = F::batch_multiplicative_inverse(&column);
        if let Some(filter) = &lookup.filter_columns[0] {
            batch_multiply_inplace(&mut acc, &filter.eval_all_rows(trace_poly_values));
        }

        for (j, ind) in col_inds.enumerate() {
            let mut column = ind.eval_all_rows(trace_poly_values);
            for x in column.iter_mut() {
                *x = challenge + *x;
            }
            column = F::batch_multiplicative_inverse(&column);
            let filter_idx = (constraint_degree - 1) * i + j + 1;
            if let Some(filter) = &lookup.filter_columns[filter_idx] {
                batch_multiply_inplace(&mut column, &filter.eval_all_rows(trace_poly_values));
            }
            batch_add_inplace(&mut acc, &column);
        }

        helper_columns.push(acc.into());
    }

    // Add `1/(table+challenge)` to the helper columns.
    // This is 1/phi_0(x) = 1/(x + t(x)) from the paper.
    // Here, we don't include m(x) in the numerator, instead multiplying it with this column later.
    let mut table = lookup.table_column.eval_all_rows(trace_poly_values);
    for x in table.iter_mut() {
        *x = challenge + *x;
    }
    let table_inverse: Vec<F> = F::batch_multiplicative_inverse(&table);

    // Compute the `Z` polynomial with `Z(1)=0` and `Z(gx) = Z(x) + sum h_i(x) - frequencies(x)g(x)`.
    // This enforces the check from the paper, that the sum of the h_k(x) polynomials is 0 over H.
    // In the paper, that sum includes m(x)/(x + t(x)) = frequencies(x)/g(x), because that was bundled
    // into the h_k(x) polynomials.
    let frequencies = &lookup.frequencies_column.eval_all_rows(trace_poly_values);
    let mut z = Vec::with_capacity(frequencies.len());
    z.push(F::ZERO);
    for i in 0..frequencies.len() - 1 {
        let x = helper_columns[..num_helper_columns - 1]
            .iter()
            .map(|col| col.values[i])
            .sum::<F>()
            - frequencies[i] * table_inverse[i];
        z.push(z[i] + x);
    }
    helper_columns.push(z.into());

    helper_columns
}

pub(crate) struct LookupCheckVars<F, FE, P, const D2: usize>
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
pub(crate) fn eval_packed_lookups_generic<F, FE, P, S, const D: usize, const D2: usize>(
    stark: &S,
    lookups: &[Lookup<F>],
    vars: &S::EvaluationFrame<FE, P, D2>,
    lookup_vars: LookupCheckVars<F, FE, P, D2>,
    yield_constr: &mut ConstraintConsumer<P>,
) where
    F: RichField + Extendable<D>,
    FE: FieldExtension<D2, BaseField = F>,
    P: PackedField<Scalar = FE>,
    S: Stark<F, D>,
{
    let local_values = vars.get_local_values();
    let next_values = vars.get_next_values();
    let degree = stark.constraint_degree();
    assert_eq!(degree, 3, "TODO: Allow other constraint degrees.");
    let mut start = 0;
    for lookup in lookups {
        let num_helper_columns = lookup.num_helper_columns(degree);
        for &challenge in &lookup_vars.challenges {
            let challenge = FE::from_basefield(challenge);
            // For each chunk, check that `h_i (x+f_2i) (x+f_{2i+1}) = (x+f_2i) * filter_{2i+1} + (x+f_{2i+1}) * filter_2i` if the chunk has length 2
            // or if it has length 1, check that `h_i * (x+f_2i) = filter_2i`, where x is the challenge
            for (j, chunk) in lookup.columns.chunks(degree - 1).enumerate() {
                let mut x = lookup_vars.local_values[start + j];
                let mut y = P::ZEROS;
                let col_values = chunk
                    .iter()
                    .map(|col| col.eval_with_next(local_values, next_values));
                let filters = lookup.filter_columns
                    [(degree - 1) * j..(degree - 1) * j + chunk.len()]
                    .iter()
                    .map(|maybe_filter| {
                        if let Some(filter) = maybe_filter {
                            filter.eval_filter(local_values, next_values)
                        } else {
                            P::ONES
                        }
                    })
                    .rev()
                    .collect::<Vec<_>>();
                let last_filter_value = filters[0];
                for (val, f) in col_values.zip_eq(filters) {
                    x *= val + challenge;
                    y += (val + challenge) * f;
                }
                match chunk.len() {
                    2 => yield_constr.constraint(x - y),
                    1 => yield_constr.constraint(x - last_filter_value),
                    _ => todo!("Allow other constraint degrees."),
                }
            }

            // Check the `Z` polynomial.
            let z = lookup_vars.local_values[start + num_helper_columns - 1];
            let next_z = lookup_vars.next_values[start + num_helper_columns - 1];
            let table_with_challenge = lookup.table_column.eval(local_values) + challenge;
            let y = lookup_vars.local_values[start..start + num_helper_columns - 1]
                .iter()
                .fold(P::ZEROS, |acc, x| acc + *x)
                * table_with_challenge
                - lookup.frequencies_column.eval(local_values);
            // Check that in the first row, z = 0;
            yield_constr.constraint_first_row(z);
            yield_constr.constraint((next_z - z) * table_with_challenge - y);
            start += num_helper_columns;
        }
    }
}

pub(crate) struct LookupCheckVarsTarget<const D: usize> {
    pub(crate) local_values: Vec<ExtensionTarget<D>>,
    pub(crate) next_values: Vec<ExtensionTarget<D>>,
    pub(crate) challenges: Vec<Target>,
}

pub(crate) fn eval_ext_lookups_circuit<
    F: RichField + Extendable<D>,
    S: Stark<F, D>,
    const D: usize,
>(
    builder: &mut CircuitBuilder<F, D>,
    stark: &S,
    vars: &S::EvaluationFrameTarget,
    lookup_vars: LookupCheckVarsTarget<D>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let one = builder.one_extension();
    let degree = stark.constraint_degree();
    let lookups = stark.lookups();

    let local_values = vars.get_local_values();
    let next_values = vars.get_next_values();
    assert_eq!(degree, 3, "TODO: Allow other constraint degrees.");
    let mut start = 0;
    for lookup in lookups {
        let num_helper_columns = lookup.num_helper_columns(degree);
        for &challenge in &lookup_vars.challenges {
            let challenge = builder.convert_to_ext(challenge);
            for (j, chunk) in lookup.columns.chunks(degree - 1).enumerate() {
                let mut x = lookup_vars.local_values[start + j];
                let mut y = builder.zero_extension();
                let col_values = chunk
                    .iter()
                    .map(|k| k.eval_with_next_circuit(builder, local_values, next_values))
                    .collect::<Vec<_>>();
                let filters = lookup.filter_columns
                    [(degree - 1) * j..(degree - 1) * j + chunk.len()]
                    .iter()
                    .map(|maybe_filter| {
                        if let Some(filter) = maybe_filter {
                            filter.eval_filter_circuit(builder, local_values, next_values)
                        } else {
                            one
                        }
                    })
                    .rev()
                    .collect::<Vec<_>>();
                let last_filter_value = filters[0];
                for (&val, f) in col_values.iter().zip_eq(filters) {
                    let tmp = builder.add_extension(val, challenge);
                    x = builder.mul_extension(x, tmp);
                    y = builder.mul_add_extension(f, tmp, y);
                }
                match chunk.len() {
                    2 => {
                        let tmp = builder.sub_extension(x, y);
                        yield_constr.constraint(builder, tmp)
                    }
                    1 => {
                        let tmp = builder.sub_extension(x, last_filter_value);
                        yield_constr.constraint(builder, tmp)
                    }
                    _ => todo!("Allow other constraint degrees."),
                }
            }

            let z = lookup_vars.local_values[start + num_helper_columns - 1];
            let next_z = lookup_vars.next_values[start + num_helper_columns - 1];
            let table_column = lookup
                .table_column
                .eval_circuit(builder, vars.get_local_values());
            let table_with_challenge = builder.add_extension(table_column, challenge);
            let mut y = builder.add_many_extension(
                &lookup_vars.local_values[start..start + num_helper_columns - 1],
            );

            let frequencies_column = lookup
                .frequencies_column
                .eval_circuit(builder, vars.get_local_values());
            y = builder.mul_extension(y, table_with_challenge);
            y = builder.sub_extension(y, frequencies_column);

            // Check that in the first row, z = 0;
            yield_constr.constraint_first_row(builder, z);
            let mut constraint = builder.sub_extension(next_z, z);
            constraint = builder.mul_extension(constraint, table_with_challenge);
            constraint = builder.sub_extension(constraint, y);
            yield_constr.constraint(builder, constraint);
            start += num_helper_columns;
        }
    }
}
