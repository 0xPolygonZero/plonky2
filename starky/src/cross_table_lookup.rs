//! This crate provides support for cross-table lookups.
//!
//! If a STARK S_1 calls an operation that is carried out by another STARK S_2,
//! S_1 provides the inputs to S_2 and reads the output from S_1. To ensure that
//! the operation was correctly carried out, we must check that the provided inputs
//! and outputs are correctly read. Cross-table lookups carry out that check.
//!
//! To achieve this, smaller CTL tables are created on both sides: looking and looked tables.
//! In our example, we create a table S_1' comprised of columns -- or linear combinations
//! of columns -- of S_1, and rows that call operations carried out in S_2. We also create a
//! table S_2' comprised of columns -- or linear combinations od columns -- of S_2 and rows
//! that carry out the operations needed by other STARKs. Then, S_1' is a looking table for
//! the looked S_2', since we want to check that the operation outputs in S_1' are indeeed in S_2'.
//! Furthermore, the concatenation of all tables looking into S_2' must be equal to S_2'.
//!
//! To achieve this, we construct, for each table, a permutation polynomial Z(x).
//! Z(x) is computed as the product of all its column combinations.
//! To check it was correctly constructed, we check:
//! - Z(gw) = Z(w) * combine(w) where combine(w) is the column combination at point w.
//! - Z(g^(n-1)) = combine(1).
//! - The verifier also checks that the product of looking table Z polynomials is equal
//!   to the associated looked table Z polynomial.
//!
//! Note that the first two checks are written that way because Z polynomials are computed
//! upside down for convenience.
//!
//! Additionally, we support cross-table lookups over two rows. The permutation principle
//! is similar, but we provide not only `local_values` but also `next_values` -- corresponding to
//! the current and next row values -- when computing the linear combinations.

#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};
use core::fmt::Debug;
use core::iter::once;

use anyhow::{ensure, Result};
use hashbrown::HashMap;
use itertools::Itertools;
use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::packed::PackedField;
use plonky2::field::polynomial::PolynomialValues;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::challenger::Challenger;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::iop::target::Target;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::config::GenericConfig;

use crate::config::StarkConfig;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::evaluation_frame::StarkEvaluationFrame;
use crate::lookup::{
    eval_helper_columns, eval_helper_columns_circuit, get_grand_product_challenge_set,
    get_helper_cols, Column, ColumnFilter, Filter, GrandProductChallenge, GrandProductChallengeSet,
};
use crate::proof::{StarkProof, StarkProofTarget};
use crate::stark::Stark;

/// An alias for `usize`, to represent the index of a STARK table in a multi-STARK setting.
pub type TableIdx = usize;

/// A `table` index with a linear combination of columns and a filter.
/// `filter` is used to determine the rows to select in `table`.
/// `columns` represents linear combinations of the columns of `table`.
#[derive(Clone, Debug)]
pub struct TableWithColumns<F: Field> {
    table: TableIdx,
    columns: Vec<Column<F>>,
    filter: Filter<F>,
}

impl<F: Field> TableWithColumns<F> {
    /// Generates a new `TableWithColumns` given a `table` index, a linear combination of columns `columns` and a `filter`.
    pub fn new(table: TableIdx, columns: Vec<Column<F>>, filter: Filter<F>) -> Self {
        Self {
            table,
            columns,
            filter,
        }
    }
}

/// Cross-table lookup data consisting in the lookup table (`looked_table`) and all the tables that look into `looked_table` (`looking_tables`).
/// Each `looking_table` corresponds to a STARK's table whose rows have been filtered out and whose columns have been through a linear combination (see `eval_table`). The concatenation of those smaller tables should result in the `looked_table`.
#[derive(Clone, Debug)]
pub struct CrossTableLookup<F: Field> {
    /// Column linear combinations for all tables that are looking into the current table.
    pub(crate) looking_tables: Vec<TableWithColumns<F>>,
    /// Column linear combination for the current table.
    pub(crate) looked_table: TableWithColumns<F>,
}

impl<F: Field> CrossTableLookup<F> {
    /// Creates a new `CrossTableLookup` given some looking tables and a looked table.
    /// All tables should have the same width.
    pub fn new(
        looking_tables: Vec<TableWithColumns<F>>,
        looked_table: TableWithColumns<F>,
    ) -> Self {
        assert!(looking_tables
            .iter()
            .all(|twc| twc.columns.len() == looked_table.columns.len()));
        Self {
            looking_tables,
            looked_table,
        }
    }

    /// Given a table, returns:
    /// - the total number of helper columns for this table, over all Cross-table lookups,
    /// - the total number of z polynomials for this table, over all Cross-table lookups,
    /// - the number of helper columns for this table, for each Cross-table lookup.
    pub fn num_ctl_helpers_zs_all(
        ctls: &[Self],
        table: TableIdx,
        num_challenges: usize,
        constraint_degree: usize,
    ) -> (usize, usize, Vec<usize>) {
        let mut num_helpers = 0;
        let mut num_ctls = 0;
        let mut num_helpers_by_ctl = vec![0; ctls.len()];
        for (i, ctl) in ctls.iter().enumerate() {
            let all_tables = once(&ctl.looked_table).chain(&ctl.looking_tables);
            let num_appearances = all_tables.filter(|twc| twc.table == table).count();
            let is_helpers = num_appearances > 1;
            if is_helpers {
                num_helpers_by_ctl[i] = num_appearances.div_ceil(constraint_degree - 1);
                num_helpers += num_helpers_by_ctl[i];
            }

            if num_appearances > 0 {
                num_ctls += 1;
            }
        }
        (
            num_helpers * num_challenges,
            num_ctls * num_challenges,
            num_helpers_by_ctl,
        )
    }
}

/// Cross-table lookup data for one table.
#[derive(Clone, Default, Debug)]
pub struct CtlData<'a, F: Field> {
    /// Data associated with all Z(x) polynomials for one table.
    pub zs_columns: Vec<CtlZData<'a, F>>,
}

/// Cross-table lookup data associated with one Z(x) polynomial.
/// One Z(x) polynomial can be associated to multiple tables,
/// built from the same STARK.
#[derive(Clone, Debug)]
pub struct CtlZData<'a, F: Field> {
    /// Helper columns to verify the Z polynomial values.
    pub(crate) helper_columns: Vec<PolynomialValues<F>>,
    /// Z polynomial values.
    pub(crate) z: PolynomialValues<F>,
    /// Cross-table lookup challenge.
    pub challenge: GrandProductChallenge<F>,
    /// Vector of column linear combinations for the current tables.
    pub(crate) columns: Vec<&'a [Column<F>]>,
    /// Vector of filter columns for the current table.
    /// Each filter evaluates to either 1 or 0.
    pub(crate) filter: Vec<Filter<F>>,
}

impl<'a, F: Field> CtlZData<'a, F> {
    /// Returns new CTL data from the provided arguments.
    pub fn new(
        helper_columns: Vec<PolynomialValues<F>>,
        z: PolynomialValues<F>,
        challenge: GrandProductChallenge<F>,
        columns: Vec<&'a [Column<F>]>,
        filter: Vec<Filter<F>>,
    ) -> Self {
        Self {
            helper_columns,
            z,
            challenge,
            columns,
            filter,
        }
    }
}

impl<F: Field> CtlData<'_, F> {
    /// Returns all the cross-table lookup helper polynomials.
    pub(crate) fn ctl_helper_polys(&self) -> Vec<PolynomialValues<F>> {
        let num_polys = self
            .zs_columns
            .iter()
            .fold(0, |acc, z| acc + z.helper_columns.len());
        let mut res = Vec::with_capacity(num_polys);
        for z in &self.zs_columns {
            res.extend(z.helper_columns.clone());
        }

        res
    }

    /// Returns all the Z cross-table-lookup polynomials.
    pub(crate) fn ctl_z_polys(&self) -> Vec<PolynomialValues<F>> {
        let mut res = Vec::with_capacity(self.zs_columns.len());
        for z in &self.zs_columns {
            res.push(z.z.clone());
        }

        res
    }
    /// Returns the number of helper columns for each STARK in each
    /// `CtlZData`.
    pub(crate) fn num_ctl_helper_polys(&self) -> Vec<usize> {
        let mut res = Vec::with_capacity(self.zs_columns.len());
        for z in &self.zs_columns {
            res.push(z.helper_columns.len());
        }

        res
    }
}

/// Outputs a tuple of (challenges, data) of CTL challenges and all
/// the CTL data necessary to prove a multi-STARK system.
pub fn get_ctl_data<'a, F, C, const D: usize, const N: usize>(
    config: &StarkConfig,
    trace_poly_values: &[Vec<PolynomialValues<F>>; N],
    all_cross_table_lookups: &'a [CrossTableLookup<F>],
    challenger: &mut Challenger<F, C::Hasher>,
    max_constraint_degree: usize,
) -> (GrandProductChallengeSet<F>, [CtlData<'a, F>; N])
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
{
    // Get challenges for the cross-table lookups.
    let ctl_challenges = get_grand_product_challenge_set(challenger, config.num_challenges);

    // For each STARK, compute its cross-table lookup Z polynomials
    // and get the associated `CtlData`.
    let ctl_data = cross_table_lookup_data::<F, D, N>(
        trace_poly_values,
        all_cross_table_lookups,
        &ctl_challenges,
        max_constraint_degree,
    );

    (ctl_challenges, ctl_data)
}

/// Gets the auxiliary polynomials associated to these CTL data.
pub(crate) fn get_ctl_auxiliary_polys<F: Field>(
    ctl_data: Option<&CtlData<F>>,
) -> Option<Vec<PolynomialValues<F>>> {
    ctl_data.map(|data| {
        let mut ctl_polys = data.ctl_helper_polys();
        ctl_polys.extend(data.ctl_z_polys());
        ctl_polys
    })
}

/// Generates all the cross-table lookup data, for all tables.
/// - `trace_poly_values` corresponds to the trace values for all tables.
/// - `cross_table_lookups` corresponds to all the cross-table lookups, i.e. the looked and looking tables, as described in `CrossTableLookup`.
/// - `ctl_challenges` corresponds to the challenges used for CTLs.
/// - `constraint_degree` is the maximal constraint degree for the table.
///
/// For each `CrossTableLookup`, and each looking/looked table, the partial products for the CTL are computed, and added to the said table's `CtlZData`.
pub(crate) fn cross_table_lookup_data<'a, F: RichField, const D: usize, const N: usize>(
    trace_poly_values: &[Vec<PolynomialValues<F>>; N],
    cross_table_lookups: &'a [CrossTableLookup<F>],
    ctl_challenges: &GrandProductChallengeSet<F>,
    constraint_degree: usize,
) -> [CtlData<'a, F>; N] {
    let mut ctl_data_per_table = [0; N].map(|_| CtlData::default());
    for CrossTableLookup {
        looking_tables,
        looked_table,
    } in cross_table_lookups
    {
        log::debug!("Processing CTL for {:?}", looked_table.table);
        for &challenge in &ctl_challenges.challenges {
            let helper_zs_looking = ctl_helper_zs_cols(
                trace_poly_values,
                looking_tables.clone(),
                challenge,
                constraint_degree,
            );

            let z_looked = partial_sums(
                &trace_poly_values[looked_table.table],
                &[(&looked_table.columns, &looked_table.filter)],
                challenge,
                constraint_degree,
            );

            for (table, helpers_zs) in helper_zs_looking {
                let num_helpers = helpers_zs.len() - 1;
                let count = looking_tables
                    .iter()
                    .filter(|looking_table| looking_table.table == table)
                    .count();
                let cols_filts = looking_tables.iter().filter_map(|looking_table| {
                    if looking_table.table == table {
                        Some((&looking_table.columns, &looking_table.filter))
                    } else {
                        None
                    }
                });
                let mut columns = Vec::with_capacity(count);
                let mut filter = Vec::with_capacity(count);
                for (col, filt) in cols_filts {
                    columns.push(&col[..]);
                    filter.push(filt.clone());
                }
                ctl_data_per_table[table].zs_columns.push(CtlZData {
                    helper_columns: helpers_zs[..num_helpers].to_vec(),
                    z: helpers_zs[num_helpers].clone(),
                    challenge,
                    columns,
                    filter,
                });
            }
            // There is no helper column for the looking table.
            let looked_poly = z_looked[0].clone();
            ctl_data_per_table[looked_table.table]
                .zs_columns
                .push(CtlZData {
                    helper_columns: vec![],
                    z: looked_poly,
                    challenge,
                    columns: vec![&looked_table.columns[..]],
                    filter: vec![looked_table.filter.clone()],
                });
        }
    }
    ctl_data_per_table
}

/// Computes helper columns and Z polynomials for all looking tables
/// of one cross-table lookup (i.e. for one looked table).
fn ctl_helper_zs_cols<F: Field, const N: usize>(
    all_stark_traces: &[Vec<PolynomialValues<F>>; N],
    looking_tables: Vec<TableWithColumns<F>>,
    challenge: GrandProductChallenge<F>,
    constraint_degree: usize,
) -> Vec<(usize, Vec<PolynomialValues<F>>)> {
    let grouped_lookups = looking_tables.iter().group_by(|a| a.table);

    grouped_lookups
        .into_iter()
        .map(|(table, group)| {
            let columns_filters = group
                .map(|table| (&table.columns[..], &table.filter))
                .collect::<Vec<(&[Column<F>], &Filter<F>)>>();
            (
                table,
                partial_sums(
                    &all_stark_traces[table],
                    &columns_filters,
                    challenge,
                    constraint_degree,
                ),
            )
        })
        .collect::<Vec<(usize, Vec<PolynomialValues<F>>)>>()
}

/// Computes the cross-table lookup partial sums for one table and given column linear combinations.
/// `trace` represents the trace values for the given table.
/// `columns` is a vector of column linear combinations to evaluate. Each element in the vector represents columns that need to be combined.
/// `filter_cols` are column linear combinations used to determine whether a row should be selected.
/// `challenge` is a cross-table lookup challenge.
/// The initial sum `s` is 0.
/// For each row, if the `filter_column` evaluates to 1, then the row is selected. All the column linear combinations are evaluated at said row.
/// The evaluations of each elements of `columns` are then combined together to form a value `v`.
/// The values `v`` are grouped together, in groups of size `constraint_degree - 1`. For each group, we construct a helper
/// column: h = \sum_i 1/(v_i).
///
/// The sum is updated: `s += \sum h_i`, and is pushed to the vector of partial sums `z``.
/// Returns the helper columns and `z`.
fn partial_sums<F: Field>(
    trace: &[PolynomialValues<F>],
    columns_filters: &[ColumnFilter<F>],
    challenge: GrandProductChallenge<F>,
    constraint_degree: usize,
) -> Vec<PolynomialValues<F>> {
    let degree = trace[0].len();
    let mut z = Vec::with_capacity(degree);

    let mut helper_columns =
        get_helper_cols(trace, degree, columns_filters, challenge, constraint_degree);

    let x = helper_columns
        .iter()
        .map(|col| col.values[degree - 1])
        .sum::<F>();
    z.push(x);

    for i in (0..degree - 1).rev() {
        let x = helper_columns.iter().map(|col| col.values[i]).sum::<F>();

        z.push(z[z.len() - 1] + x);
    }
    z.reverse();
    if columns_filters.len() > 1 {
        helper_columns.push(z.into());
    } else {
        helper_columns = vec![z.into()];
    }

    helper_columns
}

/// Data necessary to check the cross-table lookups of a given table.
#[derive(Clone, Debug)]
pub struct CtlCheckVars<'a, F, FE, P, const D2: usize>
where
    F: Field,
    FE: FieldExtension<D2, BaseField = F>,
    P: PackedField<Scalar = FE>,
{
    /// Helper columns to check that the Z polyomial
    /// was constructed correctly.
    pub(crate) helper_columns: Vec<P>,
    /// Evaluation of the trace polynomials at point `zeta`.
    pub(crate) local_z: P,
    /// Evaluation of the trace polynomials at point `g * zeta`
    pub(crate) next_z: P,
    /// Cross-table lookup challenges.
    pub(crate) challenges: GrandProductChallenge<F>,
    /// Column linear combinations of the `CrossTableLookup`s.
    pub(crate) columns: Vec<&'a [Column<F>]>,
    /// Filter that evaluates to either 1 or 0.
    pub(crate) filter: Vec<Filter<F>>,
}

impl<'a, F: RichField + Extendable<D>, const D: usize>
    CtlCheckVars<'a, F, F::Extension, F::Extension, D>
{
    /// Extracts the `CtlCheckVars` from a single proof.
    pub fn from_proof<C: GenericConfig<D, F = F>>(
        table_idx: TableIdx,
        proof: &StarkProof<F, C, D>,
        cross_table_lookups: &'a [CrossTableLookup<F>],
        ctl_challenges: &'a GrandProductChallengeSet<F>,
        num_lookup_columns: usize,
        total_num_helper_columns: usize,
        num_helper_ctl_columns: &[usize],
    ) -> Vec<Self> {
        // Get all cross-table lookup polynomial openings for the provided STARK opening set.
        let ctl_zs = {
            let auxiliary_polys = proof
                .openings
                .auxiliary_polys
                .as_ref()
                .expect("We cannot have CTLs without auxiliary polynomials.");
            let auxiliary_polys_next = proof
                .openings
                .auxiliary_polys_next
                .as_ref()
                .expect("We cannot have CTLs without auxiliary polynomials.");

            auxiliary_polys
                .iter()
                .skip(num_lookup_columns)
                .zip(auxiliary_polys_next.iter().skip(num_lookup_columns))
                .collect::<Vec<_>>()
        };

        let mut z_index = 0;
        let mut start_index = 0;
        let mut ctl_vars = vec![];

        for (
            i,
            CrossTableLookup {
                looking_tables,
                looked_table,
            },
        ) in cross_table_lookups.iter().enumerate()
        {
            for &challenges in &ctl_challenges.challenges {
                // Group the looking tables by `Table` to process them together.
                let count = looking_tables
                    .iter()
                    .filter(|looking_table| looking_table.table == table_idx)
                    .count();

                let cols_filts = looking_tables.iter().filter_map(|looking_table| {
                    if looking_table.table == table_idx {
                        Some((&looking_table.columns, &looking_table.filter))
                    } else {
                        None
                    }
                });

                if count > 0 {
                    let mut columns = Vec::with_capacity(count);
                    let mut filter = Vec::with_capacity(count);
                    for (col, filt) in cols_filts {
                        columns.push(&col[..]);
                        filter.push(filt.clone());
                    }

                    let (looking_z, looking_z_next) = ctl_zs[total_num_helper_columns + z_index];
                    let helper_columns = ctl_zs
                        [start_index..start_index + num_helper_ctl_columns[i]]
                        .iter()
                        .map(|&(h, _)| *h)
                        .collect::<Vec<_>>();

                    start_index += num_helper_ctl_columns[i];
                    z_index += 1;

                    ctl_vars.push(Self {
                        helper_columns,
                        local_z: *looking_z,
                        next_z: *looking_z_next,
                        challenges,
                        columns,
                        filter,
                    });
                }

                if looked_table.table == table_idx {
                    let (looked_z, looked_z_next) = ctl_zs[total_num_helper_columns + z_index];
                    z_index += 1;

                    let columns = vec![&looked_table.columns[..]];
                    let filter = vec![looked_table.filter.clone()];

                    ctl_vars.push(Self {
                        helper_columns: vec![],
                        local_z: *looked_z,
                        next_z: *looked_z_next,
                        challenges,
                        columns,
                        filter,
                    });
                }
            }
        }

        ctl_vars
    }
}

/// Checks the cross-table lookup Z polynomials for each table:
/// - Checks that the CTL `Z` partial sums are correctly updated.
/// - Checks that the final value of the CTL sum is the combination of all STARKs' CTL polynomials.
///
/// CTL `Z` partial sums are upside down: the complete sum is on the first row, and
/// the first term is on the last row. This allows the transition constraint to be:
/// `combine(w) * (Z(w) - Z(gw)) = filter` where combine is called on the local row
/// and not the next. This enables CTLs across two rows.
pub(crate) fn eval_cross_table_lookup_checks<F, FE, P, S, const D: usize, const D2: usize>(
    vars: &S::EvaluationFrame<FE, P, D2>,
    ctl_vars: &[CtlCheckVars<F, FE, P, D2>],
    consumer: &mut ConstraintConsumer<P>,
    constraint_degree: usize,
) where
    F: RichField + Extendable<D>,
    FE: FieldExtension<D2, BaseField = F>,
    P: PackedField<Scalar = FE>,
    S: Stark<F, D>,
{
    let local_values = vars.get_local_values();
    let next_values = vars.get_next_values();

    for lookup_vars in ctl_vars {
        let CtlCheckVars {
            helper_columns,
            local_z,
            next_z,
            challenges,
            columns,
            filter,
        } = lookup_vars;

        // Compute all linear combinations on the current table, and combine them using the challenge.
        let evals = columns
            .iter()
            .map(|col| {
                col.iter()
                    .map(|c| c.eval_with_next(local_values, next_values))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        // Check helper columns.
        eval_helper_columns(
            filter,
            &evals,
            local_values,
            next_values,
            helper_columns,
            constraint_degree,
            challenges,
            consumer,
        );

        if !helper_columns.is_empty() {
            let h_sum = helper_columns.iter().fold(P::ZEROS, |acc, x| acc + *x);
            // Check value of `Z(g^(n-1))`
            consumer.constraint_last_row(*local_z - h_sum);
            // Check `Z(w) = Z(gw) + \sum h_i`
            consumer.constraint_transition(*local_z - *next_z - h_sum);
        } else if columns.len() > 1 {
            let combin0 = challenges.combine(&evals[0]);
            let combin1 = challenges.combine(&evals[1]);

            let f0 = filter[0].eval_filter(local_values, next_values);
            let f1 = filter[1].eval_filter(local_values, next_values);

            consumer
                .constraint_last_row(combin0 * combin1 * *local_z - f0 * combin1 - f1 * combin0);
            consumer.constraint_transition(
                combin0 * combin1 * (*local_z - *next_z) - f0 * combin1 - f1 * combin0,
            );
        } else {
            let combin0 = challenges.combine(&evals[0]);
            let f0 = filter[0].eval_filter(local_values, next_values);
            consumer.constraint_last_row(combin0 * *local_z - f0);
            consumer.constraint_transition(combin0 * (*local_z - *next_z) - f0);
        }
    }
}

/// Circuit version of `CtlCheckVars`. Data necessary to check the cross-table lookups of a given table.
#[derive(Clone, Debug)]
pub struct CtlCheckVarsTarget<F: Field, const D: usize> {
    ///Evaluation of the helper columns to check that the Z polyomial
    /// was constructed correctly.
    pub(crate) helper_columns: Vec<ExtensionTarget<D>>,
    /// Evaluation of the trace polynomials at point `zeta`.
    pub(crate) local_z: ExtensionTarget<D>,
    /// Evaluation of the trace polynomials at point `g * zeta`.
    pub(crate) next_z: ExtensionTarget<D>,
    /// Cross-table lookup challenges.
    pub(crate) challenges: GrandProductChallenge<Target>,
    /// Column linear combinations of the `CrossTableLookup`s.
    pub(crate) columns: Vec<Vec<Column<F>>>,
    /// Filter that evaluates to either 1 or 0.
    pub(crate) filter: Vec<Filter<F>>,
}

impl<'a, F: Field, const D: usize> CtlCheckVarsTarget<F, D> {
    /// Circuit version of `from_proof`, for a single STARK.
    pub fn from_proof(
        table: TableIdx,
        proof: &StarkProofTarget<D>,
        cross_table_lookups: &'a [CrossTableLookup<F>],
        ctl_challenges: &'a GrandProductChallengeSet<Target>,
        num_lookup_columns: usize,
        total_num_helper_columns: usize,
        num_helper_ctl_columns: &[usize],
    ) -> Vec<Self> {
        // Get all cross-table lookup polynomial openings.
        let ctl_zs = {
            let ctl_zs = proof
                .openings
                .auxiliary_polys
                .as_ref()
                .expect("We cannot have CTLs without auxiliary polynomials.")
                .iter()
                .skip(num_lookup_columns);
            let ctl_zs_next = proof
                .openings
                .auxiliary_polys_next
                .as_ref()
                .expect("We cannot have CTLs without auxiliary polynomials.")
                .iter()
                .skip(num_lookup_columns);
            ctl_zs.zip(ctl_zs_next).collect::<Vec<_>>()
        };

        // Put each cross-table lookup polynomial into the correct table's data.
        // If a CTL polynomial is extracted from the looking/looked table `t``,
        // then we add it to the `CtlCheckVars` of table `t``.
        let mut z_index = 0;
        let mut start_index = 0;
        let mut ctl_vars = vec![];
        for (
            i,
            CrossTableLookup {
                looking_tables,
                looked_table,
            },
        ) in cross_table_lookups.iter().enumerate()
        {
            for &challenges in &ctl_challenges.challenges {
                // Group looking tables by `Table`, since we bundle the looking tables
                // taken from the same `Table` together thanks to helper columns.

                let count = looking_tables
                    .iter()
                    .filter(|looking_table| looking_table.table == table)
                    .count();
                let cols_filts = looking_tables.iter().filter_map(|looking_table| {
                    if looking_table.table == table {
                        Some((&looking_table.columns, &looking_table.filter))
                    } else {
                        None
                    }
                });
                if count > 0 {
                    let mut columns = Vec::with_capacity(count);
                    let mut filter = Vec::with_capacity(count);
                    for (col, filt) in cols_filts {
                        columns.push(col.clone());
                        filter.push(filt.clone());
                    }
                    let (looking_z, looking_z_next) = ctl_zs[total_num_helper_columns + z_index];
                    let helper_columns = ctl_zs
                        [start_index..start_index + num_helper_ctl_columns[i]]
                        .iter()
                        .map(|(&h, _)| h)
                        .collect::<Vec<_>>();

                    start_index += num_helper_ctl_columns[i];
                    z_index += 1;
                    ctl_vars.push(Self {
                        helper_columns,
                        local_z: *looking_z,
                        next_z: *looking_z_next,
                        challenges,
                        columns,
                        filter,
                    });
                }

                if looked_table.table == table {
                    let (looked_z, looked_z_next) = ctl_zs[total_num_helper_columns + z_index];
                    z_index += 1;

                    let columns = vec![looked_table.columns.clone()];
                    let filter = vec![looked_table.filter.clone()];
                    ctl_vars.push(Self {
                        helper_columns: vec![],
                        local_z: *looked_z,
                        next_z: *looked_z_next,
                        challenges,
                        columns,
                        filter,
                    });
                }
            }
        }

        ctl_vars
    }
}

/// Circuit version of `eval_cross_table_lookup_checks`. Checks the cross-table lookup Z polynomials for each table:
/// - Checks that the CTL `Z` partial sums are correctly updated.
/// - Checks that the final value of the CTL sum is the combination of all STARKs' CTL polynomials.
///
/// CTL `Z` partial sums are upside down: the complete sum is on the first row, and
/// the first term is on the last row. This allows the transition constraint to be:
/// `combine(w) * (Z(w) - Z(gw)) = filter` where combine is called on the local row
/// and not the next. This enables CTLs across two rows.
pub(crate) fn eval_cross_table_lookup_checks_circuit<
    S: Stark<F, D>,
    F: RichField + Extendable<D>,
    const D: usize,
>(
    builder: &mut CircuitBuilder<F, D>,
    vars: &S::EvaluationFrameTarget,
    ctl_vars: &[CtlCheckVarsTarget<F, D>],
    consumer: &mut RecursiveConstraintConsumer<F, D>,
    constraint_degree: usize,
) {
    let local_values = vars.get_local_values();
    let next_values = vars.get_next_values();

    for lookup_vars in ctl_vars {
        let CtlCheckVarsTarget {
            helper_columns,
            local_z,
            next_z,
            challenges,
            columns,
            filter,
        } = lookup_vars;

        // Compute all linear combinations on the current table, and combine them using the challenge.
        let evals = columns
            .iter()
            .map(|col| {
                col.iter()
                    .map(|c| c.eval_with_next_circuit(builder, local_values, next_values))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        // Check helper columns.
        eval_helper_columns_circuit(
            builder,
            filter,
            &evals,
            local_values,
            next_values,
            helper_columns,
            constraint_degree,
            challenges,
            consumer,
        );

        let z_diff = builder.sub_extension(*local_z, *next_z);
        if !helper_columns.is_empty() {
            // Check value of `Z(g^(n-1))`
            let h_sum = builder.add_many_extension(helper_columns);

            let last_row = builder.sub_extension(*local_z, h_sum);
            consumer.constraint_last_row(builder, last_row);
            // Check `Z(w) = Z(gw) * (filter / combination)`

            let transition = builder.sub_extension(z_diff, h_sum);
            consumer.constraint_transition(builder, transition);
        } else if columns.len() > 1 {
            let combin0 = challenges.combine_circuit(builder, &evals[0]);
            let combin1 = challenges.combine_circuit(builder, &evals[1]);

            let f0 = filter[0].eval_filter_circuit(builder, local_values, next_values);
            let f1 = filter[1].eval_filter_circuit(builder, local_values, next_values);

            let combined = builder.mul_sub_extension(combin1, *local_z, f1);
            let combined = builder.mul_extension(combined, combin0);
            let constr = builder.arithmetic_extension(F::NEG_ONE, F::ONE, f0, combin1, combined);
            consumer.constraint_last_row(builder, constr);

            let combined = builder.mul_sub_extension(combin1, z_diff, f1);
            let combined = builder.mul_extension(combined, combin0);
            let constr = builder.arithmetic_extension(F::NEG_ONE, F::ONE, f0, combin1, combined);
            consumer.constraint_last_row(builder, constr);
        } else {
            let combin0 = challenges.combine_circuit(builder, &evals[0]);
            let f0 = filter[0].eval_filter_circuit(builder, local_values, next_values);

            let constr = builder.mul_sub_extension(combin0, *local_z, f0);
            consumer.constraint_last_row(builder, constr);
            let constr = builder.mul_sub_extension(combin0, z_diff, f0);
            consumer.constraint_transition(builder, constr);
        }
    }
}

/// Verifies all cross-table lookups.
/// The key of `ctl_extra_looking_sums` is the corresponding CTL's position within `cross_table_lookups`.
pub fn verify_cross_table_lookups<F: RichField + Extendable<D>, const D: usize, const N: usize>(
    cross_table_lookups: &[CrossTableLookup<F>],
    ctl_zs_first: [Vec<F>; N],
    ctl_extra_looking_sums: &HashMap<usize, Vec<F>>,
    config: &StarkConfig,
) -> Result<()> {
    let mut ctl_zs_openings = ctl_zs_first.iter().map(|v| v.iter()).collect::<Vec<_>>();
    for (
        index,
        CrossTableLookup {
            looking_tables,
            looked_table,
        },
    ) in cross_table_lookups.iter().enumerate()
    {
        let ctl_extra_looking_sum = ctl_extra_looking_sums.get(&index);
        // We want to iterate on each looking table only once.
        let mut filtered_looking_tables = vec![];
        for table in looking_tables {
            if !filtered_looking_tables.contains(&(table.table)) {
                filtered_looking_tables.push(table.table);
            }
        }
        for c in 0..config.num_challenges {
            // Compute the combination of all looking table CTL polynomial openings.

            let looking_zs_sum = filtered_looking_tables
                .iter()
                .map(|&table| *ctl_zs_openings[table].next().unwrap())
                .sum::<F>()
                // Get elements looking into `looked_table` that are not associated to any STARK.
                + ctl_extra_looking_sum.map(|v| v[c]).unwrap_or_default();

            // Get the looked table CTL polynomial opening.
            let looked_z = *ctl_zs_openings[looked_table.table].next().unwrap();
            // Ensure that the combination of looking table openings is equal to the looked table opening.
            ensure!(
                looking_zs_sum == looked_z,
                "Cross-table lookup {:?} verification failed.",
                index
            );
        }
    }
    debug_assert!(ctl_zs_openings.iter_mut().all(|iter| iter.next().is_none()));

    Ok(())
}

/// Circuit version of `verify_cross_table_lookups`. Verifies all cross-table lookups.
/// The key of `ctl_extra_looking_sums` is the corresponding CTL's position within `cross_table_lookups`.
pub fn verify_cross_table_lookups_circuit<
    F: RichField + Extendable<D>,
    const D: usize,
    const N: usize,
>(
    builder: &mut CircuitBuilder<F, D>,
    cross_table_lookups: Vec<CrossTableLookup<F>>,
    ctl_zs_first: [Vec<Target>; N],
    ctl_extra_looking_sums: &HashMap<usize, Vec<Target>>,
    inner_config: &StarkConfig,
) {
    let mut ctl_zs_openings = ctl_zs_first.iter().map(|v| v.iter()).collect::<Vec<_>>();
    for (
        index,
        CrossTableLookup {
            looking_tables,
            looked_table,
        },
    ) in cross_table_lookups.into_iter().enumerate()
    {
        let ctl_extra_looking_sum = ctl_extra_looking_sums.get(&index);
        // We want to iterate on each looking table only once.
        let mut filtered_looking_tables = vec![];
        for table in looking_tables {
            if !filtered_looking_tables.contains(&(table.table)) {
                filtered_looking_tables.push(table.table);
            }
        }
        for c in 0..inner_config.num_challenges {
            // Compute the combination of all looking table CTL polynomial openings.
            let mut looking_zs_sum = builder.add_many(
                filtered_looking_tables
                    .iter()
                    .map(|&table| *ctl_zs_openings[table].next().unwrap()),
            );

            // Get elements looking into `looked_table` that are not associated to any STARK.
            let extra_sum = ctl_extra_looking_sum.map(|v| v[c]).unwrap_or_default();
            looking_zs_sum = builder.add(looking_zs_sum, extra_sum);

            // Get the looked table CTL polynomial opening.
            let looked_z = *ctl_zs_openings[looked_table.table].next().unwrap();
            // Verify that the combination of looking table openings is equal to the looked table opening.
            builder.connect(looked_z, looking_zs_sum);
        }
    }
    debug_assert!(ctl_zs_openings.iter_mut().all(|iter| iter.next().is_none()));
}

/// Debugging module used to assert correctness of the different CTLs of a multi-STARK system,
/// that can be used during the proof generation process.
///
/// **Note**: This is an expensive check.
pub mod debug_utils {
    #[cfg(not(feature = "std"))]
    use alloc::{vec, vec::Vec};

    use hashbrown::HashMap;
    use plonky2::field::polynomial::PolynomialValues;
    use plonky2::field::types::Field;

    use super::{CrossTableLookup, TableIdx, TableWithColumns};

    type MultiSet<F> = HashMap<Vec<F>, Vec<(TableIdx, usize)>>;

    /// Check that the provided traces and cross-table lookups are consistent.
    /// The key of `extra_looking_values` is the corresponding CTL's position within `cross_table_lookups`.
    pub fn check_ctls<F: Field>(
        trace_poly_values: &[Vec<PolynomialValues<F>>],
        cross_table_lookups: &[CrossTableLookup<F>],
        extra_looking_values: &HashMap<usize, Vec<Vec<F>>>,
    ) {
        for (i, ctl) in cross_table_lookups.iter().enumerate() {
            check_ctl(trace_poly_values, ctl, i, extra_looking_values.get(&i));
        }
    }

    fn check_ctl<F: Field>(
        trace_poly_values: &[Vec<PolynomialValues<F>>],
        ctl: &CrossTableLookup<F>,
        ctl_index: usize,
        extra_looking_values: Option<&Vec<Vec<F>>>,
    ) {
        let CrossTableLookup {
            looking_tables,
            looked_table,
        } = ctl;

        // Maps `m` with `(table, i) in m[row]` iff the `i`-th row of `table` is equal to `row` and
        // the filter is 1. Without default values, the CTL check holds iff `looking_multiset == looked_multiset`.
        let mut looking_multiset = MultiSet::<F>::new();
        let mut looked_multiset = MultiSet::<F>::new();

        for table in looking_tables {
            process_table(trace_poly_values, table, &mut looking_multiset);
        }
        process_table(trace_poly_values, looked_table, &mut looked_multiset);

        // Include extra looking values if any for this `ctl_index`.
        if let Some(values) = extra_looking_values {
            for row in values.iter() {
                // The table and the row index don't matter here, as we just want to enforce
                // that the special extra values do appear when looking against the specified table.
                looking_multiset
                    .entry(row.to_vec())
                    .or_default()
                    .push((0, 0));
            }
        }

        let empty = &vec![];
        // Check that every row in the looking tables appears in the looked table the same number of times.
        for (row, looking_locations) in &looking_multiset {
            let looked_locations = looked_multiset.get(row).unwrap_or(empty);
            check_locations(looking_locations, looked_locations, ctl_index, row);
        }
        // Check that every row in the looked tables appears in the looked table the same number of times.
        for (row, looked_locations) in &looked_multiset {
            let looking_locations = looking_multiset.get(row).unwrap_or(empty);
            check_locations(looking_locations, looked_locations, ctl_index, row);
        }
    }

    fn process_table<F: Field>(
        trace_poly_values: &[Vec<PolynomialValues<F>>],
        table: &TableWithColumns<F>,
        multiset: &mut MultiSet<F>,
    ) {
        let trace = &trace_poly_values[table.table];
        for i in 0..trace[0].len() {
            let filter = table.filter.eval_table(trace, i);
            if filter.is_one() {
                let row = table
                    .columns
                    .iter()
                    .map(|c| c.eval_table(trace, i))
                    .collect::<Vec<_>>();
                multiset.entry(row).or_default().push((table.table, i));
            } else {
                assert_eq!(filter, F::ZERO, "Non-binary filter?")
            }
        }
    }

    fn check_locations<F: Field>(
        looking_locations: &[(TableIdx, usize)],
        looked_locations: &[(TableIdx, usize)],
        ctl_index: usize,
        row: &[F],
    ) {
        if looking_locations.len() != looked_locations.len() {
            panic!(
                "CTL #{ctl_index}:\n\
                 Row {row:?} is present {l0} times in the looking tables, but {l1} times in the looked table.\n\
                 Looking locations (Table, Row index): {looking_locations:?}.\n\
                 Looked locations (Table, Row index): {looked_locations:?}.",
                l0 = looking_locations.len(),
                l1 = looked_locations.len(),
            );
        }
    }
}
