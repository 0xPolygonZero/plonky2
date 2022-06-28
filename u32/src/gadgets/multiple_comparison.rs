use plonky2::hash::hash_types::RichField;
use plonky2::iop::target::{BoolTarget, Target};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2_field::extension::Extendable;
use plonky2_util::ceil_div_usize;

use super::arithmetic_u32::U32Target;
use crate::gates::comparison::ComparisonGate;

/// Returns true if a is less than or equal to b, considered as base-`2^num_bits` limbs of a large value.
/// This range-checks its inputs.
pub fn list_le_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    a: Vec<Target>,
    b: Vec<Target>,
    num_bits: usize,
) -> BoolTarget {
    assert_eq!(
        a.len(),
        b.len(),
        "Comparison must be between same number of inputs and outputs"
    );
    let n = a.len();

    let chunk_bits = 2;
    let num_chunks = ceil_div_usize(num_bits, chunk_bits);

    let one = builder.one();
    let mut result = one;
    for i in 0..n {
        let a_le_b_gate = ComparisonGate::new(num_bits, num_chunks);
        let a_le_b_row = builder.add_gate(a_le_b_gate.clone(), vec![]);
        builder.connect(
            Target::wire(a_le_b_row, a_le_b_gate.wire_first_input()),
            a[i],
        );
        builder.connect(
            Target::wire(a_le_b_row, a_le_b_gate.wire_second_input()),
            b[i],
        );
        let a_le_b_result = Target::wire(a_le_b_row, a_le_b_gate.wire_result_bool());

        let b_le_a_gate = ComparisonGate::new(num_bits, num_chunks);
        let b_le_a_row = builder.add_gate(b_le_a_gate.clone(), vec![]);
        builder.connect(
            Target::wire(b_le_a_row, b_le_a_gate.wire_first_input()),
            b[i],
        );
        builder.connect(
            Target::wire(b_le_a_row, b_le_a_gate.wire_second_input()),
            a[i],
        );
        let b_le_a_result = Target::wire(b_le_a_row, b_le_a_gate.wire_result_bool());

        let these_limbs_equal = builder.mul(a_le_b_result, b_le_a_result);
        let these_limbs_less_than = builder.sub(one, b_le_a_result);
        result = builder.mul_add(these_limbs_equal, result, these_limbs_less_than);
    }

    // `result` being boolean is an invariant, maintained because its new value is always
    // `x * result + y`, where `x` and `y` are booleans that are not simultaneously true.
    BoolTarget::new_unsafe(result)
}

/// Helper function for comparing, specifically, lists of `U32Target`s.
pub fn list_le_u32_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    a: Vec<U32Target>,
    b: Vec<U32Target>,
) -> BoolTarget {
    let a_targets: Vec<Target> = a.iter().map(|&t| t.0).collect();
    let b_targets: Vec<Target> = b.iter().map(|&t| t.0).collect();

    list_le_circuit(builder, a_targets, b_targets, 32)
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use num::BigUint;
    use plonky2::iop::witness::PartialWitness;
    use plonky2::plonk::circuit_builder::CircuitBuilder;
    use plonky2::plonk::circuit_data::CircuitConfig;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use plonky2_field::types::Field;
    use rand::Rng;

    use crate::gadgets::multiple_comparison::list_le_circuit;

    fn test_list_le(size: usize, num_bits: usize) -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        let config = CircuitConfig::standard_recursion_config();
        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let mut rng = rand::thread_rng();

        let lst1: Vec<u64> = (0..size)
            .map(|_| rng.gen_range(0..(1 << num_bits)))
            .collect();
        let lst2: Vec<u64> = (0..size)
            .map(|_| rng.gen_range(0..(1 << num_bits)))
            .collect();

        let a_biguint = BigUint::from_slice(
            &lst1
                .iter()
                .flat_map(|&x| [x as u32, (x >> 32) as u32])
                .collect::<Vec<_>>(),
        );
        let b_biguint = BigUint::from_slice(
            &lst2
                .iter()
                .flat_map(|&x| [x as u32, (x >> 32) as u32])
                .collect::<Vec<_>>(),
        );

        let a = lst1
            .iter()
            .map(|&x| builder.constant(F::from_canonical_u64(x)))
            .collect();
        let b = lst2
            .iter()
            .map(|&x| builder.constant(F::from_canonical_u64(x)))
            .collect();

        let result = list_le_circuit(&mut builder, a, b, num_bits);

        let expected_result = builder.constant_bool(a_biguint <= b_biguint);
        builder.connect(result.target, expected_result.target);

        let data = builder.build::<C>();
        let proof = data.prove(pw).unwrap();
        data.verify(proof)
    }

    #[test]
    fn test_multiple_comparison() -> Result<()> {
        for size in [1, 3, 6] {
            for num_bits in [20, 32, 40, 44] {
                test_list_le(size, num_bits).unwrap();
            }
        }

        Ok(())
    }
}
