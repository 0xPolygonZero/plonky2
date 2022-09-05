use std::borrow::Borrow;
use std::iter::{once, repeat};
use std::marker::PhantomData;
use std::mem::size_of;

use itertools::Itertools;
use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::packed::PackedField;
use plonky2::field::polynomial::PolynomialValues;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::timed;
use plonky2::util::timing::TimingTree;
use plonky2_util::ceil_div_usize;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::kernel::keccak_util::keccakf_u32s;
use crate::cross_table_lookup::Column;
use crate::keccak_sponge::columns::*;
use crate::memory::segments::Segment;
use crate::stark::Stark;
use crate::util::trace_rows_to_poly_values;
use crate::vars::StarkEvaluationTargets;
use crate::vars::StarkEvaluationVars;

#[allow(unused)] // TODO: Should be used soon.
pub(crate) fn ctl_looked_data<F: Field>() -> Vec<Column<F>> {
    let cols = KECCAK_SPONGE_COL_MAP;
    let outputs = Column::singles(&cols.updated_state_u32s[..8]);
    Column::singles([
        cols.context,
        cols.segment,
        cols.virt,
        cols.timestamp,
        cols.len,
    ])
    .chain(outputs)
    .collect()
}

#[allow(unused)] // TODO: Should be used soon.
pub(crate) fn ctl_looking_keccak<F: Field>() -> Vec<Column<F>> {
    let cols = KECCAK_SPONGE_COL_MAP;
    Column::singles(
        [
            cols.original_rate_u32s.as_slice(),
            &cols.original_capacity_u32s,
            &cols.updated_state_u32s,
        ]
        .concat(),
    )
    .collect()
}

#[allow(unused)] // TODO: Should be used soon.
pub(crate) fn ctl_looking_memory<F: Field>(i: usize) -> Vec<Column<F>> {
    let cols = KECCAK_SPONGE_COL_MAP;

    let mut res = vec![Column::constant(F::ONE)]; // is_read

    res.extend(Column::singles([cols.context, cols.segment]));

    // The address of the byte being read is `virt + already_absorbed_bytes + i`.
    res.push(Column::linear_combination_with_constant(
        [(cols.virt, F::ONE), (cols.already_absorbed_bytes, F::ONE)],
        F::from_canonical_usize(i),
    ));

    // The i'th input byte being read.
    res.push(Column::single(cols.block_bytes[i]));

    // Since we're reading a single byte, the higher limbs must be zero.
    res.extend((1..8).map(|_| Column::zero()));

    res.push(Column::single(cols.timestamp));

    assert_eq!(
        res.len(),
        crate::memory::memory_stark::ctl_data::<F>().len()
    );
    res
}

/// CTL for performing the `i`th logic CTL. Since we need to do 136 byte XORs, and the logic CTL can
/// XOR 32 bytes per CTL, there are 5 such CTLs.
#[allow(unused)] // TODO: Should be used soon.
pub(crate) fn ctl_looking_logic<F: Field>(i: usize) -> Vec<Column<F>> {
    const U32S_PER_CTL: usize = 8;
    const U8S_PER_CTL: usize = 32;

    debug_assert!(i < ceil_div_usize(KECCAK_RATE_BYTES, U8S_PER_CTL));
    let cols = KECCAK_SPONGE_COL_MAP;

    let mut res = vec![
        Column::zero(), // is_and
        Column::zero(), // is_or
        Column::one(),  // is_xor
    ];

    // Input 0 contains some of the sponge's original rate chunks. If this is the last CTL, we won't
    // need to use all of the CTL's inputs, so we will pass some zeros.
    res.extend(
        Column::singles(&cols.original_rate_u32s[i * U32S_PER_CTL..])
            .chain(repeat(Column::zero()))
            .take(U32S_PER_CTL),
    );

    // Input 1 contains some of block's chunks. Again, for the last CTL it will include some zeros.
    res.extend(
        cols.block_bytes[i * U8S_PER_CTL..]
            .chunks(size_of::<u32>())
            .map(|chunk| Column::le_bytes(chunk))
            .chain(repeat(Column::zero()))
            .take(U8S_PER_CTL),
    );

    // The output contains the XOR'd rate part.
    res.extend(
        Column::singles(&cols.xored_rate_u32s[i * U32S_PER_CTL..])
            .chain(repeat(Column::zero()))
            .take(U32S_PER_CTL),
    );

    res
}

#[allow(unused)] // TODO: Should be used soon.
pub(crate) fn ctl_looked_filter<F: Field>() -> Column<F> {
    // The CPU table is only interested in our final-block rows, since those contain the final
    // sponge output.
    Column::single(KECCAK_SPONGE_COL_MAP.is_final_block)
}

#[allow(unused)] // TODO: Should be used soon.
/// CTL filter for reading the `i`th byte of input from memory.
pub(crate) fn ctl_looking_memory_filter<F: Field>(i: usize) -> Column<F> {
    // We perform the `i`th read if either
    // - this is a full input block, or
    // - this is a final block of length `i` or greater
    let cols = KECCAK_SPONGE_COL_MAP;
    Column::sum(once(&cols.is_full_input_block).chain(&cols.is_final_input_len[i..]))
}

/// Information about a Keccak sponge operation needed for witness generation.
#[derive(Debug)]
pub(crate) struct KeccakSpongeOp {
    // The address at which inputs are read.
    pub(crate) context: usize,
    pub(crate) segment: Segment,
    pub(crate) virt: usize,

    /// The timestamp at which inputs are read.
    pub(crate) timestamp: usize,

    /// The length of the input, in bytes.
    pub(crate) len: usize,

    /// The input that was read.
    pub(crate) input: Vec<u8>,
}

#[derive(Copy, Clone, Default)]
pub(crate) struct KeccakSpongeStark<F, const D: usize> {
    f: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> KeccakSpongeStark<F, D> {
    #[allow(unused)] // TODO: Should be used soon.
    pub(crate) fn generate_trace(
        &self,
        operations: Vec<KeccakSpongeOp>,
        min_rows: usize,
    ) -> Vec<PolynomialValues<F>> {
        let mut timing = TimingTree::new("generate trace", log::Level::Debug);

        // Generate the witness row-wise.
        let trace_rows = timed!(
            &mut timing,
            "generate trace rows",
            self.generate_trace_rows(operations, min_rows)
        );

        let trace_polys = timed!(
            &mut timing,
            "convert to PolynomialValues",
            trace_rows_to_poly_values(trace_rows)
        );

        timing.print();
        trace_polys
    }

    fn generate_trace_rows(
        &self,
        operations: Vec<KeccakSpongeOp>,
        min_rows: usize,
    ) -> Vec<[F; NUM_KECCAK_SPONGE_COLUMNS]> {
        let num_rows = operations.len().max(min_rows).next_power_of_two();
        operations
            .into_iter()
            .flat_map(|op| self.generate_rows_for_op(op))
            .chain(repeat(self.generate_padding_row()))
            .take(num_rows)
            .collect()
    }

    fn generate_rows_for_op(&self, op: KeccakSpongeOp) -> Vec<[F; NUM_KECCAK_SPONGE_COLUMNS]> {
        let mut rows = vec![];

        let mut sponge_state = [0u32; KECCAK_WIDTH_U32S];

        let mut input_blocks = op.input.chunks_exact(KECCAK_RATE_BYTES);
        let mut already_absorbed_bytes = 0;
        for block in input_blocks.by_ref() {
            let row = self.generate_full_input_row(
                &op,
                already_absorbed_bytes,
                sponge_state,
                block.try_into().unwrap(),
            );

            sponge_state = row.updated_state_u32s.map(|f| f.to_canonical_u64() as u32);

            rows.push(row.into());
            already_absorbed_bytes += KECCAK_RATE_BYTES;
        }

        rows.push(
            self.generate_final_row(
                &op,
                already_absorbed_bytes,
                sponge_state,
                input_blocks.remainder(),
            )
            .into(),
        );

        rows
    }

    fn generate_full_input_row(
        &self,
        op: &KeccakSpongeOp,
        already_absorbed_bytes: usize,
        sponge_state: [u32; KECCAK_WIDTH_U32S],
        block: [u8; KECCAK_RATE_BYTES],
    ) -> KeccakSpongeColumnsView<F> {
        let mut row = KeccakSpongeColumnsView {
            is_full_input_block: F::ONE,
            ..Default::default()
        };

        row.block_bytes = block.map(F::from_canonical_u8);

        Self::generate_common_fields(&mut row, op, already_absorbed_bytes, sponge_state);
        row
    }

    fn generate_final_row(
        &self,
        op: &KeccakSpongeOp,
        already_absorbed_bytes: usize,
        sponge_state: [u32; KECCAK_WIDTH_U32S],
        final_inputs: &[u8],
    ) -> KeccakSpongeColumnsView<F> {
        assert_eq!(already_absorbed_bytes + final_inputs.len(), op.len);

        let mut row = KeccakSpongeColumnsView {
            is_final_block: F::ONE,
            ..Default::default()
        };

        for (block_byte, input_byte) in row.block_bytes.iter_mut().zip(final_inputs) {
            *block_byte = F::from_canonical_u8(*input_byte);
        }

        // pad10*1 rule
        if final_inputs.len() == KECCAK_RATE_BYTES - 1 {
            // Both 1s are placed in the same byte.
            row.block_bytes[final_inputs.len()] = F::from_canonical_u8(0b10000001);
        } else {
            row.block_bytes[final_inputs.len()] = F::ONE;
            row.block_bytes[KECCAK_RATE_BYTES - 1] = F::from_canonical_u8(0b10000000);
        }

        row.is_final_input_len[final_inputs.len()] = F::ONE;

        Self::generate_common_fields(&mut row, op, already_absorbed_bytes, sponge_state);
        row
    }

    /// Generate fields that are common to both full-input-block rows and final-block rows.
    /// Also updates the sponge state with a single absorption.
    fn generate_common_fields(
        row: &mut KeccakSpongeColumnsView<F>,
        op: &KeccakSpongeOp,
        already_absorbed_bytes: usize,
        mut sponge_state: [u32; KECCAK_WIDTH_U32S],
    ) {
        row.context = F::from_canonical_usize(op.context);
        row.segment = F::from_canonical_usize(op.segment as usize);
        row.virt = F::from_canonical_usize(op.virt);
        row.timestamp = F::from_canonical_usize(op.timestamp);
        row.len = F::from_canonical_usize(op.len);
        row.already_absorbed_bytes = F::from_canonical_usize(already_absorbed_bytes);

        row.original_rate_u32s = sponge_state[..KECCAK_RATE_U32S]
            .iter()
            .map(|x| F::from_canonical_u32(*x))
            .collect_vec()
            .try_into()
            .unwrap();

        row.original_capacity_u32s = sponge_state[KECCAK_RATE_U32S..]
            .iter()
            .map(|x| F::from_canonical_u32(*x))
            .collect_vec()
            .try_into()
            .unwrap();

        let block_u32s = (0..KECCAK_RATE_U32S).map(|i| {
            u32::from_le_bytes(
                row.block_bytes[i * 4..(i + 1) * 4]
                    .iter()
                    .map(|x| x.to_canonical_u64() as u8)
                    .collect_vec()
                    .try_into()
                    .unwrap(),
            )
        });

        // xor in the block
        for (state_i, block_i) in sponge_state.iter_mut().zip(block_u32s) {
            *state_i ^= block_i;
        }
        let xored_rate_u32s: [u32; KECCAK_RATE_U32S] = sponge_state[..KECCAK_RATE_U32S]
            .to_vec()
            .try_into()
            .unwrap();
        row.xored_rate_u32s = xored_rate_u32s.map(F::from_canonical_u32);

        keccakf_u32s(&mut sponge_state);
        row.updated_state_u32s = sponge_state.map(F::from_canonical_u32);
    }

    fn generate_padding_row(&self) -> [F; NUM_KECCAK_SPONGE_COLUMNS] {
        // The default instance has is_full_input_block = is_final_block = 0,
        // indicating that it's a dummy/padding row.
        KeccakSpongeColumnsView::default().into()
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Stark<F, D> for KeccakSpongeStark<F, D> {
    const COLUMNS: usize = NUM_KECCAK_SPONGE_COLUMNS;

    fn eval_packed_generic<FE, P, const D2: usize>(
        &self,
        vars: StarkEvaluationVars<FE, P, { Self::COLUMNS }>,
        _yield_constr: &mut ConstraintConsumer<P>,
    ) where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>,
    {
        let _local_values: &KeccakSpongeColumnsView<P> = vars.local_values.borrow();

        // TODO: Each flag (full-input block, final block or implied dummy flag) must be boolean.
        // TODO: before_rate_bits, block_bits and is_final_input_len must contain booleans.

        // TODO: Sum of is_final_input_len should equal is_final_block (which will be 0 or 1).

        // TODO: If this is the first row, the original sponge state should be 0 and already_absorbed_bytes = 0.
        // TODO: If this is a final block, the next row's original sponge state should be 0 and already_absorbed_bytes = 0.

        // TODO: If this is a full-input block, the next row's address, time and len must match.
        // TODO: If this is a full-input block, the next row's "before" should match our "after" state.
        // TODO: If this is a full-input block, the next row's already_absorbed_bytes should be ours plus 136.

        // TODO: A dummy row is always followed by another dummy row, so the prover can't put dummy rows "in between" to avoid the above checks.

        // TODO: is_final_input_len implies `len - already_absorbed == i`.
    }

    fn eval_ext_circuit(
        &self,
        _builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
        vars: StarkEvaluationTargets<D, { Self::COLUMNS }>,
        _yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    ) {
        let _local_values: &KeccakSpongeColumnsView<ExtensionTarget<D>> =
            vars.local_values.borrow();

        // TODO
    }

    fn constraint_degree(&self) -> usize {
        3
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Borrow;

    use anyhow::Result;
    use itertools::Itertools;
    use keccak_hash::keccak;
    use plonky2::field::goldilocks_field::GoldilocksField;
    use plonky2::field::types::PrimeField64;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};

    use crate::keccak_sponge::columns::KeccakSpongeColumnsView;
    use crate::keccak_sponge::keccak_sponge_stark::{KeccakSpongeOp, KeccakSpongeStark};
    use crate::memory::segments::Segment;
    use crate::stark_testing::{test_stark_circuit_constraints, test_stark_low_degree};

    #[test]
    fn test_stark_degree() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type S = KeccakSpongeStark<F, D>;

        let stark = S::default();
        test_stark_low_degree(stark)
    }

    #[test]
    fn test_stark_circuit() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type S = KeccakSpongeStark<F, D>;

        let stark = S::default();
        test_stark_circuit_constraints::<F, C, S, D>(stark)
    }

    #[test]
    fn test_generation() -> Result<()> {
        const D: usize = 2;
        type F = GoldilocksField;
        type S = KeccakSpongeStark<F, D>;

        let input = vec![1, 2, 3];
        let expected_output = keccak(&input);

        let op = KeccakSpongeOp {
            context: 0,
            segment: Segment::Code,
            virt: 0,
            timestamp: 0,
            len: input.len(),
            input,
        };
        let stark = S::default();
        let rows = stark.generate_rows_for_op(op);
        assert_eq!(rows.len(), 1);
        let last_row: &KeccakSpongeColumnsView<F> = rows.last().unwrap().borrow();
        let output = last_row.updated_state_u32s[..8]
            .iter()
            .flat_map(|x| (x.to_canonical_u64() as u32).to_le_bytes())
            .collect_vec();

        assert_eq!(output, expected_output.0);
        Ok(())
    }
}
