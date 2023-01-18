use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::{format, vec};
use core::marker::PhantomData;

use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::{Field, Field64};
use plonky2::gates::gate::Gate;
use plonky2::gates::packed_util::PackedEvaluableBase;
use plonky2::gates::util::StridedConstraintConsumer;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::iop::generator::{GeneratedValues, SimpleGenerator, WitnessGenerator};
use plonky2::iop::target::Target;
use plonky2::iop::wire::Wire;
use plonky2::iop::witness::{PartitionWitness, Witness, WitnessWrite};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::plonk_common::{reduce_with_powers, reduce_with_powers_ext_circuit};
use plonky2::plonk::vars::{
    EvaluationTargets, EvaluationVars, EvaluationVarsBase, EvaluationVarsBaseBatch,
    EvaluationVarsBasePacked,
};
use plonky2::util::{bits_u64, ceil_div_usize};

/// A gate for checking that one value is less than or equal to another.
#[derive(Clone, Debug)]
pub struct ComparisonGate<F: Field64 + Extendable<D>, const D: usize> {
    pub(crate) num_bits: usize,
    pub(crate) num_chunks: usize,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> ComparisonGate<F, D> {
    pub fn new(num_bits: usize, num_chunks: usize) -> Self {
        debug_assert!(num_bits < bits_u64(F::ORDER));
        Self {
            num_bits,
            num_chunks,
            _phantom: PhantomData,
        }
    }

    pub fn chunk_bits(&self) -> usize {
        ceil_div_usize(self.num_bits, self.num_chunks)
    }

    pub fn wire_first_input(&self) -> usize {
        0
    }

    pub fn wire_second_input(&self) -> usize {
        1
    }

    pub fn wire_result_bool(&self) -> usize {
        2
    }

    pub fn wire_most_significant_diff(&self) -> usize {
        3
    }

    pub fn wire_first_chunk_val(&self, chunk: usize) -> usize {
        debug_assert!(chunk < self.num_chunks);
        4 + chunk
    }

    pub fn wire_second_chunk_val(&self, chunk: usize) -> usize {
        debug_assert!(chunk < self.num_chunks);
        4 + self.num_chunks + chunk
    }

    pub fn wire_equality_dummy(&self, chunk: usize) -> usize {
        debug_assert!(chunk < self.num_chunks);
        4 + 2 * self.num_chunks + chunk
    }

    pub fn wire_chunks_equal(&self, chunk: usize) -> usize {
        debug_assert!(chunk < self.num_chunks);
        4 + 3 * self.num_chunks + chunk
    }

    pub fn wire_intermediate_value(&self, chunk: usize) -> usize {
        debug_assert!(chunk < self.num_chunks);
        4 + 4 * self.num_chunks + chunk
    }

    /// The `bit_index`th bit of 2^n - 1 + most_significant_diff.
    pub fn wire_most_significant_diff_bit(&self, bit_index: usize) -> usize {
        4 + 5 * self.num_chunks + bit_index
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Gate<F, D> for ComparisonGate<F, D> {
    fn id(&self) -> String {
        format!("{self:?}<D={D}>")
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        let mut constraints = Vec::with_capacity(self.num_constraints());

        let first_input = vars.local_wires[self.wire_first_input()];
        let second_input = vars.local_wires[self.wire_second_input()];

        // Get chunks and assert that they match
        let first_chunks: Vec<F::Extension> = (0..self.num_chunks)
            .map(|i| vars.local_wires[self.wire_first_chunk_val(i)])
            .collect();
        let second_chunks: Vec<F::Extension> = (0..self.num_chunks)
            .map(|i| vars.local_wires[self.wire_second_chunk_val(i)])
            .collect();

        let first_chunks_combined = reduce_with_powers(
            &first_chunks,
            F::Extension::from_canonical_usize(1 << self.chunk_bits()),
        );
        let second_chunks_combined = reduce_with_powers(
            &second_chunks,
            F::Extension::from_canonical_usize(1 << self.chunk_bits()),
        );

        constraints.push(first_chunks_combined - first_input);
        constraints.push(second_chunks_combined - second_input);

        let chunk_size = 1 << self.chunk_bits();

        let mut most_significant_diff_so_far = F::Extension::ZERO;

        for i in 0..self.num_chunks {
            // Range-check the chunks to be less than `chunk_size`.
            let first_product: F::Extension = (0..chunk_size)
                .map(|x| first_chunks[i] - F::Extension::from_canonical_usize(x))
                .product();
            let second_product: F::Extension = (0..chunk_size)
                .map(|x| second_chunks[i] - F::Extension::from_canonical_usize(x))
                .product();
            constraints.push(first_product);
            constraints.push(second_product);

            let difference = second_chunks[i] - first_chunks[i];
            let equality_dummy = vars.local_wires[self.wire_equality_dummy(i)];
            let chunks_equal = vars.local_wires[self.wire_chunks_equal(i)];

            // Two constraints to assert that `chunks_equal` is valid.
            constraints.push(difference * equality_dummy - (F::Extension::ONE - chunks_equal));
            constraints.push(chunks_equal * difference);

            // Update `most_significant_diff_so_far`.
            let intermediate_value = vars.local_wires[self.wire_intermediate_value(i)];
            constraints.push(intermediate_value - chunks_equal * most_significant_diff_so_far);
            most_significant_diff_so_far =
                intermediate_value + (F::Extension::ONE - chunks_equal) * difference;
        }

        let most_significant_diff = vars.local_wires[self.wire_most_significant_diff()];
        constraints.push(most_significant_diff - most_significant_diff_so_far);

        let most_significant_diff_bits: Vec<F::Extension> = (0..self.chunk_bits() + 1)
            .map(|i| vars.local_wires[self.wire_most_significant_diff_bit(i)])
            .collect();

        // Range-check the bits.
        for &bit in &most_significant_diff_bits {
            constraints.push(bit * (F::Extension::ONE - bit));
        }

        let bits_combined = reduce_with_powers(&most_significant_diff_bits, F::Extension::TWO);
        let two_n = F::Extension::from_canonical_u64(1 << self.chunk_bits());
        constraints.push((two_n + most_significant_diff) - bits_combined);

        // Iff first <= second, the top (n + 1st) bit of (2^n + most_significant_diff) will be 1.
        let result_bool = vars.local_wires[self.wire_result_bool()];
        constraints.push(result_bool - most_significant_diff_bits[self.chunk_bits()]);

        constraints
    }

    fn eval_unfiltered_base_one(
        &self,
        _vars: EvaluationVarsBase<F>,
        _yield_constr: StridedConstraintConsumer<F>,
    ) {
        panic!("use eval_unfiltered_base_packed instead");
    }

    fn eval_unfiltered_base_batch(&self, vars_base: EvaluationVarsBaseBatch<F>) -> Vec<F> {
        self.eval_unfiltered_base_batch_packed(vars_base)
    }

    fn eval_unfiltered_circuit(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>> {
        let mut constraints = Vec::with_capacity(self.num_constraints());

        let first_input = vars.local_wires[self.wire_first_input()];
        let second_input = vars.local_wires[self.wire_second_input()];

        // Get chunks and assert that they match
        let first_chunks: Vec<ExtensionTarget<D>> = (0..self.num_chunks)
            .map(|i| vars.local_wires[self.wire_first_chunk_val(i)])
            .collect();
        let second_chunks: Vec<ExtensionTarget<D>> = (0..self.num_chunks)
            .map(|i| vars.local_wires[self.wire_second_chunk_val(i)])
            .collect();

        let chunk_base = builder.constant(F::from_canonical_usize(1 << self.chunk_bits()));
        let first_chunks_combined =
            reduce_with_powers_ext_circuit(builder, &first_chunks, chunk_base);
        let second_chunks_combined =
            reduce_with_powers_ext_circuit(builder, &second_chunks, chunk_base);

        constraints.push(builder.sub_extension(first_chunks_combined, first_input));
        constraints.push(builder.sub_extension(second_chunks_combined, second_input));

        let chunk_size = 1 << self.chunk_bits();

        let mut most_significant_diff_so_far = builder.zero_extension();

        let one = builder.one_extension();
        // Find the chosen chunk.
        for i in 0..self.num_chunks {
            // Range-check the chunks to be less than `chunk_size`.
            let mut first_product = one;
            let mut second_product = one;
            for x in 0..chunk_size {
                let x_f = builder.constant_extension(F::Extension::from_canonical_usize(x));
                let first_diff = builder.sub_extension(first_chunks[i], x_f);
                let second_diff = builder.sub_extension(second_chunks[i], x_f);
                first_product = builder.mul_extension(first_product, first_diff);
                second_product = builder.mul_extension(second_product, second_diff);
            }
            constraints.push(first_product);
            constraints.push(second_product);

            let difference = builder.sub_extension(second_chunks[i], first_chunks[i]);
            let equality_dummy = vars.local_wires[self.wire_equality_dummy(i)];
            let chunks_equal = vars.local_wires[self.wire_chunks_equal(i)];

            // Two constraints to assert that `chunks_equal` is valid.
            let diff_times_equal = builder.mul_extension(difference, equality_dummy);
            let not_equal = builder.sub_extension(one, chunks_equal);
            constraints.push(builder.sub_extension(diff_times_equal, not_equal));
            constraints.push(builder.mul_extension(chunks_equal, difference));

            // Update `most_significant_diff_so_far`.
            let intermediate_value = vars.local_wires[self.wire_intermediate_value(i)];
            let old_diff = builder.mul_extension(chunks_equal, most_significant_diff_so_far);
            constraints.push(builder.sub_extension(intermediate_value, old_diff));

            let not_equal = builder.sub_extension(one, chunks_equal);
            let new_diff = builder.mul_extension(not_equal, difference);
            most_significant_diff_so_far = builder.add_extension(intermediate_value, new_diff);
        }

        let most_significant_diff = vars.local_wires[self.wire_most_significant_diff()];
        constraints
            .push(builder.sub_extension(most_significant_diff, most_significant_diff_so_far));

        let most_significant_diff_bits: Vec<ExtensionTarget<D>> = (0..self.chunk_bits() + 1)
            .map(|i| vars.local_wires[self.wire_most_significant_diff_bit(i)])
            .collect();

        // Range-check the bits.
        for &this_bit in &most_significant_diff_bits {
            let inverse = builder.sub_extension(one, this_bit);
            constraints.push(builder.mul_extension(this_bit, inverse));
        }

        let two = builder.two();
        let bits_combined =
            reduce_with_powers_ext_circuit(builder, &most_significant_diff_bits, two);
        let two_n =
            builder.constant_extension(F::Extension::from_canonical_u64(1 << self.chunk_bits()));
        let sum = builder.add_extension(two_n, most_significant_diff);
        constraints.push(builder.sub_extension(sum, bits_combined));

        // Iff first <= second, the top (n + 1st) bit of (2^n + most_significant_diff) will be 1.
        let result_bool = vars.local_wires[self.wire_result_bool()];
        constraints.push(
            builder.sub_extension(result_bool, most_significant_diff_bits[self.chunk_bits()]),
        );

        constraints
    }

    fn generators(&self, row: usize, _local_constants: &[F]) -> Vec<Box<dyn WitnessGenerator<F>>> {
        let gen = ComparisonGenerator::<F, D> {
            row,
            gate: self.clone(),
        };
        vec![Box::new(gen.adapter())]
    }

    fn num_wires(&self) -> usize {
        4 + 5 * self.num_chunks + (self.chunk_bits() + 1)
    }

    fn num_constants(&self) -> usize {
        0
    }

    fn degree(&self) -> usize {
        1 << self.chunk_bits()
    }

    fn num_constraints(&self) -> usize {
        6 + 5 * self.num_chunks + self.chunk_bits()
    }
}

impl<F: RichField + Extendable<D>, const D: usize> PackedEvaluableBase<F, D>
    for ComparisonGate<F, D>
{
    fn eval_unfiltered_base_packed<P: PackedField<Scalar = F>>(
        &self,
        vars: EvaluationVarsBasePacked<P>,
        mut yield_constr: StridedConstraintConsumer<P>,
    ) {
        let first_input = vars.local_wires[self.wire_first_input()];
        let second_input = vars.local_wires[self.wire_second_input()];

        // Get chunks and assert that they match
        let first_chunks: Vec<_> = (0..self.num_chunks)
            .map(|i| vars.local_wires[self.wire_first_chunk_val(i)])
            .collect();
        let second_chunks: Vec<_> = (0..self.num_chunks)
            .map(|i| vars.local_wires[self.wire_second_chunk_val(i)])
            .collect();

        let first_chunks_combined = reduce_with_powers(
            &first_chunks,
            F::from_canonical_usize(1 << self.chunk_bits()),
        );
        let second_chunks_combined = reduce_with_powers(
            &second_chunks,
            F::from_canonical_usize(1 << self.chunk_bits()),
        );

        yield_constr.one(first_chunks_combined - first_input);
        yield_constr.one(second_chunks_combined - second_input);

        let chunk_size = 1 << self.chunk_bits();

        let mut most_significant_diff_so_far = P::ZEROS;

        for i in 0..self.num_chunks {
            // Range-check the chunks to be less than `chunk_size`.
            let first_product: P = (0..chunk_size)
                .map(|x| first_chunks[i] - F::from_canonical_usize(x))
                .product();
            let second_product: P = (0..chunk_size)
                .map(|x| second_chunks[i] - F::from_canonical_usize(x))
                .product();
            yield_constr.one(first_product);
            yield_constr.one(second_product);

            let difference = second_chunks[i] - first_chunks[i];
            let equality_dummy = vars.local_wires[self.wire_equality_dummy(i)];
            let chunks_equal = vars.local_wires[self.wire_chunks_equal(i)];

            // Two constraints to assert that `chunks_equal` is valid.
            yield_constr.one(difference * equality_dummy - (P::ONES - chunks_equal));
            yield_constr.one(chunks_equal * difference);

            // Update `most_significant_diff_so_far`.
            let intermediate_value = vars.local_wires[self.wire_intermediate_value(i)];
            yield_constr.one(intermediate_value - chunks_equal * most_significant_diff_so_far);
            most_significant_diff_so_far =
                intermediate_value + (P::ONES - chunks_equal) * difference;
        }

        let most_significant_diff = vars.local_wires[self.wire_most_significant_diff()];
        yield_constr.one(most_significant_diff - most_significant_diff_so_far);

        let most_significant_diff_bits: Vec<_> = (0..self.chunk_bits() + 1)
            .map(|i| vars.local_wires[self.wire_most_significant_diff_bit(i)])
            .collect();

        // Range-check the bits.
        for &bit in &most_significant_diff_bits {
            yield_constr.one(bit * (P::ONES - bit));
        }

        let bits_combined = reduce_with_powers(&most_significant_diff_bits, F::TWO);
        let two_n = F::from_canonical_u64(1 << self.chunk_bits());
        yield_constr.one((most_significant_diff + two_n) - bits_combined);

        // Iff first <= second, the top (n + 1st) bit of (2^n - 1 + most_significant_diff) will be 1.
        let result_bool = vars.local_wires[self.wire_result_bool()];
        yield_constr.one(result_bool - most_significant_diff_bits[self.chunk_bits()]);
    }
}

#[derive(Debug)]
struct ComparisonGenerator<F: RichField + Extendable<D>, const D: usize> {
    row: usize,
    gate: ComparisonGate<F, D>,
}

impl<F: RichField + Extendable<D>, const D: usize> SimpleGenerator<F>
    for ComparisonGenerator<F, D>
{
    fn dependencies(&self) -> Vec<Target> {
        let local_target = |column| Target::wire(self.row, column);

        vec![
            local_target(self.gate.wire_first_input()),
            local_target(self.gate.wire_second_input()),
        ]
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let local_wire = |column| Wire {
            row: self.row,
            column,
        };

        let get_local_wire = |column| witness.get_wire(local_wire(column));

        let first_input = get_local_wire(self.gate.wire_first_input());
        let second_input = get_local_wire(self.gate.wire_second_input());

        let first_input_u64 = first_input.to_canonical_u64();
        let second_input_u64 = second_input.to_canonical_u64();

        let result = F::from_canonical_usize((first_input_u64 <= second_input_u64) as usize);

        let chunk_size = 1 << self.gate.chunk_bits();
        let first_input_chunks: Vec<F> = (0..self.gate.num_chunks)
            .scan(first_input_u64, |acc, _| {
                let tmp = *acc % chunk_size;
                *acc /= chunk_size;
                Some(F::from_canonical_u64(tmp))
            })
            .collect();
        let second_input_chunks: Vec<F> = (0..self.gate.num_chunks)
            .scan(second_input_u64, |acc, _| {
                let tmp = *acc % chunk_size;
                *acc /= chunk_size;
                Some(F::from_canonical_u64(tmp))
            })
            .collect();

        let chunks_equal: Vec<F> = (0..self.gate.num_chunks)
            .map(|i| F::from_bool(first_input_chunks[i] == second_input_chunks[i]))
            .collect();
        let equality_dummies: Vec<F> = first_input_chunks
            .iter()
            .zip(second_input_chunks.iter())
            .map(|(&f, &s)| if f == s { F::ONE } else { F::ONE / (s - f) })
            .collect();

        let mut most_significant_diff_so_far = F::ZERO;
        let mut intermediate_values = Vec::new();
        for i in 0..self.gate.num_chunks {
            if first_input_chunks[i] != second_input_chunks[i] {
                most_significant_diff_so_far = second_input_chunks[i] - first_input_chunks[i];
                intermediate_values.push(F::ZERO);
            } else {
                intermediate_values.push(most_significant_diff_so_far);
            }
        }
        let most_significant_diff = most_significant_diff_so_far;

        let two_n = F::from_canonical_usize(1 << self.gate.chunk_bits());
        let two_n_plus_msd = (two_n + most_significant_diff).to_canonical_u64();

        let msd_bits_u64: Vec<u64> = (0..self.gate.chunk_bits() + 1)
            .scan(two_n_plus_msd, |acc, _| {
                let tmp = *acc % 2;
                *acc /= 2;
                Some(tmp)
            })
            .collect();
        let msd_bits: Vec<F> = msd_bits_u64
            .iter()
            .map(|x| F::from_canonical_u64(*x))
            .collect();

        out_buffer.set_wire(local_wire(self.gate.wire_result_bool()), result);
        out_buffer.set_wire(
            local_wire(self.gate.wire_most_significant_diff()),
            most_significant_diff,
        );
        for i in 0..self.gate.num_chunks {
            out_buffer.set_wire(
                local_wire(self.gate.wire_first_chunk_val(i)),
                first_input_chunks[i],
            );
            out_buffer.set_wire(
                local_wire(self.gate.wire_second_chunk_val(i)),
                second_input_chunks[i],
            );
            out_buffer.set_wire(
                local_wire(self.gate.wire_equality_dummy(i)),
                equality_dummies[i],
            );
            out_buffer.set_wire(local_wire(self.gate.wire_chunks_equal(i)), chunks_equal[i]);
            out_buffer.set_wire(
                local_wire(self.gate.wire_intermediate_value(i)),
                intermediate_values[i],
            );
        }
        for i in 0..self.gate.chunk_bits() + 1 {
            out_buffer.set_wire(
                local_wire(self.gate.wire_most_significant_diff_bit(i)),
                msd_bits[i],
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use plonky2::field::goldilocks_field::GoldilocksField;
    use plonky2::field::types::{PrimeField64, Sample};
    use plonky2::gates::gate_testing::{test_eval_fns, test_low_degree};
    use plonky2::hash::hash_types::HashOut;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use rand::rngs::OsRng;
    use rand::Rng;

    use super::*;

    #[test]
    fn wire_indices() {
        type CG = ComparisonGate<GoldilocksField, 4>;
        let num_bits = 40;
        let num_chunks = 5;

        let gate = CG {
            num_bits,
            num_chunks,
            _phantom: PhantomData,
        };

        assert_eq!(gate.wire_first_input(), 0);
        assert_eq!(gate.wire_second_input(), 1);
        assert_eq!(gate.wire_result_bool(), 2);
        assert_eq!(gate.wire_most_significant_diff(), 3);
        assert_eq!(gate.wire_first_chunk_val(0), 4);
        assert_eq!(gate.wire_first_chunk_val(4), 8);
        assert_eq!(gate.wire_second_chunk_val(0), 9);
        assert_eq!(gate.wire_second_chunk_val(4), 13);
        assert_eq!(gate.wire_equality_dummy(0), 14);
        assert_eq!(gate.wire_equality_dummy(4), 18);
        assert_eq!(gate.wire_chunks_equal(0), 19);
        assert_eq!(gate.wire_chunks_equal(4), 23);
        assert_eq!(gate.wire_intermediate_value(0), 24);
        assert_eq!(gate.wire_intermediate_value(4), 28);
        assert_eq!(gate.wire_most_significant_diff_bit(0), 29);
        assert_eq!(gate.wire_most_significant_diff_bit(8), 37);
    }

    #[test]
    fn low_degree() {
        let num_bits = 40;
        let num_chunks = 5;

        test_low_degree::<GoldilocksField, _, 4>(ComparisonGate::<_, 4>::new(num_bits, num_chunks))
    }

    #[test]
    fn eval_fns() -> Result<()> {
        let num_bits = 40;
        let num_chunks = 5;
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;

        test_eval_fns::<F, C, _, D>(ComparisonGate::<_, 2>::new(num_bits, num_chunks))
    }

    #[test]
    fn test_gate_constraint() {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type FF = <C as GenericConfig<D>>::FE;

        let num_bits = 40;
        let num_chunks = 5;
        let chunk_bits = num_bits / num_chunks;

        // Returns the local wires for a comparison gate given the two inputs.
        let get_wires = |first_input: F, second_input: F| -> Vec<FF> {
            let mut v = Vec::new();

            let first_input_u64 = first_input.to_canonical_u64();
            let second_input_u64 = second_input.to_canonical_u64();

            let result_bool = F::from_bool(first_input_u64 <= second_input_u64);

            let chunk_size = 1 << chunk_bits;
            let mut first_input_chunks: Vec<F> = (0..num_chunks)
                .scan(first_input_u64, |acc, _| {
                    let tmp = *acc % chunk_size;
                    *acc /= chunk_size;
                    Some(F::from_canonical_u64(tmp))
                })
                .collect();
            let mut second_input_chunks: Vec<F> = (0..num_chunks)
                .scan(second_input_u64, |acc, _| {
                    let tmp = *acc % chunk_size;
                    *acc /= chunk_size;
                    Some(F::from_canonical_u64(tmp))
                })
                .collect();

            let mut chunks_equal: Vec<F> = (0..num_chunks)
                .map(|i| F::from_bool(first_input_chunks[i] == second_input_chunks[i]))
                .collect();
            let mut equality_dummies: Vec<F> = first_input_chunks
                .iter()
                .zip(second_input_chunks.iter())
                .map(|(&f, &s)| if f == s { F::ONE } else { F::ONE / (s - f) })
                .collect();

            let mut most_significant_diff_so_far = F::ZERO;
            let mut intermediate_values = Vec::new();
            for i in 0..num_chunks {
                if first_input_chunks[i] != second_input_chunks[i] {
                    most_significant_diff_so_far = second_input_chunks[i] - first_input_chunks[i];
                    intermediate_values.push(F::ZERO);
                } else {
                    intermediate_values.push(most_significant_diff_so_far);
                }
            }
            let most_significant_diff = most_significant_diff_so_far;

            let two_n_plus_msd =
                (1 << chunk_bits) as u64 + most_significant_diff.to_canonical_u64();
            let mut msd_bits: Vec<F> = (0..chunk_bits + 1)
                .scan(two_n_plus_msd, |acc, _| {
                    let tmp = *acc % 2;
                    *acc /= 2;
                    Some(F::from_canonical_u64(tmp))
                })
                .collect();

            v.push(first_input);
            v.push(second_input);
            v.push(result_bool);
            v.push(most_significant_diff);
            v.append(&mut first_input_chunks);
            v.append(&mut second_input_chunks);
            v.append(&mut equality_dummies);
            v.append(&mut chunks_equal);
            v.append(&mut intermediate_values);
            v.append(&mut msd_bits);

            v.iter().map(|&x| x.into()).collect()
        };

        let mut rng = OsRng;
        let max: u64 = 1 << (num_bits - 1);
        let first_input_u64 = rng.gen_range(0..max);
        let second_input_u64 = {
            let mut val = rng.gen_range(0..max);
            while val < first_input_u64 {
                val = rng.gen_range(0..max);
            }
            val
        };

        let first_input = F::from_canonical_u64(first_input_u64);
        let second_input = F::from_canonical_u64(second_input_u64);

        let less_than_gate = ComparisonGate::<F, D> {
            num_bits,
            num_chunks,
            _phantom: PhantomData,
        };
        let less_than_vars = EvaluationVars {
            local_constants: &[],
            local_wires: &get_wires(first_input, second_input),
            public_inputs_hash: &HashOut::rand(),
        };
        assert!(
            less_than_gate
                .eval_unfiltered(less_than_vars)
                .iter()
                .all(|x| x.is_zero()),
            "Gate constraints are not satisfied."
        );

        let equal_gate = ComparisonGate::<F, D> {
            num_bits,
            num_chunks,
            _phantom: PhantomData,
        };
        let equal_vars = EvaluationVars {
            local_constants: &[],
            local_wires: &get_wires(first_input, first_input),
            public_inputs_hash: &HashOut::rand(),
        };
        assert!(
            equal_gate
                .eval_unfiltered(equal_vars)
                .iter()
                .all(|x| x.is_zero()),
            "Gate constraints are not satisfied."
        );
    }
}
