#[cfg(not(feature = "std"))]
use alloc::{borrow::ToOwned, vec};

use crate::field::extension::Extendable;
use crate::gates::lookup::LookupGate;
use crate::gates::lookup_table::{LookupTable, LookupTableGate};
use crate::gates::noop::NoopGate;
use crate::hash::hash_types::RichField;
use crate::iop::target::Target;
use crate::plonk::circuit_builder::CircuitBuilder;

/// Lookup tables used in the tests and benchmarks.
///
/// The following table was taken from the Tip5 paper.
pub const TIP5_TABLE: [u16; 256] = [
    0, 7, 26, 63, 124, 215, 85, 254, 214, 228, 45, 185, 140, 173, 33, 240, 29, 177, 176, 32, 8,
    110, 87, 202, 204, 99, 150, 106, 230, 14, 235, 128, 213, 239, 212, 138, 23, 130, 208, 6, 44,
    71, 93, 116, 146, 189, 251, 81, 199, 97, 38, 28, 73, 179, 95, 84, 152, 48, 35, 119, 49, 88,
    242, 3, 148, 169, 72, 120, 62, 161, 166, 83, 175, 191, 137, 19, 100, 129, 112, 55, 221, 102,
    218, 61, 151, 237, 68, 164, 17, 147, 46, 234, 203, 216, 22, 141, 65, 57, 123, 12, 244, 54, 219,
    231, 96, 77, 180, 154, 5, 253, 133, 165, 98, 195, 205, 134, 245, 30, 9, 188, 59, 142, 186, 197,
    181, 144, 92, 31, 224, 163, 111, 74, 58, 69, 113, 196, 67, 246, 225, 10, 121, 50, 60, 157, 90,
    122, 2, 250, 101, 75, 178, 159, 24, 36, 201, 11, 243, 132, 198, 190, 114, 233, 39, 52, 21, 209,
    108, 238, 91, 187, 18, 104, 194, 37, 153, 34, 200, 143, 126, 155, 236, 118, 64, 80, 172, 89,
    94, 193, 135, 183, 86, 107, 252, 13, 167, 206, 136, 220, 207, 103, 171, 160, 76, 182, 227, 217,
    158, 56, 174, 4, 66, 109, 139, 162, 184, 211, 249, 47, 125, 232, 117, 43, 16, 42, 127, 20, 241,
    25, 149, 105, 156, 51, 53, 168, 145, 247, 223, 79, 78, 226, 15, 222, 82, 115, 70, 210, 27, 41,
    1, 170, 40, 131, 192, 229, 248, 255,
];

/// This is a table with 256 arbitrary values.
pub const OTHER_TABLE: [u16; 256] = [
    2, 6, 25, 3, 9, 7, 0, 3, 25, 35, 10, 19, 36, 45, 216, 247, 35, 39, 57, 126, 2, 6, 25, 3, 9, 7,
    0, 3, 25, 35, 10, 19, 36, 45, 216, 247, 35, 39, 57, 126, 2, 6, 25, 3, 9, 7, 0, 3, 25, 35, 10,
    19, 36, 45, 216, 247, 35, 39, 57, 126, 2, 6, 25, 3, 9, 7, 0, 3, 25, 35, 10, 19, 36, 45, 216,
    247, 35, 39, 57, 126, 2, 6, 25, 3, 9, 7, 0, 3, 25, 35, 10, 19, 36, 45, 216, 247, 35, 39, 57,
    126, 2, 6, 25, 3, 9, 7, 0, 3, 25, 35, 10, 19, 36, 45, 216, 247, 35, 39, 57, 126, 2, 6, 25, 3,
    9, 7, 0, 3, 25, 35, 10, 19, 36, 45, 216, 247, 35, 39, 57, 126, 2, 6, 25, 3, 9, 7, 0, 3, 25, 35,
    10, 19, 36, 45, 216, 247, 35, 39, 57, 126, 2, 6, 25, 3, 9, 7, 0, 3, 25, 35, 10, 19, 36, 45,
    216, 247, 35, 39, 57, 126, 2, 6, 25, 3, 9, 7, 0, 3, 25, 35, 10, 19, 36, 45, 216, 247, 35, 39,
    57, 126, 2, 6, 25, 3, 9, 7, 0, 3, 25, 35, 10, 19, 36, 45, 216, 247, 35, 39, 57, 126, 2, 6, 25,
    3, 9, 7, 0, 3, 25, 35, 10, 19, 36, 45, 216, 247, 35, 39, 57, 126, 2, 6, 25, 3, 9, 7, 0, 3, 25,
    35, 10, 19, 36, 45, 216, 247,
];

/// This is a smaller lookup table with arbitrary values.
pub const SMALLER_TABLE: [u16; 8] = [2, 24, 56, 100, 128, 16, 20, 49];

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    /// Adds a lookup table to the list of stored lookup tables `self.luts` based on a table of (input, output) pairs. It returns the index of the LUT within `self.luts`.
    pub fn add_lookup_table_from_pairs(&mut self, table: LookupTable) -> usize {
        self.update_luts_from_pairs(table)
    }

    /// Adds a lookup table to the list of stored lookup tables `self.luts` based on a table, represented as a slice `&[u16]` of inputs and a slice `&[u16]` of outputs. It returns the index of the LUT within `self.luts`.
    pub fn add_lookup_table_from_table(&mut self, inps: &[u16], outs: &[u16]) -> usize {
        self.update_luts_from_table(inps, outs)
    }

    /// Adds a lookup table to the list of stored lookup tables `self.luts` based on a function. It returns the index of the LUT within `self.luts`.
    pub fn add_lookup_table_from_fn(&mut self, f: fn(u16) -> u16, inputs: &[u16]) -> usize {
        self.update_luts_from_fn(f, inputs)
    }

    /// Adds a lookup (input, output) pair to the stored lookups. Takes a `Target` input and returns a `Target` output.
    pub fn add_lookup_from_index(&mut self, looking_in: Target, lut_index: usize) -> Target {
        assert!(
            lut_index < self.get_luts_length(),
            "lut number {} not in luts (length = {})",
            lut_index,
            self.get_luts_length()
        );
        let looking_out = self.add_virtual_target();
        self.update_lookups(looking_in, looking_out, lut_index);
        looking_out
    }

    /// We call this function at the end of circuit building right before the PI gate to add all `LookupTableGate` and `LookupGate`.
    /// It also updates `self.lookup_rows` accordingly.
    pub fn add_all_lookups(&mut self) {
        for lut_index in 0..self.num_luts() {
            assert!(
                !self.get_lut_lookups(lut_index).is_empty(),
                "LUT number {:?} is unused",
                lut_index
            );
            if !self.get_lut_lookups(lut_index).is_empty() {
                // Create LU gates. Connect them to the stored lookups.
                let last_lu_gate = self.num_gates();

                let lut = self.get_lut(lut_index);

                let lookups = self.get_lut_lookups(lut_index).to_owned();

                let gate = LookupGate::new_from_table(&self.config, lut.clone());
                let num_slots = LookupGate::num_slots(&self.config);

                // Given the number of lookups and the number of slots for each gate, it is possible
                // to compute the number of gates that will employ all their slots; such gates can
                // can be instantiated with `add_gate` rather than being instantiated slot by slot

                // lookup_iter will iterate over the lookups that can be placed in fully utilized
                // gates, splitting them in chunks that can be placed in the same `LookupGate`
                let lookup_iter = lookups.chunks_exact(num_slots);
                // `last_chunk` will contain the remainder of lookups, which cannot fill all the
                // slots of a `LookupGate`; this last chunk will be processed by incrementally
                // filling slots, to avoid that the `LookupGenerator` is run on unused slots
                let last_chunk = lookup_iter.remainder();
                // handle chunks that can fill all the slots of a `LookupGate`
                lookup_iter.for_each(|chunk| {
                    let row = self.add_gate(gate.clone(), vec![]);
                    for (i, (looking_in, looking_out)) in chunk.iter().enumerate() {
                        let gate_in = Target::wire(row, LookupGate::wire_ith_looking_inp(i));
                        let gate_out = Target::wire(row, LookupGate::wire_ith_looking_out(i));
                        self.connect(gate_in, *looking_in);
                        self.connect(gate_out, *looking_out);
                    }
                });
                // deal with the last chunk
                for (looking_in, looking_out) in last_chunk.iter() {
                    let (gate, i) =
                        self.find_slot(gate.clone(), &[F::from_canonical_usize(lut_index)], &[]);
                    let gate_in = Target::wire(gate, LookupGate::wire_ith_looking_inp(i));
                    let gate_out = Target::wire(gate, LookupGate::wire_ith_looking_out(i));
                    self.connect(gate_in, *looking_in);
                    self.connect(gate_out, *looking_out);
                }

                // Create LUT gates. Nothing is connected to them.
                let last_lut_gate = self.num_gates();
                let num_lut_entries = LookupTableGate::num_slots(&self.config);
                let num_lut_rows = (self.get_luts_idx_length(lut_index) - 1) / num_lut_entries + 1;
                let gate =
                    LookupTableGate::new_from_table(&self.config, lut.clone(), last_lut_gate);
                // Also instances of `LookupTableGate` can be placed with the `add_gate` function
                // rather than being instantiated slot by slot; note that in this case there is no
                // need to separately handle the last chunk of LUT entries that cannot fill all the
                // slots of a `LookupTableGate`, as the generator already handles empty slots
                for _ in 0..num_lut_rows {
                    self.add_gate(gate.clone(), vec![]);
                }

                let first_lut_gate = self.num_gates() - 1;

                // Will ensure the next row's wires will be all zeros. With this, there is no distinction between the transition constraints on the first row
                // and on the other rows. Additionally, initial constraints become a simple zero check.
                self.add_gate(NoopGate, vec![]);

                // These elements are increasing: the gate rows are deliberately upside down.
                // This is necessary for constraint evaluation so that you do not need values of the next
                // row's wires, which aren't provided in the evaluation variables.
                self.add_lookup_rows(last_lu_gate, last_lut_gate, first_lut_gate);
            }
        }
    }
}
