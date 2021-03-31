use crate::constraint_polynomial::{EvaluationTargets, EvaluationVars};
use crate::field::field::Field;
use crate::gates::gate::GateRef;
use crate::generator::WitnessGenerator;
use crate::proof::{Hash, Proof};
use crate::prover::prove;
use crate::target::Target;
use crate::verifier::verify;
use crate::witness::PartialWitness;

#[derive(Copy, Clone)]
pub struct CircuitConfig {
    pub num_wires: usize,
    pub num_routed_wires: usize,
    pub security_bits: usize,
    pub rate_bits: usize,
    /// The number of times to repeat checks that have soundness errors of (roughly) `degree / |F|`.
    pub num_checks: usize,
}

impl Default for CircuitConfig {
    fn default() -> Self {
        CircuitConfig {
            num_wires: 3,
            num_routed_wires: 3,
            security_bits: 128,
            rate_bits: 3,
            num_checks: 3,
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
    pub(crate) verifier_only: VerifierOnlyCircuitData,
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

/// Circuit data required by the prover.
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
    pub(crate) verifier_only: VerifierOnlyCircuitData,
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
    pub constant_ldes_t: Vec<Vec<F>>,
    /// Transpose of LDEs of sigma polynomials (in the context of Plonk's permutation argument).
    pub sigma_ldes_t: Vec<Vec<F>>,
}

/// Circuit data required by the verifier, but not the prover.
pub(crate) struct VerifierOnlyCircuitData {}

/// Circuit data required by both the prover and the verifier.
pub(crate) struct CommonCircuitData<F: Field> {
    pub(crate) config: CircuitConfig,

    pub(crate) degree_bits: usize,

    /// The types of gates used in this circuit.
    pub(crate) gates: Vec<GateRef<F>>,

    pub(crate) num_gate_constraints: usize,

    /// A commitment to each constant polynomial.
    pub(crate) constants_root: Hash<F>,

    /// A commitment to each permutation polynomial.
    pub(crate) sigmas_root: Hash<F>,

    /// {k_i}. See `get_subgroup_shift`.
    pub(crate) k_is: Vec<F>,
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
        self.gates.iter()
            .map(|g| g.0.degree())
            .max()
            .expect("No gates?")
    }

    pub fn total_constraints(&self) -> usize {
        // 2 constraints for each Z check.
        self.config.num_checks * 2 + self.num_gate_constraints
    }

    pub fn evaluate(&self, vars: EvaluationVars<F>) -> Vec<F> {
        let mut constraints = vec![F::ZERO; self.num_gate_constraints];
        for gate in &self.gates {
            let gate_constraints = gate.0.eval_filtered(vars);
            for (i, c) in gate_constraints.into_iter().enumerate() {
                constraints[i] += c;
            }
        }
        constraints
    }

    pub fn evaluate_recursive(&self, vars: EvaluationTargets) -> Vec<Target> {
        todo!()
    }
}
