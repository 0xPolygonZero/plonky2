use std::ops::{Range, RangeFrom};

use anyhow::Result;

use crate::field::extension_field::Extendable;
use crate::field::field::Field;
use crate::fri::FriConfig;
use crate::gates::gate::{GateInstance, PrefixedGate};
use crate::generator::WitnessGenerator;
use crate::polynomial::commitment::ListPolynomialCommitment;
use crate::proof::{Hash, HashTarget, Proof};
use crate::prover::prove;
use crate::target::Target;
use crate::verifier::verify;
use crate::witness::PartialWitness;

#[derive(Clone)]
pub struct CircuitConfig {
    pub num_wires: usize,
    pub num_routed_wires: usize,
    pub security_bits: usize,
    pub rate_bits: usize,
    /// The number of challenge points to generate, for IOPs that have soundness errors of (roughly)
    /// `degree / |F|`.
    pub num_challenges: usize,

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
            num_challenges: 3,
            fri_config: FriConfig {
                proof_of_work_bits: 1,
                rate_bits: 1,
                reduction_arity_bits: vec![1, 1, 1, 1],
                num_query_rounds: 1,
            },
        }
    }
}

impl CircuitConfig {
    pub fn num_advice_wires(&self) -> usize {
        self.num_wires - self.num_routed_wires
    }

    pub(crate) fn large_config() -> Self {
        Self {
            num_wires: 134,
            num_routed_wires: 34,
            security_bits: 128,
            rate_bits: 3,
            num_challenges: 3,
            fri_config: FriConfig {
                proof_of_work_bits: 1,
                rate_bits: 3,
                reduction_arity_bits: vec![1, 1, 1, 1],
                num_query_rounds: 1,
            },
        }
    }
}

/// Circuit data required by the prover or the verifier.
pub struct CircuitData<F: Extendable<D>, const D: usize> {
    pub(crate) prover_only: ProverOnlyCircuitData<F, D>,
    pub(crate) verifier_only: VerifierOnlyCircuitData<F>,
    pub(crate) common: CommonCircuitData<F, D>,
}

impl<F: Extendable<D>, const D: usize> CircuitData<F, D> {
    pub fn prove(&self, inputs: PartialWitness<F>) -> Proof<F, D> {
        prove(&self.prover_only, &self.common, inputs)
    }

    pub fn verify(&self, proof: Proof<F, D>) -> Result<()> {
        verify(proof, &self.verifier_only, &self.common)
    }
}

/// Circuit data required by the prover. This may be thought of as a proving key, although it
/// includes code for witness generation.
///
/// The goal here is to make proof generation as fast as we can, rather than making this prover
/// structure as succinct as we can. Thus we include various precomputed data which isn't strictly
/// required, like LDEs of preprocessed polynomials. If more succinctness was desired, we could
/// construct a more minimal prover structure and convert back and forth.
pub struct ProverCircuitData<F: Extendable<D>, const D: usize> {
    pub(crate) prover_only: ProverOnlyCircuitData<F, D>,
    pub(crate) common: CommonCircuitData<F, D>,
}

impl<F: Extendable<D>, const D: usize> ProverCircuitData<F, D> {
    pub fn prove(&self, inputs: PartialWitness<F>) -> Proof<F, D> {
        prove(&self.prover_only, &self.common, inputs)
    }
}

/// Circuit data required by the prover.
pub struct VerifierCircuitData<F: Extendable<D>, const D: usize> {
    pub(crate) verifier_only: VerifierOnlyCircuitData<F>,
    pub(crate) common: CommonCircuitData<F, D>,
}

impl<F: Extendable<D>, const D: usize> VerifierCircuitData<F, D> {
    pub fn verify(&self, proof: Proof<F, D>) -> Result<()> {
        verify(proof, &self.verifier_only, &self.common)
    }
}

/// Circuit data required by the prover, but not the verifier.
pub(crate) struct ProverOnlyCircuitData<F: Extendable<D>, const D: usize> {
    pub generators: Vec<Box<dyn WitnessGenerator<F>>>,
    /// Commitments to the constants polynomials and sigma polynomials.
    pub constants_sigmas_commitment: ListPolynomialCommitment<F>,
    /// The transpose of the list of sigma polynomials.
    pub sigmas: Vec<Vec<F>>,
    /// Subgroup of order `degree`.
    pub subgroup: Vec<F>,
    /// The circuit's copy constraints.
    pub copy_constraints: Vec<(Target, Target)>,
    /// The concrete placement of each gate in the circuit.
    pub gate_instances: Vec<GateInstance<F, D>>,
}

/// Circuit data required by the verifier, but not the prover.
pub(crate) struct VerifierOnlyCircuitData<F: Field> {
    /// A commitment to each constant polynomial and each permutation polynomial.
    pub(crate) constants_sigmas_root: Hash<F>,
}

/// Circuit data required by both the prover and the verifier.
pub struct CommonCircuitData<F: Extendable<D>, const D: usize> {
    pub(crate) config: CircuitConfig,

    pub(crate) degree_bits: usize,

    /// The types of gates used in this circuit, along with their prefixes.
    pub(crate) gates: Vec<PrefixedGate<F, D>>,

    /// The degree of the PLONK quotient polynomial.
    pub(crate) quotient_degree_factor: usize,

    /// The largest number of constraints imposed by any gate.
    pub(crate) num_gate_constraints: usize,

    /// The number of constant wires.
    pub(crate) num_constants: usize,

    /// The `{k_i}` valued used in `S_ID_i` in Plonk's permutation argument.
    pub(crate) k_is: Vec<F>,

    /// The number of partial products needed to compute the `Z` polynomials and the number
    /// of partial products needed to compute the final product.
    pub(crate) num_partial_products: (usize, usize),

    /// A digest of the "circuit" (i.e. the instance, minus public inputs), which can be used to
    /// seed Fiat-Shamir.
    pub(crate) circuit_digest: Hash<F>,
}

impl<F: Extendable<D>, const D: usize> CommonCircuitData<F, D> {
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
            .map(|g| g.gate.0.degree())
            .max()
            .expect("No gates?")
    }

    pub fn quotient_degree(&self) -> usize {
        self.quotient_degree_factor * self.degree()
    }

    pub fn total_constraints(&self) -> usize {
        // 2 constraints for each Z check.
        self.config.num_challenges * 2 + self.num_gate_constraints
    }

    /// Range of the constants polynomials in the `constants_sigmas_commitment`.
    pub fn constants_range(&self) -> Range<usize> {
        0..self.num_constants
    }

    /// Range of the sigma polynomials in the `constants_sigmas_commitment`.
    pub fn sigmas_range(&self) -> Range<usize> {
        self.num_constants..self.num_constants + self.config.num_routed_wires
    }

    /// Range of the `z`s polynomials in the `zs_partial_products_commitment`.
    pub fn zs_range(&self) -> Range<usize> {
        0..self.config.num_challenges
    }

    /// Range of the partial products polynomials in the `zs_partial_products_commitment`.
    pub fn partial_products_range(&self) -> RangeFrom<usize> {
        self.config.num_challenges..
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
