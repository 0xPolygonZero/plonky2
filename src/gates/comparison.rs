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
use crate::plonk::circuit_data::CircuitConfig;
use crate::plonk::vars::{EvaluationTargets, EvaluationVars, EvaluationVarsBase};
use crate::util::{bits_u64, ceil_div_usize};

/// A gate for checking that one value is smaller than another.
#[derive(Clone, Debug)]
pub(crate) struct ComparisonGate<F: PrimeField + Extendable<D>, const D: usize> {
    pub(crate) num_copies: usize,
    pub(crate) num_bits: usize,
    pub(crate) num_chunks: usize,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> ComparisonGate<F, D> {
    pub fn new(num_copies: usize, num_bits: usize, num_chunks: usize) -> Self {
        Self {
            num_copies,
            num_bits,
            num_chunks,
            _phantom: PhantomData,
        }
    }

    pub fn chunk_bits(&self) -> usize {
        ceil_div_usize(self.num_bits, self.num_chunks)
    }

    pub fn new_from_config(config: CircuitConfig, num_bits: usize, num_chunks: usize) -> Self {
        let num_copies = Self::max_num_copies(config.num_routed_wires, num_bits, num_chunks);
        Self::new(num_copies, num_bits, num_chunks)
    }

    pub fn max_num_copies(num_routed_wires: usize, num_bits: usize, num_chunks: usize) -> usize {
        let chunk_bits = ceil_div_usize(num_bits, num_chunks);
        let wires_per_copy = 4 + chunk_bits + 4 * num_chunks;
        num_routed_wires / wires_per_copy
    }

    pub fn wires_per_copy(&self) -> usize {
        4 + self.chunk_bits() + 4 * self.num_chunks
    }

    pub fn wire_first_input(&self, copy: usize) -> usize {
        debug_assert!(copy < self.num_copies);
        copy * self.wires_per_copy()
    }

    pub fn wire_second_input(&self, copy: usize) -> usize {
        debug_assert!(copy < self.num_copies);
        copy * self.wires_per_copy() + 1
    }

    pub fn wire_z_val(&self, copy: usize) -> usize {
        copy * self.wires_per_copy() + 2
    }

    pub fn wire_z_bit(&self, copy: usize, bit_index: usize) -> usize {
        debug_assert!(bit_index < self.chunk_bits() + 1);
        copy * self.wires_per_copy() + 3 + bit_index
    }

    pub fn wire_first_chunk_val(&self, copy: usize, chunk: usize) -> usize {
        debug_assert!(copy < self.num_copies);
        debug_assert!(chunk < self.num_chunks);
        copy * self.wires_per_copy() + 4 + self.chunk_bits() + chunk
    }

    pub fn wire_second_chunk_val(&self, copy: usize, chunk: usize) -> usize {
        debug_assert!(copy < self.num_copies);
        debug_assert!(chunk < self.num_chunks);
        copy * self.wires_per_copy() + 4 + self.chunk_bits() + self.num_chunks + chunk
    }

    pub fn wire_equality_dummy(&self, copy: usize, chunk: usize) -> usize {
        debug_assert!(copy < self.num_copies);
        debug_assert!(chunk < self.num_chunks);
        copy * self.wires_per_copy() + 4 + self.chunk_bits() + 2 * self.num_chunks + chunk
    }

    pub fn wire_chunks_equal(&self, copy: usize, chunk: usize) -> usize {
        debug_assert!(copy < self.num_copies);
        debug_assert!(chunk < self.num_chunks);
        copy * self.wires_per_copy() + 4 + self.chunk_bits() + 3 * self.num_chunks + chunk
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Gate<F, D> for ComparisonGate<F, D> {
    fn id(&self) -> String {
        format!("{:?}<D={}>", self, D)
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        let mut constraints = Vec::with_capacity(self.num_constraints());

        for c in 0..self.num_copies {
            let first_input = vars.local_wires[self.wire_first_input(c)];
            let second_input = vars.local_wires[self.wire_second_input(c)];

            // Get chunks and assert that they match
            let first_chunks: Vec<F::Extension> = (0..self.num_chunks)
                .map(|i| vars.local_wires[self.wire_first_chunk_val(c, i)])
                .collect();
            let second_chunks: Vec<F::Extension> = (0..self.num_chunks)
                .map(|i| vars.local_wires[self.wire_second_chunk_val(c, i)])
                .collect();

            let chunk_base_powers: Vec<F::Extension> = (0..self.num_chunks)
                .map(|i| F::Extension::TWO.exp_u64((i * self.chunk_bits()) as u64))
                .collect();

            let first_chunks_combined = first_chunks
                .iter()
                .zip(chunk_base_powers.iter())
                .map(|(b, x)| *b * *x)
                .fold(F::Extension::ZERO, |a, b| a + b);
            let second_chunks_combined = second_chunks
                .iter()
                .zip(chunk_base_powers.iter())
                .map(|(b, x)| *b * *x)
                .fold(F::Extension::ZERO, |a, b| a + b);

            constraints.push(first_chunks_combined - first_input);
            constraints.push(second_chunks_combined - second_input);

            let mut most_significant_diff =
                first_chunks[self.num_chunks - 1] - second_chunks[self.num_chunks - 1];

            // Find the chosen chunk.
            for i in (0..self.num_chunks).rev() {
                let difference = first_chunks[i] - second_chunks[i];
                let equality_dummy = vars.local_wires[self.wire_equality_dummy(c, i)];
                let chunks_equal = vars.local_wires[self.wire_chunks_equal(c, i)];

                // Two constraints identifying index.
                //constraints.push(difference * equality_dummy - (F::Extension::ONE - chunks_equal));
                //constraints.push(chunks_equal * difference);

                let this_diff = first_chunks[i] - second_chunks[i];
                most_significant_diff = chunks_equal * most_significant_diff
                    + (F::Extension::ONE - chunks_equal) * this_diff;
            }

            let z_bits: Vec<F::Extension> = (0..self.chunk_bits() + 1)
                .map(|i| vars.local_wires[self.wire_z_bit(c, i)])
                .collect();

            let powers_of_two: Vec<F::Extension> = (0..self.chunk_bits() + 1)
                .map(|i| F::Extension::TWO.exp_u64(i as u64))
                .collect();
            let z_bits_combined = z_bits
                .iter()
                .zip(powers_of_two.iter())
                .map(|(b, x)| *b * *x)
                .fold(F::Extension::ZERO, |a, b| a + b);

            let two_n = F::Extension::TWO.exp_u64(self.chunk_bits() as u64);
            //constraints.push(z_bits_combined - (two_n + most_significant_diff));

            //constraints.push(z_bits[self.chunk_bits() - 1]);
        }

        constraints
    }

    fn eval_unfiltered_base(&self, vars: EvaluationVarsBase<F>) -> Vec<F> {
        todo!()
    }

    fn eval_unfiltered_recursively(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>> {
        todo!()
    }

    fn generators(
        &self,
        gate_index: usize,
        _local_constants: &[F],
    ) -> Vec<Box<dyn WitnessGenerator<F>>> {
        (0..self.num_copies)
            .map(|c| {
                let gen = ComparisonGenerator::<F, D> {
                    gate_index,
                    gate: self.clone(),
                    copy: c,
                };
                let g: Box<dyn WitnessGenerator<F>> = Box::new(gen.adapter());
                g
            })
            .collect()
    }

    fn num_wires(&self) -> usize {
        self.wire_chunks_equal(self.num_copies - 1, self.num_chunks - 1) + 1
    }

    fn num_constants(&self) -> usize {
        0
    }

    fn degree(&self) -> usize {
        self.num_chunks + 1
    }

    fn num_constraints(&self) -> usize {
        self.num_copies * (4 + 2 * self.num_chunks)
    }
}

#[derive(Debug)]
struct ComparisonGenerator<F: RichField + Extendable<D>, const D: usize> {
    gate_index: usize,
    gate: ComparisonGate<F, D>,
    copy: usize,
}

impl<F: RichField + Extendable<D>, const D: usize> SimpleGenerator<F>
    for ComparisonGenerator<F, D>
{
    fn dependencies(&self) -> Vec<Target> {
        let local_target = |input| Target::wire(self.gate_index, input);

        let mut deps = Vec::new();
        deps.push(local_target(self.gate.wire_first_input(self.copy)));
        deps.push(local_target(self.gate.wire_second_input(self.copy)));
        deps
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let local_wire = |input| Wire {
            gate: self.gate_index,
            input,
        };

        let get_local_wire = |input| witness.get_wire(local_wire(input));

        let first_input = get_local_wire(self.gate.wire_first_input(self.copy));
        let second_input = get_local_wire(self.gate.wire_second_input(self.copy));

        let first_input_u64 = first_input.to_canonical_u64();
        let second_input_u64 = second_input.to_canonical_u64();

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

        let powers_of_two: Vec<F> = (0..self.gate.chunk_bits())
            .map(|i| F::TWO.exp_u64(i as u64))
            .collect();
        let first_input_chunks: Vec<F> = first_input_bits
            .chunks(self.gate.chunk_bits())
            .map(|bits| {
                bits.iter()
                    .zip(powers_of_two.iter())
                    .map(|(b, x)| *b * *x)
                    .fold(F::ZERO, |a, b| a + b)
            })
            .collect();
        let second_input_chunks: Vec<F> = second_input_bits
            .chunks(self.gate.chunk_bits())
            .map(|bits| {
                bits.iter()
                    .zip(powers_of_two.iter())
                    .map(|(b, x)| *b * *x)
                    .fold(F::ZERO, |a, b| a + b)
            })
            .collect();

        let chunks_equal: Vec<F> = (0..self.gate.num_chunks)
            .map(|i| F::from_bool(first_input_chunks[i] == second_input_chunks[i]))
            .collect();
        let equality_dummies: Vec<F> = first_input_chunks
            .iter()
            .zip(second_input_chunks.iter())
            .map(|(f, s)| if *f == *s { F::ONE } else { F::ONE / (*f - *s) })
            .collect();

        let z = F::TWO.exp_u64(self.gate.chunk_bits() as u64) + first_input - second_input;
        let z_bits: Vec<F> = (0..self.gate.chunk_bits() + 1)
            .scan(z.to_canonical_u64(), |acc, _| {
                let tmp = *acc % 2;
                *acc /= 2;
                Some(F::from_canonical_u64(tmp))
            })
            .collect();

        out_buffer.set_wire(local_wire(self.gate.wire_z_val(self.copy)), z);
        for b in 0..self.gate.chunk_bits() + 1 {
            out_buffer.set_wire(local_wire(self.gate.wire_z_bit(self.copy, b)), z_bits[b]);
        }
        for i in 0..self.gate.num_chunks {
            out_buffer.set_wire(
                local_wire(self.gate.wire_first_chunk_val(self.copy, i)),
                first_input_chunks[i],
            );
            out_buffer.set_wire(
                local_wire(self.gate.wire_second_chunk_val(self.copy, i)),
                second_input_chunks[i],
            );
            out_buffer.set_wire(
                local_wire(self.gate.wire_chunks_equal(self.copy, i)),
                chunks_equal[i],
            );
            out_buffer.set_wire(
                local_wire(self.gate.wire_equality_dummy(self.copy, i)),
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
    use crate::plonk::circuit_data::CircuitConfig;
    use crate::plonk::vars::EvaluationVars;

    #[test]
    fn wire_indices() {
        type CG = ComparisonGate<CrandallField, 4>;
        let num_bits = 40;
        let num_copies = 3;
        let num_chunks = 5;

        let gate = CG {
            num_bits,
            num_chunks,
            num_copies,
            _phantom: PhantomData,
        };

        assert_eq!(gate.wire_first_input(0), 0);
        assert_eq!(gate.wire_second_input(0), 1);
        assert_eq!(gate.wire_z_val(0), 2);
        assert_eq!(gate.wire_z_bit(0, 0), 3);
        assert_eq!(gate.wire_z_bit(0, 8), 11);
        assert_eq!(gate.wire_first_chunk_val(0, 0), 12);
        assert_eq!(gate.wire_first_chunk_val(0, 4), 16);
        assert_eq!(gate.wire_second_chunk_val(0, 0), 17);
        assert_eq!(gate.wire_second_chunk_val(0, 4), 21);
        assert_eq!(gate.wire_equality_dummy(0, 0), 22);
        assert_eq!(gate.wire_equality_dummy(0, 4), 26);
        assert_eq!(gate.wire_chunks_equal(0, 0), 27);
        assert_eq!(gate.wire_chunks_equal(0, 4), 31);
        assert_eq!(gate.wire_first_input(1), 32);
        assert_eq!(gate.wire_chunks_equal(1, 4), 63);
        assert_eq!(gate.wire_first_input(2), 64);
        assert_eq!(gate.wire_chunks_equal(2, 4), 95);
    }

    #[test]
    fn low_degree() {
        let num_bits = 40;
        let num_chunks = 5;

        test_low_degree::<CrandallField, _, 4>(ComparisonGate::<_, 4>::new_from_config(
            CircuitConfig::large_config(),
            num_bits,
            num_chunks,
        ))
    }

    #[test]
    fn eval_fns() -> Result<()> {
        let num_bits = 40;
        let num_chunks = 5;

        test_eval_fns::<CrandallField, _, 4>(ComparisonGate::<_, 4>::new_from_config(
            CircuitConfig::large_config(),
            num_bits,
            num_chunks,
        ))
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

                let powers_of_two: Vec<F> =
                    (0..chunk_bits).map(|i| F::TWO.exp_u64(i as u64)).collect();
                let mut first_input_chunks: Vec<F> = first_input_bits
                    .chunks(chunk_bits)
                    .map(|bits| {
                        bits.iter()
                            .zip(powers_of_two.iter())
                            .map(|(b, x)| *b * *x)
                            .fold(F::ZERO, |a, b| a + b)
                    })
                    .collect();
                let mut second_input_chunks: Vec<F> = second_input_bits
                    .chunks(chunk_bits)
                    .map(|bits| {
                        bits.iter()
                            .zip(powers_of_two.iter())
                            .map(|(b, x)| *b * *x)
                            .fold(F::ZERO, |a, b| a + b)
                    })
                    .collect();

                let mut chunks_equal: Vec<F> = (0..num_chunks)
                    .map(|i| F::from_bool(first_input_chunks[i] == second_input_chunks[i]))
                    .collect();
                let mut equality_dummies: Vec<F> = first_input_chunks
                    .iter()
                    .zip(second_input_chunks.iter())
                    .map(|(f, s)| if *f == *s { F::ONE } else { F::ONE / (*f - *s) })
                    .collect();

                let z = F::TWO.exp_u64(chunk_bits as u64) + first_input - second_input;
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
                v.append(&mut first_input_chunks);
                v.append(&mut second_input_chunks);
                v.append(&mut z_bits);
                v.append(&mut equality_dummies);
                v.append(&mut chunks_equal);
            }

            v.iter().map(|&x| x.into()).collect::<Vec<_>>()
        };

        let mut rng = rand::thread_rng();
        let max: u64 = 1 << num_bits - 1;
        let first_inputs = (0..num_copies)
            .map(|_| F::from_canonical_u64(rng.gen_range(0..max)))
            .collect();
        let second_inputs = (0..num_copies)
            .map(|_| F::from_canonical_u64(rng.gen_range(0..max)))
            .collect();

        let gate = ComparisonGate::<F, D> {
            num_copies,
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
