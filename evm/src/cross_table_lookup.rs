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
//! to the associated looked table Z polynomial.
//! Note that the first two checks are written that way because Z polynomials are computed
//! upside down for convenience.
//!
//! Additionally, we support cross-table lookups over two rows. The permutation principle
//! is similar, but we provide not only `local_values` but also `next_values` -- corresponding to
//! the current and next row values -- when computing the linear combinations.

use std::borrow::Borrow;
use std::cmp::min;
use std::fmt::Debug;
use std::iter::repeat;

use anyhow::{ensure, Result};
use hashbrown::HashMap;
use itertools::Itertools;
use plonky2::field::batch_util::{batch_add_inplace, batch_multiply_inplace};
use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::packed::PackedField;
use plonky2::field::polynomial::PolynomialValues;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::challenger::{Challenger, RecursiveChallenger};
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::iop::target::Target;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::config::{AlgebraicHasher, GenericConfig, Hasher};
use plonky2::plonk::plonk_common::{
    reduce_with_powers, reduce_with_powers_circuit, reduce_with_powers_ext_circuit,
};
use plonky2::util::ceil_div_usize;
use plonky2::util::serialization::{Buffer, IoResult, Read, Write};

use crate::all_stark::{Table, NUM_TABLES};
use crate::config::StarkConfig;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::evaluation_frame::StarkEvaluationFrame;
use crate::proof::{StarkProofTarget, StarkProofWithMetadata};
use crate::stark::Stark;

/// Represent two linear combination of columns, corresponding to the current and next row values.
/// Each linear combination is represented as:
/// - a vector of `(usize, F)` corresponding to the column number and the associated multiplicand
/// - the constant of the linear combination.
#[derive(Clone, Debug)]
pub(crate) struct Column<F: Field> {
    linear_combination: Vec<(usize, F)>,
    next_row_linear_combination: Vec<(usize, F)>,
    constant: F,
}

impl<F: Field> Column<F> {
    /// Returns the representation of a single column in the current row.
    pub(crate) fn single(c: usize) -> Self {
        Self {
            linear_combination: vec![(c, F::ONE)],
            next_row_linear_combination: vec![],
            constant: F::ZERO,
        }
    }

    /// Returns multiple single columns in the current row.
    pub(crate) fn singles<I: IntoIterator<Item = impl Borrow<usize>>>(
        cs: I,
    ) -> impl Iterator<Item = Self> {
        cs.into_iter().map(|c| Self::single(*c.borrow()))
    }

    /// Returns the representation of a single column in the next row.
    pub(crate) fn single_next_row(c: usize) -> Self {
        Self {
            linear_combination: vec![],
            next_row_linear_combination: vec![(c, F::ONE)],
            constant: F::ZERO,
        }
    }

    /// Returns multiple single columns for the next row.
    pub(crate) fn singles_next_row<I: IntoIterator<Item = impl Borrow<usize>>>(
        cs: I,
    ) -> impl Iterator<Item = Self> {
        cs.into_iter().map(|c| Self::single_next_row(*c.borrow()))
    }

    /// Returns a linear combination corresponding to a constant.
    pub(crate) fn constant(constant: F) -> Self {
        Self {
            linear_combination: vec![],
            next_row_linear_combination: vec![],
            constant,
        }
    }

    /// Returns a linear combination corresponding to 0.
    pub(crate) fn zero() -> Self {
        Self::constant(F::ZERO)
    }

    /// Returns a linear combination corresponding to 1.
    pub(crate) fn one() -> Self {
        Self::constant(F::ONE)
    }

    /// Given an iterator of `(usize, F)` and a constant, returns the association linear combination of columns for the current row.
    pub(crate) fn linear_combination_with_constant<I: IntoIterator<Item = (usize, F)>>(
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
            next_row_linear_combination: vec![],
            constant,
        }
    }

    /// Given an iterator of `(usize, F)` and a constant, returns the associated linear combination of columns for the current and the next rows.
    pub(crate) fn linear_combination_and_next_row_with_constant<
        I: IntoIterator<Item = (usize, F)>,
    >(
        iter: I,
        next_row_iter: I,
        constant: F,
    ) -> Self {
        let v = iter.into_iter().collect::<Vec<_>>();
        let next_row_v = next_row_iter.into_iter().collect::<Vec<_>>();

        assert!(!v.is_empty() || !next_row_v.is_empty());
        debug_assert_eq!(
            v.iter().map(|(c, _)| c).unique().count(),
            v.len(),
            "Duplicate columns."
        );
        debug_assert_eq!(
            next_row_v.iter().map(|(c, _)| c).unique().count(),
            next_row_v.len(),
            "Duplicate columns."
        );

        Self {
            linear_combination: v,
            next_row_linear_combination: next_row_v,
            constant,
        }
    }

    /// Returns a linear combination of columns, with no additional constant.
    pub(crate) fn linear_combination<I: IntoIterator<Item = (usize, F)>>(iter: I) -> Self {
        Self::linear_combination_with_constant(iter, F::ZERO)
    }

    /// Given an iterator of columns (c_0, ..., c_n) containing bits in little endian order:
    /// returns the representation of c_0 + 2 * c_1 + ... + 2^n * c_n.
    pub(crate) fn le_bits<I: IntoIterator<Item = impl Borrow<usize>>>(cs: I) -> Self {
        Self::linear_combination(cs.into_iter().map(|c| *c.borrow()).zip(F::TWO.powers()))
    }

    /// Given an iterator of columns (c_0, ..., c_n) containing bits in little endian order:
    /// returns the representation of c_0 + 2 * c_1 + ... + 2^n * c_n + k where `k` is an
    /// additional constant.
    pub(crate) fn le_bits_with_constant<I: IntoIterator<Item = impl Borrow<usize>>>(
        cs: I,
        constant: F,
    ) -> Self {
        Self::linear_combination_with_constant(
            cs.into_iter().map(|c| *c.borrow()).zip(F::TWO.powers()),
            constant,
        )
    }

    /// Given an iterator of columns (c_0, ..., c_n) containing bytes in little endian order:
    /// returns the representation of c_0 + 256 * c_1 + ... + 256^n * c_n.
    pub(crate) fn le_bytes<I: IntoIterator<Item = impl Borrow<usize>>>(cs: I) -> Self {
        Self::linear_combination(
            cs.into_iter()
                .map(|c| *c.borrow())
                .zip(F::from_canonical_u16(256).powers()),
        )
    }

    /// Given an iterator of columns, returns the representation of their sum.
    pub(crate) fn sum<I: IntoIterator<Item = impl Borrow<usize>>>(cs: I) -> Self {
        Self::linear_combination(cs.into_iter().map(|c| *c.borrow()).zip(repeat(F::ONE)))
    }

    /// Given the column values for the current row, returns the evaluation of the linear combination.
    pub(crate) fn eval<FE, P, const D: usize>(&self, v: &[P]) -> P
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

    /// Given the column values for the current and next rows, evaluates the current and next linear combinations and returns their sum.
    pub(crate) fn eval_with_next<FE, P, const D: usize>(&self, v: &[P], next_v: &[P]) -> P
    where
        FE: FieldExtension<D, BaseField = F>,
        P: PackedField<Scalar = FE>,
    {
        self.linear_combination
            .iter()
            .map(|&(c, f)| v[c] * FE::from_basefield(f))
            .sum::<P>()
            + self
                .next_row_linear_combination
                .iter()
                .map(|&(c, f)| next_v[c] * FE::from_basefield(f))
                .sum::<P>()
            + FE::from_basefield(self.constant)
    }

    /// Evaluate on a row of a table given in column-major form.
    pub(crate) fn eval_table(&self, table: &[PolynomialValues<F>], row: usize) -> F {
        let mut res = self
            .linear_combination
            .iter()
            .map(|&(c, f)| table[c].values[row] * f)
            .sum::<F>()
            + self.constant;

        // If we access the next row at the last row, for sanity, we consider the next row's values to be 0.
        // If CTLs are correctly written, the filter should be 0 in that case anyway.
        if !self.next_row_linear_combination.is_empty() && row < table[0].values.len() - 1 {
            res += self
                .next_row_linear_combination
                .iter()
                .map(|&(c, f)| table[c].values[row + 1] * f)
                .sum::<F>();
        }

        res
    }

    /// Evaluates the column on all rows.
    pub(crate) fn eval_all_rows(&self, table: &[PolynomialValues<F>]) -> Vec<F> {
        let length = table[0].len();
        (0..length)
            .map(|row| self.eval_table(table, row))
            .collect::<Vec<F>>()
    }

    /// Circuit version of `eval`: Given a row's targets, returns their linear combination.
    pub(crate) fn eval_circuit<const D: usize>(
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

    /// Circuit version of `eval_with_next`:
    /// Given the targets of the current and next row, returns the sum of their linear combinations.
    pub(crate) fn eval_with_next_circuit<const D: usize>(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        v: &[ExtensionTarget<D>],
        next_v: &[ExtensionTarget<D>],
    ) -> ExtensionTarget<D>
    where
        F: RichField + Extendable<D>,
    {
        let mut pairs = self
            .linear_combination
            .iter()
            .map(|&(c, f)| {
                (
                    v[c],
                    builder.constant_extension(F::Extension::from_basefield(f)),
                )
            })
            .collect::<Vec<_>>();
        let next_row_pairs = self.next_row_linear_combination.iter().map(|&(c, f)| {
            (
                next_v[c],
                builder.constant_extension(F::Extension::from_basefield(f)),
            )
        });
        pairs.extend(next_row_pairs);
        let constant = builder.constant_extension(F::Extension::from_basefield(self.constant));
        builder.inner_product_extension(F::ONE, constant, pairs)
    }
}

/// Represents a CTL filter, which evaluates to 1 if the row must be considered for the CTL and 0 otherwise.
/// It's an arbitrary degree 2 combination of columns: `products` are the degree 2 terms, and `constants` are
/// the degree 1 terms.
#[derive(Clone, Debug)]
pub(crate) struct Filter<F: Field> {
    products: Vec<(Column<F>, Column<F>)>,
    constants: Vec<Column<F>>,
}

impl<F: Field> Filter<F> {
    pub(crate) fn new(products: Vec<(Column<F>, Column<F>)>, constants: Vec<Column<F>>) -> Self {
        Self {
            products,
            constants,
        }
    }

    /// Returns a filter made of a single column.
    pub(crate) fn new_simple(col: Column<F>) -> Self {
        Self {
            products: vec![],
            constants: vec![col],
        }
    }

    /// Given the column values for the current and next rows, evaluates the filter.
    pub(crate) fn eval_filter<FE, P, const D: usize>(&self, v: &[P], next_v: &[P]) -> P
    where
        FE: FieldExtension<D, BaseField = F>,
        P: PackedField<Scalar = FE>,
    {
        self.products
            .iter()
            .map(|(col1, col2)| col1.eval_with_next(v, next_v) * col2.eval_with_next(v, next_v))
            .sum::<P>()
            + self
                .constants
                .iter()
                .map(|col| col.eval_with_next(v, next_v))
                .sum::<P>()
    }

    /// Circuit version of `eval_filter`:
    /// Given the column values for the current and next rows, evaluates the filter.
    pub(crate) fn eval_filter_circuit<const D: usize>(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        v: &[ExtensionTarget<D>],
        next_v: &[ExtensionTarget<D>],
    ) -> ExtensionTarget<D>
    where
        F: RichField + Extendable<D>,
    {
        let prods = self
            .products
            .iter()
            .map(|(col1, col2)| {
                let col1_eval = col1.eval_with_next_circuit(builder, v, next_v);
                let col2_eval = col2.eval_with_next_circuit(builder, v, next_v);
                builder.mul_extension(col1_eval, col2_eval)
            })
            .collect::<Vec<_>>();

        let consts = self
            .constants
            .iter()
            .map(|col| col.eval_with_next_circuit(builder, v, next_v))
            .collect::<Vec<_>>();

        let prods = builder.add_many_extension(prods);
        let consts = builder.add_many_extension(consts);
        builder.add_extension(prods, consts)
    }

    /// Evaluate on a row of a table given in column-major form.
    pub(crate) fn eval_table(&self, table: &[PolynomialValues<F>], row: usize) -> F {
        self.products
            .iter()
            .map(|(col1, col2)| col1.eval_table(table, row) * col2.eval_table(table, row))
            .sum::<F>()
            + self
                .constants
                .iter()
                .map(|col| col.eval_table(table, row))
                .sum()
    }

    pub(crate) fn eval_all_rows(&self, table: &[PolynomialValues<F>]) -> Vec<F> {
        let length = table[0].len();

        (0..length)
            .map(|row| self.eval_table(table, row))
            .collect::<Vec<F>>()
    }
}

/// A `Table` with a linear combination of columns and a filter.
/// `filter` is used to determine the rows to select in `Table`.
/// `columns` represents linear combinations of the columns of `Table`.
#[derive(Clone, Debug)]
pub(crate) struct TableWithColumns<F: Field> {
    table: Table,
    columns: Vec<Column<F>>,
    pub(crate) filter: Option<Filter<F>>,
}

impl<F: Field> TableWithColumns<F> {
    /// Generates a new `TableWithColumns` given a `Table`, a linear combination of columns `columns` and a `filter`.
    pub(crate) fn new(table: Table, columns: Vec<Column<F>>, filter: Option<Filter<F>>) -> Self {
        Self {
            table,
            columns,
            filter,
        }
    }
}

/// Cross-table lookup data consisting in the lookup table (`looked_table`) and all the tables that look into `looked_table` (`looking_tables`).
/// Each `looking_table` corresponds to a STARK's table whose rows have been filtered out and whose columns have been through a linear combination (see `eval_table`). The concatenation of those smaller tables should result in the `looked_table`.
#[derive(Clone)]
pub(crate) struct CrossTableLookup<F: Field> {
    /// Column linear combinations for all tables that are looking into the current table.
    pub(crate) looking_tables: Vec<TableWithColumns<F>>,
    /// Column linear combination for the current table.
    pub(crate) looked_table: TableWithColumns<F>,
}

impl<F: Field> CrossTableLookup<F> {
    /// Creates a new `CrossTableLookup` given some looking tables and a looked table.
    /// All tables should have the same width.
    pub(crate) fn new(
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
    pub(crate) fn num_ctl_helpers_zs_all(
        ctls: &[Self],
        table: Table,
        num_challenges: usize,
        constraint_degree: usize,
    ) -> (usize, usize, Vec<usize>) {
        let mut num_helpers = 0;
        let mut num_ctls = 0;
        let mut num_helpers_by_ctl = vec![0; ctls.len()];
        for (i, ctl) in ctls.iter().enumerate() {
            let all_tables = std::iter::once(&ctl.looked_table).chain(&ctl.looking_tables);
            let num_appearances = all_tables.filter(|twc| twc.table == table).count();
            let is_helpers = num_appearances > 2;
            if is_helpers {
                num_helpers_by_ctl[i] = ceil_div_usize(num_appearances, constraint_degree - 1);
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
#[derive(Clone, Default)]
pub(crate) struct CtlData<'a, F: Field> {
    /// Data associated with all Z(x) polynomials for one table.
    pub(crate) zs_columns: Vec<CtlZData<'a, F>>,
}

/// Cross-table lookup data associated with one Z(x) polynomial.
/// One Z(x) polynomial can be associated to multiple tables,
/// built from the same STARK.
#[derive(Clone)]
pub(crate) struct CtlZData<'a, F: Field> {
    /// Helper columns to verify the Z polynomial values.
    pub(crate) helper_columns: Vec<PolynomialValues<F>>,
    /// Z polynomial values.
    pub(crate) z: PolynomialValues<F>,
    /// Cross-table lookup challenge.
    pub(crate) challenge: GrandProductChallenge<F>,
    /// Vector of column linear combinations for the current tables.
    pub(crate) columns: Vec<&'a [Column<F>]>,
    /// Vector of filter columns for the current table.
    /// Each filter evaluates to either 1 or 0.
    pub(crate) filter: Vec<Option<Filter<F>>>,
}

impl<'a, F: Field> CtlData<'a, F> {
    /// Returns the number of cross-table lookup polynomials.
    pub(crate) fn len(&self) -> usize {
        self.zs_columns.len()
    }

    /// Returns whether there are no cross-table lookups.
    pub(crate) fn is_empty(&self) -> bool {
        self.zs_columns.is_empty()
    }

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

/// Randomness for a single instance of a permutation check protocol.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(crate) struct GrandProductChallenge<T: Copy + Eq + PartialEq + Debug> {
    /// Randomness used to combine multiple columns into one.
    pub(crate) beta: T,
    /// Random offset that's added to the beta-reduced column values.
    pub(crate) gamma: T,
}

impl<F: Field> GrandProductChallenge<F> {
    pub(crate) fn combine<'a, FE, P, T: IntoIterator<Item = &'a P>, const D2: usize>(
        &self,
        terms: T,
    ) -> P
    where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>,
        T::IntoIter: DoubleEndedIterator,
    {
        reduce_with_powers(terms, FE::from_basefield(self.beta)) + FE::from_basefield(self.gamma)
    }
}

impl GrandProductChallenge<Target> {
    pub(crate) fn combine_circuit<F: RichField + Extendable<D>, const D: usize>(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        terms: &[ExtensionTarget<D>],
    ) -> ExtensionTarget<D> {
        let reduced = reduce_with_powers_ext_circuit(builder, terms, self.beta);
        let gamma = builder.convert_to_ext(self.gamma);
        builder.add_extension(reduced, gamma)
    }
}

impl GrandProductChallenge<Target> {
    pub(crate) fn combine_base_circuit<F: RichField + Extendable<D>, const D: usize>(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        terms: &[Target],
    ) -> Target {
        let reduced = reduce_with_powers_circuit(builder, terms, self.beta);
        builder.add(reduced, self.gamma)
    }
}

/// Like `PermutationChallenge`, but with `num_challenges` copies to boost soundness.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct GrandProductChallengeSet<T: Copy + Eq + PartialEq + Debug> {
    pub(crate) challenges: Vec<GrandProductChallenge<T>>,
}

impl GrandProductChallengeSet<Target> {
    pub(crate) fn to_buffer(&self, buffer: &mut Vec<u8>) -> IoResult<()> {
        buffer.write_usize(self.challenges.len())?;
        for challenge in &self.challenges {
            buffer.write_target(challenge.beta)?;
            buffer.write_target(challenge.gamma)?;
        }
        Ok(())
    }

    pub(crate) fn from_buffer(buffer: &mut Buffer) -> IoResult<Self> {
        let length = buffer.read_usize()?;
        let mut challenges = Vec::with_capacity(length);
        for _ in 0..length {
            challenges.push(GrandProductChallenge {
                beta: buffer.read_target()?,
                gamma: buffer.read_target()?,
            });
        }

        Ok(GrandProductChallengeSet { challenges })
    }
}

fn get_grand_product_challenge<F: RichField, H: Hasher<F>>(
    challenger: &mut Challenger<F, H>,
) -> GrandProductChallenge<F> {
    let beta = challenger.get_challenge();
    let gamma = challenger.get_challenge();
    GrandProductChallenge { beta, gamma }
}

pub(crate) fn get_grand_product_challenge_set<F: RichField, H: Hasher<F>>(
    challenger: &mut Challenger<F, H>,
    num_challenges: usize,
) -> GrandProductChallengeSet<F> {
    let challenges = (0..num_challenges)
        .map(|_| get_grand_product_challenge(challenger))
        .collect();
    GrandProductChallengeSet { challenges }
}

fn get_grand_product_challenge_target<
    F: RichField + Extendable<D>,
    H: AlgebraicHasher<F>,
    const D: usize,
>(
    builder: &mut CircuitBuilder<F, D>,
    challenger: &mut RecursiveChallenger<F, H, D>,
) -> GrandProductChallenge<Target> {
    let beta = challenger.get_challenge(builder);
    let gamma = challenger.get_challenge(builder);
    GrandProductChallenge { beta, gamma }
}

pub(crate) fn get_grand_product_challenge_set_target<
    F: RichField + Extendable<D>,
    H: AlgebraicHasher<F>,
    const D: usize,
>(
    builder: &mut CircuitBuilder<F, D>,
    challenger: &mut RecursiveChallenger<F, H, D>,
    num_challenges: usize,
) -> GrandProductChallengeSet<Target> {
    let challenges = (0..num_challenges)
        .map(|_| get_grand_product_challenge_target(builder, challenger))
        .collect();
    GrandProductChallengeSet { challenges }
}

/// Returns the number of helper columns for each `Table`.
pub(crate) fn num_ctl_helper_columns_by_table<F: Field>(
    ctls: &[CrossTableLookup<F>],
    constraint_degree: usize,
) -> Vec<[usize; NUM_TABLES]> {
    let mut res = vec![[0; NUM_TABLES]; ctls.len()];
    for (i, ctl) in ctls.iter().enumerate() {
        let CrossTableLookup {
            looking_tables,
            looked_table,
        } = ctl;
        let mut num_by_table = [0; NUM_TABLES];

        let grouped_lookups = looking_tables.iter().group_by(|&a| a.table);

        for (table, group) in grouped_lookups.into_iter() {
            let sum = group.count();
            if sum > 2 {
                // We only need helper columns if there are more than 2 columns.
                num_by_table[table as usize] = ceil_div_usize(sum, constraint_degree - 1);
            }
        }

        res[i] = num_by_table;
    }
    res
}

/// Generates all the cross-table lookup data, for all tables.
/// - `trace_poly_values` corresponds to the trace values for all tables.
/// - `cross_table_lookups` corresponds to all the cross-table lookups, i.e. the looked and looking tables, as described in `CrossTableLookup`.
/// - `ctl_challenges` corresponds to the challenges used for CTLs.
/// - `constraint_degree` is the maximal constraint degree for the table.
/// For each `CrossTableLookup`, and each looking/looked table, the partial products for the CTL are computed, and added to the said table's `CtlZData`.
pub(crate) fn cross_table_lookup_data<'a, F: RichField, const D: usize>(
    trace_poly_values: &[Vec<PolynomialValues<F>>; NUM_TABLES],
    cross_table_lookups: &'a [CrossTableLookup<F>],
    ctl_challenges: &GrandProductChallengeSet<F>,
    constraint_degree: usize,
) -> [CtlData<'a, F>; NUM_TABLES] {
    let mut ctl_data_per_table = [0; NUM_TABLES].map(|_| CtlData::default());
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

            let mut z_looked = partial_sums(
                &trace_poly_values[looked_table.table as usize],
                &[(&looked_table.columns, &looked_table.filter)],
                challenge,
                constraint_degree,
            );

            for (table, helpers_zs) in helper_zs_looking {
                let num_helpers = helpers_zs.len() - 1;
                let count = looking_tables
                    .iter()
                    .filter(|looking_table| looking_table.table as usize == table)
                    .count();
                let cols_filts = looking_tables.iter().filter_map(|looking_table| {
                    if looking_table.table as usize == table {
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
            ctl_data_per_table[looked_table.table as usize]
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

type ColumnFilter<'a, F> = (&'a [Column<F>], &'a Option<Filter<F>>);

/// Given a STARK's trace, and the data associated to one lookup (either CTL or range check),
/// returns the associated helper polynomials.
pub(crate) fn get_helper_cols<F: Field>(
    trace: &[PolynomialValues<F>],
    degree: usize,
    columns_filters: &[ColumnFilter<F>],
    challenge: GrandProductChallenge<F>,
    constraint_degree: usize,
) -> Vec<PolynomialValues<F>> {
    let num_helper_columns = ceil_div_usize(columns_filters.len(), constraint_degree - 1);

    let mut helper_columns = Vec::with_capacity(num_helper_columns);

    let mut filter_index = 0;
    for mut cols_filts in &columns_filters.iter().chunks(constraint_degree - 1) {
        let (first_col, first_filter) = cols_filts.next().unwrap();

        let mut filter_col = Vec::with_capacity(degree);
        let first_combined = (0..degree)
            .map(|d| {
                let f = if let Some(filter) = first_filter {
                    let f = filter.eval_table(trace, d);
                    filter_col.push(f);
                    f
                } else {
                    filter_col.push(F::ONE);
                    F::ONE
                };
                if f.is_one() {
                    let evals = first_col
                        .iter()
                        .map(|c| c.eval_table(trace, d))
                        .collect::<Vec<F>>();
                    challenge.combine(evals.iter())
                } else {
                    assert_eq!(f, F::ZERO, "Non-binary filter?");
                    // Dummy value. Cannot be zero since it will be batch-inverted.
                    F::ONE
                }
            })
            .collect::<Vec<F>>();

        let mut acc = F::batch_multiplicative_inverse(&first_combined);
        for d in 0..degree {
            if filter_col[d].is_zero() {
                acc[d] = F::ZERO;
            }
        }

        for (col, filt) in cols_filts {
            let mut filter_col = Vec::with_capacity(degree);
            let mut combined = (0..degree)
                .map(|d| {
                    let f = if let Some(filter) = filt {
                        let f = filter.eval_table(trace, d);
                        filter_col.push(f);
                        f
                    } else {
                        filter_col.push(F::ONE);
                        F::ONE
                    };
                    if f.is_one() {
                        let evals = col
                            .iter()
                            .map(|c| c.eval_table(trace, d))
                            .collect::<Vec<F>>();
                        challenge.combine(evals.iter())
                    } else {
                        assert_eq!(f, F::ZERO, "Non-binary filter?");
                        // Dummy value. Cannot be zero since it will be batch-inverted.
                        F::ONE
                    }
                })
                .collect::<Vec<F>>();

            combined = F::batch_multiplicative_inverse(&combined);

            for d in 0..degree {
                if filter_col[d].is_zero() {
                    combined[d] = F::ZERO;
                }
            }

            batch_add_inplace(&mut acc, &combined);
        }

        helper_columns.push(acc.into());
    }
    assert_eq!(helper_columns.len(), num_helper_columns);

    helper_columns
}

/// Computes helper columns and Z polynomials for all looking tables
/// of one cross-table lookup (i.e. for one looked table).
fn ctl_helper_zs_cols<F: Field>(
    all_stark_traces: &[Vec<PolynomialValues<F>>; NUM_TABLES],
    looking_tables: Vec<TableWithColumns<F>>,
    challenge: GrandProductChallenge<F>,
    constraint_degree: usize,
) -> Vec<(usize, Vec<PolynomialValues<F>>)> {
    let grouped_lookups = looking_tables.iter().group_by(|a| a.table);

    grouped_lookups
        .into_iter()
        .map(|(table, group)| {
            let degree = all_stark_traces[table as usize][0].len();
            let columns_filters = group
                .map(|table| (&table.columns[..], &table.filter))
                .collect::<Vec<(&[Column<F>], &Option<Filter<F>>)>>();
            (
                table as usize,
                partial_sums(
                    &all_stark_traces[table as usize],
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
/// The values `v`` are grouped together, in groups of size `constraint_degree - 1` (2 in our case). For each group, we construct a helper
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
    if columns_filters.len() > 2 {
        helper_columns.push(z.into());
    } else {
        helper_columns = vec![z.into()];
    }

    helper_columns
}

/// Data necessary to check the cross-table lookups of a given table.
#[derive(Clone)]
pub(crate) struct CtlCheckVars<'a, F, FE, P, const D2: usize>
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
    pub(crate) filter: Vec<Option<Filter<F>>>,
}

impl<'a, F: RichField + Extendable<D>, const D: usize>
    CtlCheckVars<'a, F, F::Extension, F::Extension, D>
{
    /// Extracts the `CtlCheckVars` for each STARK.
    pub(crate) fn from_proofs<C: GenericConfig<D, F = F>>(
        proofs: &[StarkProofWithMetadata<F, C, D>; NUM_TABLES],
        cross_table_lookups: &'a [CrossTableLookup<F>],
        ctl_challenges: &'a GrandProductChallengeSet<F>,
        num_lookup_columns: &[usize; NUM_TABLES],
        num_helper_ctl_columns: &Vec<[usize; NUM_TABLES]>,
    ) -> [Vec<Self>; NUM_TABLES] {
        let mut total_num_helper_cols_by_table = [0; NUM_TABLES];
        for p_ctls in num_helper_ctl_columns {
            for j in 0..NUM_TABLES {
                total_num_helper_cols_by_table[j] += p_ctls[j] * ctl_challenges.challenges.len();
            }
        }

        // Get all cross-table lookup polynomial openings for each STARK proof.
        let mut ctl_zs = proofs
            .iter()
            .zip(num_lookup_columns)
            .map(|(p, &num_lookup)| {
                let openings = &p.proof.openings;

                let ctl_zs = &openings.auxiliary_polys[num_lookup..];
                let ctl_zs_next = &openings.auxiliary_polys_next[num_lookup..];
                ctl_zs.iter().zip(ctl_zs_next).collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        // Put each cross-table lookup polynomial into the correct table data: if a CTL polynomial is extracted from looking/looked table t, then we add it to the `CtlCheckVars` of table t.
        let mut start_indices = [0; NUM_TABLES];
        let mut z_indices = [0; NUM_TABLES];
        let mut ctl_vars_per_table = [0; NUM_TABLES].map(|_| vec![]);
        for (
            CrossTableLookup {
                looking_tables,
                looked_table,
            },
            num_ctls,
        ) in cross_table_lookups.iter().zip(num_helper_ctl_columns)
        {
            for &challenges in &ctl_challenges.challenges {
                // Group looking tables by `Table`, since we bundle the looking tables taken from the same `Table` together thanks to helper columns.
                // We want to only iterate on each `Table` once.
                let mut filtered_looking_tables =
                    Vec::with_capacity(min(looking_tables.len(), NUM_TABLES));
                for table in looking_tables {
                    if !filtered_looking_tables.contains(&(table.table as usize)) {
                        filtered_looking_tables.push(table.table as usize);
                    }
                }

                for (i, &table) in filtered_looking_tables.iter().enumerate() {
                    // We have first all the helper polynomials, then all the z polynomials.
                    let (looking_z, looking_z_next) =
                        ctl_zs[table][total_num_helper_cols_by_table[table] + z_indices[table]];

                    let count = looking_tables
                        .iter()
                        .filter(|looking_table| looking_table.table as usize == table)
                        .count();
                    let cols_filts = looking_tables.iter().filter_map(|looking_table| {
                        if looking_table.table as usize == table {
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
                    let helper_columns = ctl_zs[table]
                        [start_indices[table]..start_indices[table] + num_ctls[table]]
                        .iter()
                        .map(|&(h, _)| *h)
                        .collect::<Vec<_>>();

                    start_indices[table] += num_ctls[table];

                    z_indices[table] += 1;
                    ctl_vars_per_table[table].push(Self {
                        helper_columns,
                        local_z: *looking_z,
                        next_z: *looking_z_next,
                        challenges,
                        columns,
                        filter,
                    });
                }

                let (looked_z, looked_z_next) = ctl_zs[looked_table.table as usize]
                    [total_num_helper_cols_by_table[looked_table.table as usize]
                        + z_indices[looked_table.table as usize]];

                z_indices[looked_table.table as usize] += 1;

                let columns = vec![&looked_table.columns[..]];
                let filter = vec![looked_table.filter.clone()];
                ctl_vars_per_table[looked_table.table as usize].push(Self {
                    helper_columns: vec![],
                    local_z: *looked_z,
                    next_z: *looked_z_next,
                    challenges,
                    columns,
                    filter,
                });
            }
        }
        ctl_vars_per_table
    }
}

/// Given data associated to a lookup (either a CTL or a range-check), check the associated helper polynomials.
pub(crate) fn eval_helper_columns<F, FE, P, const D: usize, const D2: usize>(
    filter: &[Option<Filter<F>>],
    columns: &[Vec<P>],
    local_values: &[P],
    next_values: &[P],
    helper_columns: &[P],
    constraint_degree: usize,
    challenges: &GrandProductChallenge<F>,
    consumer: &mut ConstraintConsumer<P>,
) where
    F: RichField + Extendable<D>,
    FE: FieldExtension<D2, BaseField = F>,
    P: PackedField<Scalar = FE>,
{
    if !helper_columns.is_empty() {
        for (j, chunk) in columns.chunks(constraint_degree - 1).enumerate() {
            let fs =
                &filter[(constraint_degree - 1) * j..(constraint_degree - 1) * j + chunk.len()];
            let h = helper_columns[j];

            match chunk.len() {
                2 => {
                    let combin0 = challenges.combine(&chunk[0]);
                    let combin1 = challenges.combine(chunk[1].iter());

                    let f0 = if let Some(filter0) = &fs[0] {
                        filter0.eval_filter(local_values, next_values)
                    } else {
                        P::ONES
                    };
                    let f1 = if let Some(filter1) = &fs[1] {
                        filter1.eval_filter(local_values, next_values)
                    } else {
                        P::ONES
                    };

                    consumer.constraint(combin1 * combin0 * h - f0 * combin1 - f1 * combin0);
                }
                1 => {
                    let combin = challenges.combine(&chunk[0]);
                    let f0 = if let Some(filter1) = &fs[0] {
                        filter1.eval_filter(local_values, next_values)
                    } else {
                        P::ONES
                    };
                    consumer.constraint(combin * h - f0);
                }

                _ => todo!("Allow other constraint degrees"),
            }
        }
    }
}

/// Checks the cross-table lookup Z polynomials for each table:
/// - Checks that the CTL `Z` partial sums are correctly updated.
/// - Checks that the final value of the CTL sum is the combination of all STARKs' CTL polynomials.
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

            let f0 = if let Some(filter0) = &filter[0] {
                filter0.eval_filter(local_values, next_values)
            } else {
                P::ONES
            };
            let f1 = if let Some(filter1) = &filter[1] {
                filter1.eval_filter(local_values, next_values)
            } else {
                P::ONES
            };

            consumer
                .constraint_last_row(combin0 * combin1 * *local_z - f0 * combin1 - f1 * combin0);
            consumer.constraint_transition(
                combin0 * combin1 * (*local_z - *next_z) - f0 * combin1 - f1 * combin0,
            );
        } else {
            let combin0 = challenges.combine(&evals[0]);
            let f0 = if let Some(filter0) = &filter[0] {
                filter0.eval_filter(local_values, next_values)
            } else {
                P::ONES
            };
            consumer.constraint_last_row(combin0 * *local_z - f0);
            consumer.constraint_transition(combin0 * (*local_z - *next_z) - f0);
        }
    }
}

/// Circuit version of `CtlCheckVars`. Data necessary to check the cross-table lookups of a given table.
#[derive(Clone)]
pub(crate) struct CtlCheckVarsTarget<F: Field, const D: usize> {
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
    pub(crate) filter: Vec<Option<Filter<F>>>,
}

impl<'a, F: Field, const D: usize> CtlCheckVarsTarget<F, D> {
    /// Circuit version of `from_proofs`. Extracts the `CtlCheckVarsTarget` for each STARK.
    pub(crate) fn from_proof(
        table: Table,
        proof: &StarkProofTarget<D>,
        cross_table_lookups: &'a [CrossTableLookup<F>],
        ctl_challenges: &'a GrandProductChallengeSet<Target>,
        num_lookup_columns: usize,
        total_num_helper_columns: usize,
        num_helper_ctl_columns: &[usize],
    ) -> Vec<Self> {
        // Get all cross-table lookup polynomial openings for each STARK proof.
        let mut ctl_zs = {
            let openings = &proof.openings;
            let ctl_zs = openings.auxiliary_polys.iter().skip(num_lookup_columns);
            let ctl_zs_next = openings
                .auxiliary_polys_next
                .iter()
                .skip(num_lookup_columns);
            ctl_zs.zip(ctl_zs_next).collect::<Vec<_>>()
        };

        // Put each cross-table lookup polynomial into the correct table data: if a CTL polynomial is extracted from looking/looked table t, then we add it to the `CtlCheckVars` of table t.
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
                // Group looking tables by `Table`, since we bundle the looking tables taken from the same `Table` together thanks to helper columns.

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
                    // let columns = group.0.clone();
                    // let filter = group.1.clone();
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

/// Circuit version of `eval_helper_columns`.
/// Given data associated to a lookup (either a CTL or a range-check), check the associated helper polynomials.
pub(crate) fn eval_helper_columns_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    filter: &[Option<Filter<F>>],
    columns: &[Vec<ExtensionTarget<D>>],
    local_values: &[ExtensionTarget<D>],
    next_values: &[ExtensionTarget<D>],
    helper_columns: &[ExtensionTarget<D>],
    constraint_degree: usize,
    challenges: &GrandProductChallenge<Target>,
    consumer: &mut RecursiveConstraintConsumer<F, D>,
) {
    if !helper_columns.is_empty() {
        for (j, chunk) in columns.chunks(constraint_degree - 1).enumerate() {
            let fs =
                &filter[(constraint_degree - 1) * j..(constraint_degree - 1) * j + chunk.len()];
            let h = helper_columns[j];

            let one = builder.one_extension();
            match chunk.len() {
                2 => {
                    let combin0 = challenges.combine_circuit(builder, &chunk[0]);
                    let combin1 = challenges.combine_circuit(builder, &chunk[1]);

                    let f0 = if let Some(filter0) = &fs[0] {
                        filter0.eval_filter_circuit(builder, local_values, next_values)
                    } else {
                        one
                    };
                    let f1 = if let Some(filter1) = &fs[1] {
                        filter1.eval_filter_circuit(builder, local_values, next_values)
                    } else {
                        one
                    };

                    let constr = builder.mul_sub_extension(combin0, h, f0);
                    let constr = builder.mul_extension(constr, combin1);
                    let f1_constr = builder.mul_extension(f1, combin0);
                    let constr = builder.sub_extension(constr, f1_constr);

                    consumer.constraint(builder, constr);
                }
                1 => {
                    let combin = challenges.combine_circuit(builder, &chunk[0]);
                    let f0 = if let Some(filter1) = &fs[0] {
                        filter1.eval_filter_circuit(builder, local_values, next_values)
                    } else {
                        one
                    };
                    let constr = builder.mul_sub_extension(combin, h, f0);
                    consumer.constraint(builder, constr);
                }

                _ => todo!("Allow other constraint degrees"),
            }
        }
    }
}

/// Circuit version of `eval_cross_table_lookup_checks`. Checks the cross-table lookup Z polynomials for each table:
/// - Checks that the CTL `Z` partial sums are correctly updated.
/// - Checks that the final value of the CTL sum is the combination of all STARKs' CTL polynomials.
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

    let one = builder.one_extension();

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

            let f0 = if let Some(filter0) = &filter[0] {
                filter0.eval_filter_circuit(builder, local_values, next_values)
            } else {
                one
            };
            let f1 = if let Some(filter1) = &filter[1] {
                filter1.eval_filter_circuit(builder, local_values, next_values)
            } else {
                one
            };

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
            let f0 = if let Some(filter0) = &filter[0] {
                filter0.eval_filter_circuit(builder, local_values, next_values)
            } else {
                one
            };

            let constr = builder.mul_sub_extension(combin0, *local_z, f0);
            consumer.constraint_last_row(builder, constr);
            let constr = builder.mul_sub_extension(combin0, z_diff, f0);
            consumer.constraint_transition(builder, constr);
        }
    }
}

/// Verifies all cross-table lookups.
pub(crate) fn verify_cross_table_lookups<F: RichField + Extendable<D>, const D: usize>(
    cross_table_lookups: &[CrossTableLookup<F>],
    ctl_zs_first: [Vec<F>; NUM_TABLES],
    ctl_extra_looking_sums: Vec<Vec<F>>,
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
        // Get elements looking into `looked_table` that are not associated to any STARK.
        let extra_sum_vec = &ctl_extra_looking_sums[looked_table.table as usize];
        // We want to iterate on each looking table only once.
        let mut filtered_looking_tables = vec![];
        for table in looking_tables {
            if !filtered_looking_tables.contains(&(table.table as usize)) {
                filtered_looking_tables.push(table.table as usize);
            }
        }
        for c in 0..config.num_challenges {
            // Compute the combination of all looking table CTL polynomial openings.

            let looking_zs_sum = filtered_looking_tables
                .iter()
                .map(|&table| *ctl_zs_openings[table].next().unwrap())
                .sum::<F>()
                + extra_sum_vec[c];

            // Get the looked table CTL polynomial opening.
            let looked_z = *ctl_zs_openings[looked_table.table as usize].next().unwrap();
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
pub(crate) fn verify_cross_table_lookups_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    cross_table_lookups: Vec<CrossTableLookup<F>>,
    ctl_zs_first: [Vec<Target>; NUM_TABLES],
    ctl_extra_looking_sums: Vec<Vec<Target>>,
    inner_config: &StarkConfig,
) {
    let mut ctl_zs_openings = ctl_zs_first.iter().map(|v| v.iter()).collect::<Vec<_>>();
    for CrossTableLookup {
        looking_tables,
        looked_table,
    } in cross_table_lookups.into_iter()
    {
        // Get elements looking into `looked_table` that are not associated to any STARK.
        let extra_sum_vec = &ctl_extra_looking_sums[looked_table.table as usize];
        // We want to iterate on each looking table only once.
        let mut filtered_looking_tables = vec![];
        for table in looking_tables {
            if !filtered_looking_tables.contains(&(table.table as usize)) {
                filtered_looking_tables.push(table.table as usize);
            }
        }
        for c in 0..inner_config.num_challenges {
            // Compute the combination of all looking table CTL polynomial openings.
            let mut looking_zs_sum = builder.add_many(
                filtered_looking_tables
                    .iter()
                    .map(|&table| *ctl_zs_openings[table].next().unwrap()),
            );

            looking_zs_sum = builder.add(looking_zs_sum, extra_sum_vec[c]);

            // Get the looked table CTL polynomial opening.
            let looked_z = *ctl_zs_openings[looked_table.table as usize].next().unwrap();
            // Verify that the combination of looking table openings is equal to the looked table opening.
            builder.connect(looked_z, looking_zs_sum);
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
        extra_memory_looking_values: &[Vec<F>],
    ) {
        for (i, ctl) in cross_table_lookups.iter().enumerate() {
            check_ctl(trace_poly_values, ctl, i, extra_memory_looking_values);
        }
    }

    fn check_ctl<F: Field>(
        trace_poly_values: &[Vec<PolynomialValues<F>>],
        ctl: &CrossTableLookup<F>,
        ctl_index: usize,
        extra_memory_looking_values: &[Vec<F>],
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

        // Extra looking values for memory
        if ctl_index == Table::Memory as usize {
            for row in extra_memory_looking_values.iter() {
                // The table and the row index don't matter here, as we just want to enforce
                // that the special extra values do appear when looking against the Memory table.
                looking_multiset
                    .entry(row.to_vec())
                    .or_default()
                    .push((Table::Cpu, 0));
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
        let trace = &trace_poly_values[table.table as usize];
        for i in 0..trace[0].len() {
            let filter = if let Some(combin) = &table.filter {
                combin.eval_table(trace, i)
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
