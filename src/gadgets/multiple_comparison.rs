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
            "Comparison must be between same number of inputs and outputs"
        );
        let n = a.len();

        let chunk_bits = 2;
        let num_chunks = ceil_div_usize(num_bits, chunk_bits);

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

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use rand::Rng;

    use crate::field::crandall_field::CrandallField;
    use crate::field::field_types::Field;
    use crate::iop::witness::PartialWitness;
    use crate::plonk::circuit_builder::CircuitBuilder;
    use crate::plonk::circuit_data::CircuitConfig;
    use crate::plonk::verifier::verify;

    fn test_list_le(size: usize) -> Result<()> {
        type F = CrandallField;
        let config = CircuitConfig::large_config();
        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, 4>::new(config);

        let mut rng = rand::thread_rng();

        let lst1: Vec<u32> = (0..size)
            .map(|_| rng.gen())
            .collect();
        let lst2: Vec<u32> = (0..size)
            .map(|i| {
                let mut res = rng.gen();
                while res < lst1[i] {
                    res = rng.gen();
                }
                res
            })
            .collect();
        let a = lst1.iter().map(|&x| builder.constant(F::from_canonical_u32(x))).collect();
        let b = lst2.iter().map(|&x| builder.constant(F::from_canonical_u32(x))).collect();

        let result = builder.list_le(a, b, 32);

        let expected_result = builder.constant_bool(true);
        builder.connect(result.target, expected_result.target);

        let data = builder.build();
        let proof = data.prove(pw).unwrap();
        verify(proof, &data.verifier_only, &data.common)
    }

    #[test]
    fn test_multiple_comparison_trivial() -> Result<()> {
        test_list_le(1)
    }
}
