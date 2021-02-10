use crate::field::fft::FftPrecomputation;
use crate::field::field::Field;
use crate::proof::{Hash, Proof2};
use crate::prover::prove2;
use crate::verifier::verify2;

#[derive(Copy, Clone)]
pub struct CircuitConfig {
    pub num_wires: usize,
    pub num_routed_wires: usize,
    pub security_bits: usize,
}

impl CircuitConfig {
    pub fn advice_wires(&self) -> usize {
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
    pub fn prove2(&self) -> Proof2<F> {
        prove2(&self.prover_only, &self.common)
    }

    pub fn verify2(&self) {
        verify2(&self.verifier_only, &self.common)
    }
}

/// Circuit data required by the prover.
pub struct ProverCircuitData<F: Field> {
    prover_only: ProverOnlyCircuitData<F>,
    common: CommonCircuitData<F>,
}

impl<F: Field> ProverCircuitData<F> {
    pub fn prove2(&self) -> Proof2<F> {
        prove2(&self.prover_only, &self.common)
    }
}

/// Circuit data required by the prover.
pub struct VerifierCircuitData<F: Field> {
    verifier_only: VerifierOnlyCircuitData,
    common: CommonCircuitData<F>,
}

impl<F: Field> VerifierCircuitData<F> {
    pub fn verify2(&self) {
        verify2(&self.verifier_only, &self.common)
    }
}

/// Circuit data required by the prover, but not the verifier.
pub(crate) struct ProverOnlyCircuitData<F: Field> {
    /// A precomputation used for FFTs of degree 8n, where n is the number of gates.
    pub fft_precomputation_8n: FftPrecomputation<F>,
}

/// Circuit data required by the verifier, but not the prover.
pub(crate) struct VerifierOnlyCircuitData {}

/// Circuit data required by both the prover and the verifier.
pub(crate) struct CommonCircuitData<F: Field> {
    pub config: CircuitConfig,

    pub degree: usize,

    /// A commitment to each constant polynomial.
    pub constants_root: Hash<F>,

    /// A commitment to each permutation polynomial.
    pub sigmas_root: Hash<F>,

    /// A precomputation used for FFTs of degree n, where n is the number of gates.
    pub fft_precomputation_n: FftPrecomputation<F>,
}
