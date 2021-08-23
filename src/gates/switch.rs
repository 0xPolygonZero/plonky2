use std::marker::PhantomData;
use std::ops::Range;

use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::field::field_types::Field;
use crate::gates::gate::Gate;
use crate::iop::generator::{GeneratedValues, SimpleGenerator, WitnessGenerator};
use crate::iop::target::Target;
use crate::iop::wire::Wire;
use crate::iop::witness::PartialWitness;
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::circuit_data::CircuitConfig;
use crate::plonk::vars::{EvaluationTargets, EvaluationVars, EvaluationVarsBase};

/// A gate for conditionally swapping input values based on a boolean.
#[derive(Clone, Debug)]
pub(crate) struct SwitchGate<F: Extendable<D>, const D: usize, const CHUNK_SIZE: usize> {
    num_copies: usize,
    _phantom: PhantomData<F>,
}

impl<F: Extendable<D>, const D: usize, const CHUNK_SIZE: usize> SwitchGate<F, D, CHUNK_SIZE> {
    pub fn new(num_copies: usize) -> Self {
        Self {
            num_copies,
            _phantom: PhantomData,
        }
    }

    pub fn new_from_config(config: CircuitConfig) -> Self {
        let num_copies = Self::max_num_copies(config.num_routed_wires);
        Self::new(num_copies)
    }

    fn max_num_copies(num_routed_wires: usize) -> usize {
        num_routed_wires / (4 * CHUNK_SIZE)
    }

    pub fn wire_first_input(&self, copy: usize, element: usize) -> usize {
        debug_assert!(copy < self.num_copies);
        debug_assert!(element < CHUNK_SIZE);
        copy * (4 * CHUNK_SIZE) + element
    }

    pub fn wire_second_input(&self, copy: usize, element: usize) -> usize {
        debug_assert!(copy < self.num_copies);
        debug_assert!(element < CHUNK_SIZE);
        copy * (4 * CHUNK_SIZE) + CHUNK_SIZE + element
    }

    pub fn wire_first_output(&self, copy: usize, element: usize) -> usize {
        debug_assert!(copy < self.num_copies);
        debug_assert!(element < CHUNK_SIZE);
        copy * (4 * CHUNK_SIZE) + 2 * CHUNK_SIZE + element
    }

    pub fn wire_second_output(&self, copy: usize, element: usize) -> usize {
        debug_assert!(copy < self.num_copies);
        debug_assert!(element < CHUNK_SIZE);
        copy * (4 * CHUNK_SIZE) + 3 * CHUNK_SIZE + element
    }

    pub fn wire_switch_bool(&self, copy: usize) -> usize {
        debug_assert!(copy < self.num_copies);
        self.num_copies * (4 * CHUNK_SIZE) + copy
    }
}

impl<F: Extendable<D>, const D: usize, const CHUNK_SIZE: usize> Gate<F, D>
    for SwitchGate<F, D, CHUNK_SIZE>
{
    fn id(&self) -> String {
        format!("{:?}<D={},CHUNK_SIZE={}>", self, D, CHUNK_SIZE)
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        let mut constraints = Vec::with_capacity(self.num_constraints());

        for c in 0..self.num_copies {
            let switch_bool = vars.local_wires[self.wire_switch_bool(c)];
            let not_switch = F::Extension::ONE - switch_bool;

            for e in 0..CHUNK_SIZE {
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

    fn eval_unfiltered_base(&self, vars: EvaluationVarsBase<F>) -> Vec<F> {
        let mut constraints = Vec::with_capacity(self.num_constraints());

        for c in 0..self.num_copies {
            let switch_bool = vars.local_wires[self.wire_switch_bool(c)];
            let not_switch = F::ONE - switch_bool;

            for e in 0..CHUNK_SIZE {
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

    fn eval_unfiltered_recursively(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>> {
        let mut constraints = Vec::with_capacity(self.num_constraints());

        let one = builder.one_extension();
        for c in 0..self.num_copies {
            let switch_bool = vars.local_wires[self.wire_switch_bool(c)];
            let not_switch = builder.sub_extension(one, switch_bool);

            for e in 0..CHUNK_SIZE {
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

    fn generators(
        &self,
        gate_index: usize,
        _local_constants: &[F],
    ) -> Vec<Box<dyn WitnessGenerator<F>>> {
        let gen = SwitchGenerator::<F, D, CHUNK_SIZE> {
            gate_index,
            gate: self.clone(),
        };
        vec![Box::new(gen)]
    }

    fn num_wires(&self) -> usize {
        self.wire_second_output(self.num_copies - 1, CHUNK_SIZE - 1) + 1
    }

    fn num_constants(&self) -> usize {
        0
    }

    fn degree(&self) -> usize {
        2
    }

    fn num_constraints(&self) -> usize {
        4 * self.num_copies * CHUNK_SIZE
    }
}

#[derive(Debug)]
struct SwitchGenerator<F: Extendable<D>, const D: usize, const CHUNK_SIZE: usize> {
    gate_index: usize,
    gate: SwitchGate<F, D, CHUNK_SIZE>,
}

impl<F: Extendable<D>, const D: usize, const CHUNK_SIZE: usize> SimpleGenerator<F>
    for SwitchGenerator<F, D, CHUNK_SIZE>
{
    fn dependencies(&self) -> Vec<Target> {
        let local_target = |input| Target::wire(self.gate_index, input);

        let local_targets = |inputs: Range<usize>| inputs.map(local_target);

        let mut deps = Vec::new();
        for c in 0..self.gate.num_copies {
            for e in 0..CHUNK_SIZE {
                deps.push(local_target(self.gate.wire_first_input(c, e)));
                deps.push(local_target(self.gate.wire_second_input(c, e)));
                deps.push(local_target(self.gate.wire_first_output(c, e)));
                deps.push(local_target(self.gate.wire_second_output(c, e)));
            }
        }

        deps
    }

    fn run_once(&self, witness: &PartialWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let local_wire = |input| Wire {
            gate: self.gate_index,
            input,
        };

        let get_local_wire = |input| witness.get_wire(local_wire(input));

        for c in 0..self.gate.num_copies {
            let switch_bool_wire = local_wire(self.gate.wire_switch_bool(c));
            for e in 0..CHUNK_SIZE {
                let first_input = get_local_wire(self.gate.wire_first_input(c, e));
                let second_input = get_local_wire(self.gate.wire_second_input(c, e));
                let first_output = get_local_wire(self.gate.wire_first_output(c, e));
                let second_output = get_local_wire(self.gate.wire_second_output(c, e));

                if first_input == first_output {
                    out_buffer.set_wire(switch_bool_wire, F::ONE);
                } else {
                    out_buffer.set_wire(switch_bool_wire, F::ZERO);
                }
            }
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
    use crate::gates::gate::Gate;
    use crate::gates::gate_testing::{test_eval_fns, test_low_degree};
    use crate::gates::switch::SwitchGate;
    use crate::hash::hash_types::HashOut;
    use crate::plonk::circuit_data::CircuitConfig;
    use crate::plonk::vars::EvaluationVars;

    #[test]
    fn wire_indices() {
        let gate = SwitchGate::<CrandallField, 4, 3> {
            num_copies: 3,
            _phantom: PhantomData,
        };

        assert_eq!(gate.wire_first_input(0, 0), 0);
        assert_eq!(gate.wire_first_input(0, 2), 2);
        assert_eq!(gate.wire_second_input(0, 0), 3);
        assert_eq!(gate.wire_second_input(0, 2), 5);
        assert_eq!(gate.wire_first_output(0, 0), 6);
        assert_eq!(gate.wire_second_output(0, 2), 11);
        assert_eq!(gate.wire_first_input(1, 0), 12);
        assert_eq!(gate.wire_second_output(1, 2), 23);
        assert_eq!(gate.wire_first_input(2, 0), 24);
        assert_eq!(gate.wire_second_output(2, 2), 35);
        assert_eq!(gate.wire_switch_bool(0), 36);
        assert_eq!(gate.wire_switch_bool(1), 37);
        assert_eq!(gate.wire_switch_bool(2), 38);
    }

    #[test]
    fn low_degree() {
        test_low_degree::<CrandallField, _, 4>(SwitchGate::<_, 4, 3>::new(
            CircuitConfig::large_config(),
        ));
    }

    #[test]
    fn eval_fns() -> Result<()> {
        test_eval_fns::<CrandallField, _, 4>(SwitchGate::<_, 4, 3>::new(
            CircuitConfig::large_config(),
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
                v.push(F::from_bool(switch));
                for e in 0..CHUNK_SIZE {
                    let first_input = first_inputs[c][e];
                    let second_input = second_inputs[c][e];
                    let first_output = if switch { second_input } else { first_input };
                    let second_output = if switch { first_input } else { second_input };
                    v.push(first_input);
                    v.push(second_input);
                    v.push(first_output);
                    v.push(second_output);
                }
            }

            v.iter().map(|&x| x.into()).collect::<Vec<_>>()
        }

        let first_inputs: Vec<Vec<F>> = (0..num_copies).map(|_| F::rand_vec(CHUNK_SIZE)).collect();
        let second_inputs: Vec<Vec<F>> = (0..num_copies).map(|_| F::rand_vec(CHUNK_SIZE)).collect();
        let switch_bools = vec![true, false, true];

        let gate = SwitchGate::<F, D, CHUNK_SIZE> {
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
