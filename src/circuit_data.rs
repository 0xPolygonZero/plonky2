use crate::field::fft::FftPrecomputation;
use crate::field::field::Field;
use crate::generator::WitnessGenerator;
use crate::proof::{Hash, Proof2};
use crate::prover::prove;
use crate::verifier::verify;
use crate::witness::PartialWitness;
use crate::gates::gate::{GateRef, Gate};

#[derive(Copy, Clone)]
pub struct CircuitConfig {
    pub num_wires: usize,
    pub num_routed_wires: usize,
    pub security_bits: usize,
}

impl CircuitConfig {
    pub fn num_advice_wires(&self) -> usize {
        self.num_wires - self.num_routed_wires
    }
}

/// Circuit data required by the prover or the verifier.
pub struct CircuitData<F: Field> {
    prover_only: ProverOnlyCircuitData<F>,
    verifier_only: VerifierOnlyCircuitData,
    common: CommonCircuitData<F>,
}

impl<F: Field> CircuitData<F> {
    pub fn prove(&self, inputs: PartialWitness<F>) -> Proof2<F> {
        prove(&self.prover_only, &self.common, inputs)
    }

    pub fn verify(&self) {
        verify(&self.verifier_only, &self.common)
    }
}

/// Circuit data required by the prover.
pub struct ProverCircuitData<F: Field> {
    prover_only: ProverOnlyCircuitData<F>,
    common: CommonCircuitData<F>,
}

impl<F: Field> ProverCircuitData<F> {
    pub fn prove(&self, inputs: PartialWitness<F>) -> Proof2<F> {
        prove(&self.prover_only, &self.common, inputs)
    }
}

/// Circuit data required by the prover.
pub struct VerifierCircuitData<F: Field> {
    verifier_only: VerifierOnlyCircuitData,
    common: CommonCircuitData<F>,
}

impl<F: Field> VerifierCircuitData<F> {
    pub fn verify2(&self) {
        verify(&self.verifier_only, &self.common)
    }
}

/// Circuit data required by the prover, but not the verifier.
pub(crate) struct ProverOnlyCircuitData<F: Field> {
    pub generators: Vec<Box<dyn WitnessGenerator<F>>>,
}

/// Circuit data required by the verifier, but not the prover.
pub(crate) struct VerifierOnlyCircuitData {}

/// Circuit data required by both the prover and the verifier.
pub(crate) struct CommonCircuitData<F: Field> {
    pub config: CircuitConfig,

    pub degree: usize,

    /// The types of gates used in this circuit.
    pub gates: Vec<GateRef<F>>,

    /// A commitment to each constant polynomial.
    pub constants_root: Hash<F>,

    /// A commitment to each permutation polynomial.
    pub sigmas_root: Hash<F>,
}

impl<F: Field> CommonCircuitData<F> {
    pub fn constraint_degree(&self, config: CircuitConfig) -> usize {
        self.gates.iter()
            .map(|g| g.0.degree(config))
            .max()
            .expect("No gates?")
    }
}
