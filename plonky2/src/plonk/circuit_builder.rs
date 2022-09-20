use std::cmp::max;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::Instant;

use itertools::Itertools;
use log::{debug, info, Level};
use plonky2_field::cosets::get_unique_coset_shifts;
use plonky2_field::extension::{Extendable, FieldExtension};
use plonky2_field::fft::fft_root_table;
use plonky2_field::polynomial::PolynomialValues;
use plonky2_field::types::Field;
use plonky2_util::{log2_ceil, log2_strict};

use crate::fri::oracle::PolynomialBatch;
use crate::fri::{FriConfig, FriParams};
use crate::gadgets::arithmetic::BaseArithmeticOperation;
use crate::gadgets::arithmetic_extension::ExtensionArithmeticOperation;
use crate::gadgets::polynomial::PolynomialCoeffsExtTarget;
use crate::gates::arithmetic_base::ArithmeticGate;
use crate::gates::arithmetic_extension::ArithmeticExtensionGate;
use crate::gates::constant::ConstantGate;
use crate::gates::gate::{CurrentSlot, Gate, GateInstance, GateRef};
use crate::gates::noop::NoopGate;
use crate::gates::public_input::PublicInputGate;
use crate::gates::selectors::selector_polynomials;
use crate::hash::hash_types::{HashOutTarget, MerkleCapTarget, RichField};
use crate::hash::merkle_proofs::MerkleProofTarget;
use crate::iop::ext_target::ExtensionTarget;
use crate::iop::generator::{
    ConstantGenerator, CopyGenerator, RandomValueGenerator, SimpleGenerator, WitnessGenerator,
};
use crate::iop::target::{BoolTarget, Target};
use crate::iop::wire::Wire;
use crate::plonk::circuit_data::{
    CircuitConfig, CircuitData, CommonCircuitData, ProverCircuitData, ProverOnlyCircuitData,
    VerifierCircuitData, VerifierOnlyCircuitData,
};
use crate::plonk::config::{GenericConfig, Hasher};
use crate::plonk::copy_constraint::CopyConstraint;
use crate::plonk::permutation_argument::Forest;
use crate::plonk::plonk_common::PlonkOracle;
use crate::timed;
use crate::util::context_tree::ContextTree;
use crate::util::partial_products::num_partial_products;
use crate::util::timing::TimingTree;
use crate::util::{transpose, transpose_poly_values};

pub struct CircuitBuilder<F: RichField + Extendable<D>, const D: usize> {
    pub config: CircuitConfig,

    /// The types of gates used in this circuit.
    gates: HashSet<GateRef<F, D>>,

    /// The concrete placement of each gate.
    pub(crate) gate_instances: Vec<GateInstance<F, D>>,

    /// Targets to be made public.
    public_inputs: Vec<Target>,

    /// The next available index for a `VirtualTarget`.
    virtual_target_index: usize,

    copy_constraints: Vec<CopyConstraint>,

    /// A tree of named scopes, used for debugging.
    context_log: ContextTree,

    /// Generators used to generate the witness.
    generators: Vec<Box<dyn WitnessGenerator<F>>>,

    constants_to_targets: HashMap<F, Target>,
    targets_to_constants: HashMap<Target, F>,

    /// Memoized results of `arithmetic` calls.
    pub(crate) base_arithmetic_results: HashMap<BaseArithmeticOperation<F>, Target>,

    /// Memoized results of `arithmetic_extension` calls.
    pub(crate) arithmetic_results: HashMap<ExtensionArithmeticOperation<F, D>, ExtensionTarget<D>>,

    /// Map between gate type and the current gate of this type with available slots.
    current_slots: HashMap<GateRef<F, D>, CurrentSlot<F, D>>,

    /// List of constant generators used to fill the constant wires.
    constant_generators: Vec<ConstantGenerator<F>>,
}

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    pub fn new(config: CircuitConfig) -> Self {
        let builder = CircuitBuilder {
            config,
            gates: HashSet::new(),
            gate_instances: Vec::new(),
            public_inputs: Vec::new(),
            virtual_target_index: 0,
            copy_constraints: Vec::new(),
            context_log: ContextTree::new(),
            generators: Vec::new(),
            constants_to_targets: HashMap::new(),
            targets_to_constants: HashMap::new(),
            base_arithmetic_results: HashMap::new(),
            arithmetic_results: HashMap::new(),
            current_slots: HashMap::new(),
            constant_generators: Vec::new(),
        };
        builder.check_config();
        builder
    }

    fn check_config(&self) {
        let &CircuitConfig {
            security_bits,
            fri_config:
                FriConfig {
                    rate_bits,
                    proof_of_work_bits,
                    num_query_rounds,
                    ..
                },
            ..
        } = &self.config;

        // Conjectured FRI security; see the ethSTARK paper.
        let fri_field_bits = F::Extension::order().bits() as usize;
        let fri_query_security_bits = num_query_rounds * rate_bits + proof_of_work_bits as usize;
        let fri_security_bits = fri_field_bits.min(fri_query_security_bits);
        assert!(
            fri_security_bits >= security_bits,
            "FRI params fall short of target security"
        );
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

    pub fn add_virtual_target_arr<const N: usize>(&mut self) -> [Target; N] {
        [0; N].map(|_| self.add_virtual_target())
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

    pub(crate) fn add_virtual_merkle_proof(&mut self, len: usize) -> MerkleProofTarget {
        MerkleProofTarget {
            siblings: self.add_virtual_hashes(len),
        }
    }

    pub fn add_virtual_extension_target(&mut self) -> ExtensionTarget<D> {
        ExtensionTarget(self.add_virtual_targets(D).try_into().unwrap())
    }

    pub fn add_virtual_extension_targets(&mut self, n: usize) -> Vec<ExtensionTarget<D>> {
        (0..n)
            .map(|_i| self.add_virtual_extension_target())
            .collect()
    }

    pub(crate) fn add_virtual_poly_coeff_ext(
        &mut self,
        num_coeffs: usize,
    ) -> PolynomialCoeffsExtTarget<D> {
        let coeffs = self.add_virtual_extension_targets(num_coeffs);
        PolynomialCoeffsExtTarget(coeffs)
    }

    // TODO: Unsafe
    pub fn add_virtual_bool_target(&mut self) -> BoolTarget {
        BoolTarget::new_unsafe(self.add_virtual_target())
    }

    pub fn add_virtual_bool_target_safe(&mut self) -> BoolTarget {
        let b = BoolTarget::new_unsafe(self.add_virtual_target());
        self.assert_bool(b);
        b
    }

    /// Adds a gate to the circuit, and returns its index.
    pub fn add_gate<G: Gate<F, D>>(&mut self, gate_type: G, mut constants: Vec<F>) -> usize {
        self.check_gate_compatibility(&gate_type);

        assert!(
            constants.len() <= gate_type.num_constants(),
            "Too many constants."
        );
        constants.resize(gate_type.num_constants(), F::ZERO);

        let row = self.gate_instances.len();

        self.constant_generators
            .extend(gate_type.extra_constant_wires().into_iter().map(
                |(constant_index, wire_index)| ConstantGenerator {
                    row,
                    constant_index,
                    wire_index,
                    constant: F::ZERO, // Placeholder; will be replaced later.
                },
            ));

        // Note that we can't immediately add this gate's generators, because the list of constants
        // could be modified later, i.e. in the case of `ConstantGate`. We will add them later in
        // `build` instead.

        // Register this gate type if we haven't seen it before.
        let gate_ref = GateRef::new(gate_type);
        self.gates.insert(gate_ref.clone());

        self.gate_instances.push(GateInstance {
            gate_ref,
            constants,
        });

        row
    }

    fn check_gate_compatibility<G: Gate<F, D>>(&self, gate: &G) {
        assert!(
            gate.num_wires() <= self.config.num_wires,
            "{:?} requires {} wires, but our CircuitConfig has only {}",
            gate.id(),
            gate.num_wires(),
            self.config.num_wires
        );
        assert!(
            gate.num_constants() <= self.config.num_constants,
            "{:?} requires {} constants, but our CircuitConfig has only {}",
            gate.id(),
            gate.num_constants(),
            self.config.num_constants
        );
    }

    pub fn connect_extension(&mut self, src: ExtensionTarget<D>, dst: ExtensionTarget<D>) {
        for i in 0..D {
            self.connect(src.0[i], dst.0[i]);
        }
    }

    /// Adds a generator which will copy `src` to `dst`.
    pub fn generate_copy(&mut self, src: Target, dst: Target) {
        self.add_simple_generator(CopyGenerator { src, dst });
    }

    /// Uses Plonk's permutation argument to require that two elements be equal.
    /// Both elements must be routable, otherwise this method will panic.
    pub fn connect(&mut self, x: Target, y: Target) {
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

    pub fn assert_zero(&mut self, x: Target) {
        let zero = self.zero();
        self.connect(x, zero);
    }

    pub fn assert_one(&mut self, x: Target) {
        let one = self.one();
        self.connect(x, one);
    }

    pub fn add_generators(&mut self, generators: Vec<Box<dyn WitnessGenerator<F>>>) {
        self.generators.extend(generators);
    }

    pub fn add_simple_generator<G: SimpleGenerator<F>>(&mut self, generator: G) {
        self.generators.push(Box::new(generator.adapter()));
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

        let target = self.add_virtual_target();
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

    /// Find an available slot, of the form `(row, op)` for gate `G` using parameters `params`
    /// and constants `constants`. Parameters are any data used to differentiate which gate should be
    /// used for the given operation.
    pub fn find_slot<G: Gate<F, D> + Clone>(
        &mut self,
        gate: G,
        params: &[F],
        constants: &[F],
    ) -> (usize, usize) {
        let num_gates = self.num_gates();
        let num_ops = gate.num_ops();
        let gate_ref = GateRef::new(gate.clone());
        let gate_slot = self.current_slots.entry(gate_ref.clone()).or_default();
        let slot = gate_slot.current_slot.get(params);
        let (gate_idx, slot_idx) = if let Some(&s) = slot {
            s
        } else {
            self.add_gate(gate, constants.to_vec());
            (num_gates, 0)
        };
        let current_slot = &mut self.current_slots.get_mut(&gate_ref).unwrap().current_slot;
        if slot_idx == num_ops - 1 {
            // We've filled up the slots at this index.
            current_slot.remove(params);
        } else {
            // Increment the slot operation index.
            current_slot.insert(params.to_vec(), (gate_idx, slot_idx + 1));
        }

        (gate_idx, slot_idx)
    }

    fn fri_params(&self, degree_bits: usize) -> FriParams {
        self.config
            .fri_config
            .fri_params(degree_bits, self.config.zero_knowledge)
    }

    /// The number of (base field) `arithmetic` operations that can be performed in a single gate.
    pub(crate) fn num_base_arithmetic_ops_per_gate(&self) -> usize {
        if self.config.use_base_arithmetic_gate {
            ArithmeticGate::new_from_config(&self.config).num_ops
        } else {
            self.num_ext_arithmetic_ops_per_gate()
        }
    }

    /// The number of `arithmetic_extension` operations that can be performed in a single gate.
    pub(crate) fn num_ext_arithmetic_ops_per_gate(&self) -> usize {
        ArithmeticExtensionGate::<D>::new_from_config(&self.config).num_ops
    }

    /// The number of polynomial values that will be revealed per opening, both for the "regular"
    /// polynomials and for the Z polynomials. Because calculating these values involves a recursive
    /// dependence (the amount of blinding depends on the degree, which depends on the blinding),
    /// this function takes in an estimate of the degree.
    fn num_blinding_gates(&self, degree_estimate: usize) -> (usize, usize) {
        let degree_bits_estimate = log2_strict(degree_estimate);
        let fri_queries = self.config.fri_config.num_query_rounds;
        let arities: Vec<usize> = self
            .fri_params(degree_bits_estimate)
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
            let row = self.add_gate(NoopGate, vec![]);
            for w in 0..num_wires {
                self.add_simple_generator(RandomValueGenerator {
                    target: Target::Wire(Wire { row, column: w }),
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
                self.add_simple_generator(RandomValueGenerator {
                    target: Target::Wire(Wire {
                        row: gate_1,
                        column: w,
                    }),
                });
                self.generate_copy(
                    Target::Wire(Wire {
                        row: gate_1,
                        column: w,
                    }),
                    Target::Wire(Wire {
                        row: gate_2,
                        column: w,
                    }),
                );
            }
        }
    }

    fn constant_polys(&self) -> Vec<PolynomialValues<F>> {
        let max_constants = self
            .gates
            .iter()
            .map(|g| g.0.num_constants())
            .max()
            .unwrap();
        transpose(
            &self
                .gate_instances
                .iter()
                .map(|g| {
                    let mut consts = g.constants.clone();
                    consts.resize(max_constants, F::ZERO);
                    consts
                })
                .collect::<Vec<_>>(),
        )
        .into_iter()
        .map(PolynomialValues::new)
        .collect()
    }

    fn sigma_vecs(&self, k_is: &[F], subgroup: &[F]) -> (Vec<PolynomialValues<F>>, Forest) {
        let degree = self.gate_instances.len();
        let degree_log = log2_strict(degree);
        let config = &self.config;
        let mut forest = Forest::new(
            config.num_wires,
            config.num_routed_wires,
            degree,
            self.virtual_target_index,
        );

        for gate in 0..degree {
            for input in 0..config.num_wires {
                forest.add(Target::Wire(Wire {
                    row: gate,
                    column: input,
                }));
            }
        }

        for index in 0..self.virtual_target_index {
            forest.add(Target::VirtualTarget { index });
        }

        for &CopyConstraint { pair: (a, b), .. } in &self.copy_constraints {
            forest.merge(a, b);
        }

        forest.compress_paths();

        let wire_partition = forest.wire_partition();
        (
            wire_partition.get_sigma_polys(degree_log, k_is, subgroup),
            forest,
        )
    }

    pub fn print_gate_counts(&self, min_delta: usize) {
        // Print gate counts for each context.
        self.context_log
            .filter(self.num_gates(), min_delta)
            .print(self.num_gates());

        // Print total count of each gate type.
        debug!("Total gate counts:");
        for gate in self.gates.iter().cloned() {
            let count = self
                .gate_instances
                .iter()
                .filter(|inst| inst.gate_ref == gate)
                .count();
            debug!("- {} instances of {}", count, gate.0.id());
        }
    }

    /// Builds a "full circuit", with both prover and verifier data.
    pub fn build<C: GenericConfig<D, F = F>>(mut self) -> CircuitData<F, C, D>
    where
        [(); C::Hasher::HASH_SIZE]:,
    {
        let mut timing = TimingTree::new("preprocess", Level::Trace);
        let start = Instant::now();
        let rate_bits = self.config.fri_config.rate_bits;
        let cap_height = self.config.fri_config.cap_height;

        // Hash the public inputs, and route them to a `PublicInputGate` which will enforce that
        // those hash wires match the claimed public inputs.
        let num_public_inputs = self.public_inputs.len();
        let public_inputs_hash =
            self.hash_n_to_hash_no_pad::<C::InnerHasher>(self.public_inputs.clone());
        let pi_gate = self.add_gate(PublicInputGate, vec![]);
        for (&hash_part, wire) in public_inputs_hash
            .elements
            .iter()
            .zip(PublicInputGate::wires_public_inputs_hash())
        {
            self.connect(hash_part, Target::wire(pi_gate, wire))
        }

        // Make sure we have enough constant generators. If not, add a `ConstantGate`.
        while self.constants_to_targets.len() > self.constant_generators.len() {
            self.add_gate(
                ConstantGate {
                    num_consts: self.config.num_constants,
                },
                vec![],
            );
        }

        // For each constant-target pair used in the circuit, use a constant generator to fill this target.
        for ((c, t), mut const_gen) in self
            .constants_to_targets
            .clone()
            .into_iter()
            // We need to enumerate constants_to_targets in some deterministic order to ensure that
            // building a circuit is deterministic.
            .sorted_by_key(|(c, _t)| c.to_canonical_u64())
            .zip(self.constant_generators.clone())
        {
            // Set the constant in the constant polynomial.
            self.gate_instances[const_gen.row].constants[const_gen.constant_index] = c;
            // Generate a copy between the target and the routable wire.
            self.connect(Target::wire(const_gen.row, const_gen.wire_index), t);
            // Set the constant in the generator (it's initially set with a dummy value).
            const_gen.set_constant(c);
            self.add_simple_generator(const_gen);
        }

        info!(
            "Degree before blinding & padding: {}",
            self.gate_instances.len()
        );
        self.blind_and_pad();
        let degree = self.gate_instances.len();
        info!("Degree after blinding & padding: {}", degree);
        let degree_bits = log2_strict(degree);
        let fri_params = self.fri_params(degree_bits);
        assert!(
            fri_params.total_arities() <= degree_bits + rate_bits - cap_height,
            "FRI total reduction arity is too large.",
        );

        let quotient_degree_factor = self.config.max_quotient_degree_factor;
        let mut gates = self.gates.iter().cloned().collect::<Vec<_>>();
        // Gates need to be sorted by their degrees (and ID to make the ordering deterministic) to compute the selector polynomials.
        gates.sort_unstable_by_key(|g| (g.0.degree(), g.0.id()));
        let (mut constant_vecs, selectors_info) =
            selector_polynomials(&gates, &self.gate_instances, quotient_degree_factor + 1);
        constant_vecs.extend(self.constant_polys());
        let num_constants = constant_vecs.len();

        let subgroup = F::two_adic_subgroup(degree_bits);

        let k_is = get_unique_coset_shifts(degree, self.config.num_routed_wires);
        let (sigma_vecs, forest) = timed!(
            timing,
            "generate sigma polynomials",
            self.sigma_vecs(&k_is, &subgroup)
        );

        // Precompute FFT roots.
        let max_fft_points = 1 << (degree_bits + max(rate_bits, log2_ceil(quotient_degree_factor)));
        let fft_root_table = fft_root_table(max_fft_points);

        let constants_sigmas_vecs = [constant_vecs, sigma_vecs.clone()].concat();
        let constants_sigmas_commitment = PolynomialBatch::from_values(
            constants_sigmas_vecs,
            rate_bits,
            PlonkOracle::CONSTANTS_SIGMAS.blinding,
            cap_height,
            &mut timing,
            Some(&fft_root_table),
        );

        let constants_sigmas_cap = constants_sigmas_commitment.merkle_tree.cap.clone();
        let verifier_only = VerifierOnlyCircuitData {
            constants_sigmas_cap: constants_sigmas_cap.clone(),
        };

        // Map between gates where not all generators are used and the gate's number of used generators.
        let incomplete_gates = self
            .current_slots
            .values()
            .flat_map(|current_slot| current_slot.current_slot.values().copied())
            .collect::<HashMap<_, _>>();

        // Add gate generators.
        self.add_generators(
            self.gate_instances
                .iter()
                .enumerate()
                .flat_map(|(index, gate)| {
                    let mut gens = gate.gate_ref.0.generators(index, &gate.constants);
                    // Remove unused generators, if any.
                    if let Some(&op) = incomplete_gates.get(&index) {
                        gens.drain(op..);
                    }
                    gens
                })
                .collect(),
        );

        // Index generator indices by their watched targets.
        let mut generator_indices_by_watches = BTreeMap::new();
        for (i, generator) in self.generators.iter().enumerate() {
            for watch in generator.watch_list() {
                let watch_index = forest.target_index(watch);
                let watch_rep_index = forest.parents[watch_index];
                generator_indices_by_watches
                    .entry(watch_rep_index)
                    .or_insert_with(Vec::new)
                    .push(i);
            }
        }
        for indices in generator_indices_by_watches.values_mut() {
            indices.dedup();
            indices.shrink_to_fit();
        }

        let prover_only = ProverOnlyCircuitData {
            generators: self.generators,
            generator_indices_by_watches,
            constants_sigmas_commitment,
            sigmas: transpose_poly_values(sigma_vecs),
            subgroup,
            public_inputs: self.public_inputs,
            representative_map: forest.parents,
            fft_root_table: Some(fft_root_table),
        };

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
            vec![
                F::from_canonical_usize(degree_bits),
                /* Add other circuit data here */
            ],
        ];
        let circuit_digest = C::Hasher::hash_no_pad(&circuit_digest_parts.concat());

        let common = CommonCircuitData {
            config: self.config,
            fri_params,
            degree_bits,
            gates,
            selectors_info,
            quotient_degree_factor,
            num_gate_constraints,
            num_constants,
            num_public_inputs,
            k_is,
            num_partial_products,
            circuit_digest,
        };

        timing.print();
        debug!("Building circuit took {}s", start.elapsed().as_secs_f32());
        CircuitData {
            prover_only,
            verifier_only,
            common,
        }
    }

    /// Builds a "prover circuit", with data needed to generate proofs but not verify them.
    pub fn build_prover<C: GenericConfig<D, F = F>>(self) -> ProverCircuitData<F, C, D>
    where
        [(); C::Hasher::HASH_SIZE]:,
    {
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
    pub fn build_verifier<C: GenericConfig<D, F = F>>(self) -> VerifierCircuitData<F, C, D>
    where
        [(); C::Hasher::HASH_SIZE]:,
    {
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
