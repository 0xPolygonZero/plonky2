//! A Lookup protocol leveraging logarithmic derivatives,
//! introduced in <https://eprint.iacr.org/2022/1530.pdf>.

#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};
use core::borrow::Borrow;
use core::fmt::Debug;
use core::iter::repeat;

#[cfg(feature = "std")]
use itertools::Itertools;
use num_bigint::BigUint;
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
use plonky2::plonk::config::{AlgebraicHasher, Hasher};
use plonky2::plonk::plonk_common::{
    reduce_with_powers, reduce_with_powers_circuit, reduce_with_powers_ext_circuit,
};
use plonky2::util::serialization::{Buffer, IoResult, Read, Write};

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::evaluation_frame::StarkEvaluationFrame;
use crate::stark::Stark;

/// Represents a filter, which evaluates to 1 if the row must be considered and 0 if it should be ignored.
/// It's an arbitrary degree 2 combination of columns: `products` are the degree 2 terms, and `constants` are
/// the degree 1 terms.
#[derive(Clone, Debug)]
pub struct Filter<F: Field> {
    products: Vec<(Column<F>, Column<F>)>,
    constants: Vec<Column<F>>,
}

/// The default filter is always on.
impl<F: Field> Default for Filter<F> {
    fn default() -> Self {
        Self {
            products: vec![],
            constants: vec![Column::constant(F::ONE)],
        }
    }
}

impl<F: Field> Filter<F> {
    /// Returns a filter from the provided `products` and `constants` vectors.
    pub fn new(products: Vec<(Column<F>, Column<F>)>, constants: Vec<Column<F>>) -> Self {
        Self {
            products,
            constants,
        }
    }

    /// Returns a filter made of a single column.
    pub fn new_simple(col: Column<F>) -> Self {
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
}

/// Represent two linear combination of columns, corresponding to the current and next row values.
/// Each linear combination is represented as:
/// - a vector of `(usize, F)` corresponding to the column number and the associated multiplicand
/// - the constant of the linear combination.
#[derive(Clone, Debug)]
pub struct Column<F: Field> {
    linear_combination: Vec<(usize, F)>,
    next_row_linear_combination: Vec<(usize, F)>,
    constant: F,
}

impl<F: Field> Column<F> {
    /// Returns the representation of a single column in the current row.
    pub fn single(c: usize) -> Self {
        Self {
            linear_combination: vec![(c, F::ONE)],
            next_row_linear_combination: vec![],
            constant: F::ZERO,
        }
    }

    /// Returns multiple single columns in the current row.
    pub fn singles<I: IntoIterator<Item = impl Borrow<usize>>>(
        cs: I,
    ) -> impl Iterator<Item = Self> {
        cs.into_iter().map(|c| Self::single(*c.borrow()))
    }

    /// Returns the representation of a single column in the next row.
    pub fn single_next_row(c: usize) -> Self {
        Self {
            linear_combination: vec![],
            next_row_linear_combination: vec![(c, F::ONE)],
            constant: F::ZERO,
        }
    }

    /// Returns multiple single columns for the next row.
    pub fn singles_next_row<I: IntoIterator<Item = impl Borrow<usize>>>(
        cs: I,
    ) -> impl Iterator<Item = Self> {
        cs.into_iter().map(|c| Self::single_next_row(*c.borrow()))
    }

    /// Returns a linear combination corresponding to a constant.
    pub fn constant(constant: F) -> Self {
        Self {
            linear_combination: vec![],
            next_row_linear_combination: vec![],
            constant,
        }
    }

    /// Returns a linear combination corresponding to 0.
    pub fn zero() -> Self {
        Self::constant(F::ZERO)
    }

    /// Returns a linear combination corresponding to 1.
    pub fn one() -> Self {
        Self::constant(F::ONE)
    }

    /// Given an iterator of `(usize, F)` and a constant, returns the association linear combination of columns for the current row.
    pub fn linear_combination_with_constant<I: IntoIterator<Item = (usize, F)>>(
        iter: I,
        constant: F,
    ) -> Self {
        let v = iter.into_iter().collect::<Vec<_>>();
        assert!(!v.is_empty());

        // Because this is a debug assertion, we only check it when the `std`
        // feature is activated, as `Itertools::unique` relies on collections.
        #[cfg(feature = "std")]
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
    pub fn linear_combination_and_next_row_with_constant<I: IntoIterator<Item = (usize, F)>>(
        iter: I,
        next_row_iter: I,
        constant: F,
    ) -> Self {
        let v = iter.into_iter().collect::<Vec<_>>();
        let next_row_v = next_row_iter.into_iter().collect::<Vec<_>>();

        assert!(!v.is_empty() || !next_row_v.is_empty());

        // Because these are debug assertions, we only check them when the `std`
        // feature is activated, as `Itertools::unique` relies on collections.
        #[cfg(feature = "std")]
        {
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
        }

        Self {
            linear_combination: v,
            next_row_linear_combination: next_row_v,
            constant,
        }
    }

    /// Returns a linear combination of columns, with no additional constant.
    pub fn linear_combination<I: IntoIterator<Item = (usize, F)>>(iter: I) -> Self {
        Self::linear_combination_with_constant(iter, F::ZERO)
    }

    /// Given an iterator of columns (c_0, ..., c_n) containing bits in little endian order:
    /// returns the representation of c_0 + 2 * c_1 + ... + 2^n * c_n.
    pub fn le_bits<I: IntoIterator<Item = impl Borrow<usize>>>(cs: I) -> Self {
        Self::linear_combination(cs.into_iter().map(|c| *c.borrow()).zip(F::TWO.powers()))
    }

    /// Given an iterator of columns (c_0, ..., c_n) containing bits in little endian order:
    /// returns the representation of c_0 + 2 * c_1 + ... + 2^n * c_n + k where `k` is an
    /// additional constant.
    pub fn le_bits_with_constant<I: IntoIterator<Item = impl Borrow<usize>>>(
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
    pub fn le_bytes<I: IntoIterator<Item = impl Borrow<usize>>>(cs: I) -> Self {
        Self::linear_combination(
            cs.into_iter()
                .map(|c| *c.borrow())
                .zip(F::from_canonical_u16(256).powers()),
        )
    }

    /// Given an iterator of columns, returns the representation of their sum.
    pub fn sum<I: IntoIterator<Item = impl Borrow<usize>>>(cs: I) -> Self {
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
        self.linear_combination
            .iter()
            .map(|&(c, f)| table[c].values[row] * f)
            .sum::<F>()
            + self
                .next_row_linear_combination
                .iter()
                .map(|&(c, f)| table[c].values[(row + 1) % table[c].values.len()] * f)
                .sum::<F>()
            + self.constant
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

pub(crate) type ColumnFilter<'a, F> = (&'a [Column<F>], &'a Filter<F>);

/// A [`Lookup`] defines a set of `columns`` whose values should appear in a
/// `table_column` (i.e. the lookup table associated to these looking columns),
/// along with a `frequencies_column` indicating the frequency of each looking
/// column in the looked table.
///
/// It also features a `filter_columns` vector, optionally adding at most one
/// filter per looking column.
///
/// The lookup argumented implemented here is based on logarithmic derivatives,
/// a technique described with the whole lookup protocol in
/// <https://eprint.iacr.org/2022/1530>.
#[derive(Debug)]
pub struct Lookup<F: Field> {
    /// Columns whose values should be contained in the lookup table.
    /// These are the f_i(x) polynomials in the logUp paper.
    pub columns: Vec<Column<F>>,
    /// Column containing the lookup table.
    /// This is the t(x) polynomial in the logUp paper.
    pub table_column: Column<F>,
    /// Column containing the frequencies of `columns` in `table_column`.
    /// This is the m(x) polynomial in the paper.
    pub frequencies_column: Column<F>,

    /// Columns to filter some elements. There is at most one filter
    /// column per column to lookup.
    pub filter_columns: Vec<Filter<F>>,
}

impl<F: Field> Lookup<F> {
    /// Outputs the number of helper columns needed by this [`Lookup`].
    pub fn num_helper_columns(&self, constraint_degree: usize) -> usize {
        // One helper column for each column batch of size `constraint_degree-1`,
        // then one column for the inverse of `table + challenge` and one for the `Z` polynomial.

        self.columns
            .len()
            .div_ceil(constraint_degree.checked_sub(1).unwrap_or(1))
            + 1
    }
}

/// Randomness for a single instance of a permutation check protocol.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct GrandProductChallenge<T: Copy + Eq + PartialEq + Debug> {
    /// Randomness used to combine multiple columns into one.
    pub beta: T,
    /// Random offset that's added to the beta-reduced column values.
    pub gamma: T,
}

impl<F: Field> GrandProductChallenge<F> {
    /// Combines a series of values `t_i` with these challenge random values.
    /// In particular, given `beta` and `gamma` challenges, this will compute
    /// `(Σ t_i * beta^i) + gamma`.
    pub fn combine<'a, FE, P, T: IntoIterator<Item = &'a P>, const D2: usize>(&self, terms: T) -> P
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
    /// Circuit version of `combine`.
    pub fn combine_base_circuit<F: RichField + Extendable<D>, const D: usize>(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        terms: &[Target],
    ) -> Target {
        let reduced = reduce_with_powers_circuit(builder, terms, self.beta);
        builder.add(reduced, self.gamma)
    }
}

/// Like `GrandProductChallenge`, but with `num_challenges` copies to boost soundness.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct GrandProductChallengeSet<T: Copy + Eq + PartialEq + Debug> {
    /// A sequence of `num_challenges` challenge pairs, where `num_challenges`
    /// is defined in [`StarkConfig`][crate::config::StarkConfig].
    pub challenges: Vec<GrandProductChallenge<T>>,
}

impl GrandProductChallengeSet<Target> {
    /// Serializes this `GrandProductChallengeSet` of `Target`s.
    pub fn to_buffer(&self, buffer: &mut Vec<u8>) -> IoResult<()> {
        buffer.write_usize(self.challenges.len())?;
        for challenge in &self.challenges {
            buffer.write_target(challenge.beta)?;
            buffer.write_target(challenge.gamma)?;
        }
        Ok(())
    }

    /// Serializes a `GrandProductChallengeSet` of `Target`s from the provided buffer.
    pub fn from_buffer(buffer: &mut Buffer) -> IoResult<Self> {
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

/// Generates a new `GrandProductChallengeSet` containing `num_challenges`
/// pairs of challenges from the current `challenger` state.
pub fn get_grand_product_challenge_set<F: RichField, H: Hasher<F>>(
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

/// Circuit version of `get_grand_product_challenge_set`.
pub fn get_grand_product_challenge_set_target<
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
    assert_eq!(lookup.columns.len(), lookup.filter_columns.len());

    let num_total_logup_entries = trace_poly_values[0].values.len() * lookup.columns.len();
    assert!(BigUint::from(num_total_logup_entries) < F::characteristic());

    let num_helper_columns = lookup.num_helper_columns(constraint_degree);

    let looking_cols = lookup
        .columns
        .iter()
        .map(|col| vec![col.clone()])
        .collect::<Vec<Vec<Column<F>>>>();

    let grand_challenge = GrandProductChallenge {
        beta: F::ONE,
        gamma: challenge,
    };

    let columns_filters = looking_cols
        .iter()
        .zip(lookup.filter_columns.iter())
        .map(|(col, filter)| (&col[..], filter))
        .collect::<Vec<_>>();
    // For each batch of `constraint_degree-1` columns `fi`, compute `sum 1/(f_i+challenge)` and
    // add it to the helper columns.
    // Note: these are the h_k(x) polynomials in the paper, with a few differences:
    //       * Here, the first ratio m_0(x)/phi_0(x) is not included with the columns batched up to create the
    //         h_k polynomials; instead there's a separate helper column for it (see below).
    //       * Here, we use 1 instead of -1 as the numerator (and subtract later).
    //       * Here, for now, the batch size (l) is always constraint_degree - 1 = 2.
    //       * Here, there are filters for the columns, to only select some rows
    //         in a given column.
    let mut helper_columns = get_helper_cols(
        trace_poly_values,
        trace_poly_values[0].len(),
        &columns_filters,
        grand_challenge,
        constraint_degree,
    );

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

/// Given data associated to a lookup, check the associated helper polynomials.
pub(crate) fn eval_helper_columns<F, FE, P, const D: usize, const D2: usize>(
    filter: &[Filter<F>],
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
        let chunk_size = constraint_degree.checked_sub(1).unwrap_or(1);
        for (chunk, (fs, &h)) in columns
            .chunks(chunk_size)
            .zip(filter.chunks(chunk_size).zip(helper_columns))
        {
            match chunk.len() {
                2 => {
                    let combin0 = challenges.combine(&chunk[0]);
                    let combin1 = challenges.combine(chunk[1].iter());

                    let f0 = fs[0].eval_filter(local_values, next_values);
                    let f1 = fs[1].eval_filter(local_values, next_values);

                    consumer.constraint(combin1 * combin0 * h - f0 * combin1 - f1 * combin0);
                }
                1 => {
                    let combin = challenges.combine(&chunk[0]);
                    let f0 = fs[0].eval_filter(local_values, next_values);
                    consumer.constraint(combin * h - f0);
                }

                _ => todo!("Allow other constraint degrees"),
            }
        }
    }
}

/// Circuit version of `eval_helper_columns`.
/// Given data associated to a lookup (either a CTL or a range-check), check the associated helper polynomials.
pub(crate) fn eval_helper_columns_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    filter: &[Filter<F>],
    columns: &[Vec<ExtensionTarget<D>>],
    local_values: &[ExtensionTarget<D>],
    next_values: &[ExtensionTarget<D>],
    helper_columns: &[ExtensionTarget<D>],
    constraint_degree: usize,
    challenges: &GrandProductChallenge<Target>,
    consumer: &mut RecursiveConstraintConsumer<F, D>,
) {
    if !helper_columns.is_empty() {
        let chunk_size = constraint_degree.checked_sub(1).unwrap_or(1);
        for (chunk, (fs, &h)) in columns
            .chunks(chunk_size)
            .zip(filter.chunks(chunk_size).zip(helper_columns))
        {
            match chunk.len() {
                2 => {
                    let combin0 = challenges.combine_circuit(builder, &chunk[0]);
                    let combin1 = challenges.combine_circuit(builder, &chunk[1]);

                    let f0 = fs[0].eval_filter_circuit(builder, local_values, next_values);
                    let f1 = fs[1].eval_filter_circuit(builder, local_values, next_values);

                    let constr = builder.mul_sub_extension(combin0, h, f0);
                    let constr = builder.mul_extension(constr, combin1);
                    let f1_constr = builder.mul_extension(f1, combin0);
                    let constr = builder.sub_extension(constr, f1_constr);

                    consumer.constraint(builder, constr);
                }
                1 => {
                    let combin = challenges.combine_circuit(builder, &chunk[0]);
                    let f0 = fs[0].eval_filter_circuit(builder, local_values, next_values);
                    let constr = builder.mul_sub_extension(combin, h, f0);
                    consumer.constraint(builder, constr);
                }

                _ => todo!("Allow other constraint degrees"),
            }
        }
    }
}

/// Given a STARK's trace, and the data associated to one lookup (either CTL or range check),
/// returns the associated helper polynomials.
pub(crate) fn get_helper_cols<F: Field>(
    trace: &[PolynomialValues<F>],
    degree: usize,
    columns_filters: &[ColumnFilter<F>],
    challenge: GrandProductChallenge<F>,
    constraint_degree: usize,
) -> Vec<PolynomialValues<F>> {
    let num_helper_columns = columns_filters
        .len()
        .div_ceil(constraint_degree.checked_sub(1).unwrap_or(1));

    let chunks = columns_filters.chunks(constraint_degree.checked_sub(1).unwrap_or(1));
    let helper_columns: Vec<_> = chunks
        .filter_map(|cols_filts| {
            cols_filts
                .iter()
                .map(|(col, filter)| {
                    let combined = (0..degree)
                        .map(|d| {
                            let evals = col
                                .iter()
                                .map(|c| c.eval_table(trace, d))
                                .collect::<Vec<F>>();
                            challenge.combine(&evals)
                        })
                        .collect::<Vec<F>>();

                    let mut combined = F::batch_multiplicative_inverse(&combined);
                    let filter_col: Vec<_> =
                        (0..degree).map(|d| filter.eval_table(trace, d)).collect();
                    batch_multiply_inplace(&mut combined, &filter_col);
                    combined
                })
                .reduce(|mut acc, combined| {
                    batch_add_inplace(&mut acc, &combined);
                    acc
                })
                .map(PolynomialValues::from)
        })
        .collect();
    assert_eq!(helper_columns.len(), num_helper_columns);

    helper_columns
}

#[derive(Debug)]
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
    let mut start = 0;
    for lookup in lookups {
        let num_helper_columns = lookup.num_helper_columns(degree);
        for &challenge in &lookup_vars.challenges {
            let grand_challenge = GrandProductChallenge {
                beta: F::ONE,
                gamma: challenge,
            };
            let lookup_columns = lookup
                .columns
                .iter()
                .map(|col| vec![col.eval_with_next(local_values, next_values)])
                .collect::<Vec<Vec<P>>>();

            // For each chunk, check that `h_i (x+f_2i) (x+f_{2i+1}) = (x+f_2i) * filter_{2i+1} + (x+f_{2i+1}) * filter_2i`
            // if the chunk has length 2 or if it has length 1, check that `h_i * (x+f_2i) = filter_2i`, where x is the challenge
            eval_helper_columns(
                &lookup.filter_columns,
                &lookup_columns,
                local_values,
                next_values,
                &lookup_vars.local_values[start..start + num_helper_columns - 1],
                degree,
                &grand_challenge,
                yield_constr,
            );

            let challenge = FE::from_basefield(challenge);

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

#[derive(Debug)]
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
    let degree = stark.constraint_degree();
    let lookups = stark.lookups();

    let local_values = vars.get_local_values();
    let next_values = vars.get_next_values();
    let mut start = 0;
    for lookup in lookups {
        let num_helper_columns = lookup.num_helper_columns(degree);
        let col_values = lookup
            .columns
            .iter()
            .map(|col| vec![col.eval_with_next_circuit(builder, local_values, next_values)])
            .collect::<Vec<_>>();

        for &challenge in &lookup_vars.challenges {
            let grand_challenge = GrandProductChallenge {
                beta: builder.one(),
                gamma: challenge,
            };

            eval_helper_columns_circuit(
                builder,
                &lookup.filter_columns,
                &col_values,
                local_values,
                next_values,
                &lookup_vars.local_values[start..start + num_helper_columns - 1],
                degree,
                &grand_challenge,
                yield_constr,
            );
            let challenge = builder.convert_to_ext(challenge);

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
