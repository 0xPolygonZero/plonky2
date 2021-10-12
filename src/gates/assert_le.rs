use std::marker::PhantomData;

use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::field::field_types::{Field, PrimeField, RichField};
use crate::gates::gate::Gate;
use crate::iop::generator::{GeneratedValues, SimpleGenerator, WitnessGenerator};
use crate::iop::target::Target;
use crate::iop::wire::Wire;
use crate::iop::witness::{PartitionWitness, Witness};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::plonk_common::{reduce_with_powers, reduce_with_powers_ext_recursive};
use crate::plonk::vars::{EvaluationTargets, EvaluationVars, EvaluationVarsBase};
use crate::util::{bits_u64, ceil_div_usize};

/// A gate for checking that one value is less than or equal to another.
#[derive(Clone, Debug)]
pub struct AssertLessThanGate<F: PrimeField + Extendable<D>, const D: usize> {
    pub(crate) num_bits: usize,
    pub(crate) num_chunks: usize,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> AssertLessThanGate<F, D> {
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

    pub fn wire_most_significant_diff(&self) -> usize {
        2
    }

    pub fn wire_first_chunk_val(&self, chunk: usize) -> usize {
        debug_assert!(chunk < self.num_chunks);
        3 + chunk
    }

    pub fn wire_second_chunk_val(&self, chunk: usize) -> usize {
        debug_assert!(chunk < self.num_chunks);
        3 + self.num_chunks + chunk
    }

    pub fn wire_equality_dummy(&self, chunk: usize) -> usize {
        debug_assert!(chunk < self.num_chunks);
        3 + 2 * self.num_chunks + chunk
    }

    pub fn wire_chunks_equal(&self, chunk: usize) -> usize {
        debug_assert!(chunk < self.num_chunks);
        3 + 3 * self.num_chunks + chunk
    }

    pub fn wire_intermediate_value(&self, chunk: usize) -> usize {
        debug_assert!(chunk < self.num_chunks);
        3 + 4 * self.num_chunks + chunk
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Gate<F, D> for AssertLessThanGate<F, D> {
    fn id(&self) -> String {
        format!("{:?}<D={}>", self, D)
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
            let first_product = (0..chunk_size)
                .map(|x| first_chunks[i] - F::Extension::from_canonical_usize(x))
                .product();
            let second_product = (0..chunk_size)
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

        // Range check `most_significant_diff` to be less than `chunk_size`.
        let product = (0..chunk_size)
            .map(|x| most_significant_diff - F::Extension::from_canonical_usize(x))
            .product();
        constraints.push(product);

        constraints
    }

    fn eval_unfiltered_base(&self, vars: EvaluationVarsBase<F>) -> Vec<F> {
        let mut constraints = Vec::with_capacity(self.num_constraints());

        let first_input = vars.local_wires[self.wire_first_input()];
        let second_input = vars.local_wires[self.wire_second_input()];

        // Get chunks and assert that they match
        let first_chunks: Vec<F> = (0..self.num_chunks)
            .map(|i| vars.local_wires[self.wire_first_chunk_val(i)])
            .collect();
        let second_chunks: Vec<F> = (0..self.num_chunks)
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

        constraints.push(first_chunks_combined - first_input);
        constraints.push(second_chunks_combined - second_input);

        let chunk_size = 1 << self.chunk_bits();

        let mut most_significant_diff_so_far = F::ZERO;

        for i in 0..self.num_chunks {
            // Range-check the chunks to be less than `chunk_size`.
            let first_product = (0..chunk_size)
                .map(|x| first_chunks[i] - F::from_canonical_usize(x))
                .product();
            let second_product = (0..chunk_size)
                .map(|x| second_chunks[i] - F::from_canonical_usize(x))
                .product();
            constraints.push(first_product);
            constraints.push(second_product);

            let difference = second_chunks[i] - first_chunks[i];
            let equality_dummy = vars.local_wires[self.wire_equality_dummy(i)];
            let chunks_equal = vars.local_wires[self.wire_chunks_equal(i)];

            // Two constraints to assert that `chunks_equal` is valid.
            constraints.push(difference * equality_dummy - (F::ONE - chunks_equal));
            constraints.push(chunks_equal * difference);

            // Update `most_significant_diff_so_far`.
            let intermediate_value = vars.local_wires[self.wire_intermediate_value(i)];
            constraints.push(intermediate_value - chunks_equal * most_significant_diff_so_far);
            most_significant_diff_so_far =
                intermediate_value + (F::ONE - chunks_equal) * difference;
        }

        let most_significant_diff = vars.local_wires[self.wire_most_significant_diff()];
        constraints.push(most_significant_diff - most_significant_diff_so_far);

        // Range check `most_significant_diff` to be less than `chunk_size`.
        let product = (0..chunk_size)
            .map(|x| most_significant_diff - F::from_canonical_usize(x))
            .product();
        constraints.push(product);

        constraints
    }

    fn eval_unfiltered_recursively(
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
            reduce_with_powers_ext_recursive(builder, &first_chunks, chunk_base);
        let second_chunks_combined =
            reduce_with_powers_ext_recursive(builder, &second_chunks, chunk_base);

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

        // Range check `most_significant_diff` to be less than `chunk_size`.
        let mut product = builder.one_extension();
        for x in 0..chunk_size {
            let x_f = builder.constant_extension(F::Extension::from_canonical_usize(x));
            let diff = builder.sub_extension(most_significant_diff, x_f);
            product = builder.mul_extension(product, diff);
        }
        constraints.push(product);

        constraints
    }

    fn generators(
        &self,
        gate_index: usize,
        _local_constants: &[F],
    ) -> Vec<Box<dyn WitnessGenerator<F>>> {
        let gen = AssertLessThanGenerator::<F, D> {
            gate_index,
            gate: self.clone(),
        };
        vec![Box::new(gen.adapter())]
    }

    fn num_wires(&self) -> usize {
        self.wire_intermediate_value(self.num_chunks - 1) + 1
    }

    fn num_constants(&self) -> usize {
        0
    }

    fn degree(&self) -> usize {
        1 << self.chunk_bits()
    }

    fn num_constraints(&self) -> usize {
        4 + 5 * self.num_chunks
    }
}

#[derive(Debug)]
struct AssertLessThanGenerator<F: RichField + Extendable<D>, const D: usize> {
    gate_index: usize,
    gate: AssertLessThanGate<F, D>,
}

impl<F: RichField + Extendable<D>, const D: usize> SimpleGenerator<F>
    for AssertLessThanGenerator<F, D>
{
    fn dependencies(&self) -> Vec<Target> {
        let local_target = |input| Target::wire(self.gate_index, input);

        let mut deps = Vec::new();
        deps.push(local_target(self.gate.wire_first_input()));
        deps.push(local_target(self.gate.wire_second_input()));
        deps
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let local_wire = |input| Wire {
            gate: self.gate_index,
            input,
        };

        let get_local_wire = |input| witness.get_wire(local_wire(input));

        let first_input = get_local_wire(self.gate.wire_first_input());
        let second_input = get_local_wire(self.gate.wire_second_input());

        let first_input_u64 = first_input.to_canonical_u64();
        let second_input_u64 = second_input.to_canonical_u64();

        debug_assert!(first_input_u64 < second_input_u64);

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
    }
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use anyhow::Result;
    use rand::Rng;

    use crate::field::crandall_field::CrandallField;
    use crate::field::extension_field::quartic::QuarticExtension;
    use crate::field::field_types::{Field, PrimeField};
    use crate::gates::assert_le::AssertLessThanGate;
    use crate::gates::gate::Gate;
    use crate::gates::gate_testing::{test_eval_fns, test_low_degree};
    use crate::hash::hash_types::HashOut;
    use crate::plonk::vars::EvaluationVars;

    #[test]
    fn wire_indices() {
        type AG = AssertLessThanGate<CrandallField, 4>;
        let num_bits = 40;
        let num_chunks = 5;

        let gate = AG {
            num_bits,
            num_chunks,
            _phantom: PhantomData,
        };

        assert_eq!(gate.wire_first_input(), 0);
        assert_eq!(gate.wire_second_input(), 1);
        assert_eq!(gate.wire_most_significant_diff(), 2);
        assert_eq!(gate.wire_first_chunk_val(0), 3);
        assert_eq!(gate.wire_first_chunk_val(4), 7);
        assert_eq!(gate.wire_second_chunk_val(0), 8);
        assert_eq!(gate.wire_second_chunk_val(4), 12);
        assert_eq!(gate.wire_equality_dummy(0), 13);
        assert_eq!(gate.wire_equality_dummy(4), 17);
        assert_eq!(gate.wire_chunks_equal(0), 18);
        assert_eq!(gate.wire_chunks_equal(4), 22);
        assert_eq!(gate.wire_intermediate_value(0), 23);
        assert_eq!(gate.wire_intermediate_value(4), 27);
    }

    #[test]
    fn low_degree() {
        let num_bits = 40;
        let num_chunks = 5;

        test_low_degree::<CrandallField, _, 4>(AssertLessThanGate::<_, 4>::new(
            num_bits, num_chunks,
        ))
    }

    #[test]
    fn eval_fns() -> Result<()> {
        let num_bits = 40;
        let num_chunks = 5;

        test_eval_fns::<CrandallField, _, 4>(AssertLessThanGate::<_, 4>::new(num_bits, num_chunks))
    }

    #[test]
    fn test_gate_constraint() {
        type F = CrandallField;
        type FF = QuarticExtension<CrandallField>;
        const D: usize = 4;

        let num_bits = 40;
        let num_chunks = 5;
        let chunk_bits = num_bits / num_chunks;

        // Returns the local wires for an AssertLessThanGate given the two inputs.
        let get_wires = |first_input: F, second_input: F| -> Vec<FF> {
            let mut v = Vec::new();

            let first_input_u64 = first_input.to_canonical_u64();
            let second_input_u64 = second_input.to_canonical_u64();

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

            v.push(first_input);
            v.push(second_input);
            v.push(most_significant_diff);
            v.append(&mut first_input_chunks);
            v.append(&mut second_input_chunks);
            v.append(&mut equality_dummies);
            v.append(&mut chunks_equal);
            v.append(&mut intermediate_values);

            v.iter().map(|&x| x.into()).collect::<Vec<_>>()
        };

        let mut rng = rand::thread_rng();
        let max: u64 = 1 << num_bits - 1;
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

        let less_than_gate = AssertLessThanGate::<F, D> {
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

        let equal_gate = AssertLessThanGate::<F, D> {
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
