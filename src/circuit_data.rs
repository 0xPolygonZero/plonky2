use crate::field::field::Field;
use crate::fri::FriConfig;
use crate::gates::gate::GateRef;
use crate::generator::WitnessGenerator;
use crate::merkle_tree::MerkleTree;
use crate::polynomial::commitment::ListPolynomialCommitment;
use crate::proof::{Hash, HashTarget, Proof};
use crate::prover::prove;
use crate::verifier::verify;
use crate::witness::PartialWitness;

#[derive(Clone)]
pub struct CircuitConfig {
    pub num_wires: usize,
    pub num_routed_wires: usize,
    pub security_bits: usize,
    pub rate_bits: usize,
    /// The number of times to repeat checks that have soundness errors of (roughly) `degree / |F|`.
    pub num_checks: usize,

    // TODO: Find a better place for this.
    pub fri_config: FriConfig,
}

impl Default for CircuitConfig {
    fn default() -> Self {
        CircuitConfig {
            num_wires: 4,
            num_routed_wires: 4,
            security_bits: 128,
            rate_bits: 3,
            num_checks: 3,
            fri_config: FriConfig {
                proof_of_work_bits: 1,
                rate_bits: 1,
                reduction_arity_bits: vec![1],
                num_query_rounds: 1,
                blinding: true,
            },
        }
    }
}

impl CircuitConfig {
    pub fn num_advice_wires(&self) -> usize {
        self.num_wires - self.num_routed_wires
    }
}

/// Circuit data required by the prover or the verifier.
pub struct CircuitData<F: Field> {
    pub(crate) prover_only: ProverOnlyCircuitData<F>,
    pub(crate) verifier_only: VerifierOnlyCircuitData<F>,
    pub(crate) common: CommonCircuitData<F>,
}

impl<F: Field> CircuitData<F> {
    pub fn prove(&self, inputs: PartialWitness<F>) -> Proof<F> {
        prove(&self.prover_only, &self.common, inputs)
    }

    pub fn verify(&self) {
        verify(&self.verifier_only, &self.common)
    }
}

/// Circuit data required by the prover. This may be thought of as a proving key, although it
/// includes code for witness generation.
///
/// The goal here is to make proof generation as fast as we can, rather than making this prover
/// structure as succinct as we can. Thus we include various precomputed data which isn't strictly
/// required, like LDEs of preprocessed polynomials. If more succinctness was desired, we could
/// construct a more minimal prover structure and convert back and forth.
pub struct ProverCircuitData<F: Field> {
    pub(crate) prover_only: ProverOnlyCircuitData<F>,
    pub(crate) common: CommonCircuitData<F>,
}

impl<F: Field> ProverCircuitData<F> {
    pub fn prove(&self, inputs: PartialWitness<F>) -> Proof<F> {
        prove(&self.prover_only, &self.common, inputs)
    }
}

/// Circuit data required by the prover.
pub struct VerifierCircuitData<F: Field> {
    pub(crate) verifier_only: VerifierOnlyCircuitData<F>,
    pub(crate) common: CommonCircuitData<F>,
}

impl<F: Field> VerifierCircuitData<F> {
    pub fn verify2(&self) {
        verify(&self.verifier_only, &self.common)
    }
}

/// Circuit data required by the prover, but not the verifier.
pub(crate) struct ProverOnlyCircuitData<F: Field> {
    pub generators: Vec<Box<dyn WitnessGenerator<F>>>,
    /// Commitments to the constants polynomial.
    pub constants_commitment: ListPolynomialCommitment<F>,
    /// Commitments to the sigma polynomial.
    pub sigmas_commitment: ListPolynomialCommitment<F>,
}

/// Circuit data required by the verifier, but not the prover.
pub(crate) struct VerifierOnlyCircuitData<F: Field> {
    /// A commitment to each constant polynomial.
    pub(crate) constants_root: Hash<F>,

    /// A commitment to each permutation polynomial.
    pub(crate) sigmas_root: Hash<F>,
}

/// Circuit data required by both the prover and the verifier.
pub(crate) struct CommonCircuitData<F: Field> {
    pub(crate) config: CircuitConfig,

    pub(crate) degree_bits: usize,

    /// The types of gates used in this circuit.
    pub(crate) gates: Vec<GateRef<F>>,

    /// The largest number of constraints imposed by any gate.
    pub(crate) num_gate_constraints: usize,

    /// The `{k_i}` valued used in `S_ID_i` in Plonk's permutation argument.
    pub(crate) k_is: Vec<F>,

    /// A digest of the "circuit" (i.e. the instance, minus public inputs), which can be used to
    /// seed Fiat-Shamir.
    pub(crate) circuit_digest: Hash<F>,
}

impl<F: Field> CommonCircuitData<F> {
    pub fn degree(&self) -> usize {
        1 << self.degree_bits
    }

    pub fn lde_size(&self) -> usize {
        1 << (self.degree_bits + self.config.rate_bits)
    }

    pub fn lde_generator(&self) -> F {
        F::primitive_root_of_unity(self.degree_bits + self.config.rate_bits)
    }

    pub fn constraint_degree(&self) -> usize {
        self.gates
            .iter()
            .map(|g| g.0.degree())
            .max()
            .expect("No gates?")
    }

    pub fn quotient_degree(&self) -> usize {
        self.constraint_degree() - 1
    }

    pub fn total_constraints(&self) -> usize {
        // 2 constraints for each Z check.
        self.config.num_checks * 2 + self.num_gate_constraints
    }
}

/// The `Target` version of `VerifierCircuitData`, for use inside recursive circuits. Note that this
/// is intentionally missing certain fields, such as `CircuitConfig`, because we support only a
/// limited form of dynamic inner circuits. We can't practically make things like the wire count
/// dynamic, at least not without setting a maximum wire count and paying for the worst case.
pub struct VerifierCircuitTarget {
    /// A commitment to each constant polynomial.
    pub(crate) constants_root: HashTarget,

    /// A commitment to each permutation polynomial.
    pub(crate) sigmas_root: HashTarget,
}
