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
use crate::util::ceil_div_usize;

/// A gate for checking that one value is smaller than another.
#[derive(Clone, Debug)]
pub(crate) struct ComparisonGate<F: PrimeField + Extendable<D>, const D: usize> {
    pub(crate) num_bits: usize,
    pub(crate) num_chunks: usize,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> ComparisonGate<F, D> {
    pub fn new(num_bits: usize, num_chunks: usize) -> Self {
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

    pub fn wire_z_val(&self) -> usize {
        2
    }

    pub fn wire_z_bit(&self, bit_index: usize) -> usize {
        debug_assert!(bit_index < self.chunk_bits() + 1);
        3 + bit_index
    }

    pub fn wire_first_chunk_val(&self, chunk: usize) -> usize {
        debug_assert!(chunk < self.num_chunks);
        4 + self.chunk_bits() + chunk
    }

    pub fn wire_second_chunk_val(&self, chunk: usize) -> usize {
        debug_assert!(chunk < self.num_chunks);
        4 + self.chunk_bits() + self.num_chunks + chunk
    }

    pub fn wire_equality_dummy(&self, chunk: usize) -> usize {
        debug_assert!(chunk < self.num_chunks);
        4 + self.chunk_bits() + 2 * self.num_chunks + chunk
    }

    pub fn wire_chunks_equal(&self, chunk: usize) -> usize {
        debug_assert!(chunk < self.num_chunks);
        4 + self.chunk_bits() + 3 * self.num_chunks + chunk
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Gate<F, D> for ComparisonGate<F, D> {
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

        let mut most_significant_diff = F::Extension::ZERO;

        // Find the chosen chunk.
        for i in 0..self.num_chunks {
            let max_chunk_size = 1 << self.chunk_bits();
            let mut first_product = F::Extension::ONE;
            let mut second_product = F::Extension::ONE;
            for x in 1..max_chunk_size {
                let x_F = F::Extension::from_canonical_usize(x);
                first_product = first_product * (first_chunks[i] - x_F);
                second_product = second_product * (second_chunks[i] - x_F);
            }
            constraints.push(first_product);
            constraints.push(second_product);

            let difference = first_chunks[i] - second_chunks[i];
            let equality_dummy = vars.local_wires[self.wire_equality_dummy(i)];
            let chunks_equal = vars.local_wires[self.wire_chunks_equal(i)];

            // Two constraints identifying index.
            constraints.push(difference * equality_dummy - (F::Extension::ONE - chunks_equal));
            constraints.push(chunks_equal * difference);

            let this_diff = first_chunks[i] - second_chunks[i];
            most_significant_diff = chunks_equal * most_significant_diff
                + (F::Extension::ONE - chunks_equal) * this_diff;
        }

        let z_bits: Vec<F::Extension> = (0..self.chunk_bits() + 1)
            .map(|i| vars.local_wires[self.wire_z_bit(i)])
            .collect();
        let z_bits_combined = reduce_with_powers(&z_bits, F::Extension::TWO);

        let two_n = F::Extension::TWO.exp_u64(self.chunk_bits() as u64);
        constraints.push(z_bits_combined - (two_n + most_significant_diff));

        constraints.push(z_bits[self.chunk_bits()]);

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

        let mut most_significant_diff = F::ZERO;

        // Find the chosen chunk.
        for i in 0..self.num_chunks {
            let max_chunk_size = 1 << self.chunk_bits();
            let mut first_product = F::ONE;
            let mut second_product = F::ONE;
            for x in 1..max_chunk_size {
                let x_F = F::from_canonical_usize(x);
                first_product = first_product * (first_chunks[i] - x_F);
                second_product = second_product * (second_chunks[i] - x_F);
            }
            constraints.push(first_product);
            constraints.push(second_product);

            let difference = first_chunks[i] - second_chunks[i];
            let equality_dummy = vars.local_wires[self.wire_equality_dummy(i)];
            let chunks_equal = vars.local_wires[self.wire_chunks_equal(i)];

            // Two constraints identifying index.
            constraints.push(difference * equality_dummy - (F::ONE - chunks_equal));
            constraints.push(chunks_equal * difference);

            let this_diff = first_chunks[i] - second_chunks[i];
            most_significant_diff =
                chunks_equal * most_significant_diff + (F::ONE - chunks_equal) * this_diff;
        }

        let z_bits: Vec<F> = (0..self.chunk_bits() + 1)
            .map(|i| vars.local_wires[self.wire_z_bit(i)])
            .collect();
        let z_bits_combined = reduce_with_powers(&z_bits, F::TWO);

        let two_n = F::TWO.exp_u64(self.chunk_bits() as u64);
        constraints.push(z_bits_combined - (two_n + most_significant_diff));

        constraints.push(z_bits[self.chunk_bits()]);

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

        let mut most_significant_diff = builder.zero_extension();

        let one = builder.one_extension();
        // Find the chosen chunk.
        for i in 0..self.num_chunks {
            let max_chunk_size = 1 << self.chunk_bits();
            let mut first_product = one;
            let mut second_product = one;
            for x in 1..max_chunk_size {
                let x_F = builder.constant_extension(F::Extension::from_canonical_usize(x));
                let first_diff = builder.sub_extension(first_chunks[i], x_F);
                let second_diff = builder.sub_extension(second_chunks[i], x_F);
                first_product = builder.mul_extension(first_product, first_diff);
                second_product = builder.mul_extension(second_product, second_diff);
            }
            constraints.push(first_product);
            constraints.push(second_product);

            let difference = builder.sub_extension(first_chunks[i], second_chunks[i]);
            let equality_dummy = vars.local_wires[self.wire_equality_dummy(i)];
            let chunks_equal = vars.local_wires[self.wire_chunks_equal(i)];

            // Two constraints identifying index.
            let diff_times_equal = builder.mul_extension(difference, equality_dummy);
            let not_equal = builder.sub_extension(one, chunks_equal);
            constraints.push(builder.sub_extension(diff_times_equal, not_equal));
            constraints.push(builder.mul_extension(chunks_equal, difference));

            let this_diff = builder.sub_extension(first_chunks[i], second_chunks[i]);
            let old_diff = builder.mul_extension(chunks_equal, most_significant_diff);
            let not_equal = builder.sub_extension(one, chunks_equal);
            let new_diff = builder.mul_extension(not_equal, this_diff);
            most_significant_diff = builder.add_extension(old_diff, new_diff);
        }

        let two = builder.constant(F::TWO);
        let z_bits: Vec<ExtensionTarget<D>> = (0..self.chunk_bits() + 1)
            .map(|i| vars.local_wires[self.wire_z_bit(i)])
            .collect();
        let z_bits_combined = reduce_with_powers_ext_recursive(builder, &z_bits, two);

        let two_n = builder.constant_extension(F::Extension::TWO.exp_u64(self.chunk_bits() as u64));
        let expected_z = builder.add_extension(two_n, most_significant_diff);
        let z_diff = builder.sub_extension(z_bits_combined, expected_z);
        constraints.push(z_diff);

        constraints.push(z_bits[self.chunk_bits()]);

        constraints
    }

    fn generators(
        &self,
        gate_index: usize,
        _local_constants: &[F],
    ) -> Vec<Box<dyn WitnessGenerator<F>>> {
        let gen = ComparisonGenerator::<F, D> {
            gate_index,
            gate: self.clone(),
        };
        vec![Box::new(gen.adapter())]
    }

    fn num_wires(&self) -> usize {
        self.wire_chunks_equal(self.num_chunks - 1) + 1
    }

    fn num_constants(&self) -> usize {
        0
    }

    fn degree(&self) -> usize {
        (self.num_chunks + 1).max(1 << self.chunk_bits())
    }

    fn num_constraints(&self) -> usize {
        4 + 4 * self.num_chunks
    }
}

#[derive(Debug)]
struct ComparisonGenerator<F: RichField + Extendable<D>, const D: usize> {
    gate_index: usize,
    gate: ComparisonGate<F, D>,
}

impl<F: RichField + Extendable<D>, const D: usize> SimpleGenerator<F>
    for ComparisonGenerator<F, D>
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

        let first_input_bits: Vec<F> = (0..self.gate.num_bits)
            .scan(first_input_u64, |acc, _| {
                let tmp = *acc % 2;
                *acc /= 2;
                Some(F::from_canonical_u64(tmp))
            })
            .collect();
        let second_input_bits: Vec<F> = (0..self.gate.num_bits)
            .scan(second_input_u64, |acc, _| {
                let tmp = *acc % 2;
                *acc /= 2;
                Some(F::from_canonical_u64(tmp))
            })
            .collect();

        let first_input_chunks: Vec<F> = first_input_bits
            .chunks(self.gate.chunk_bits())
            .map(|bits| reduce_with_powers(&bits, F::TWO))
            .collect();
        let second_input_chunks: Vec<F> = second_input_bits
            .chunks(self.gate.chunk_bits())
            .map(|bits| reduce_with_powers(&bits, F::TWO))
            .collect();

        let chunks_equal: Vec<F> = (0..self.gate.num_chunks)
            .map(|i| F::from_bool(first_input_chunks[i] == second_input_chunks[i]))
            .collect();
        let equality_dummies: Vec<F> = first_input_chunks
            .iter()
            .zip(second_input_chunks.iter())
            .map(|(f, s)| if *f == *s { F::ONE } else { F::ONE / (*f - *s) })
            .collect();

        let mut diff_index = 0;
        for i in 1..self.gate.num_chunks {
            if first_input_chunks[i] != second_input_chunks[i] {
                diff_index = i;
            }
        }
        let most_significant_diff =
            first_input_chunks[diff_index] - second_input_chunks[diff_index];

        let z = F::TWO.exp_u64(self.gate.chunk_bits() as u64) + most_significant_diff;
        let z_bits: Vec<F> = (0..self.gate.chunk_bits() + 1)
            .scan(z.to_canonical_u64(), |acc, _| {
                let tmp = *acc % 2;
                *acc /= 2;
                Some(F::from_canonical_u64(tmp))
            })
            .collect();

        out_buffer.set_wire(local_wire(self.gate.wire_z_val()), z);
        for b in 0..self.gate.chunk_bits() + 1 {
            out_buffer.set_wire(local_wire(self.gate.wire_z_bit(b)), z_bits[b]);
        }
        for i in 0..self.gate.num_chunks {
            out_buffer.set_wire(
                local_wire(self.gate.wire_first_chunk_val(i)),
                first_input_chunks[i],
            );
            out_buffer.set_wire(
                local_wire(self.gate.wire_second_chunk_val(i)),
                second_input_chunks[i],
            );
            out_buffer.set_wire(local_wire(self.gate.wire_chunks_equal(i)), chunks_equal[i]);
            out_buffer.set_wire(
                local_wire(self.gate.wire_equality_dummy(i)),
                equality_dummies[i],
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
    use crate::gates::comparison::ComparisonGate;
    use crate::gates::gate::Gate;
    use crate::gates::gate_testing::{test_eval_fns, test_low_degree};
    use crate::hash::hash_types::HashOut;
    use crate::plonk::plonk_common::reduce_with_powers;
    use crate::plonk::vars::EvaluationVars;

    #[test]
    fn wire_indices() {
        type CG = ComparisonGate<CrandallField, 4>;
        let num_bits = 40;
        let num_chunks = 5;

        let gate = CG {
            num_bits,
            num_chunks,
            _phantom: PhantomData,
        };

        assert_eq!(gate.wire_first_input(), 0);
        assert_eq!(gate.wire_second_input(), 1);
        assert_eq!(gate.wire_z_val(), 2);
        assert_eq!(gate.wire_z_bit(0), 3);
        assert_eq!(gate.wire_z_bit(8), 11);
        assert_eq!(gate.wire_first_chunk_val(0), 12);
        assert_eq!(gate.wire_first_chunk_val(4), 16);
        assert_eq!(gate.wire_second_chunk_val(0), 17);
        assert_eq!(gate.wire_second_chunk_val(4), 21);
        assert_eq!(gate.wire_equality_dummy(0), 22);
        assert_eq!(gate.wire_equality_dummy(4), 26);
        assert_eq!(gate.wire_chunks_equal(0), 27);
        assert_eq!(gate.wire_chunks_equal(4), 31);
    }

    #[test]
    fn low_degree() {
        let num_bits = 40;
        let num_chunks = 5;

        test_low_degree::<CrandallField, _, 4>(ComparisonGate::<_, 4>::new(num_bits, num_chunks))
    }

    #[test]
    fn eval_fns() -> Result<()> {
        let num_bits = 40;
        let num_chunks = 5;

        test_eval_fns::<CrandallField, _, 4>(ComparisonGate::<_, 4>::new(num_bits, num_chunks))
    }

    #[test]
    fn test_gate_constraint() {
        type F = CrandallField;
        type FF = QuarticExtension<CrandallField>;
        const D: usize = 4;

        let num_copies = 3;
        let num_bits = 40;
        let num_chunks = 5;
        let chunk_bits = num_bits / num_chunks;

        // Returns the local wires for a comparison gate given the two inputs.
        let get_wires = |first_inputs: Vec<F>, second_inputs: Vec<F>| -> Vec<FF> {
            let num_copies = first_inputs.len();

            let mut v = Vec::new();
            for c in 0..num_copies {
                let first_input = first_inputs[c];
                let second_input = second_inputs[c];

                let first_input_u64 = first_input.to_canonical_u64();
                let second_input_u64 = second_input.to_canonical_u64();

                let first_input_bits: Vec<F> = (0..num_bits)
                    .scan(first_input_u64, |acc, _| {
                        let tmp = *acc % 2;
                        *acc /= 2;
                        Some(F::from_canonical_u64(tmp))
                    })
                    .collect();
                let second_input_bits: Vec<F> = (0..num_bits)
                    .scan(second_input_u64, |acc, _| {
                        let tmp = *acc % 2;
                        *acc /= 2;
                        Some(F::from_canonical_u64(tmp))
                    })
                    .collect();

                let mut first_input_chunks: Vec<F> = first_input_bits
                    .chunks(chunk_bits)
                    .map(|bits| reduce_with_powers(&bits, F::TWO))
                    .collect();
                let mut second_input_chunks: Vec<F> = second_input_bits
                    .chunks(chunk_bits)
                    .map(|bits| reduce_with_powers(&bits, F::TWO))
                    .collect();

                let mut chunks_equal: Vec<F> = (0..num_chunks)
                    .map(|i| F::from_bool(first_input_chunks[i] == second_input_chunks[i]))
                    .collect();
                let mut equality_dummies: Vec<F> = first_input_chunks
                    .iter()
                    .zip(second_input_chunks.iter())
                    .map(|(&f, &s)| if f == s { F::ONE } else { F::ONE / (f - s) })
                    .collect();

                let mut diff_index = 0;
                for i in 1..num_chunks {
                    if first_input_chunks[i] != second_input_chunks[i] {
                        diff_index = i;
                    }
                }
                let most_significant_diff =
                    first_input_chunks[diff_index] - second_input_chunks[diff_index];

                let z = F::TWO.exp_u64(chunk_bits as u64) + most_significant_diff;
                let mut z_bits: Vec<F> = (0..chunk_bits + 1)
                    .scan(z.to_canonical_u64(), |acc, _| {
                        let tmp = *acc % 2;
                        *acc /= 2;
                        Some(F::from_canonical_u64(tmp))
                    })
                    .collect();

                v.push(first_input);
                v.push(second_input);
                v.push(z);
                v.append(&mut z_bits);
                v.append(&mut first_input_chunks);
                v.append(&mut second_input_chunks);
                v.append(&mut equality_dummies);
                v.append(&mut chunks_equal);
            }

            v.iter().map(|&x| x.into()).collect::<Vec<_>>()
        };

        let mut rng = rand::thread_rng();
        let max: u64 = 1 << num_bits - 1;
        let first_inputs_u64: Vec<u64> = (0..num_copies).map(|_| rng.gen_range(0..max)).collect();
        let second_inputs_u64: Vec<u64> = (0..num_copies)
            .map(|i| {
                let mut val = rng.gen_range(0..max);
                while val <= first_inputs_u64[i] {
                    val = rng.gen_range(0..max);
                }
                val
            })
            .collect();

        let first_inputs = first_inputs_u64
            .iter()
            .map(|&x| F::from_canonical_u64(x))
            .collect();
        let second_inputs = second_inputs_u64
            .iter()
            .map(|&x| F::from_canonical_u64(x))
            .collect();

        let gate = ComparisonGate::<F, D> {
            num_bits,
            num_chunks,
            _phantom: PhantomData,
        };

        let vars = EvaluationVars {
            local_constants: &[],
            local_wires: &get_wires(first_inputs, second_inputs),
            public_inputs_hash: &HashOut::rand(),
        };

        assert!(
            gate.eval_unfiltered(vars).iter().all(|x| x.is_zero()),
            "Gate constraints are not satisfied."
        );
    }
}
