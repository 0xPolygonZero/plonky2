use std::collections::{HashMap, HashSet};
use std::time::Instant;

use log::info;

use crate::circuit_data::{
    CircuitConfig, CircuitData, CommonCircuitData, ProverCircuitData, ProverOnlyCircuitData,
    VerifierCircuitData, VerifierOnlyCircuitData,
};
use crate::field::cosets::get_unique_coset_shifts;
use crate::field::field::Field;
use crate::gates::constant::ConstantGate;
use crate::gates::gate::{GateInstance, GateRef};
use crate::gates::noop::NoopGate;
use crate::generator::{CopyGenerator, WitnessGenerator};
use crate::hash::{hash_n_to_hash, merkle_root_bit_rev_order};
use crate::polynomial::polynomial::PolynomialValues;
use crate::target::Target;
use crate::util::{log2_strict, transpose, transpose_poly_values};
use crate::wire::Wire;

pub struct CircuitBuilder<F: Field> {
    pub(crate) config: CircuitConfig,

    /// The types of gates used in this circuit.
    gates: HashSet<GateRef<F>>,

    /// The concrete placement of each gate.
    gate_instances: Vec<GateInstance<F>>,

    /// The next available index for a VirtualAdviceTarget.
    virtual_target_index: usize,

    /// Generators used to generate the witness.
    generators: Vec<Box<dyn WitnessGenerator<F>>>,

    constants_to_targets: HashMap<F, Target>,
    targets_to_constants: HashMap<Target, F>,
}

impl<F: Field> CircuitBuilder<F> {
    pub fn new(config: CircuitConfig) -> Self {
        CircuitBuilder {
            config,
            gates: HashSet::new(),
            gate_instances: Vec::new(),
            virtual_target_index: 0,
            generators: Vec::new(),
            constants_to_targets: HashMap::new(),
            targets_to_constants: HashMap::new(),
        }
    }

    /// Adds a new "virtual" advice target. This is not an actual wire in the witness, but just a
    /// target that help facilitate witness generation. In particular, a generator can assign a
    /// values to a virtual target, which can then be copied to other (virtual or concrete) targets
    /// via `generate_copy`. When we generate the final witness (a grid of wire values), these
    /// virtual targets will go away.
    ///
    /// Since virtual targets are not part of the actual permutation argument, they cannot be used
    /// with `assert_equal`.
    pub fn add_virtual_advice_target(&mut self) -> Target {
        let index = self.virtual_target_index;
        self.virtual_target_index += 1;
        Target::VirtualAdviceTarget { index }
    }

    pub fn add_virtual_advice_targets(&mut self, n: usize) -> Vec<Target> {
        (0..n).map(|_i| self.add_virtual_advice_target()).collect()
    }

    pub fn add_gate_no_constants(&mut self, gate_type: GateRef<F>) -> usize {
        self.add_gate(gate_type, Vec::new())
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

        // TODO: Not passing next constants for now. Not sure if it's really useful...
        self.add_generators(gate_type.0.generators(index, &constants, &[]));

        self.gate_instances.push(GateInstance {
            gate_type,
            constants,
        });
        index
    }

    fn check_gate_compatibility(&self, gate: &GateRef<F>) {
        assert!(
            gate.0.num_wires() <= self.config.num_wires,
            "{:?} requires {} wires, but our GateConfig has only {}",
            gate.0.id(),
            gate.0.num_wires(),
            self.config.num_wires
        );
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
        assert!(
            x.is_routable(self.config),
            "Tried to route a wire that isn't routable"
        );
        assert!(
            y.is_routable(self.config),
            "Tried to route a wire that isn't routable"
        );
        // TODO: Add to copy_constraints.
    }

    pub fn add_generators(&mut self, generators: Vec<Box<dyn WitnessGenerator<F>>>) {
        self.generators.extend(generators);
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
        if let Some(&target) = self.constants_to_targets.get(&c) {
            // We already have a wire for this constant.
            return target;
        }

        let gate = self.add_gate(ConstantGate::get(), vec![c]);
        let target = Target::Wire(Wire {
            gate,
            input: ConstantGate::WIRE_OUTPUT,
        });
        self.constants_to_targets.insert(c, target);
        self.targets_to_constants.insert(target, c);
        target
    }

    pub fn constants(&mut self, constants: &[F]) -> Vec<Target> {
        constants.iter().map(|&c| self.constant(c)).collect()
    }

    /// If the given target is a constant (i.e. it was created by the `constant(F)` method), returns
    /// its constant value. Otherwise, returns `None`.
    pub fn target_as_constant(&self, target: Target) -> Option<F> {
        self.targets_to_constants.get(&target).cloned()
    }

    fn blind_and_pad(&mut self) {
        // TODO: Blind.

        while !self.gate_instances.len().is_power_of_two() {
            self.add_gate_no_constants(NoopGate::get());
        }
    }

    fn constant_polys(&self) -> Vec<PolynomialValues<F>> {
        let num_constants = self
            .gate_instances
            .iter()
            .map(|gate_inst| gate_inst.constants.len())
            .max()
            .unwrap();
        let constants_per_gate = self
            .gate_instances
            .iter()
            .map(|gate_inst| {
                let mut padded_constants = gate_inst.constants.clone();
                for _ in padded_constants.len()..num_constants {
                    padded_constants.push(F::ZERO);
                }
                padded_constants
            })
            .collect::<Vec<_>>();

        transpose(&constants_per_gate)
            .into_iter()
            .map(PolynomialValues::new)
            .collect()
    }

    fn sigma_vecs(&self) -> Vec<PolynomialValues<F>> {
        vec![PolynomialValues::zero(self.gate_instances.len()); self.config.num_routed_wires]
        // TODO
    }

    /// Builds a "full circuit", with both prover and verifier data.
    pub fn build(mut self) -> CircuitData<F> {
        let start = Instant::now();
        info!(
            "degree before blinding & padding: {}",
            self.gate_instances.len()
        );
        self.blind_and_pad();
        let degree = self.gate_instances.len();
        info!("degree after blinding & padding: {}", degree);

        let constant_vecs = self.constant_polys();
        let constant_ldes = PolynomialValues::lde_multiple(constant_vecs, self.config.rate_bits);
        let constant_ldes_t = transpose_poly_values(constant_ldes);
        let constants_root = merkle_root_bit_rev_order(constant_ldes_t.clone());

        let sigma_vecs = self.sigma_vecs();
        let sigma_ldes = PolynomialValues::lde_multiple(sigma_vecs, self.config.rate_bits);
        let sigma_ldes_t = transpose_poly_values(sigma_ldes);
        let sigmas_root = merkle_root_bit_rev_order(sigma_ldes_t.clone());

        let generators = self.generators;
        let prover_only = ProverOnlyCircuitData {
            generators,
            constant_ldes_t,
            sigma_ldes_t,
        };
        let verifier_only = VerifierOnlyCircuitData {};

        // The HashSet of gates will have a non-deterministic order. When converting to a Vec, we
        // sort by ID to make the ordering deterministic.
        let mut gates = self.gates.iter().cloned().collect::<Vec<_>>();
        gates.sort_unstable_by_key(|gate| gate.0.id());

        let num_gate_constraints = gates
            .iter()
            .map(|gate| gate.0.num_constraints())
            .max()
            .expect("No gates?");

        let degree_bits = log2_strict(degree);
        let k_is = get_unique_coset_shifts(degree, self.config.num_routed_wires);

        // TODO: This should also include an encoding of gate constraints.
        let circuit_digest_parts = [constants_root.elements, sigmas_root.elements];
        let circuit_digest = hash_n_to_hash(circuit_digest_parts.concat(), false);

        let common = CommonCircuitData {
            config: self.config,
            degree_bits,
            gates,
            num_gate_constraints,
            constants_root,
            sigmas_root,
            k_is,
            circuit_digest,
        };

        info!("Building circuit took {}s", start.elapsed().as_secs_f32());
        CircuitData {
            prover_only,
            verifier_only,
            common,
        }
    }

    /// Builds a "prover circuit", with data needed to generate proofs but not verify them.
    pub fn build_prover(self) -> ProverCircuitData<F> {
        // TODO: Can skip parts of this.
        let CircuitData {
            prover_only,
            common,
            ..
        } = self.build();
        ProverCircuitData {
            prover_only,
            common,
        }
    }

    /// Builds a "verifier circuit", with data needed to verify proofs but not generate them.
    pub fn build_verifier(self) -> VerifierCircuitData<F> {
        // TODO: Can skip parts of this.
        let CircuitData {
            verifier_only,
            common,
            ..
        } = self.build();
        VerifierCircuitData {
            verifier_only,
            common,
        }
    }
}
