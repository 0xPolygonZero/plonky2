use std::convert::TryInto;
use std::sync::Arc;

use crate::circuit_data::CircuitConfig;
use crate::constraint_polynomial::ConstraintPolynomial;
use crate::field::field::Field;
use crate::gates::deterministic_gate::DeterministicGate;
use crate::gates::gate::{Gate, GateRef};
use crate::generator::{SimpleGenerator, WitnessGenerator2};
use crate::gmimc::gmimc_permute;
use crate::target::Target2;
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
        GateRef::new(GMiMCGate { constants })
    }

    pub fn with_automatic_constants() -> GateRef<F> {
        todo!()
    }
}

impl<F: Field, const W: usize, const R: usize> Gate<F> for GMiMCGate<F, W, R> {
    fn id(&self) -> String {
        // TODO: Add W/R
        format!("{:?}", self)
    }

    fn constraints(&self, config: CircuitConfig) -> Vec<ConstraintPolynomial<F>> {
        let mut state = (0..W)
            .map(|i| ConstraintPolynomial::local_wire_value(i))
            .collect::<Vec<_>>();

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

        state
    }

    fn generators(
        &self,
        config: CircuitConfig,
        gate_index: usize,
        local_constants: Vec<F>,
        next_constants: Vec<F>,
    ) -> Vec<Box<dyn WitnessGenerator2<F>>> {
        let generator = GMiMCGenerator::<F, W, R> {
            round_constants: self.constants.clone(),
            gate_index,
        };
        vec![Box::new(generator)]
    }
}

struct GMiMCGenerator<F: Field, const W: usize, const R: usize> {
    round_constants: Arc<[F; R]>,
    gate_index: usize,
}

impl<F: Field, const W: usize, const R: usize> SimpleGenerator<F> for GMiMCGenerator<F, W, R> {
    fn dependencies(&self) -> Vec<Target2> {
        (0..W)
            .map(|i| Target2::Wire(
                Wire { gate: self.gate_index, input: i }))
            .collect()
    }

    fn run_once(&mut self, witness: &PartialWitness<F>) -> PartialWitness<F> {
        let mut inputs: [F; W] = [F::ZERO; W];
        for i in 0..W {
            inputs[i] = witness.get_wire(
                Wire { gate: self.gate_index, input: i });
        }

        let outputs = gmimc_permute::<F, W, R>(inputs, self.round_constants.clone());

        let mut result = PartialWitness::new();
        for i in 0..W {
            result.set_wire(
                Wire { gate: self.gate_index + 1, input: i },
                outputs[i]);
        }
        result
    }
}
