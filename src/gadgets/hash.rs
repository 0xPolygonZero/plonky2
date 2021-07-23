use std::convert::TryInto;

use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::Extendable;
use crate::gates::gmimc::GMiMCGate;
use crate::hash::GMIMC_ROUNDS;
use crate::target::Target;
use crate::wire::Wire;

// TODO: Move to be next to native `permute`?
impl<F: Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    pub fn permute(&mut self, inputs: [Target; 12]) -> [Target; 12] {
        let zero = self.zero();
        let gate =
            self.add_gate_no_constants(GMiMCGate::<F, D, GMIMC_ROUNDS>::with_automatic_constants());

        // We don't want to swap any inputs, so set that wire to 0.
        let swap_wire = GMiMCGate::<F, D, GMIMC_ROUNDS>::WIRE_SWAP;
        let swap_wire = Target::Wire(Wire {
            gate,
            input: swap_wire,
        });
        self.route(zero, swap_wire);

        // Route input wires.
        for i in 0..12 {
            let in_wire = GMiMCGate::<F, D, GMIMC_ROUNDS>::wire_input(i);
            let in_wire = Target::Wire(Wire {
                gate,
                input: in_wire,
            });
            self.route(inputs[i], in_wire);
        }

        // Collect output wires.
        (0..12)
            .map(|i| {
                Target::Wire(Wire {
                    gate,
                    input: GMiMCGate::<F, D, GMIMC_ROUNDS>::wire_output(i),
                })
            })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
    }
}
