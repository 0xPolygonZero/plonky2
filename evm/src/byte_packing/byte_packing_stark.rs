//! This crate enforces the correctness of reading and writing sequences
//! of bytes in Big-Endian ordering from and to the memory.
//!
//! The trace layout consists in one row for an `N` byte sequence (where 32 ≥ `N` > 0).
//!
//! At each row the `i`-th byte flag will be activated to indicate a sequence of
//! length i+1.
//!
//! The length of a sequence can be retrieved for CTLs as:
//!
//!    sequence_length = \sum_{i=0}^31 b[i] * (i + 1)
//!
//! where b[i] is the `i`-th byte flag.
//!
//! Because of the discrepancy in endianness between the different tables, the byte sequences
//! are actually written in the trace in reverse order from the order they are provided.
//! We only store the virtual address `virt` of the first byte, and the virtual address for byte `i`
//! can be recovered as:
//!     virt_i = virt + sequence_length - 1 - i
//!
//! Note that, when writing a sequence of bytes to memory, both the `U256` value and the
//! corresponding sequence length are being read from the stack. Because of the endianness
//! discrepancy mentioned above, we first convert the value to a byte sequence in Little-Endian,
//! then resize the sequence to prune unneeded zeros before reverting the sequence order.
//! This means that the higher-order bytes will be thrown away during the process, if the value
//! is greater than 256^length, and as a result a different value will be stored in memory.

use std::marker::PhantomData;

use itertools::Itertools;
use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::packed::PackedField;
use plonky2::field::polynomial::PolynomialValues;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::timed;
use plonky2::util::timing::TimingTree;
use plonky2::util::transpose;

use super::columns::BYTE_VALUES_RANGE;
use super::NUM_BYTES;
use crate::byte_packing::columns::{
    index_len, value_bytes, ADDR_CONTEXT, ADDR_SEGMENT, ADDR_VIRTUAL, IS_READ, LEN_INDICES_COLS,
    NUM_COLUMNS, RANGE_COUNTER, RC_FREQUENCIES, TIMESTAMP,
};
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cross_table_lookup::{Column, Filter};
use crate::evaluation_frame::{StarkEvaluationFrame, StarkFrame};
use crate::lookup::Lookup;
use crate::stark::Stark;
use crate::witness::memory::MemoryAddress;

/// Strict upper bound for the individual bytes range-check.
const BYTE_RANGE_MAX: usize = 1usize << 8;

/// Creates the vector of `Columns` for `BytePackingStark` corresponding to the final packed limbs being read/written.
/// `CpuStark` will look into these columns, as the CPU needs the output of byte packing.
pub(crate) fn ctl_looked_data<F: Field>() -> Vec<Column<F>> {
    // Reconstruct the u32 limbs composing the final `U256` word
    // being read/written from the underlying byte values. For each,
    // we pack 4 consecutive bytes and shift them accordingly to
    // obtain the corresponding limb.
    let outputs: Vec<Column<F>> = (0..8)
        .map(|i| {
            let range = (value_bytes(i * 4)..value_bytes(i * 4) + 4);
            Column::linear_combination(
                range
                    .enumerate()
                    .map(|(j, c)| (c, F::from_canonical_u64(1 << (8 * j)))),
            )
        })
        .collect();

    let sequence_len: Column<F> = Column::linear_combination(
        (0..NUM_BYTES).map(|i| (index_len(i), F::from_canonical_usize(i + 1))),
    );

    Column::singles([IS_READ, ADDR_CONTEXT, ADDR_SEGMENT, ADDR_VIRTUAL])
        .chain([sequence_len])
        .chain(Column::singles(&[TIMESTAMP]))
        .chain(outputs)
        .collect()
}

/// CTL filter for the `BytePackingStark` looked table.
pub(crate) fn ctl_looked_filter<F: Field>() -> Filter<F> {
    // The CPU table is only interested in our sequence end rows,
    // since those contain the final limbs of our packed int.
    Filter::new_simple(Column::sum((0..NUM_BYTES).map(index_len)))
}

/// Column linear combination for the `BytePackingStark` table reading/writing the `i`th byte sequence from `MemoryStark`.
pub(crate) fn ctl_looking_memory<F: Field>(i: usize) -> Vec<Column<F>> {
    let mut res = Column::singles([IS_READ, ADDR_CONTEXT, ADDR_SEGMENT]).collect_vec();

    // Compute the virtual address: `ADDR_VIRTUAL` + `sequence_len` - 1 - i.
    let sequence_len_minus_one = (0..NUM_BYTES)
        .map(|j| (index_len(j), F::from_canonical_usize(j)))
        .collect::<Vec<_>>();
    let mut addr_virt_cols = vec![(ADDR_VIRTUAL, F::ONE)];
    addr_virt_cols.extend(sequence_len_minus_one);
    let addr_virt = Column::linear_combination_with_constant(
        addr_virt_cols,
        F::NEG_ONE * F::from_canonical_usize(i),
    );

    res.push(addr_virt);

    // The i'th input byte being read/written.
    res.push(Column::single(value_bytes(i)));

    // Since we're reading a single byte, the higher limbs must be zero.
    res.extend((1..8).map(|_| Column::zero()));

    res.push(Column::single(TIMESTAMP));

    res
}

/// CTL filter for reading/writing the `i`th byte of the byte sequence from/to memory.
pub(crate) fn ctl_looking_memory_filter<F: Field>(i: usize) -> Filter<F> {
    Filter::new_simple(Column::sum((i..NUM_BYTES).map(index_len)))
}

/// Information about a byte packing operation needed for witness generation.
#[derive(Clone, Debug)]
pub(crate) struct BytePackingOp {
    /// Whether this is a read (packing) or write (unpacking) operation.
    pub(crate) is_read: bool,

    /// The base address at which inputs are read/written.
    pub(crate) base_address: MemoryAddress,

    /// The timestamp at which inputs are read/written.
    pub(crate) timestamp: usize,

    /// The byte sequence that was read/written.
    /// Its length is required to be at most 32.
    pub(crate) bytes: Vec<u8>,
}

#[derive(Copy, Clone, Default)]
pub(crate) struct BytePackingStark<F, const D: usize> {
    pub(crate) f: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> BytePackingStark<F, D> {
    pub(crate) fn generate_trace(
        &self,
        ops: Vec<BytePackingOp>,
        min_rows: usize,
        timing: &mut TimingTree,
    ) -> Vec<PolynomialValues<F>> {
        // Generate most of the trace in row-major form.
        let trace_rows = timed!(
            timing,
            "generate trace rows",
            self.generate_trace_rows(ops, min_rows)
        );
        let trace_row_vecs: Vec<_> = trace_rows.into_iter().map(|row| row.to_vec()).collect();

        let mut trace_cols = transpose(&trace_row_vecs);
        self.generate_range_checks(&mut trace_cols);

        trace_cols.into_iter().map(PolynomialValues::new).collect()
    }

    fn generate_trace_rows(
        &self,
        ops: Vec<BytePackingOp>,
        min_rows: usize,
    ) -> Vec<[F; NUM_COLUMNS]> {
        let base_len: usize = ops.iter().map(|op| usize::from(!op.bytes.is_empty())).sum();
        let num_rows = core::cmp::max(base_len.max(BYTE_RANGE_MAX), min_rows).next_power_of_two();
        let mut rows = Vec::with_capacity(num_rows);

        for op in ops {
            if !op.bytes.is_empty() {
                rows.push(self.generate_row_for_op(op));
            }
        }

        for _ in rows.len()..num_rows {
            rows.push(self.generate_padding_row());
        }

        rows
    }

    fn generate_row_for_op(&self, op: BytePackingOp) -> [F; NUM_COLUMNS] {
        let BytePackingOp {
            is_read,
            base_address,
            timestamp,
            bytes,
        } = op;

        let MemoryAddress {
            context,
            segment,
            virt,
        } = base_address;

        let mut row = [F::ZERO; NUM_COLUMNS];
        row[IS_READ] = F::from_bool(is_read);

        row[ADDR_CONTEXT] = F::from_canonical_usize(context);
        row[ADDR_SEGMENT] = F::from_canonical_usize(segment);
        // We store the initial virtual segment. But the CTLs,
        // we start with virt + sequence_len - 1.
        row[ADDR_VIRTUAL] = F::from_canonical_usize(virt);

        row[TIMESTAMP] = F::from_canonical_usize(timestamp);

        row[index_len(bytes.len() - 1)] = F::ONE;

        for (i, &byte) in bytes.iter().rev().enumerate() {
            row[value_bytes(i)] = F::from_canonical_u8(byte);
        }

        row
    }

    const fn generate_padding_row(&self) -> [F; NUM_COLUMNS] {
        [F::ZERO; NUM_COLUMNS]
    }

    /// Expects input in *column*-major layout
    fn generate_range_checks(&self, cols: &mut [Vec<F>]) {
        debug_assert!(cols.len() == NUM_COLUMNS);

        let n_rows = cols[0].len();
        debug_assert!(cols.iter().all(|col| col.len() == n_rows));

        for i in 0..BYTE_RANGE_MAX {
            cols[RANGE_COUNTER][i] = F::from_canonical_usize(i);
        }
        for i in BYTE_RANGE_MAX..n_rows {
            cols[RANGE_COUNTER][i] = F::from_canonical_usize(BYTE_RANGE_MAX - 1);
        }

        // For each column c in cols, generate the range-check
        // permutations and put them in the corresponding range-check
        // columns rc_c and rc_c+1.
        for col in 0..NUM_BYTES {
            for i in 0..n_rows {
                let c = value_bytes(col);
                let x = cols[c][i].to_canonical_u64() as usize;
                assert!(
                    x < BYTE_RANGE_MAX,
                    "column value {} exceeds the max range value {}",
                    x,
                    BYTE_RANGE_MAX
                );
                cols[RC_FREQUENCIES][x] += F::ONE;
            }
        }
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Stark<F, D> for BytePackingStark<F, D> {
    type EvaluationFrame<FE, P, const D2: usize> = StarkFrame<P, NUM_COLUMNS>
    where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>;

    type EvaluationFrameTarget = StarkFrame<ExtensionTarget<D>, NUM_COLUMNS>;

    fn eval_packed_generic<FE, P, const D2: usize>(
        &self,
        vars: &Self::EvaluationFrame<FE, P, D2>,
        yield_constr: &mut ConstraintConsumer<P>,
    ) where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>,
    {
        let local_values: &[P; NUM_COLUMNS] = vars.get_local_values().try_into().unwrap();
        let next_values: &[P; NUM_COLUMNS] = vars.get_next_values().try_into().unwrap();

        // Check the range column: First value must be 0, last row
        // must be 255, and intermediate rows must increment by 0
        // or 1.
        let rc1 = local_values[RANGE_COUNTER];
        let rc2 = next_values[RANGE_COUNTER];
        yield_constr.constraint_first_row(rc1);
        let incr = rc2 - rc1;
        yield_constr.constraint_transition(incr * incr - incr);
        let range_max = P::Scalar::from_canonical_u64((BYTE_RANGE_MAX - 1) as u64);
        yield_constr.constraint_last_row(rc1 - range_max);

        let one = P::ONES;

        // We filter active columns by summing all the byte indices.
        // Constraining each of them to be boolean is done later on below.
        let current_filter = local_values[LEN_INDICES_COLS].iter().copied().sum::<P>();
        yield_constr.constraint(current_filter * (current_filter - one));

        // The filter column must start by one.
        yield_constr.constraint_first_row(current_filter - one);

        // The is_read flag must be boolean.
        let current_is_read = local_values[IS_READ];
        yield_constr.constraint(current_is_read * (current_is_read - one));

        // Each byte index must be boolean.
        for i in 0..NUM_BYTES {
            let idx_i = local_values[index_len(i)];
            yield_constr.constraint(idx_i * (idx_i - one));
        }

        // Only padding rows have their filter turned off.
        let next_filter = next_values[LEN_INDICES_COLS].iter().copied().sum::<P>();
        yield_constr.constraint_transition(next_filter * (next_filter - current_filter));

        // Check that all limbs after final length are 0.
        for i in 0..NUM_BYTES - 1 {
            // If the length is i+1, then value_bytes(i+1),...,value_bytes(NUM_BYTES-1) must be 0.
            for j in i + 1..NUM_BYTES {
                yield_constr.constraint(local_values[index_len(i)] * local_values[value_bytes(j)]);
            }
        }
    }

    fn eval_ext_circuit(
        &self,
        builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
        vars: &Self::EvaluationFrameTarget,
        yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    ) {
        let local_values: &[ExtensionTarget<D>; NUM_COLUMNS] =
            vars.get_local_values().try_into().unwrap();
        let next_values: &[ExtensionTarget<D>; NUM_COLUMNS] =
            vars.get_next_values().try_into().unwrap();

        // Check the range column: First value must be 0, last row
        // must be 255, and intermediate rows must increment by 0
        // or 1.
        let rc1 = local_values[RANGE_COUNTER];
        let rc2 = next_values[RANGE_COUNTER];
        yield_constr.constraint_first_row(builder, rc1);
        let incr = builder.sub_extension(rc2, rc1);
        let t = builder.mul_sub_extension(incr, incr, incr);
        yield_constr.constraint_transition(builder, t);
        let range_max =
            builder.constant_extension(F::Extension::from_canonical_usize(BYTE_RANGE_MAX - 1));
        let t = builder.sub_extension(rc1, range_max);
        yield_constr.constraint_last_row(builder, t);

        // We filter active columns by summing all the byte indices.
        // Constraining each of them to be boolean is done later on below.
        let current_filter = builder.add_many_extension(&local_values[LEN_INDICES_COLS]);
        let constraint = builder.mul_sub_extension(current_filter, current_filter, current_filter);
        yield_constr.constraint(builder, constraint);

        // The filter column must start by one.
        let constraint = builder.add_const_extension(current_filter, F::NEG_ONE);
        yield_constr.constraint_first_row(builder, constraint);

        // The is_read flag must be boolean.
        let current_is_read = local_values[IS_READ];
        let constraint =
            builder.mul_sub_extension(current_is_read, current_is_read, current_is_read);
        yield_constr.constraint(builder, constraint);

        // Each byte index must be boolean.
        for i in 0..NUM_BYTES {
            let idx_i = local_values[index_len(i)];
            let constraint = builder.mul_sub_extension(idx_i, idx_i, idx_i);
            yield_constr.constraint(builder, constraint);
        }

        // Only padding rows have their filter turned off.
        let next_filter = builder.add_many_extension(&next_values[LEN_INDICES_COLS]);
        let constraint = builder.sub_extension(next_filter, current_filter);
        let constraint = builder.mul_extension(next_filter, constraint);
        yield_constr.constraint_transition(builder, constraint);

        // Check that all limbs after final length are 0.
        for i in 0..NUM_BYTES - 1 {
            // If the length is i+1, then value_bytes(i+1),...,value_bytes(NUM_BYTES-1) must be 0.
            for j in i + 1..NUM_BYTES {
                let constr =
                    builder.mul_extension(local_values[index_len(i)], local_values[value_bytes(j)]);
                yield_constr.constraint(builder, constr);
            }
        }
    }

    fn constraint_degree(&self) -> usize {
        3
    }

    fn lookups(&self) -> Vec<Lookup<F>> {
        vec![Lookup {
            columns: Column::singles(value_bytes(0)..value_bytes(0) + NUM_BYTES).collect(),
            table_column: Column::single(RANGE_COUNTER),
            frequencies_column: Column::single(RC_FREQUENCIES),
            filter_columns: vec![None; NUM_BYTES],
        }]
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use anyhow::Result;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};

    use crate::byte_packing::byte_packing_stark::BytePackingStark;
    use crate::stark_testing::{test_stark_circuit_constraints, test_stark_low_degree};

    #[test]
    fn test_stark_degree() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type S = BytePackingStark<F, D>;

        let stark = S {
            f: Default::default(),
        };
        test_stark_low_degree(stark)
    }

    #[test]
    fn test_stark_circuit() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type S = BytePackingStark<F, D>;

        let stark = S {
            f: Default::default(),
        };
        test_stark_circuit_constraints::<F, C, S, D>(stark)
    }
}
