use std::convert::TryInto;

use crate::field::extension_field::Extendable;
use crate::field::field_types::RichField;
use crate::gates::gmimc::GMiMCGate;
use crate::iop::target::Target;
use crate::iop::wire::Wire;
use crate::plonk::circuit_builder::CircuitBuilder;

// TODO: Move to be next to native `permute`?
impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    pub fn permute(&mut self, inputs: [Target; 12]) -> [Target; 12] {
        let zero = self.zero();
        let gate_type = GMiMCGate::<F, D, 12>::new();
        let gate = self.add_gate(gate_type, vec![]);

        // We don't want to swap any inputs, so set that wire to 0.
        let swap_wire = GMiMCGate::<F, D, 12>::WIRE_SWAP;
        let swap_wire = Target::Wire(Wire {
            gate,
            input: swap_wire,
        });
        self.connect(zero, swap_wire);

        // Route input wires.
        for i in 0..12 {
            let in_wire = GMiMCGate::<F, D, 12>::wire_input(i);
            let in_wire = Target::Wire(Wire {
                gate,
                input: in_wire,
            });
            self.connect(inputs[i], in_wire);
        }

        // Collect output wires.
        (0..12)
            .map(|i| {
                Target::Wire(Wire {
                    gate,
                    input: GMiMCGate::<F, D, 12>::wire_output(i),
                })
            })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
    }
}
