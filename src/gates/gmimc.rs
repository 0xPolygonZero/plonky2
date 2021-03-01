use std::convert::TryInto;
use std::sync::Arc;

use crate::circuit_data::CircuitConfig;
use crate::constraint_polynomial::ConstraintPolynomial;
use crate::field::field::Field;
use crate::gates::deterministic_gate::{DeterministicGate, DeterministicGateAdapter};
use crate::gates::gate::{Gate, GateRef};
use crate::gates::output_graph::{GateOutputLocation, OutputGraph};
use crate::generator::{SimpleGenerator, WitnessGenerator2};
use crate::gmimc::{gmimc_permute, gmimc_permute_array};
use crate::target::Target;
use crate::wire::Wire;
use crate::witness::PartialWitness;

/// Evaluates a full GMiMC permutation, and writes the output to the next gate's first `width`
/// wires (which could be the input of another `GMiMCGate`).
#[derive(Debug)]
pub struct GMiMCGate<F: Field, const W: usize, const R: usize> {
    constants: Arc<[F; R]>,
}

impl<F: Field, const W: usize, const R: usize> GMiMCGate<F, W, R> {
    pub fn with_constants(constants: Arc<[F; R]>) -> GateRef<F> {
        let gate = GMiMCGate::<F, W, R> { constants };
        let adapter = DeterministicGateAdapter::new(gate);
        GateRef::new(adapter)
    }

    pub fn with_automatic_constants() -> GateRef<F> {
        todo!()
    }

    /// If this is set to 1, the first four inputs will be swapped with the next four inputs. This
    /// is useful for ordering hashes in Merkle proofs. Otherwise, this should be set to 0.
    // TODO: Assert binary.
    pub const WIRE_SWITCH: usize = 0;

    /// The wire index for the i'th input to the permutation.
    pub fn wire_input(i: usize) -> usize {
        i + 1
    }

    /// The wire index for the i'th output to the permutation.
    /// Note that outputs are written to the next gate's wires.
    pub fn wire_output(i: usize) -> usize {
        i + 1
    }
}

impl<F: Field, const W: usize, const R: usize> DeterministicGate<F> for GMiMCGate<F, W, R> {
    fn id(&self) -> String {
        // TODO: This won't include generic params?
        format!("{:?}", self)
    }

    fn outputs(&self, config: CircuitConfig) -> OutputGraph<F> {
        let original_inputs = (0..W)
            .map(|i| ConstraintPolynomial::local_wire_value(Self::wire_input(i)))
            .collect::<Vec<_>>();

        // Conditionally switch inputs based on the (boolean) switch wire.
        let switch = ConstraintPolynomial::local_wire_value(Self::WIRE_SWITCH);
        let mut state = Vec::new();
        for i in 0..4 {
            let a = &original_inputs[i];
            let b = &original_inputs[i + 4];
            state.push(a + &switch * (b - a));
        }
        for i in 0..4 {
            let a = &original_inputs[i + 4];
            let b = &original_inputs[i];
            state.push(a + &switch * (b - a));
        }
        for i in 8..W {
            state.push(original_inputs[i].clone());
        }

        // Value that is implicitly added to each element.
        // See https://affine.group/2020/02/starkware-challenge
        let mut addition_buffer = ConstraintPolynomial::zero();

        for r in 0..R {
            let active = r % W;
            let round_constant = ConstraintPolynomial::constant(self.constants[r]);
            let f = (&state[active] + &addition_buffer + round_constant).cube();
            addition_buffer += &f;
            state[active] -= f;
        }

        for i in 0..W {
            state[i] += &addition_buffer;
        }

        let outputs = state.into_iter()
            .enumerate()
            .map(|(i, out)| (GateOutputLocation::NextWire(Self::wire_output(i)), out))
            .collect();

        OutputGraph { outputs }
    }

    fn additional_constraints(&self, _config: CircuitConfig) -> Vec<ConstraintPolynomial<F>> {
        let switch = ConstraintPolynomial::local_wire_value(Self::WIRE_SWITCH);
        let switch_bool_constraint = &switch * (&switch - 1);
        vec![switch_bool_constraint]
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::field::crandall_field::CrandallField;
    use crate::gates::gmimc::GMiMCGate;
    use crate::field::field::Field;
    use crate::circuit_data::CircuitConfig;
    use crate::gates::deterministic_gate::DeterministicGate;

    #[test]
    #[ignore]
    fn degree() {
        type F = CrandallField;
        const W: usize = 12;
        const R: usize = 20;
        let gate = GMiMCGate::<F, W, R> { constants: Arc::new([F::TWO; R]) };
        let config = CircuitConfig {
            num_wires: 200,
            num_routed_wires: 200,
            security_bits: 128
        };
        let outs = gate.outputs(config);
        assert_eq!(outs.max_wire_input_index(), Some(50));
    }
}
