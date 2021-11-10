use std::convert::TryInto;

use crate::field::extension_field::Extendable;
use crate::field::field_types::RichField;
use crate::gates::gmimc::GMiMCGate;
use crate::gates::poseidon::PoseidonGate;
use crate::hash::gmimc::GMiMC;
use crate::hash::hashing::{HashFamily, HASH_FAMILY};
use crate::hash::poseidon::Poseidon;
use crate::iop::target::{BoolTarget, Target};
use crate::iop::wire::Wire;
use crate::plonk::circuit_builder::CircuitBuilder;

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    pub fn permute<const W: usize>(&mut self, inputs: [Target; W]) -> [Target; W]
    where
        F: GMiMC<W> + Poseidon<W>,
        [(); W - 1]:,
    {
        // We don't want to swap any inputs, so set that wire to 0.
        let _false = self._false();
        self.permute_swapped(inputs, _false)
    }

    /// Conditionally swap two chunks of the inputs (useful in verifying Merkle proofs), then apply
    /// a cryptographic permutation.
    pub(crate) fn permute_swapped<const W: usize>(
        &mut self,
        inputs: [Target; W],
        swap: BoolTarget,
    ) -> [Target; W]
    where
        F: GMiMC<W> + Poseidon<W>,
        [(); W - 1]:,
    {
        match HASH_FAMILY {
            HashFamily::GMiMC => self.gmimc_permute_swapped(inputs, swap),
            HashFamily::Poseidon => self.poseidon_permute_swapped(inputs, swap),
        }
    }

    /// Conditionally swap two chunks of the inputs (useful in verifying Merkle proofs), then apply
    /// the GMiMC permutation.
    pub(crate) fn gmimc_permute_swapped<const W: usize>(
        &mut self,
        inputs: [Target; W],
        swap: BoolTarget,
    ) -> [Target; W]
    where
        F: GMiMC<W>,
    {
        let gate_type = GMiMCGate::<F, D, W>::new();
        let gate = self.add_gate(gate_type, vec![]);

        let swap_wire = GMiMCGate::<F, D, W>::WIRE_SWAP;
        let swap_wire = Target::wire(gate, swap_wire);
        self.connect(swap.target, swap_wire);

        // Route input wires.
        for i in 0..W {
            let in_wire = GMiMCGate::<F, D, W>::wire_input(i);
            let in_wire = Target::Wire(Wire {
                gate,
                input: in_wire,
            });
            self.connect(inputs[i], in_wire);
        }

        // Collect output wires.
        (0..W)
            .map(|i| {
                Target::Wire(Wire {
                    gate,
                    input: GMiMCGate::<F, D, W>::wire_output(i),
                })
            })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
    }

    /// Conditionally swap two chunks of the inputs (useful in verifying Merkle proofs), then apply
    /// the Poseidon permutation.
    pub(crate) fn poseidon_permute_swapped<const W: usize>(
        &mut self,
        inputs: [Target; W],
        swap: BoolTarget,
    ) -> [Target; W]
    where
        F: Poseidon<W>,
        [(); W - 1]:,
    {
        let gate_type = PoseidonGate::<F, D, W>::new();
        let gate = self.add_gate(gate_type, vec![]);

        let swap_wire = PoseidonGate::<F, D, W>::WIRE_SWAP;
        let swap_wire = Target::wire(gate, swap_wire);
        self.connect(swap.target, swap_wire);

        // Route input wires.
        for i in 0..W {
            let in_wire = PoseidonGate::<F, D, W>::wire_input(i);
            let in_wire = Target::wire(gate, in_wire);
            self.connect(inputs[i], in_wire);
        }

        // Collect output wires.
        (0..W)
            .map(|i| Target::wire(gate, PoseidonGate::<F, D, W>::wire_output(i)))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
    }
}
