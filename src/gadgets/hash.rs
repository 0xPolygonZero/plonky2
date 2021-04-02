use std::convert::TryInto;

use crate::circuit_builder::CircuitBuilder;
use crate::field::field::Field;
use crate::gates::gmimc::GMiMCGate;
use crate::gates::noop::NoopGate;
use crate::hash::GMIMC_ROUNDS;
use crate::target::Target;
use crate::wire::Wire;

impl<F: Field> CircuitBuilder<F> {
    pub fn permute(&mut self, inputs: [Target; 12]) -> [Target; 12] {
        let zero = self.zero();
        self.permute_switched(inputs, zero)
    }

    pub(crate) fn permute_switched(&mut self, inputs: [Target; 12], switch: Target) -> [Target; 12] {
        let gate = self.add_gate_no_constants(
            GMiMCGate::<F, GMIMC_ROUNDS>::with_automatic_constants());

        let switch_wire = GMiMCGate::<F, GMIMC_ROUNDS>::WIRE_SWITCH;
        let switch_wire = Target::Wire(Wire { gate, input: switch_wire });
        self.route(switch, switch_wire);

        for i in 0..12 {
            let in_wire = GMiMCGate::<F, GMIMC_ROUNDS>::wire_output(i);
            let in_wire = Target::Wire(Wire { gate, input: in_wire });
            self.route(inputs[i], in_wire);
        }

        // Add a NoopGate just to receive the outputs.
        let next_gate = self.add_gate_no_constants(NoopGate::get());

        (0..12)
            .map(|i| Target::Wire(
                Wire { gate: next_gate, input: GMiMCGate::<F, GMIMC_ROUNDS>::wire_output(i) }))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
    }
}
