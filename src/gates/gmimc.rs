use std::sync::Arc;

use crate::circuit_builder::CircuitBuilder;
use crate::constraint_polynomial::{EvaluationTargets, EvaluationVars};
use crate::field::field::Field;
use crate::gates::gate::{Gate, GateRef};
use crate::generator::{SimpleGenerator, WitnessGenerator};
use crate::target::Target;
use crate::wire::Wire;
use crate::witness::PartialWitness;

/// The width of the permutation, in field elements.
const W: usize = 12;

/// Evaluates a full GMiMC permutation with 12 state elements, and writes the output to the next
/// gate's first `width` wires (which could be the input of another `GMiMCGate`).
#[derive(Debug)]
pub struct GMiMCGate<F: Field, const R: usize> {
    constants: Arc<[F; R]>,
}

impl<F: Field, const R: usize> GMiMCGate<F, R> {
    pub fn with_constants(constants: Arc<[F; R]>) -> GateRef<F> {
        let gate = GMiMCGate::<F, R> { constants };
        GateRef::new(gate)
    }

    pub fn with_automatic_constants() -> GateRef<F> {
        todo!()
    }

    /// If this is set to 1, the first four inputs will be swapped with the next four inputs. This
    /// is useful for ordering hashes in Merkle proofs. Otherwise, this should be set to 0.
    pub const WIRE_SWITCH: usize = W;

    /// The wire index for the i'th input to the permutation.
    pub fn wire_input(i: usize) -> usize {
        i
    }

    /// The wire index for the i'th output to the permutation.
    /// Note that outputs are written to the next gate's wires.
    pub fn wire_output(i: usize) -> usize {
        i
    }

    /// A wire which stores the input to the `i`th cubing.
    fn wire_cubing_input(i: usize) -> usize {
        W + 1 + i
    }
}

impl<F: Field, const R: usize> Gate<F> for GMiMCGate<F, R> {
    fn id(&self) -> String {
        // TODO: This won't include generic params?
        format!("{:?}", self)
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F>) -> Vec<F> {
        let mut constraints = Vec::with_capacity(W + R);

        // Value that is implicitly added to each element.
        // See https://affine.group/2020/02/starkware-challenge
        let mut addition_buffer = F::ZERO;

        let switch = vars.local_wires[Self::WIRE_SWITCH];
        let mut state = Vec::with_capacity(12);
        for i in 0..4 {
            let a = vars.local_wires[i];
            let b = vars.local_wires[i + 4];
            state.push(a + switch * (b - a));
        }
        for i in 0..4 {
            let a = vars.local_wires[i + 4];
            let b = vars.local_wires[i];
            state.push(a + switch * (b - a));
        }
        for i in 8..12 {
            state.push(vars.local_wires[i]);
        }

        for r in 0..R {
            let active = r % W;
            let cubing_input = state[active] + addition_buffer + self.constants[r];
            let cubing_input_wire = vars.local_wires[Self::wire_cubing_input(r)];
            constraints.push(cubing_input - cubing_input_wire);
            let f = cubing_input_wire.cube();
            addition_buffer += f;
            state[active] -= f;
        }

        for i in 0..W {
            state[i] += addition_buffer;
            constraints.push(state[i] - vars.next_wires[i]);
        }

        constraints
    }

    fn eval_unfiltered_recursively(
        &self,
        builder: &mut CircuitBuilder<F>,
        vars: EvaluationTargets,
    ) -> Vec<Target> {
        unimplemented!()
    }

    fn generators(
        &self,
        gate_index: usize,
        _local_constants: &[F],
        _next_constants: &[F],
    ) -> Vec<Box<dyn WitnessGenerator<F>>> {
        let gen = GMiMCGenerator {
            gate_index,
            constants: self.constants.clone(),
        };
        vec![Box::new(gen)]
    }

    fn num_wires(&self) -> usize {
        W + 1 + R
    }

    fn num_constants(&self) -> usize {
        0
    }

    fn degree(&self) -> usize {
        3
    }

    fn num_constraints(&self) -> usize {
        R + W
    }
}

#[derive(Debug)]
struct GMiMCGenerator<F: Field, const R: usize> {
    gate_index: usize,
    constants: Arc<[F; R]>,
}

impl<F: Field, const R: usize> SimpleGenerator<F> for GMiMCGenerator<F, R> {
    fn dependencies(&self) -> Vec<Target> {
        (0..W)
            .map(|i| Target::Wire(Wire {
                gate: self.gate_index,
                input: GMiMCGate::<F, R>::wire_input(i),
            }))
            .collect()
    }

    fn run_once(&self, witness: &PartialWitness<F>) -> PartialWitness<F> {
        let mut result = PartialWitness::new();

        let mut state = (0..W)
            .map(|i| witness.get_wire(Wire {
                gate: self.gate_index,
                input: GMiMCGate::<F, R>::wire_input(i),
            }))
            .collect::<Vec<_>>();

        let switch_value = witness.get_wire(Wire {
            gate: self.gate_index,
            input: GMiMCGate::<F, R>::WIRE_SWITCH,
        });
        debug_assert!(switch_value == F::ZERO || switch_value == F::ONE);
        if switch_value == F::ONE {
            for i in 0..4 {
                state.swap(i, 4 + i);
            }
        }

        // Value that is implicitly added to each element.
        // See https://affine.group/2020/02/starkware-challenge
        let mut addition_buffer = F::ZERO;

        for r in 0..R {
            let active = r % W;
            let cubing_input = state[active] + addition_buffer + self.constants[r];
            result.set_wire(
                Wire {
                    gate: self.gate_index,
                    input: GMiMCGate::<F, R>::wire_cubing_input(r),
                },
                cubing_input);
            let f = cubing_input.cube();
            addition_buffer += f;
            state[active] -= f;
        }

        for i in 0..W {
            state[i] += addition_buffer;
            result.set_wire(
                Wire {
                    gate: self.gate_index + 1,
                    input: GMiMCGate::<F, R>::wire_output(i),
                },
                state[i]);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use std::convert::TryInto;
    use std::sync::Arc;

    use crate::circuit_data::CircuitConfig;
    use crate::field::crandall_field::CrandallField;
    use crate::field::field::Field;
    use crate::gates::deterministic_gate::DeterministicGate;
    use crate::gates::gmimc::{GMiMCGate, W};
    use crate::generator::generate_partial_witness;
    use crate::gmimc::gmimc_permute_naive;
    use crate::wire::Wire;
    use crate::witness::PartialWitness;

    #[test]
    fn generated_output() {
        type F = CrandallField;
        const R: usize = 101;
        let constants = Arc::new([F::TWO; R]);
        type Gate = GMiMCGate::<F, R>;
        let gate = Gate::with_constants(constants.clone());

        let config = CircuitConfig {
            num_wires: 200,
            num_routed_wires: 200,
            ..Default::default()
        };

        let permutation_inputs = (0..W)
            .map(F::from_canonical_usize)
            .collect::<Vec<_>>();

        let mut witness = PartialWitness::new();
        witness.set_wire(Wire { gate: 0, input: Gate::WIRE_SWITCH }, F::ZERO);
        for i in 0..W {
            witness.set_wire(
                Wire { gate: 0, input: Gate::wire_input(i) },
                permutation_inputs[i]);
        }

        let generators = gate.0.generators(0, &[], &[]);
        generate_partial_witness(&mut witness, &generators);

        let expected_outputs: [F; W] = gmimc_permute_naive(
            permutation_inputs.try_into().unwrap(),
            constants);

        for i in 0..W {
            let out = witness.get_wire(
                Wire { gate: 1, input: Gate::wire_output(i) });
            assert_eq!(out, expected_outputs[i]);
        }
    }
}
