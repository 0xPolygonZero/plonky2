use std::borrow::Borrow;
use std::iter::repeat;

use anyhow::{ensure, Result};
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

use crate::all_stark::{Table, NUM_TABLES};
use crate::config::StarkConfig;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::permutation::{
    get_grand_product_challenge_set, GrandProductChallenge, GrandProductChallengeSet,
};
use crate::proof::{StarkProof, StarkProofTarget};
use crate::stark::Stark;
use crate::vars::{StarkEvaluationTargets, StarkEvaluationVars};

/// Represent a linear combination of columns.
#[derive(Clone, Debug)]
pub struct Column<F: Field> {
    linear_combination: Vec<(usize, F)>,
    constant: F,
}

impl<F: Field> Column<F> {
    pub fn single(c: usize) -> Self {
        Self {
            linear_combination: vec![(c, F::ONE)],
            constant: F::ZERO,
        }
    }

    pub fn singles<I: IntoIterator<Item = impl Borrow<usize>>>(
        cs: I,
    ) -> impl Iterator<Item = Self> {
        cs.into_iter().map(|c| Self::single(*c.borrow()))
    }

    pub fn constant(constant: F) -> Self {
        Self {
            linear_combination: vec![],
            constant,
        }
    }

    pub fn zero() -> Self {
        Self::constant(F::ZERO)
    }

    pub fn one() -> Self {
        Self::constant(F::ONE)
    }

    pub fn linear_combination_with_constant<I: IntoIterator<Item = (usize, F)>>(
        iter: I,
        constant: F,
    ) -> Self {
        let v = iter.into_iter().collect::<Vec<_>>();
        assert!(!v.is_empty());
        debug_assert_eq!(
            v.iter().map(|(c, _)| c).unique().count(),
            v.len(),
            "Duplicate columns."
        );
        Self {
            linear_combination: v,
            constant,
        }
    }

    pub fn linear_combination<I: IntoIterator<Item = (usize, F)>>(iter: I) -> Self {
        Self::linear_combination_with_constant(iter, F::ZERO)
    }

    pub fn le_bits<I: IntoIterator<Item = impl Borrow<usize>>>(cs: I) -> Self {
        Self::linear_combination(cs.into_iter().map(|c| *c.borrow()).zip(F::TWO.powers()))
    }

    pub fn le_bytes<I: IntoIterator<Item = impl Borrow<usize>>>(cs: I) -> Self {
        Self::linear_combination(
            cs.into_iter()
                .map(|c| *c.borrow())
                .zip(F::from_canonical_u16(256).powers()),
        )
    }

    pub fn sum<I: IntoIterator<Item = impl Borrow<usize>>>(cs: I) -> Self {
        Self::linear_combination(cs.into_iter().map(|c| *c.borrow()).zip(repeat(F::ONE)))
    }

    pub fn eval<FE, P, const D: usize>(&self, v: &[P]) -> P
    where
        FE: FieldExtension<D, BaseField = F>,
        P: PackedField<Scalar = FE>,
    {
        self.linear_combination
            .iter()
            .map(|&(c, f)| v[c] * FE::from_basefield(f))
            .sum::<P>()
            + FE::from_basefield(self.constant)
    }

    /// Evaluate on an row of a table given in column-major form.
    pub fn eval_table(&self, table: &[PolynomialValues<F>], row: usize) -> F {
        self.linear_combination
            .iter()
            .map(|&(c, f)| table[c].values[row] * f)
            .sum::<F>()
            + self.constant
    }

    pub fn eval_circuit<const D: usize>(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        v: &[ExtensionTarget<D>],
    ) -> ExtensionTarget<D>
    where
        F: RichField + Extendable<D>,
    {
        let pairs = self
            .linear_combination
            .iter()
            .map(|&(c, f)| {
                (
                    v[c],
                    builder.constant_extension(F::Extension::from_basefield(f)),
                )
            })
            .collect::<Vec<_>>();
        let constant = builder.constant_extension(F::Extension::from_basefield(self.constant));
        builder.inner_product_extension(F::ONE, constant, pairs)
    }
}

#[derive(Clone, Debug)]
pub struct TableWithColumns<F: Field> {
    table: Table,
    columns: Vec<Column<F>>,
    filter_column: Option<Column<F>>,
}

impl<F: Field> TableWithColumns<F> {
    pub fn new(table: Table, columns: Vec<Column<F>>, filter_column: Option<Column<F>>) -> Self {
        Self {
            table,
            columns,
            filter_column,
        }
    }
}

#[derive(Clone)]
pub struct CrossTableLookup<F: Field> {
    looking_tables: Vec<TableWithColumns<F>>,
    looked_table: TableWithColumns<F>,
    /// Default value if filters are not used.
    default: Option<Vec<F>>,
}

impl<F: Field> CrossTableLookup<F> {
    pub fn new(
        looking_tables: Vec<TableWithColumns<F>>,
        looked_table: TableWithColumns<F>,
        default: Option<Vec<F>>,
    ) -> Self {
        assert!(looking_tables
            .iter()
            .all(|twc| twc.columns.len() == looked_table.columns.len()));
        assert!(
            looking_tables
                .iter()
                .all(|twc| twc.filter_column.is_none() == default.is_some())
                && default.is_some() == looked_table.filter_column.is_none(),
            "Default values should be provided iff there are no filter columns."
        );
        if let Some(default) = &default {
            assert_eq!(default.len(), looked_table.columns.len());
        }
        Self {
            looking_tables,
            looked_table,
            default,
        }
    }
}

/// Cross-table lookup data for one table.
#[derive(Clone, Default)]
pub struct CtlData<F: Field> {
    pub(crate) zs_columns: Vec<CtlZData<F>>,
}

/// Cross-table lookup data associated with one Z(x) polynomial.
#[derive(Clone)]
pub(crate) struct CtlZData<F: Field> {
    pub(crate) z: PolynomialValues<F>,
    pub(crate) challenge: GrandProductChallenge<F>,
    pub(crate) columns: Vec<Column<F>>,
    pub(crate) filter_column: Option<Column<F>>,
}

impl<F: Field> CtlData<F> {
    pub fn len(&self) -> usize {
        self.zs_columns.len()
    }

    pub fn is_empty(&self) -> bool {
        self.zs_columns.is_empty()
    }

    pub fn z_polys(&self) -> Vec<PolynomialValues<F>> {
        self.zs_columns
            .iter()
            .map(|zs_columns| zs_columns.z.clone())
            .collect()
    }
}

pub fn cross_table_lookup_data<F: RichField, C: GenericConfig<D, F = F>, const D: usize>(
    config: &StarkConfig,
    trace_poly_values: &[Vec<PolynomialValues<F>>; NUM_TABLES],
    cross_table_lookups: &[CrossTableLookup<F>],
    challenger: &mut Challenger<F, C::Hasher>,
) -> [CtlData<F>; NUM_TABLES] {
    let challenges = get_grand_product_challenge_set(challenger, config.num_challenges);
    let mut ctl_data_per_table = [0; NUM_TABLES].map(|_| CtlData::default());
    for CrossTableLookup {
        looking_tables,
        looked_table,
        default,
    } in cross_table_lookups
    {
        for &challenge in &challenges.challenges {
            let zs_looking = looking_tables.iter().map(|table| {
                partial_products(
                    &trace_poly_values[table.table as usize],
                    &table.columns,
                    &table.filter_column,
                    challenge,
                )
            });
            let z_looked = partial_products(
                &trace_poly_values[looked_table.table as usize],
                &looked_table.columns,
                &looked_table.filter_column,
                challenge,
            );

            debug_assert_eq!(
                zs_looking
                    .clone()
                    .map(|z| *z.values.last().unwrap())
                    .product::<F>(),
                *z_looked.values.last().unwrap()
                    * default
                        .as_ref()
                        .map(|default| {
                            challenge.combine(default).exp_u64(
                                looking_tables
                                    .iter()
                                    .map(|table| {
                                        trace_poly_values[table.table as usize][0].len() as u64
                                    })
                                    .sum::<u64>()
                                    - trace_poly_values[looked_table.table as usize][0].len()
                                        as u64,
                            )
                        })
                        .unwrap_or(F::ONE)
            );

            for (table, z) in looking_tables.iter().zip(zs_looking) {
                ctl_data_per_table[table.table as usize]
                    .zs_columns
                    .push(CtlZData {
                        z,
                        challenge,
                        columns: table.columns.clone(),
                        filter_column: table.filter_column.clone(),
                    });
            }
            ctl_data_per_table[looked_table.table as usize]
                .zs_columns
                .push(CtlZData {
                    z: z_looked,
                    challenge,
                    columns: looked_table.columns.clone(),
                    filter_column: looked_table.filter_column.clone(),
                });
        }
    }
    ctl_data_per_table
}

fn partial_products<F: Field>(
    trace: &[PolynomialValues<F>],
    columns: &[Column<F>],
    filter_column: &Option<Column<F>>,
    challenge: GrandProductChallenge<F>,
) -> PolynomialValues<F> {
    let mut partial_prod = F::ONE;
    let degree = trace[0].len();
    let mut res = Vec::with_capacity(degree);
    for i in 0..degree {
        let filter = if let Some(column) = filter_column {
            column.eval_table(trace, i)
        } else {
            F::ONE
        };
        if filter.is_one() {
            let evals = columns
                .iter()
                .map(|c| c.eval_table(trace, i))
                .collect::<Vec<_>>();
            partial_prod *= challenge.combine(evals.iter());
        } else {
            assert_eq!(filter, F::ZERO, "Non-binary filter?")
        };
        res.push(partial_prod);
    }
    res.into()
}

#[derive(Clone)]
pub struct CtlCheckVars<'a, F, FE, P, const D2: usize>
where
    F: Field,
    FE: FieldExtension<D2, BaseField = F>,
    P: PackedField<Scalar = FE>,
{
    pub(crate) local_z: P,
    pub(crate) next_z: P,
    pub(crate) challenges: GrandProductChallenge<F>,
    pub(crate) columns: &'a [Column<F>],
    pub(crate) filter_column: &'a Option<Column<F>>,
}

impl<'a, F: RichField + Extendable<D>, const D: usize>
    CtlCheckVars<'a, F, F::Extension, F::Extension, D>
{
    pub(crate) fn from_proofs<C: GenericConfig<D, F = F>>(
        proofs: &[StarkProof<F, C, D>; NUM_TABLES],
        cross_table_lookups: &'a [CrossTableLookup<F>],
        ctl_challenges: &'a GrandProductChallengeSet<F>,
        num_permutation_zs: &[usize; NUM_TABLES],
    ) -> [Vec<Self>; NUM_TABLES] {
        let mut ctl_zs = proofs
            .iter()
            .zip(num_permutation_zs)
            .map(|(p, &num_perms)| {
                let openings = &p.openings;
                let ctl_zs = openings.permutation_ctl_zs.iter().skip(num_perms);
                let ctl_zs_next = openings.permutation_ctl_zs_next.iter().skip(num_perms);
                ctl_zs.zip(ctl_zs_next)
            })
            .collect::<Vec<_>>();

        let mut ctl_vars_per_table = [0; NUM_TABLES].map(|_| vec![]);
        for CrossTableLookup {
            looking_tables,
            looked_table,
            ..
        } in cross_table_lookups
        {
            for &challenges in &ctl_challenges.challenges {
                for table in looking_tables {
                    let (looking_z, looking_z_next) = ctl_zs[table.table as usize].next().unwrap();
                    ctl_vars_per_table[table.table as usize].push(Self {
                        local_z: *looking_z,
                        next_z: *looking_z_next,
                        challenges,
                        columns: &table.columns,
                        filter_column: &table.filter_column,
                    });
                }

                let (looked_z, looked_z_next) = ctl_zs[looked_table.table as usize].next().unwrap();
                ctl_vars_per_table[looked_table.table as usize].push(Self {
                    local_z: *looked_z,
                    next_z: *looked_z_next,
                    challenges,
                    columns: &looked_table.columns,
                    filter_column: &looked_table.filter_column,
                });
            }
        }
        ctl_vars_per_table
    }
}

pub(crate) fn eval_cross_table_lookup_checks<F, FE, P, C, S, const D: usize, const D2: usize>(
    vars: StarkEvaluationVars<FE, P, { S::COLUMNS }>,
    ctl_vars: &[CtlCheckVars<F, FE, P, D2>],
    consumer: &mut ConstraintConsumer<P>,
) where
    F: RichField + Extendable<D>,
    FE: FieldExtension<D2, BaseField = F>,
    P: PackedField<Scalar = FE>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
{
    for lookup_vars in ctl_vars {
        let CtlCheckVars {
            local_z,
            next_z,
            challenges,
            columns,
            filter_column,
        } = lookup_vars;
        let combine = |v: &[P]| -> P {
            let evals = columns.iter().map(|c| c.eval(v)).collect::<Vec<_>>();
            challenges.combine(evals.iter())
        };
        let filter = |v: &[P]| -> P {
            if let Some(column) = filter_column {
                column.eval(v)
            } else {
                P::ONES
            }
        };
        let local_filter = filter(vars.local_values);
        let next_filter = filter(vars.next_values);
        let select = |filter, x| filter * x + P::ONES - filter;

        // Check value of `Z(1)`
        consumer.constraint_first_row(*local_z - select(local_filter, combine(vars.local_values)));
        // Check `Z(gw) = combination * Z(w)`
        consumer.constraint_transition(
            *next_z - *local_z * select(next_filter, combine(vars.next_values)),
        );
    }
}

#[derive(Clone)]
pub struct CtlCheckVarsTarget<'a, F: Field, const D: usize> {
    pub(crate) local_z: ExtensionTarget<D>,
    pub(crate) next_z: ExtensionTarget<D>,
    pub(crate) challenges: GrandProductChallenge<Target>,
    pub(crate) columns: &'a [Column<F>],
    pub(crate) filter_column: &'a Option<Column<F>>,
}

impl<'a, F: Field, const D: usize> CtlCheckVarsTarget<'a, F, D> {
    pub(crate) fn from_proofs(
        proofs: &[StarkProofTarget<D>; NUM_TABLES],
        cross_table_lookups: &'a [CrossTableLookup<F>],
        ctl_challenges: &'a GrandProductChallengeSet<Target>,
        num_permutation_zs: &[usize; NUM_TABLES],
    ) -> [Vec<Self>; NUM_TABLES] {
        let mut ctl_zs = proofs
            .iter()
            .zip(num_permutation_zs)
            .map(|(p, &num_perms)| {
                let openings = &p.openings;
                let ctl_zs = openings.permutation_ctl_zs.iter().skip(num_perms);
                let ctl_zs_next = openings.permutation_ctl_zs_next.iter().skip(num_perms);
                ctl_zs.zip(ctl_zs_next)
            })
            .collect::<Vec<_>>();

        let mut ctl_vars_per_table = [0; NUM_TABLES].map(|_| vec![]);
        for CrossTableLookup {
            looking_tables,
            looked_table,
            ..
        } in cross_table_lookups
        {
            for &challenges in &ctl_challenges.challenges {
                for table in looking_tables {
                    let (looking_z, looking_z_next) = ctl_zs[table.table as usize].next().unwrap();
                    ctl_vars_per_table[table.table as usize].push(Self {
                        local_z: *looking_z,
                        next_z: *looking_z_next,
                        challenges,
                        columns: &table.columns,
                        filter_column: &table.filter_column,
                    });
                }

                let (looked_z, looked_z_next) = ctl_zs[looked_table.table as usize].next().unwrap();
                ctl_vars_per_table[looked_table.table as usize].push(Self {
                    local_z: *looked_z,
                    next_z: *looked_z_next,
                    challenges,
                    columns: &looked_table.columns,
                    filter_column: &looked_table.filter_column,
                });
            }
        }
        ctl_vars_per_table
    }
}

pub(crate) fn eval_cross_table_lookup_checks_circuit<
    S: Stark<F, D>,
    F: RichField + Extendable<D>,
    const D: usize,
>(
    builder: &mut CircuitBuilder<F, D>,
    vars: StarkEvaluationTargets<D, { S::COLUMNS }>,
    ctl_vars: &[CtlCheckVarsTarget<F, D>],
    consumer: &mut RecursiveConstraintConsumer<F, D>,
) {
    for lookup_vars in ctl_vars {
        let CtlCheckVarsTarget {
            local_z,
            next_z,
            challenges,
            columns,
            filter_column,
        } = lookup_vars;

        let one = builder.one_extension();
        let local_filter = if let Some(column) = filter_column {
            column.eval_circuit(builder, vars.local_values)
        } else {
            one
        };
        let next_filter = if let Some(column) = filter_column {
            column.eval_circuit(builder, vars.next_values)
        } else {
            one
        };
        fn select<F: RichField + Extendable<D>, const D: usize>(
            builder: &mut CircuitBuilder<F, D>,
            filter: ExtensionTarget<D>,
            x: ExtensionTarget<D>,
        ) -> ExtensionTarget<D> {
            let one = builder.one_extension();
            let tmp = builder.sub_extension(one, filter);
            builder.mul_add_extension(filter, x, tmp) // filter * x + 1 - filter
        }

        // Check value of `Z(1)`
        let local_columns_eval = columns
            .iter()
            .map(|c| c.eval_circuit(builder, vars.local_values))
            .collect::<Vec<_>>();
        let combined_local = challenges.combine_circuit(builder, &local_columns_eval);
        let selected_local = select(builder, local_filter, combined_local);
        let first_row = builder.sub_extension(*local_z, selected_local);
        consumer.constraint_first_row(builder, first_row);
        // Check `Z(gw) = combination * Z(w)`
        let next_columns_eval = columns
            .iter()
            .map(|c| c.eval_circuit(builder, vars.next_values))
            .collect::<Vec<_>>();
        let combined_next = challenges.combine_circuit(builder, &next_columns_eval);
        let selected_next = select(builder, next_filter, combined_next);
        let mut transition = builder.mul_extension(*local_z, selected_next);
        transition = builder.sub_extension(*next_z, transition);
        consumer.constraint_transition(builder, transition);
    }
}

pub(crate) fn verify_cross_table_lookups<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(
    cross_table_lookups: Vec<CrossTableLookup<F>>,
    proofs: &[StarkProof<F, C, D>; NUM_TABLES],
    challenges: GrandProductChallengeSet<F>,
    config: &StarkConfig,
) -> Result<()> {
    let degrees_bits = proofs
        .iter()
        .map(|p| p.recover_degree_bits(config))
        .collect::<Vec<_>>();
    let mut ctl_zs_openings = proofs
        .iter()
        .map(|p| p.openings.ctl_zs_last.iter())
        .collect::<Vec<_>>();
    for (
        i,
        CrossTableLookup {
            looking_tables,
            looked_table,
            default,
            ..
        },
    ) in cross_table_lookups.into_iter().enumerate()
    {
        for _ in 0..config.num_challenges {
            let looking_degrees_sum = looking_tables
                .iter()
                .map(|table| 1 << degrees_bits[table.table as usize])
                .sum::<u64>();
            let looked_degree = 1 << degrees_bits[looked_table.table as usize];
            let looking_zs_prod = looking_tables
                .iter()
                .map(|table| *ctl_zs_openings[table.table as usize].next().unwrap())
                .product::<F>();
            let looked_z = *ctl_zs_openings[looked_table.table as usize].next().unwrap();
            let challenge = challenges.challenges[i % config.num_challenges];
            let combined_default = default
                .as_ref()
                .map(|default| challenge.combine(default.iter()))
                .unwrap_or(F::ONE);

            ensure!(
                looking_zs_prod
                    == looked_z * combined_default.exp_u64(looking_degrees_sum - looked_degree),
                "Cross-table lookup verification failed."
            );
        }
    }
    debug_assert!(ctl_zs_openings.iter_mut().all(|iter| iter.next().is_none()));

    Ok(())
}

pub(crate) fn verify_cross_table_lookups_circuit<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(
    builder: &mut CircuitBuilder<F, D>,
    cross_table_lookups: Vec<CrossTableLookup<F>>,
    proofs: &[StarkProofTarget<D>; NUM_TABLES],
    challenges: GrandProductChallengeSet<Target>,
    inner_config: &StarkConfig,
) {
    let degrees_bits = proofs
        .iter()
        .map(|p| p.recover_degree_bits(inner_config))
        .collect::<Vec<_>>();
    let mut ctl_zs_openings = proofs
        .iter()
        .map(|p| p.openings.ctl_zs_last.iter())
        .collect::<Vec<_>>();
    for (
        i,
        CrossTableLookup {
            looking_tables,
            looked_table,
            default,
            ..
        },
    ) in cross_table_lookups.into_iter().enumerate()
    {
        for _ in 0..inner_config.num_challenges {
            let looking_degrees_sum = looking_tables
                .iter()
                .map(|table| 1 << degrees_bits[table.table as usize])
                .sum::<u64>();
            let looked_degree = 1 << degrees_bits[looked_table.table as usize];
            let looking_zs_prod = builder.mul_many(
                looking_tables
                    .iter()
                    .map(|table| *ctl_zs_openings[table.table as usize].next().unwrap()),
            );
            let looked_z = *ctl_zs_openings[looked_table.table as usize].next().unwrap();
            let challenge = challenges.challenges[i % inner_config.num_challenges];
            if let Some(default) = default.as_ref() {
                let default = default
                    .iter()
                    .map(|&x| builder.constant(x))
                    .collect::<Vec<_>>();
                let combined_default = challenge.combine_base_circuit(builder, &default);

                let pad = builder.exp_u64(combined_default, looking_degrees_sum - looked_degree);
                let padded_looked_z = builder.mul(looked_z, pad);
                builder.connect(looking_zs_prod, padded_looked_z);
            } else {
                builder.connect(looking_zs_prod, looked_z);
            }
        }
    }
    debug_assert!(ctl_zs_openings.iter_mut().all(|iter| iter.next().is_none()));
}

#[cfg(test)]
pub(crate) mod testutils {
    use std::collections::HashMap;

    use plonky2::field::polynomial::PolynomialValues;
    use plonky2::field::types::Field;

    use crate::all_stark::Table;
    use crate::cross_table_lookup::{CrossTableLookup, TableWithColumns};

    type MultiSet<F> = HashMap<Vec<F>, Vec<(Table, usize)>>;

    /// Check that the provided traces and cross-table lookups are consistent.
    pub(crate) fn check_ctls<F: Field>(
        trace_poly_values: &[Vec<PolynomialValues<F>>],
        cross_table_lookups: &[CrossTableLookup<F>],
    ) {
        for (i, ctl) in cross_table_lookups.iter().enumerate() {
            check_ctl(trace_poly_values, ctl, i);
        }
    }

    fn check_ctl<F: Field>(
        trace_poly_values: &[Vec<PolynomialValues<F>>],
        ctl: &CrossTableLookup<F>,
        ctl_index: usize,
    ) {
        let CrossTableLookup {
            looking_tables,
            looked_table,
            default,
        } = ctl;

        // Maps `m` with `(table, i) in m[row]` iff the `i`-th row of `table` is equal to `row` and
        // the filter is 1. Without default values, the CTL check holds iff `looking_multiset == looked_multiset`.
        let mut looking_multiset = MultiSet::<F>::new();
        let mut looked_multiset = MultiSet::<F>::new();

        for table in looking_tables {
            process_table(trace_poly_values, table, &mut looking_multiset);
        }
        process_table(trace_poly_values, looked_table, &mut looked_multiset);

        let empty = &vec![];
        // Check that every row in the looking tables appears in the looked table the same number of times
        // with some special logic for the default row.
        for (row, looking_locations) in &looking_multiset {
            let looked_locations = looked_multiset.get(row).unwrap_or(empty);
            if let Some(default) = default {
                if row == default {
                    continue;
                }
            }
            check_locations(looking_locations, looked_locations, ctl_index, row);
        }
        let extra_default_count = default.as_ref().map(|d| {
            let looking_default_locations = looking_multiset.get(d).unwrap_or(empty);
            let looked_default_locations = looked_multiset.get(d).unwrap_or(empty);
            looking_default_locations
                .len()
                .checked_sub(looked_default_locations.len())
                .unwrap_or_else(|| {
                    // If underflow, panic. There should be more default rows in the looking side.
                    check_locations(
                        looking_default_locations,
                        looked_default_locations,
                        ctl_index,
                        d,
                    );
                    unreachable!()
                })
        });
        // Check that the number of extra default rows is correct.
        if let Some(count) = extra_default_count {
            assert_eq!(
                count,
                looking_tables
                    .iter()
                    .map(|table| trace_poly_values[table.table as usize][0].len())
                    .sum::<usize>()
                    - trace_poly_values[looked_table.table as usize][0].len()
            );
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
        let trace = &trace_poly_values[table.table as usize];
        for i in 0..trace[0].len() {
            let filter = if let Some(column) = &table.filter_column {
                column.eval_table(trace, i)
            } else {
                F::ONE
            };
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
        looking_locations: &[(Table, usize)],
        looked_locations: &[(Table, usize)],
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
