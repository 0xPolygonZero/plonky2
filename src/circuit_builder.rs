use std::collections::HashSet;

use crate::circuit_data::{CircuitConfig, CircuitData, ProverCircuitData, VerifierCircuitData};
use crate::field::field::Field;
use crate::gates::constant::ConstantGate2;
use crate::gates::gate::{GateInstance, GateRef};
use crate::generator::{CopyGenerator, WitnessGenerator};
use crate::target::Target;
use crate::wire::Wire;

pub struct CircuitBuilder2<F: Field> {
    config: CircuitConfig,
    gates: HashSet<GateRef<F>>,
    gate_instances: Vec<GateInstance<F>>,
    generators: Vec<Box<dyn WitnessGenerator<F>>>,
}

impl<F: Field> CircuitBuilder2<F> {
    pub fn new(config: CircuitConfig) -> Self {
        CircuitBuilder2 {
            config,
            gates: HashSet::new(),
            gate_instances: Vec::new(),
            generators: Vec::new(),
        }
    }

    /// Adds a gate to the circuit, and returns its index.
    pub fn add_gate(&mut self, gate_type: GateRef<F>, constants: Vec<F>) -> usize {
        // If we haven't seen a gate of this type before, check that it's compatible with our
        // circuit configuration, then register it.
        if !self.gates.contains(&gate_type) {
            self.check_gate_compatibility(&gate_type);
            self.gates.insert(gate_type.clone());
        }

        let index = self.gate_instances.len();
        self.gate_instances.push(GateInstance { gate_type, constants });
        index
    }

    fn check_gate_compatibility(&self, gate: &GateRef<F>) {
        assert!(gate.0.min_wires(self.config) <= self.config.num_wires);
    }

    /// Shorthand for `generate_copy` and `assert_equal`.
    /// Both elements must be routable, otherwise this method will panic.
    pub fn route(&mut self, src: Target, dst: Target) {
        self.generate_copy(src, dst);
        self.assert_equal(src, dst);
    }

    /// Adds a generator which will copy `src` to `dst`.
    pub fn generate_copy(&mut self, src: Target, dst: Target) {
        self.add_generator(CopyGenerator { src, dst });
    }

    /// Uses Plonk's permutation argument to require that two elements be equal.
    /// Both elements must be routable, otherwise this method will panic.
    pub fn assert_equal(&mut self, x: Target, y: Target) {
        assert!(x.is_routable(self.config));
        assert!(y.is_routable(self.config));
    }

    pub fn add_generator<G: WitnessGenerator<F>>(&mut self, generator: G) {
        self.generators.push(Box::new(generator));
    }

    /// Returns a routable target with a value of 0.
    pub fn zero(&mut self) -> Target {
        self.constant(F::ZERO)
    }

    /// Returns a routable target with a value of 1.
    pub fn one(&mut self) -> Target {
        self.constant(F::ONE)
    }

    /// Returns a routable target with a value of 2.
    pub fn two(&mut self) -> Target {
        self.constant(F::TWO)
    }

    /// Returns a routable target with a value of `ORDER - 1`.
    pub fn neg_one(&mut self) -> Target {
        self.constant(F::NEG_ONE)
    }

    /// Returns a routable target with the given constant value.
    pub fn constant(&mut self, c: F) -> Target {
        let gate = self.add_gate(ConstantGate2::get(), vec![c]);
        Target::Wire(Wire { gate, input: ConstantGate2::WIRE_OUTPUT })
    }

    /// Builds a "full circuit", with both prover and verifier data.
    pub fn build(&self) -> CircuitData<F> {
        todo!()
    }

    /// Builds a "prover circuit", with data needed to generate proofs but not verify them.
    pub fn build_prover(&self) -> ProverCircuitData<F> {
        todo!()
    }

    /// Builds a "verifier circuit", with data needed to verify proofs but not generate them.
    pub fn build_verifier(&self) -> VerifierCircuitData<F> {
        todo!()
    }
}
