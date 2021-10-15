use crate::field::extension_field::Extendable;
use crate::field::field_types::RichField;
use crate::gates::comparison::ComparisonGate;
use crate::iop::target::{BoolTarget, Target};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::util::ceil_div_usize;

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    /// Returns true if a is less than or equal to b, considered as limbs of a large value.
    pub fn list_le(&mut self, a: Vec<Target>, b: Vec<Target>, num_bits: usize) -> BoolTarget {
        assert_eq!(
            a.len(),
            b.len(),
            "Permutation must have same number of inputs and outputs"
        );
        let n = a.len();

        let chunk_size = 4;
        let num_chunks = ceil_div_usize(num_bits, chunk_size);

        let one = self.one();
        let mut result = self.one();
        for i in 0..n {
            let a_le_b_gate = ComparisonGate::new(num_bits, num_chunks);
            let a_le_b_gate_index = self.add_gate(a_le_b_gate.clone(), vec![]);
            self.connect(
                Target::wire(a_le_b_gate_index, a_le_b_gate.wire_first_input()),
                a[i],
            );
            self.connect(
                Target::wire(a_le_b_gate_index, a_le_b_gate.wire_second_input()),
                b[i],
            );
            let a_le_b_result = Target::wire(a_le_b_gate_index, a_le_b_gate.wire_result_bool());

            let b_le_a_gate = ComparisonGate::new(num_bits, num_chunks);
            let b_le_a_gate_index = self.add_gate(b_le_a_gate.clone(), vec![]);
            self.connect(
                Target::wire(b_le_a_gate_index, b_le_a_gate.wire_first_input()),
                b[i],
            );
            self.connect(
                Target::wire(b_le_a_gate_index, b_le_a_gate.wire_second_input()),
                a[i],
            );
            let b_le_a_result = Target::wire(b_le_a_gate_index, b_le_a_gate.wire_result_bool());

            let these_limbs_equal = self.mul(a_le_b_result, b_le_a_result);
            let these_limbs_less_than = self.sub(one, b_le_a_result);
            result = self.mul_add(these_limbs_equal, result, these_limbs_less_than);
        }

        BoolTarget::new_unsafe(result)
    }
}
