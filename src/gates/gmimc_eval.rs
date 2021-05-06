use crate::circuit_builder::CircuitBuilder;
use crate::field::field::Field;
use crate::gates::gate::{Gate, GateRef};
use crate::generator::{SimpleGenerator, WitnessGenerator};
use crate::target::Target;
use crate::vars::{EvaluationTargets, EvaluationVars};
use crate::wire::Wire;
use crate::witness::PartialWitness;
use std::marker::PhantomData;

/// Performs some arithmetic involved in the evaluation of GMiMC's constraint polynomials for one
/// round. In particular, this performs the following computations:
///
/// - `constraint := state_a_old + addition_buffer_old + C_r - cubing_input`
/// - `f := cubing_input^3`
/// - `addition_buffer_new := addition_buffer_old + f`
/// - `state_a_new := state_a_old - f`
///
/// Here `state_a_{old,new}` represent the old and new states of the `a`th element of the GMiMC
/// permutation. `addition_buffer_{old,new}` represents a value that is implicitly added to each
/// element; see https://affine.group/2020/02/starkware-challenge. `C_r` represents the round
/// constant for round `r`.
#[derive(Debug)]
pub struct GMiMCEvalGate<F: Field> {
    _phantom: PhantomData<F>,
}

impl<F: Field> GMiMCEvalGate<F> {
    pub fn get() -> GateRef<F> {
        GateRef::new(GMiMCEvalGate {
            _phantom: PhantomData,
        })
    }

    pub const CONST_C_R: usize = 0;

    pub const WIRE_CONSTRAINT: usize = 0;
    pub const WIRE_STATE_A_OLD: usize = 1;
    pub const WIRE_STATE_A_NEW: usize = 2;
    pub const WIRE_ADDITION_BUFFER_OLD: usize = 3;
    pub const WIRE_ADDITION_BUFFER_NEW: usize = 4;
    pub const WIRE_CUBING_INPUT: usize = 5;
    const WIRE_F: usize = 6;
}

impl<F: Field> Gate<F> for GMiMCEvalGate<F> {
    fn id(&self) -> String {
        format!("{:?}", self)
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F>) -> Vec<F> {
        let c_r = vars.local_constants[Self::CONST_C_R];
        let constraint = vars.local_wires[Self::WIRE_CONSTRAINT];
        let state_a_old = vars.local_wires[Self::WIRE_STATE_A_OLD];
        let state_a_new = vars.local_wires[Self::WIRE_STATE_A_NEW];
        let addition_buffer_old = vars.local_wires[Self::WIRE_ADDITION_BUFFER_OLD];
        let addition_buffer_new = vars.local_wires[Self::WIRE_ADDITION_BUFFER_NEW];
        let cubing_input = vars.local_wires[Self::WIRE_CUBING_INPUT];
        let f = vars.local_wires[Self::WIRE_F];

        let mut constraints = Vec::with_capacity(self.num_constraints());

        // constraint := state_a_old + addition_buffer_old + C_r - cubing_input
        let computed_constraint = state_a_old + addition_buffer_old + c_r - cubing_input;
        constraints.push(constraint - computed_constraint);

        // f := cubing_input^3
        let computed_f = cubing_input.cube();
        constraints.push(f - computed_f);

        // addition_buffer_new := addition_buffer_old + f
        let computed_addition_buffer_new = addition_buffer_old + f;
        constraints.push(addition_buffer_new - computed_addition_buffer_new);

        // state_a_new := state_a_old - f
        let computed_state_a_new = state_a_old - f;
        constraints.push(state_a_new - computed_state_a_new);

        constraints
    }

    fn eval_unfiltered_recursively(
        &self,
        builder: &mut CircuitBuilder<F>,
        vars: EvaluationTargets,
    ) -> Vec<Target> {
        let c_r = vars.local_constants[Self::CONST_C_R];
        let constraint = vars.local_wires[Self::WIRE_CONSTRAINT];
        let state_a_old = vars.local_wires[Self::WIRE_STATE_A_OLD];
        let state_a_new = vars.local_wires[Self::WIRE_STATE_A_NEW];
        let addition_buffer_old = vars.local_wires[Self::WIRE_ADDITION_BUFFER_OLD];
        let addition_buffer_new = vars.local_wires[Self::WIRE_ADDITION_BUFFER_NEW];
        let cubing_input = vars.local_wires[Self::WIRE_CUBING_INPUT];
        let f = vars.local_wires[Self::WIRE_F];

        let mut constraints = Vec::with_capacity(self.num_constraints());

        // constraint := state_a_old + addition_buffer_old + C_r - cubing_input
        let sum = builder.add_many(&[state_a_old, addition_buffer_old, c_r]);
        let computed_constraint = builder.sub(sum, cubing_input);
        constraints.push(builder.sub(constraint, computed_constraint));

        // f := cubing_input^3
        let computed_f = builder.cube(cubing_input);
        constraints.push(builder.sub(f, computed_f));

        // addition_buffer_new := addition_buffer_old + f
        let computed_addition_buffer_new = builder.add(addition_buffer_old, f);
        constraints.push(builder.sub(addition_buffer_new, computed_addition_buffer_new));

        // state_a_new := state_a_old - f
        let computed_state_a_new = builder.sub(state_a_old, f);
        constraints.push(builder.sub(state_a_new, computed_state_a_new));

        constraints
    }

    fn generators(
        &self,
        gate_index: usize,
        local_constants: &[F],
    ) -> Vec<Box<dyn WitnessGenerator<F>>> {
        let gen = GMiMCEvalGenerator::<F> {
            gate_index,
            c_r: local_constants[Self::CONST_C_R],
        };
        vec![Box::new(gen)]
    }

    fn num_wires(&self) -> usize {
        7
    }

    fn num_constants(&self) -> usize {
        1
    }

    fn degree(&self) -> usize {
        3
    }

    fn num_constraints(&self) -> usize {
        4
    }
}

#[derive(Debug)]
struct GMiMCEvalGenerator<F: Field> {
    gate_index: usize,
    c_r: F,
}

impl<F: Field> SimpleGenerator<F> for GMiMCEvalGenerator<F> {
    fn dependencies(&self) -> Vec<Target> {
        let gate = self.gate_index;
        vec![
            Target::Wire(Wire {
                gate,
                input: GMiMCEvalGate::<F>::WIRE_CUBING_INPUT,
            }),
            Target::Wire(Wire {
                gate,
                input: GMiMCEvalGate::<F>::WIRE_ADDITION_BUFFER_OLD,
            }),
            Target::Wire(Wire {
                gate,
                input: GMiMCEvalGate::<F>::WIRE_STATE_A_OLD,
            }),
        ]
    }

    fn run_once(&self, witness: &PartialWitness<F>) -> PartialWitness<F> {
        let gate = self.gate_index;
        let wire_constraint = Wire {
            gate,
            input: GMiMCEvalGate::<F>::WIRE_CONSTRAINT,
        };
        let wire_state_a_old = Wire {
            gate,
            input: GMiMCEvalGate::<F>::WIRE_STATE_A_OLD,
        };
        let wire_state_a_new = Wire {
            gate,
            input: GMiMCEvalGate::<F>::WIRE_STATE_A_NEW,
        };
        let wire_addition_buffer_old = Wire {
            gate,
            input: GMiMCEvalGate::<F>::WIRE_ADDITION_BUFFER_OLD,
        };
        let wire_addition_buffer_new = Wire {
            gate,
            input: GMiMCEvalGate::<F>::WIRE_ADDITION_BUFFER_NEW,
        };
        let wire_cubing_input = Wire {
            gate,
            input: GMiMCEvalGate::<F>::WIRE_CUBING_INPUT,
        };
        let wire_f = Wire {
            gate,
            input: GMiMCEvalGate::<F>::WIRE_F,
        };

        let addition_buffer_old = witness.get_wire(wire_addition_buffer_old);
        let state_a_old = witness.get_wire(wire_state_a_old);
        let cubing_input = witness.get_wire(wire_cubing_input);

        // constraint := state_a_old + addition_buffer_old + C_r - cubing_input
        let constraint = state_a_old + addition_buffer_old + self.c_r - cubing_input;

        // f := cubing_input^3
        let f = cubing_input.cube();

        // addition_buffer_new := addition_buffer_old + f
        let addition_buffer_new = addition_buffer_old + f;

        // state_a_new := state_a_old - f
        let state_a_new = state_a_old - f;

        let mut witness = PartialWitness::new();
        witness.set_wire(wire_constraint, constraint);
        witness.set_wire(wire_f, f);
        witness.set_wire(wire_state_a_new, addition_buffer_new);
        witness.set_wire(wire_addition_buffer_new, state_a_new);
        witness
    }
}
