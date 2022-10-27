use std::marker::PhantomData;

use array_tool::vec::Union;
use plonky2::gates::gate::Gate;
use plonky2::gates::packed_util::PackedEvaluableBase;
use plonky2::gates::util::StridedConstraintConsumer;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::iop::generator::{GeneratedValues, WitnessGenerator};
use plonky2::iop::target::Target;
use plonky2::iop::wire::Wire;
use plonky2::iop::witness::{PartitionWitness, Witness};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::circuit_data::CircuitConfig;
use plonky2::plonk::vars::{
    EvaluationTargets, EvaluationVars, EvaluationVarsBase, EvaluationVarsBaseBatch,
    EvaluationVarsBasePacked,
};
use plonky2_field::extension::Extendable;
use plonky2_field::packed::PackedField;
use plonky2_field::types::Field;

/// A gate for conditionally swapping input values based on a boolean.
#[derive(Copy, Clone, Debug)]
pub struct SwitchGate<F: RichField + Extendable<D>, const D: usize> {
    pub(crate) chunk_size: usize,
    pub(crate) num_copies: usize,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> SwitchGate<F, D> {
    pub fn new(num_copies: usize, chunk_size: usize) -> Self {
        Self {
            chunk_size,
            num_copies,
            _phantom: PhantomData,
        }
    }

    pub fn new_from_config(config: &CircuitConfig, chunk_size: usize) -> Self {
        let num_copies = Self::max_num_copies(config.num_routed_wires, chunk_size);
        Self::new(num_copies, chunk_size)
    }

    pub fn max_num_copies(num_routed_wires: usize, chunk_size: usize) -> usize {
        num_routed_wires / (4 * chunk_size + 1)
    }

    pub fn wire_first_input(&self, copy: usize, element: usize) -> usize {
        debug_assert!(element < self.chunk_size);
        copy * (4 * self.chunk_size + 1) + element
    }

    pub fn wire_second_input(&self, copy: usize, element: usize) -> usize {
        debug_assert!(element < self.chunk_size);
        copy * (4 * self.chunk_size + 1) + self.chunk_size + element
    }

    pub fn wire_first_output(&self, copy: usize, element: usize) -> usize {
        debug_assert!(element < self.chunk_size);
        copy * (4 * self.chunk_size + 1) + 2 * self.chunk_size + element
    }

    pub fn wire_second_output(&self, copy: usize, element: usize) -> usize {
        debug_assert!(element < self.chunk_size);
        copy * (4 * self.chunk_size + 1) + 3 * self.chunk_size + element
    }

    pub fn wire_switch_bool(&self, copy: usize) -> usize {
        debug_assert!(copy < self.num_copies);
        copy * (4 * self.chunk_size + 1) + 4 * self.chunk_size
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Gate<F, D> for SwitchGate<F, D> {
    fn id(&self) -> String {
        format!("{self:?}<D={D}>")
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        let mut constraints = Vec::with_capacity(self.num_constraints());

        for c in 0..self.num_copies {
            let switch_bool = vars.local_wires[self.wire_switch_bool(c)];
            let not_switch = F::Extension::ONE - switch_bool;

            for e in 0..self.chunk_size {
                let first_input = vars.local_wires[self.wire_first_input(c, e)];
                let second_input = vars.local_wires[self.wire_second_input(c, e)];
                let first_output = vars.local_wires[self.wire_first_output(c, e)];
                let second_output = vars.local_wires[self.wire_second_output(c, e)];

                constraints.push(switch_bool * (first_input - second_output));
                constraints.push(switch_bool * (second_input - first_output));
                constraints.push(not_switch * (first_input - first_output));
                constraints.push(not_switch * (second_input - second_output));
            }
        }

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

        let one = builder.one_extension();
        for c in 0..self.num_copies {
            let switch_bool = vars.local_wires[self.wire_switch_bool(c)];
            let not_switch = builder.sub_extension(one, switch_bool);

            for e in 0..self.chunk_size {
                let first_input = vars.local_wires[self.wire_first_input(c, e)];
                let second_input = vars.local_wires[self.wire_second_input(c, e)];
                let first_output = vars.local_wires[self.wire_first_output(c, e)];
                let second_output = vars.local_wires[self.wire_second_output(c, e)];

                let first_switched = builder.sub_extension(first_input, second_output);
                let first_switched_constraint = builder.mul_extension(switch_bool, first_switched);
                constraints.push(first_switched_constraint);

                let second_switched = builder.sub_extension(second_input, first_output);
                let second_switched_constraint =
                    builder.mul_extension(switch_bool, second_switched);
                constraints.push(second_switched_constraint);

                let first_not_switched = builder.sub_extension(first_input, first_output);
                let first_not_switched_constraint =
                    builder.mul_extension(not_switch, first_not_switched);
                constraints.push(first_not_switched_constraint);

                let second_not_switched = builder.sub_extension(second_input, second_output);
                let second_not_switched_constraint =
                    builder.mul_extension(not_switch, second_not_switched);
                constraints.push(second_not_switched_constraint);
            }
        }

        constraints
    }

    fn generators(&self, row: usize, _local_constants: &[F]) -> Vec<Box<dyn WitnessGenerator<F>>> {
        (0..self.num_copies)
            .map(|c| {
                let g: Box<dyn WitnessGenerator<F>> = Box::new(SwitchGenerator::<F, D> {
                    row,
                    gate: *self,
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
        4 * self.num_copies * self.chunk_size
    }
}

impl<F: RichField + Extendable<D>, const D: usize> PackedEvaluableBase<F, D> for SwitchGate<F, D> {
    fn eval_unfiltered_base_packed<P: PackedField<Scalar = F>>(
        &self,
        vars: EvaluationVarsBasePacked<P>,
        mut yield_constr: StridedConstraintConsumer<P>,
    ) {
        for c in 0..self.num_copies {
            let switch_bool = vars.local_wires[self.wire_switch_bool(c)];
            let not_switch = P::ONES - switch_bool;

            for e in 0..self.chunk_size {
                let first_input = vars.local_wires[self.wire_first_input(c, e)];
                let second_input = vars.local_wires[self.wire_second_input(c, e)];
                let first_output = vars.local_wires[self.wire_first_output(c, e)];
                let second_output = vars.local_wires[self.wire_second_output(c, e)];

                yield_constr.one(switch_bool * (first_input - second_output));
                yield_constr.one(switch_bool * (second_input - first_output));
                yield_constr.one(not_switch * (first_input - first_output));
                yield_constr.one(not_switch * (second_input - second_output));
            }
        }
    }
}

#[derive(Debug)]
struct SwitchGenerator<F: RichField + Extendable<D>, const D: usize> {
    row: usize,
    gate: SwitchGate<F, D>,
    copy: usize,
}

impl<F: RichField + Extendable<D>, const D: usize> SwitchGenerator<F, D> {
    fn in_out_dependencies(&self) -> Vec<Target> {
        let local_target = |column| Target::wire(self.row, column);

        let mut deps = Vec::new();
        for e in 0..self.gate.chunk_size {
            deps.push(local_target(self.gate.wire_first_input(self.copy, e)));
            deps.push(local_target(self.gate.wire_second_input(self.copy, e)));
            deps.push(local_target(self.gate.wire_first_output(self.copy, e)));
            deps.push(local_target(self.gate.wire_second_output(self.copy, e)));
        }

        deps
    }

    fn in_switch_dependencies(&self) -> Vec<Target> {
        let local_target = |column| Target::wire(self.row, column);

        let mut deps = Vec::new();
        for e in 0..self.gate.chunk_size {
            deps.push(local_target(self.gate.wire_first_input(self.copy, e)));
            deps.push(local_target(self.gate.wire_second_input(self.copy, e)));
            deps.push(local_target(self.gate.wire_switch_bool(self.copy)));
        }

        deps
    }

    fn run_in_out(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let local_wire = |column| Wire {
            row: self.row,
            column,
        };

        let get_local_wire = |column| witness.get_wire(local_wire(column));

        let switch_bool_wire = local_wire(self.gate.wire_switch_bool(self.copy));

        let mut first_inputs = Vec::new();
        let mut second_inputs = Vec::new();
        let mut first_outputs = Vec::new();
        let mut second_outputs = Vec::new();
        for e in 0..self.gate.chunk_size {
            first_inputs.push(get_local_wire(self.gate.wire_first_input(self.copy, e)));
            second_inputs.push(get_local_wire(self.gate.wire_second_input(self.copy, e)));
            first_outputs.push(get_local_wire(self.gate.wire_first_output(self.copy, e)));
            second_outputs.push(get_local_wire(self.gate.wire_second_output(self.copy, e)));
        }

        if first_outputs == first_inputs && second_outputs == second_inputs {
            out_buffer.set_wire(switch_bool_wire, F::ZERO);
        } else if first_outputs == second_inputs && second_outputs == first_inputs {
            out_buffer.set_wire(switch_bool_wire, F::ONE);
        } else {
            panic!("No permutation from given inputs to given outputs");
        }
    }

    fn run_in_switch(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let local_wire = |column| Wire {
            row: self.row,
            column,
        };

        let get_local_wire = |column| witness.get_wire(local_wire(column));

        let switch_bool = get_local_wire(self.gate.wire_switch_bool(self.copy));
        for e in 0..self.gate.chunk_size {
            let first_output_wire = local_wire(self.gate.wire_first_output(self.copy, e));
            let second_output_wire = local_wire(self.gate.wire_second_output(self.copy, e));
            let first_input = get_local_wire(self.gate.wire_first_input(self.copy, e));
            let second_input = get_local_wire(self.gate.wire_second_input(self.copy, e));

            let (first_output, second_output) = if switch_bool == F::ZERO {
                (first_input, second_input)
            } else if switch_bool == F::ONE {
                (second_input, first_input)
            } else {
                panic!("Invalid switch bool value");
            };

            out_buffer.set_wire(first_output_wire, first_output);
            out_buffer.set_wire(second_output_wire, second_output);
        }
    }
}

impl<F: RichField + Extendable<D>, const D: usize> WitnessGenerator<F> for SwitchGenerator<F, D> {
    fn watch_list(&self) -> Vec<Target> {
        self.in_out_dependencies()
            .union(self.in_switch_dependencies())
    }

    fn run(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) -> bool {
        if witness.contains_all(&self.in_out_dependencies()) {
            self.run_in_out(witness, out_buffer);
            true
        } else if witness.contains_all(&self.in_switch_dependencies()) {
            self.run_in_switch(witness, out_buffer);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use anyhow::Result;
    use plonky2::gates::gate::Gate;
    use plonky2::gates::gate_testing::{test_eval_fns, test_low_degree};
    use plonky2::hash::hash_types::HashOut;
    use plonky2::plonk::circuit_data::CircuitConfig;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use plonky2::plonk::vars::EvaluationVars;
    use plonky2_field::goldilocks_field::GoldilocksField;
    use plonky2_field::types::Field;

    use crate::gates::switch::SwitchGate;

    #[test]
    fn wire_indices() {
        type SG = SwitchGate<GoldilocksField, 4>;
        let num_copies = 3;
        let chunk_size = 3;

        let gate = SG {
            chunk_size,
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
        test_low_degree::<GoldilocksField, _, 4>(SwitchGate::<_, 4>::new_from_config(
            &CircuitConfig::standard_recursion_config(),
            3,
        ));
    }

    #[test]
    fn eval_fns() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        test_eval_fns::<F, C, _, D>(SwitchGate::<_, D>::new_from_config(
            &CircuitConfig::standard_recursion_config(),
            3,
        ))
    }

    #[test]
    fn test_gate_constraint() {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type FF = <C as GenericConfig<D>>::FE;
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

            v.iter().map(|&x| x.into()).collect()
        }

        let first_inputs: Vec<Vec<F>> = (0..num_copies).map(|_| F::rand_vec(CHUNK_SIZE)).collect();
        let second_inputs: Vec<Vec<F>> = (0..num_copies).map(|_| F::rand_vec(CHUNK_SIZE)).collect();
        let switch_bools = vec![true, false, true];

        let gate = SwitchGate::<F, D> {
            chunk_size: CHUNK_SIZE,
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
