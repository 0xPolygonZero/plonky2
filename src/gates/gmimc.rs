use std::convert::TryInto;

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
use std::sync::Arc;

/// Evaluates a full GMiMC permutation, and writes the output to the next gate's first `width`
/// wires (which could be the input of another `GMiMCGate`).
#[derive(Debug)]
pub struct GMiMCGate<F: Field, const W: usize, const R: usize> {
    round_constants: Arc<[F; R]>,
}

impl<F: Field, const W: usize, const R: usize> GMiMCGate<F, W, R> {
    fn new() -> GateRef<F> {
        todo!()
    }
}

impl<F: Field, const W: usize, const R: usize> Gate<F> for GMiMCGate<F, W, R> {
    fn id(&self) -> String {
        // TODO: Add W/R
        format!("{:?}", self)
    }

    fn constraints(&self, config: CircuitConfig) -> Vec<ConstraintPolynomial<F>> {
        unimplemented!()
    }

    fn generators(&self, config: CircuitConfig, gate_index: usize, local_constants: Vec<F>, next_constants: Vec<F>) -> Vec<Box<dyn WitnessGenerator2<F>>> {
        let generator = GMiMCGenerator::<F, W, R> {
            round_constants: self.round_constants.clone(),
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
