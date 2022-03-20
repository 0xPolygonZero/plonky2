use plonky2::field::field_types::Field;
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::field::polynomial::PolynomialValues;
use plonky2::timed;
use plonky2::util::timing::TimingTree;
use plonky2::util::transpose;

use crate::alu::generate_alu;
use crate::core_registers::{generate_first_row_core_registers, generate_next_row_core_registers};
use crate::env::environment::Environment;
use crate::lookup::generate_lookups;
use crate::memory::TransactionMemory;
use crate::permutation_unit::generate_permutation_unit;
use crate::registers::NUM_COLUMNS;

/// Some logic in System Zero only works with the Goldilocks field, so we don't want this code to
/// be generic w.r.t. field choice.
type F = GoldilocksField;

/// We require at least 2^16 rows as it helps support efficient 16-bit range checks.
const MIN_TRACE_ROWS: usize = 1 << 16;

pub fn generate_trace<E: Environment>(env: &mut E) -> Vec<PolynomialValues<F>> {
    let mut timing = TimingTree::new("generate trace", log::Level::Debug);

    // Generate the witness, except for permuted columns in the lookup argument.
    let trace_rows = timed!(&mut timing, "generate trace rows", generate_trace_rows(env));

    // Transpose from row-wise to column-wise.
    let trace_row_vecs: Vec<_> = timed!(
        &mut timing,
        "convert to Vecs",
        trace_rows.into_iter().map(|row| row.to_vec()).collect()
    );
    let mut trace_col_vecs: Vec<Vec<F>> =
        timed!(&mut timing, "transpose", transpose(&trace_row_vecs));

    // Generate permuted columns in the lookup argument.
    timed!(
        &mut timing,
        "generate lookup columns",
        generate_lookups(&mut trace_col_vecs)
    );

    let trace_polys = timed!(
        &mut timing,
        "convert to PolynomialValues",
        trace_col_vecs
            .into_iter()
            .map(PolynomialValues::new)
            .collect()
    );

    timing.print();
    trace_polys
}

/// Generate the rows of the trace. Note that this does not generate the permuted columns used
/// in our lookup arguments, as those are computed after transposing to column-wise form.
fn generate_trace_rows<E: Environment>(env: &mut E) -> Vec<[F; NUM_COLUMNS]> {
    let memory = TransactionMemory::default();

    let mut row = [F::ZERO; NUM_COLUMNS];
    generate_first_row_core_registers(&mut row);
    generate_alu(&mut row);
    generate_permutation_unit(&mut row);

    let mut trace = Vec::with_capacity(MIN_TRACE_ROWS);

    loop {
        let mut next_row = [F::ZERO; NUM_COLUMNS];
        generate_next_row_core_registers(&row, &mut next_row);
        generate_alu(&mut next_row);
        generate_permutation_unit(&mut next_row);

        trace.push(row);
        row = next_row;

        // TODO: Replace with proper termination condition.
        if trace.len() == (1 << 16) - 1 {
            break;
        }
    }

    trace.push(row);
    trace
}
