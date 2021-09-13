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
use crate::util::{ceil_div_usize, log2_ceil};

/// A gate for checking that one value is smaller than another.
#[derive(Clone, Debug)]
pub(crate) struct ComparisonGate<F: PrimeField + Extendable<D>, const D: usize> {
    pub(crate) chunk_bits: usize,
    pub(crate) num_copies: usize,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> ComparisonGate<F, D> {
    pub fn new(num_copies: usize, chunk_bits: usize) -> Self {
        Self {
            chunk_bits,
            num_copies,
            _phantom: PhantomData,
        }
    }

    pub fn field_bits() -> usize {
        log2_ceil(F::ORDER)
    }

    pub fn num_chunks(&self) -> usize {
        ceil_div_usize(Self::field_bits(), self.chunk_bits)
    }

    pub fn new_from_config(config: CircuitConfig, chunk_bits: usize) -> Self {
        let num_copies = Self::max_num_copies(config.num_routed_wires, chunk_bits);
        Self::new(num_copies, chunk_bits)
    }

    pub fn max_num_copies(num_routed_wires: usize, chunk_bits: usize) -> usize {
        let num_chunks = ceil_div_usize(Self::field_bits(), chunk_bits);
        let wires_per_copy = 4 + chunk_bits + 4 * num_chunks;
        num_routed_wires / wires_per_copy
    }

    pub fn wire_first_input(&self, copy: usize) -> usize {
        debug_assert!(copy < self.num_copies);
        copy * (4 + self.chunk_bits + 4 * self.num_chunks())
    }

    pub fn wire_second_input(&self, copy: usize) -> usize {
        debug_assert!(copy < self.num_copies);
        copy * (4 + self.chunk_bits + 4 * self.num_chunks()) + 1
    }

    pub fn wire_z_val(&self, copy: usize) -> usize {
        copy * (4 + self.chunk_bits + 4 * self.num_chunks()) + 3
    }

    pub fn wire_z_bit(&self, copy: usize, bit_index: usize) -> usize {
        debug_assert!(bit_index < self.chunk_bits + 1);
        copy * (4 + self.chunk_bits + 4 * self.num_chunks()) + 4 + bit_index
    }

    pub fn wire_first_chunk_val(&self, copy: usize, chunk: usize) -> usize {
        debug_assert!(copy < self.num_copies);
        debug_assert!(chunk < self.num_chunks());
        copy * (4 + self.chunk_bits + 4 * self.num_chunks()) + 4 + self.chunk_bits + chunk
    }

    pub fn wire_second_chunk_val(&self, copy: usize, chunk: usize) -> usize {
        debug_assert!(copy < self.num_copies);
        debug_assert!(chunk < self.num_chunks());
        copy * (4 + self.chunk_bits + 4 * self.num_chunks())
            + 4
            + self.chunk_bits
            + self.num_chunks()
            + chunk
    }

    pub fn wire_equality_dummy(&self, copy: usize, chunk: usize) -> usize {
        debug_assert!(copy < self.num_copies);
        debug_assert!(chunk < self.num_chunks());
        copy * (4 + self.chunk_bits + 4 * self.num_chunks())
            + 4
            + self.chunk_bits
            + 2 * self.num_chunks()
            + chunk
    }

    pub fn wire_chunks_equal(&self, copy: usize, chunk: usize) -> usize {
        debug_assert!(copy < self.num_copies);
        debug_assert!(chunk < self.num_chunks());
        copy * (4 + self.chunk_bits + 4 * self.num_chunks())
            + 4
            + self.chunk_bits
            + 3 * self.num_chunks()
            + chunk
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
            let first_chunks: Vec<F> = (0..self.num_chunks())
                .map(|i| vars.local_wires[self.wire_first_chunk_val(c, i)])
                .collect();
            let second_chunks: Vec<F> = (0..self.num_chunks())
                .map(|i| vars.local_wires[self.wire_second_chunk_val(c, i)])
                .collect();

            let chunk_base_powers = (0..self.chunk_bits)
                .map(|i| F::TWO.exp_u64(i * self.chunk_bits as u64))
                .collect();

            let first_chunks_combined = first_chunks
                .iter()
                .zip(chunk_base_powers.iter())
                .map(|(b, x)| b * x)
                .fold(F::ZERO, |a, b| a + b);
            let second_chunks_combined = second_chunks
                .iter()
                .zip(chunk_base_powers.iter())
                .map(|(b, x)| b * x)
                .fold(F::ZERO, |a, b| a + b);

            constraints.push(first_chunks_combined - first_input);
            constraints.push(second_chunks_combined - second_input);

            // Get bits to assert they match the chosen chunk.
            let powers_of_two: Vec<F> = (0..self.chunk_bits)
                .map(|i| F::TWO.exp_u64(i as u64))
                .collect();

            let mut most_significant_diff =
                first_chunks[self.num_chunks() - 1] - second_chunks[self.num_chunks() - 1];

            // Find the chosen chunk.
            for i in (0..self.num_chunks()).rev() {
                let difference = first_chunks[i] - second_chunks[i];
                let equality_dummy = vars.local_wires[self.wire_equality_dummy(c, i)];
                let chunks_equal = vars.local_wires[self.wires_chunks_equal(c, i)];

                // Two constraints identifying index.
                constraints.push(difference * equality_dummy - (F::Extension::ONE - chunks_equal));
                constraints.push(chunks_equal * difference);

                let this_diff = first_chunks[i] - second_chunks[i];
                most_significant_diff = chunks_equal * most_significant_diff
                    + (F::Extension::ONE - chunks_equal) * this_diff;
            }

            constraints.push(first_bits_combined - most_significant_diff[0]);
            constraints.push(second_bits_combined - most_significant_diff[1]);

            let z_bits: Vec<F> = (0..self.chunk_size + 1)
                .map(|i| vars.local_wires[self.wire_z_bit(c, i)])
                .collect();

            let powers_of_two: Vec<F> = (0..self.chunk_bits + 1)
                .map(|i| F::TWO.exp_u64(i as u64))
                .collect();
            let z_bits_combined = z_bits
                .iter()
                .zip(powers_of_two.iter())
                .map(|(b, x)| b * x)
                .fold(F::ZERO, |a, b| a + b);

            let two_n = F::TWO.exp_u64(self.chunk_bits);
            let (x, y) = most_significant_diff;
            constraints.push(z_bits_combined - (two_n + x - y));

            constraints.push(z_bits[self.chunk_bits - 1]);
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
                let g: Box<dyn WitnessGenerator<F>> = Box::new(ComparisonGenerator::<F, D> {
                    gate_index,
                    gate: self.clone(),
                    copy: c,
                });
                g
            })
            .collect()
    }

    fn num_wires(&self) -> usize {
        self.wire_switch_bool(self.num_copies - 1) + 1
    }

    fn num_constants(&self) -> usize {
        0
    }

    fn degree(&self) -> usize {
        2
    }

    fn num_constraints(&self) -> usize {
        4 * self.num_copies * self.chunk_bits
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

        let field_bits = log2_ceil(F::ORDER);
        let first_input_u64 = first_input.to_canonical_u64();
        let second_input_u64 = second_input.to_canonical_u64();

        let first_input_bits: Vec<F> = (0..field_bits)
            .scan(first_input_u64, |acc, _| {
                let tmp = *acc % 2;
                *acc /= 2;
                Some(F::from_canonical_u64(tmp))
            })
            .collect();
        let second_input_bits: Vec<F> = (0..field_bits)
            .scan(second_input_u64, |acc, _| {
                let tmp = *acc % 2;
                *acc /= 2;
                Some(F::from_canonical_u64(tmp))
            })
            .collect();

        let powers_of_two: Vec<F> = (0..self.gate.chunk_bits)
            .map(|i| F::TWO.exp_u64(i as u64))
            .collect();
        let first_input_chunks: Vec<F> = first_input_bits
            .chunks(self.gate.chunk_bits)
            .map(|bits| {
                bits.iter()
                    .zip(powers_of_two.iter())
                    .map(|(b, x)| *b * *x)
                    .fold(F::ZERO, |a, b| a + b)
            })
            .collect();
        let second_input_chunks: Vec<F> = second_input_bits
            .chunks(self.gate.chunk_bits)
            .map(|bits| {
                bits.iter()
                    .zip(powers_of_two.iter())
                    .map(|(b, x)| *b * *x)
                    .fold(F::ZERO, |a, b| a + b)
            })
            .collect();

        let chunks_equal: Vec<F> = (0..self.gate.num_chunks())
            .map(|i| F::from_bool(first_input_chunks[i] == second_input_chunks[i]))
            .collect();
        let equality_dummies: Vec<F> = first_input_chunks
            .iter()
            .zip(second_input_chunks.iter())
            .map(|(f, s)| if *f == *s { F::ONE } else { F::ONE / (*f - *s) })
            .collect();

        let z = F::TWO.exp_u64(self.gate.chunk_bits as u64) + first_input - second_input;
        let z_bits: Vec<F> = (0..self.gate.chunk_bits + 1)
            .scan(z.to_canonical_u64(), |acc, _| {
                let tmp = *acc % 2;
                *acc /= 2;
                Some(F::from_canonical_u64(tmp))
            })
            .collect();

        out_buffer.set_wire(local_wire(self.gate.wire_z_val(self.copy)), z);
        for b in 0..self.gate.chunk_bits + 1 {
            out_buffer.set_wire(local_wire(self.gate.wire_z_bit(c, b)), z_bits[b]);
        }
        for i in 0..self.gate.num_chunks() {
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

    use crate::field::crandall_field::CrandallField;
    use crate::field::extension_field::quartic::QuarticCrandallField;
    use crate::field::field_types::Field;
    use crate::gates::comparison::ComparisonGate;
    use crate::gates::gate::Gate;
    use crate::gates::gate_testing::{test_eval_fns, test_low_degree};
    use crate::hash::hash_types::HashOut;
    use crate::plonk::circuit_data::CircuitConfig;
    use crate::plonk::vars::EvaluationVars;

    #[test]
    fn wire_indices() {
        type CG = ComparisonGate<CrandallField, 4>;
        let num_copies = 3;
        let chunk_bits = 3;

        let gate = CG {
            chunk_bits,
            num_copies,
            _phantom: PhantomData,
        };

        assert_eq!(gate.wire_first_input(0, 0), 0);
        assert_eq!(gate.wire_first_input(0, 2), 2);
        assert_eq!(gate.wire_second_input(0, 0), 3);
        assert_eq!(gate.wire_second_input(0, 2), 5);
        assert_eq!(gate.wire_first_output(0, 0), 6);
        assert_eq!(gate.wire_second_output(0, 2), 11);
        assert_eq!(gate.wire_switch_bool(0), 12);
        assert_eq!(gate.wire_first_input(1, 0), 13);
        assert_eq!(gate.wire_second_output(1, 2), 24);
        assert_eq!(gate.wire_switch_bool(1), 25);
        assert_eq!(gate.wire_first_input(2, 0), 26);
        assert_eq!(gate.wire_second_output(2, 2), 37);
        assert_eq!(gate.wire_switch_bool(2), 38);
    }

    #[test]
    fn low_degree() {
        test_low_degree::<CrandallField, _, 4>(SwitchGate::<_, 4>::new_from_config(
            CircuitConfig::large_config(),
            3,
        ));
    }

    #[test]
    fn eval_fns() -> Result<()> {
        test_eval_fns::<CrandallField, _, 4>(SwitchGate::<_, 4>::new_from_config(
            CircuitConfig::large_config(),
            3,
        ))
    }

    #[test]
    fn test_gate_constraint() {
        type F = CrandallField;
        type FF = QuarticCrandallField;
        const D: usize = 4;
        const CHUNK_SIZE: usize = 4;
        let num_copies = 3;

        /// Returns the local wires for a switch gate given the inputs and the switch booleans.
        fn get_wires(
            first_inputs: Vec<Vec<F>>,
            second_inputs: Vec<Vec<F>>,
            switch_bools: Vec<bool>,
        ) -> Vec<FF> {
            let num_copies = first_inputs.len();

            let mut v = Vec::new();
            for c in 0..num_copies {
                let switch = switch_bools[c];

                let mut first_input_chunk = Vec::with_capacity(CHUNK_SIZE);
                let mut second_input_chunk = Vec::with_capacity(CHUNK_SIZE);
                let mut first_output_chunk = Vec::with_capacity(CHUNK_SIZE);
                let mut second_output_chunk = Vec::with_capacity(CHUNK_SIZE);
                for e in 0..CHUNK_SIZE {
                    let first_input = first_inputs[c][e];
                    let second_input = second_inputs[c][e];
                    let first_output = if switch { second_input } else { first_input };
                    let second_output = if switch { first_input } else { second_input };
                    first_input_chunk.push(first_input);
                    second_input_chunk.push(second_input);
                    first_output_chunk.push(first_output);
                    second_output_chunk.push(second_output);
                }
                v.append(&mut first_input_chunk);
                v.append(&mut second_input_chunk);
                v.append(&mut first_output_chunk);
                v.append(&mut second_output_chunk);

                v.push(F::from_bool(switch));
            }

            v.iter().map(|&x| x.into()).collect::<Vec<_>>()
        }

        let first_inputs: Vec<Vec<F>> = (0..num_copies).map(|_| F::rand_vec(CHUNK_SIZE)).collect();
        let second_inputs: Vec<Vec<F>> = (0..num_copies).map(|_| F::rand_vec(CHUNK_SIZE)).collect();
        let switch_bools = vec![true, false, true];

        let gate = SwitchGate::<F, D> {
            chunk_bits: CHUNK_SIZE,
            num_copies,
            _phantom: PhantomData,
        };

        let vars = EvaluationVars {
            local_constants: &[],
            local_wires: &get_wires(first_inputs, second_inputs, switch_bools),
            public_inputs_hash: &HashOut::rand(),
        };

        assert!(
            gate.eval_unfiltered(vars).iter().all(|x| x.is_zero()),
            "Gate constraints are not satisfied."
        );
    }
}
