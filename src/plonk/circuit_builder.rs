use std::collections::{HashMap, HashSet};
use std::convert::TryInto;
use std::time::Instant;

use log::{info, Level};

use crate::field::cosets::get_unique_coset_shifts;
use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::{Extendable, FieldExtension};
use crate::fri::commitment::PolynomialBatchCommitment;
use crate::gates::arithmetic::{ArithmeticExtensionGate, NUM_ARITHMETIC_OPS};
use crate::gates::constant::ConstantGate;
use crate::gates::gate::{Gate, GateInstance, GateRef, PrefixedGate};
use crate::gates::gate_tree::Tree;
use crate::gates::noop::NoopGate;
use crate::gates::public_input::PublicInputGate;
use crate::hash::hash_types::{HashOutTarget, MerkleCapTarget};
use crate::hash::hashing::hash_n_to_hash;
use crate::iop::generator::{CopyGenerator, RandomValueGenerator, WitnessGenerator};
use crate::iop::target::{BoolTarget, Target};
use crate::iop::wire::Wire;
use crate::iop::witness::PartitionWitness;
use crate::plonk::circuit_data::{
    CircuitConfig, CircuitData, CommonCircuitData, ProverCircuitData, ProverOnlyCircuitData,
    VerifierCircuitData, VerifierOnlyCircuitData,
};
use crate::plonk::copy_constraint::CopyConstraint;
use crate::plonk::plonk_common::PlonkPolynomials;
use crate::polynomial::polynomial::PolynomialValues;
use crate::util::context_tree::ContextTree;
use crate::util::marking::{Markable, MarkedTargets};
use crate::util::partial_products::num_partial_products;
use crate::util::timing::TimingTree;
use crate::util::{log2_ceil, log2_strict, transpose, transpose_poly_values};

pub struct CircuitBuilder<F: Extendable<D>, const D: usize> {
    pub(crate) config: CircuitConfig,

    /// The types of gates used in this circuit.
    gates: HashSet<GateRef<F, D>>,

    /// The concrete placement of each gate.
    gate_instances: Vec<GateInstance<F, D>>,

    /// Targets to be made public.
    public_inputs: Vec<Target>,

    /// The next available index for a `VirtualTarget`.
    virtual_target_index: usize,

    copy_constraints: Vec<CopyConstraint>,

    /// A tree of named scopes, used for debugging.
    context_log: ContextTree,

    /// A vector of marked targets. The values assigned to these targets will be displayed by the prover.
    marked_targets: Vec<MarkedTargets<D>>,

    /// Generators used to generate the witness.
    generators: Vec<Box<dyn WitnessGenerator<F>>>,

    constants_to_targets: HashMap<F, Target>,
    targets_to_constants: HashMap<Target, F>,

    /// A map `(c0, c1) -> (g, i)` from constants `(c0,c1)` to an available arithmetic gate using
    /// these constants with gate index `g` and already using `i` arithmetic operations.
    pub(crate) free_arithmetic: HashMap<(F, F), (usize, usize)>,
}

impl<F: Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    pub fn new(config: CircuitConfig) -> Self {
        CircuitBuilder {
            config,
            gates: HashSet::new(),
            gate_instances: Vec::new(),
            public_inputs: Vec::new(),
            virtual_target_index: 0,
            copy_constraints: Vec::new(),
            context_log: ContextTree::new(),
            marked_targets: Vec::new(),
            generators: Vec::new(),
            constants_to_targets: HashMap::new(),
            targets_to_constants: HashMap::new(),
            free_arithmetic: HashMap::new(),
        }
    }

    pub fn num_gates(&self) -> usize {
        self.gate_instances.len()
    }

    /// Registers the given target as a public input.
    pub fn register_public_input(&mut self, target: Target) {
        self.public_inputs.push(target);
    }

    /// Registers the given targets as public inputs.
    pub fn register_public_inputs(&mut self, targets: &[Target]) {
        targets.iter().for_each(|&t| self.register_public_input(t));
    }

    /// Adds a new "virtual" target. This is not an actual wire in the witness, but just a target
    /// that help facilitate witness generation. In particular, a generator can assign a values to a
    /// virtual target, which can then be copied to other (virtual or concrete) targets. When we
    /// generate the final witness (a grid of wire values), these virtual targets will go away.
    pub fn add_virtual_target(&mut self) -> Target {
        let index = self.virtual_target_index;
        self.virtual_target_index += 1;
        Target::VirtualTarget { index }
    }

    pub fn add_virtual_targets(&mut self, n: usize) -> Vec<Target> {
        (0..n).map(|_i| self.add_virtual_target()).collect()
    }

    pub fn add_virtual_hash(&mut self) -> HashOutTarget {
        HashOutTarget::from_vec(self.add_virtual_targets(4))
    }

    pub fn add_virtual_cap(&mut self, cap_height: usize) -> MerkleCapTarget {
        MerkleCapTarget(self.add_virtual_hashes(1 << cap_height))
    }

    pub fn add_virtual_hashes(&mut self, n: usize) -> Vec<HashOutTarget> {
        (0..n).map(|_i| self.add_virtual_hash()).collect()
    }

    pub fn add_virtual_extension_target(&mut self) -> ExtensionTarget<D> {
        ExtensionTarget(self.add_virtual_targets(D).try_into().unwrap())
    }

    pub fn add_virtual_extension_targets(&mut self, n: usize) -> Vec<ExtensionTarget<D>> {
        (0..n)
            .map(|_i| self.add_virtual_extension_target())
            .collect()
    }

    // TODO: Unsafe
    pub fn add_virtual_bool_target(&mut self) -> BoolTarget {
        BoolTarget::new_unsafe(self.add_virtual_target())
    }

    /// Adds a gate to the circuit, and returns its index.
    pub fn add_gate<G: Gate<F, D>>(&mut self, gate_type: G, constants: Vec<F>) -> usize {
        self.check_gate_compatibility(&gate_type);
        assert_eq!(
            gate_type.num_constants(),
            constants.len(),
            "Number of constants doesn't match."
        );

        let index = self.gate_instances.len();
        self.add_generators(gate_type.generators(index, &constants));

        // Register this gate type if we haven't seen it before.
        let gate_ref = GateRef::new(gate_type);
        self.gates.insert(gate_ref.clone());

        self.gate_instances.push(GateInstance {
            gate_ref,
            constants,
        });
        index
    }

    fn check_gate_compatibility<G: Gate<F, D>>(&self, gate: &G) {
        assert!(
            gate.num_wires() <= self.config.num_wires,
            "{:?} requires {} wires, but our GateConfig has only {}",
            gate.id(),
            gate.num_wires(),
            self.config.num_wires
        );
    }

    /// Both elements must be routable, otherwise this method will panic.
    pub fn route(&mut self, src: Target, dst: Target) {
        self.assert_equal(src, dst);
    }

    /// Same as `route` with a named copy constraint.
    pub fn named_route(&mut self, src: Target, dst: Target, name: String) {
        self.named_assert_equal(src, dst, name);
    }

    pub fn route_extension(&mut self, src: ExtensionTarget<D>, dst: ExtensionTarget<D>) {
        for i in 0..D {
            self.route(src.0[i], dst.0[i]);
        }
    }

    pub fn named_route_extension(
        &mut self,
        src: ExtensionTarget<D>,
        dst: ExtensionTarget<D>,
        name: String,
    ) {
        for i in 0..D {
            self.named_route(src.0[i], dst.0[i], format!("{}: limb {}", name, i));
        }
    }

    /// Adds a generator which will copy `src` to `dst`.
    pub fn generate_copy(&mut self, src: Target, dst: Target) {
        self.add_generator(CopyGenerator { src, dst });
    }

    /// Uses Plonk's permutation argument to require that two elements be equal.
    /// Both elements must be routable, otherwise this method will panic.
    pub fn assert_equal(&mut self, x: Target, y: Target) {
        assert!(
            x.is_routable(&self.config),
            "Tried to route a wire that isn't routable"
        );
        assert!(
            y.is_routable(&self.config),
            "Tried to route a wire that isn't routable"
        );
        self.copy_constraints
            .push(CopyConstraint::new((x, y), self.context_log.open_stack()));
    }

    /// Same as `assert_equal` for a named copy constraint.
    pub fn named_assert_equal(&mut self, x: Target, y: Target, name: String) {
        assert!(
            x.is_routable(&self.config),
            "Tried to route a wire that isn't routable"
        );
        assert!(
            y.is_routable(&self.config),
            "Tried to route a wire that isn't routable"
        );
        self.copy_constraints.push(CopyConstraint::new(
            (x, y),
            format!("{} > {}", self.context_log.open_stack(), name),
        ));
    }

    pub fn assert_zero(&mut self, x: Target) {
        let zero = self.zero();
        self.assert_equal(x, zero);
    }

    pub fn assert_equal_extension(&mut self, x: ExtensionTarget<D>, y: ExtensionTarget<D>) {
        for i in 0..D {
            self.assert_equal(x.0[i], y.0[i]);
        }
    }

    pub fn named_assert_equal_extension(
        &mut self,
        x: ExtensionTarget<D>,
        y: ExtensionTarget<D>,
        name: String,
    ) {
        for i in 0..D {
            self.assert_equal(x.0[i], y.0[i]);
            self.named_assert_equal(x.0[i], y.0[i], format!("{}: limb {}", name, i));
        }
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

    /// Returns a routable target with a value of `order() - 1`.
    pub fn neg_one(&mut self) -> Target {
        self.constant(F::NEG_ONE)
    }

    pub fn _false(&mut self) -> BoolTarget {
        BoolTarget::new_unsafe(self.zero())
    }

    pub fn _true(&mut self) -> BoolTarget {
        BoolTarget::new_unsafe(self.one())
    }

    /// Returns a routable target with the given constant value.
    pub fn constant(&mut self, c: F) -> Target {
        if let Some(&target) = self.constants_to_targets.get(&c) {
            // We already have a wire for this constant.
            return target;
        }

        let gate = self.add_gate(ConstantGate, vec![c]);
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

    pub fn constant_bool(&mut self, b: bool) -> BoolTarget {
        if b {
            self._true()
        } else {
            self._false()
        }
    }

    /// If the given target is a constant (i.e. it was created by the `constant(F)` method), returns
    /// its constant value. Otherwise, returns `None`.
    pub fn target_as_constant(&self, target: Target) -> Option<F> {
        self.targets_to_constants.get(&target).cloned()
    }

    /// If the given `ExtensionTarget` is a constant (i.e. it was created by the
    /// `constant_extension(F)` method), returns its constant value. Otherwise, returns `None`.
    pub fn target_as_constant_ext(&self, target: ExtensionTarget<D>) -> Option<F::Extension> {
        // Get a Vec of any coefficients that are constant. If we end up with exactly D of them,
        // then the `ExtensionTarget` as a whole is constant.
        let const_coeffs: Vec<F> = target
            .0
            .iter()
            .filter_map(|&t| self.target_as_constant(t))
            .collect();

        if let Ok(d_const_coeffs) = const_coeffs.try_into() {
            Some(F::Extension::from_basefield_array(d_const_coeffs))
        } else {
            None
        }
    }

    pub fn push_context(&mut self, level: log::Level, ctx: &str) {
        self.context_log.push(ctx, level, self.num_gates());
    }

    pub fn pop_context(&mut self) {
        self.context_log.pop(self.num_gates());
    }

    pub fn add_marked(&mut self, targets: Markable<D>, name: &str) {
        self.marked_targets.push(MarkedTargets {
            targets,
            name: name.to_string(),
        })
    }

    /// The number of polynomial values that will be revealed per opening, both for the "regular"
    /// polynomials and for the Z polynomials. Because calculating these values involves a recursive
    /// dependence (the amount of blinding depends on the degree, which depends on the blinding),
    /// this function takes in an estimate of the degree.
    fn num_blinding_gates(&self, degree_estimate: usize) -> (usize, usize) {
        let fri_queries = self.config.fri_config.num_query_rounds;
        let arities: Vec<usize> = self
            .config
            .fri_config
            .reduction_arity_bits
            .iter()
            .map(|x| 1 << x)
            .collect();
        let total_fri_folding_points: usize = arities.iter().map(|x| x - 1).sum::<usize>();
        let final_poly_coeffs: usize = degree_estimate / arities.iter().product::<usize>();
        let fri_openings = fri_queries * (1 + D * total_fri_folding_points + D * final_poly_coeffs);

        // We add D for openings at zeta.
        let regular_poly_openings = D + fri_openings;
        // We add 2 * D for openings at zeta and g * zeta.
        let z_openings = 2 * D + fri_openings;

        (regular_poly_openings, z_openings)
    }

    /// The number of polynomial values that will be revealed per opening, both for the "regular"
    /// polynomials (which are opened at only one location) and for the Z polynomials (which are
    /// opened at two).
    fn blinding_counts(&self) -> (usize, usize) {
        let num_gates = self.gate_instances.len();
        let mut degree_estimate = 1 << log2_ceil(num_gates);

        loop {
            let (regular_poly_openings, z_openings) = self.num_blinding_gates(degree_estimate);

            // For most polynomials, we add one random element to offset each opened value.
            // But blinding Z is separate. For that, we add two random elements with a copy
            // constraint between them.
            let total_blinding_count = regular_poly_openings + 2 * z_openings;

            if num_gates + total_blinding_count <= degree_estimate {
                return (regular_poly_openings, z_openings);
            }

            // The blinding gates do not fit within our estimated degree; increase our estimate.
            degree_estimate *= 2;
        }
    }

    fn blind_and_pad(&mut self) {
        if self.config.zero_knowledge {
            self.blind();
        }

        while !self.gate_instances.len().is_power_of_two() {
            self.add_gate(NoopGate, vec![]);
        }
    }

    fn blind(&mut self) {
        let (regular_poly_openings, z_openings) = self.blinding_counts();
        info!(
            "Adding {} blinding terms for witness polynomials, and {}*2 for Z polynomials",
            regular_poly_openings, z_openings
        );

        let num_routed_wires = self.config.num_routed_wires;
        let num_wires = self.config.num_wires;

        // For each "regular" blinding factor, we simply add a no-op gate, and insert a random value
        // for each wire.
        for _ in 0..regular_poly_openings {
            let gate = self.add_gate(NoopGate, vec![]);
            for w in 0..num_wires {
                self.add_generator(RandomValueGenerator {
                    target: Target::Wire(Wire { gate, input: w }),
                });
            }
        }

        // For each z poly blinding factor, we add two new gates with the same random value, and
        // enforce a copy constraint between them.
        // See https://mirprotocol.org/blog/Adding-zero-knowledge-to-Plonk-Halo
        for _ in 0..z_openings {
            let gate_1 = self.add_gate(NoopGate, vec![]);
            let gate_2 = self.add_gate(NoopGate, vec![]);

            for w in 0..num_routed_wires {
                self.add_generator(RandomValueGenerator {
                    target: Target::Wire(Wire {
                        gate: gate_1,
                        input: w,
                    }),
                });
                self.generate_copy(
                    Target::Wire(Wire {
                        gate: gate_1,
                        input: w,
                    }),
                    Target::Wire(Wire {
                        gate: gate_2,
                        input: w,
                    }),
                );
            }
        }
    }

    fn constant_polys(
        &self,
        gates: &[PrefixedGate<F, D>],
        num_constants: usize,
    ) -> Vec<PolynomialValues<F>> {
        let constants_per_gate = self
            .gate_instances
            .iter()
            .map(|gate| {
                let prefix = &gates
                    .iter()
                    .find(|g| g.gate.0.id() == gate.gate_ref.0.id())
                    .unwrap()
                    .prefix;
                let mut prefixed_constants = Vec::with_capacity(num_constants);
                prefixed_constants.extend(prefix.iter().map(|&b| if b { F::ONE } else { F::ZERO }));
                prefixed_constants.extend_from_slice(&gate.constants);
                prefixed_constants.resize(num_constants, F::ZERO);
                prefixed_constants
            })
            .collect::<Vec<_>>();

        transpose(&constants_per_gate)
            .into_iter()
            .map(PolynomialValues::new)
            .collect()
    }

    fn sigma_vecs(
        &self,
        k_is: &[F],
        subgroup: &[F],
    ) -> (Vec<PolynomialValues<F>>, PartitionWitness<F>) {
        let degree = self.gate_instances.len();
        let degree_log = log2_strict(degree);
        let mut partition_witness = PartitionWitness::new(
            self.config.num_wires,
            self.config.num_routed_wires,
            degree,
            self.virtual_target_index,
        );

        for gate in 0..degree {
            for input in 0..self.config.num_wires {
                partition_witness.add(Target::Wire(Wire { gate, input }));
            }
        }

        for index in 0..self.virtual_target_index {
            partition_witness.add(Target::VirtualTarget { index });
        }

        for &CopyConstraint { pair: (a, b), .. } in &self.copy_constraints {
            partition_witness.merge(a, b);
        }

        let wire_partition = partition_witness.wire_partition();
        (
            wire_partition.get_sigma_polys(degree_log, k_is, subgroup),
            partition_witness,
        )
    }

    /// Fill the remaining unused arithmetic operations with zeros, so that all
    /// `ArithmeticExtensionGenerator` are run.
    fn fill_arithmetic_gates(&mut self) {
        let zero = self.zero_extension();
        let remaining_arithmetic_gates = self.free_arithmetic.values().copied().collect::<Vec<_>>();
        for (gate, i) in remaining_arithmetic_gates {
            for j in i..NUM_ARITHMETIC_OPS {
                let wires_multiplicand_0 = ExtensionTarget::from_range(
                    gate,
                    ArithmeticExtensionGate::<D>::wires_ith_multiplicand_0(j),
                );
                let wires_multiplicand_1 = ExtensionTarget::from_range(
                    gate,
                    ArithmeticExtensionGate::<D>::wires_ith_multiplicand_1(j),
                );
                let wires_addend = ExtensionTarget::from_range(
                    gate,
                    ArithmeticExtensionGate::<D>::wires_ith_addend(j),
                );

                self.route_extension(zero, wires_multiplicand_0);
                self.route_extension(zero, wires_multiplicand_1);
                self.route_extension(zero, wires_addend);
            }
        }
    }

    pub fn print_gate_counts(&self, min_delta: usize) {
        self.context_log
            .filter(self.num_gates(), min_delta)
            .print(self.num_gates());
    }

    /// Builds a "full circuit", with both prover and verifier data.
    pub fn build(mut self) -> CircuitData<F, D> {
        let mut timing = TimingTree::new("preprocess", Level::Trace);
        let start = Instant::now();

        self.fill_arithmetic_gates();

        // Hash the public inputs, and route them to a `PublicInputGate` which will enforce that
        // those hash wires match the claimed public inputs.
        let public_inputs_hash = self.hash_n_to_hash(self.public_inputs.clone(), true);
        let pi_gate = self.add_gate(PublicInputGate, vec![]);
        for (&hash_part, wire) in public_inputs_hash
            .elements
            .iter()
            .zip(PublicInputGate::wires_public_inputs_hash())
        {
            self.route(hash_part, Target::wire(pi_gate, wire))
        }

        info!(
            "Degree before blinding & padding: {}",
            self.gate_instances.len()
        );
        self.blind_and_pad();
        let degree = self.gate_instances.len();
        info!("Degree after blinding & padding: {}", degree);
        let degree_bits = log2_strict(degree);
        assert!(
            self.config
                .fri_config
                .reduction_arity_bits
                .iter()
                .sum::<usize>()
                <= degree_bits,
            "FRI total reduction arity is too large."
        );

        let gates = self.gates.iter().cloned().collect();
        let (gate_tree, max_filtered_constraint_degree, num_constants) = Tree::from_gates(gates);
        // `quotient_degree_factor` has to be between `max_filtered_constraint_degree-1` and `1<<rate_bits`.
        // We find the value that minimizes `num_partial_product + quotient_degree_factor`.
        let quotient_degree_factor = (max_filtered_constraint_degree - 1
            ..=1 << self.config.rate_bits)
            .min_by_key(|&q| num_partial_products(self.config.num_routed_wires, q).0 + q)
            .unwrap();
        info!("Quotient degree factor set to: {}.", quotient_degree_factor);
        let prefixed_gates = PrefixedGate::from_tree(gate_tree);

        let subgroup = F::two_adic_subgroup(degree_bits);

        let constant_vecs = self.constant_polys(&prefixed_gates, num_constants);

        let k_is = get_unique_coset_shifts(degree, self.config.num_routed_wires);
        let (sigma_vecs, partition_witness) = self.sigma_vecs(&k_is, &subgroup);

        let constants_sigmas_vecs = [constant_vecs, sigma_vecs.clone()].concat();
        let constants_sigmas_commitment = PolynomialBatchCommitment::from_values(
            constants_sigmas_vecs,
            self.config.rate_bits,
            self.config.zero_knowledge & PlonkPolynomials::CONSTANTS_SIGMAS.blinding,
            self.config.cap_height,
            &mut timing,
        );

        let constants_sigmas_cap = constants_sigmas_commitment.merkle_tree.cap.clone();
        let verifier_only = VerifierOnlyCircuitData {
            constants_sigmas_cap: constants_sigmas_cap.clone(),
        };

        let prover_only = ProverOnlyCircuitData {
            generators: self.generators,
            constants_sigmas_commitment,
            sigmas: transpose_poly_values(sigma_vecs),
            subgroup,
            gate_instances: self.gate_instances,
            public_inputs: self.public_inputs,
            marked_targets: self.marked_targets,
            partition_witness,
        };

        // The HashSet of gates will have a non-deterministic order. When converting to a Vec, we
        // sort by ID to make the ordering deterministic.
        let mut gates = self.gates.iter().cloned().collect::<Vec<_>>();
        gates.sort_unstable_by_key(|gate| gate.0.id());

        let num_gate_constraints = gates
            .iter()
            .map(|gate| gate.0.num_constraints())
            .max()
            .expect("No gates?");

        let num_partial_products =
            num_partial_products(self.config.num_routed_wires, quotient_degree_factor);

        // TODO: This should also include an encoding of gate constraints.
        let circuit_digest_parts = [
            constants_sigmas_cap.flatten(),
            vec![/* Add other circuit data here */],
        ];
        let circuit_digest = hash_n_to_hash(circuit_digest_parts.concat(), false);

        let common = CommonCircuitData {
            config: self.config,
            degree_bits,
            gates: prefixed_gates,
            quotient_degree_factor,
            num_gate_constraints,
            num_constants,
            k_is,
            num_partial_products,
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
    pub fn build_prover(self) -> ProverCircuitData<F, D> {
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
    pub fn build_verifier(self) -> VerifierCircuitData<F, D> {
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
