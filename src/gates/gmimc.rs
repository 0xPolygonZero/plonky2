use std::sync::Arc;

use num::{BigUint, One};

use crate::circuit_data::CircuitConfig;
use crate::constraint_polynomial::ConstraintPolynomial;
use crate::field::field::Field;
use crate::gates::deterministic_gate::{DeterministicGate, DeterministicGateAdapter};
use crate::gates::gate::GateRef;
use crate::gates::output_graph::{GateOutputLocation, OutputGraph};

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

    /// Adds a local wire output to this output graph. Returns a `ConstraintPolynomial` which
    /// references the newly-created wire.
    ///
    /// This may seem like it belongs in `OutputGraph`, but it is not that general, since it uses
    /// a notion of "next available" local wire indices specific to this gate.
    // TODO: Switch to `ExpandableOutputGraph`.
    fn add_local(
        outputs: &mut OutputGraph<F>,
        poly: ConstraintPolynomial<F>,
    ) -> ConstraintPolynomial<F> {
        let index = outputs.max_local_output_index().map_or(W + 1, |i| i + 1);
        outputs.add(GateOutputLocation::LocalWire(index), poly);
        ConstraintPolynomial::local_wire(index)
    }
}

impl<F: Field, const W: usize, const R: usize> DeterministicGate<F> for GMiMCGate<F, W, R> {
    fn id(&self) -> String {
        // TODO: This won't include generic params?
        format!("{:?}", self)
    }

    fn outputs(&self, _config: CircuitConfig) -> OutputGraph<F> {
        let original_inputs = (0..W)
            .map(|i| ConstraintPolynomial::local_wire(Self::wire_input(i)))
            .collect::<Vec<_>>();

        let mut outputs = OutputGraph::new();

        // Conditionally switch inputs based on the (boolean) switch wire.
        let switch = ConstraintPolynomial::local_wire(Self::WIRE_SWITCH);
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
            let mut f_input = &state[active] + &addition_buffer + round_constant;
            if f_input.degree() > BigUint::one() {
                f_input = Self::add_local(&mut outputs, f_input);
            }
            let f_output = f_input.cube();
            addition_buffer += &f_output;
            state[active] -= f_output;
        }

        for i in 0..W {
            outputs.add(GateOutputLocation::NextWire(i), &state[i] + &addition_buffer);
        }

        outputs
    }

    fn additional_constraints(&self, _config: CircuitConfig) -> Vec<ConstraintPolynomial<F>> {
        let switch = ConstraintPolynomial::local_wire(Self::WIRE_SWITCH);
        let switch_bool_constraint = &switch * (&switch - 1);
        vec![switch_bool_constraint]
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
    use crate::gates::gmimc::GMiMCGate;
    use crate::generator::generate_partial_witness;
    use crate::gmimc::gmimc_permute_naive;
    use crate::wire::Wire;
    use crate::witness::PartialWitness;

    #[test]
    fn degree() {
        type F = CrandallField;
        const W: usize = 12;
        const R: usize = 101;
        let gate = GMiMCGate::<F, W, R> { constants: Arc::new([F::TWO; R]) };
        let config = CircuitConfig {
            num_wires: 200,
            num_routed_wires: 200,
            security_bits: 128,
        };
        let outs = gate.outputs(config);
        assert_eq!(outs.degree(), 3);
    }

    #[test]
    fn generated_output() {
        type F = CrandallField;
        const W: usize = 12;
        const R: usize = 101;
        let constants = Arc::new([F::TWO; R]);
        type Gate = GMiMCGate::<F, W, R>;
        let gate = Gate::with_constants(constants.clone());

        let config = CircuitConfig {
            num_wires: 200,
            num_routed_wires: 200,
            security_bits: 128,
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

        let generators = gate.0.generators(config, 0, vec![], vec![]);
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
