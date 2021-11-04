use std::cmp::max;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::convert::TryInto;
use std::time::Instant;

use log::{debug, info, Level};

use crate::field::cosets::get_unique_coset_shifts;
use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::{Extendable, FieldExtension};
use crate::field::fft::fft_root_table;
use crate::field::field_types::{Field, RichField};
use crate::fri::commitment::PolynomialBatchCommitment;
use crate::fri::{FriConfig, FriParams};
use crate::gates::arithmetic::ArithmeticExtensionGate;
use crate::gates::constant::ConstantGate;
use crate::gates::gate::{Gate, GateInstance, GateRef, PrefixedGate};
use crate::gates::gate_tree::Tree;
use crate::gates::noop::NoopGate;
use crate::gates::public_input::PublicInputGate;
use crate::gates::random_access::RandomAccessGate;
use crate::gates::switch::SwitchGate;
use crate::hash::hash_types::{HashOutTarget, MerkleCapTarget};
use crate::iop::generator::{
    CopyGenerator, RandomValueGenerator, SimpleGenerator, WitnessGenerator,
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
use crate::plonk::plonk_common::PlonkPolynomials;
use crate::polynomial::polynomial::PolynomialValues;
use crate::util::context_tree::ContextTree;
use crate::util::marking::{Markable, MarkedTargets};
use crate::util::partial_products::num_partial_products;
use crate::util::timing::TimingTree;
use crate::util::{log2_ceil, log2_strict, transpose, transpose_poly_values};

pub struct CircuitBuilder<F: RichField + Extendable<D>, const D: usize> {
    pub(crate) config: CircuitConfig,

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

    /// A vector of marked targets. The values assigned to these targets will be displayed by the prover.
    marked_targets: Vec<MarkedTargets<D>>,

    /// Generators used to generate the witness.
    generators: Vec<Box<dyn WitnessGenerator<F>>>,

    constants_to_targets: HashMap<F, Target>,
    targets_to_constants: HashMap<Target, F>,

    /// A map `(c0, c1) -> (g, i)` from constants `(c0,c1)` to an available arithmetic gate using
    /// these constants with gate index `g` and already using `i` arithmetic operations.
    pub(crate) free_arithmetic: HashMap<(F, F), (usize, usize)>,

    /// A map `(c0, c1) -> (g, i)` from constants `vec_size` to an available arithmetic gate using
    /// these constants with gate index `g` and already using `i` random accesses.
    pub(crate) free_random_access: HashMap<usize, (usize, usize)>,

    // `current_switch_gates[chunk_size - 1]` contains None if we have no switch gates with the value
    // chunk_size, and contains `(g, i, c)`, if the gate `g`, at index `i`, already contains `c` copies
    // of switches
    pub(crate) current_switch_gates: Vec<Option<(SwitchGate<F, D>, usize, usize)>>,
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
            marked_targets: Vec::new(),
            generators: Vec::new(),
            constants_to_targets: HashMap::new(),
            targets_to_constants: HashMap::new(),
            free_arithmetic: HashMap::new(),
            free_random_access: HashMap::new(),
            current_switch_gates: Vec::new(),
        };
        builder.check_config();
        builder
    }

    fn check_config(&self) {
        let &CircuitConfig {
            security_bits,
            rate_bits,
            fri_config:
                FriConfig {
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

    fn fri_params(&self, degree_bits_estimate: usize) -> FriParams {
        let fri_config = &self.config.fri_config;
        let reduction_arity_bits = fri_config.reduction_strategy.reduction_arity_bits(
            degree_bits_estimate,
            self.config.rate_bits,
            fri_config.num_query_rounds,
        );
        FriParams {
            reduction_arity_bits,
        }
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
            let gate = self.add_gate(NoopGate, vec![]);
            for w in 0..num_wires {
                self.add_simple_generator(RandomValueGenerator {
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
                self.add_simple_generator(RandomValueGenerator {
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
                forest.add(Target::Wire(Wire { gate, input }));
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

    /// Fill the remaining unused arithmetic operations with zeros, so that all
    /// `ArithmeticExtensionGenerator` are run.
    fn fill_arithmetic_gates(&mut self) {
        let zero = self.zero_extension();
        let remaining_arithmetic_gates = self.free_arithmetic.values().copied().collect::<Vec<_>>();
        for (gate, i) in remaining_arithmetic_gates {
            for j in i..ArithmeticExtensionGate::<D>::num_ops(&self.config) {
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

                self.connect_extension(zero, wires_multiplicand_0);
                self.connect_extension(zero, wires_multiplicand_1);
                self.connect_extension(zero, wires_addend);
            }
        }
    }

    /// Fill the remaining unused random access operations with zeros, so that all
    /// `RandomAccessGenerator`s are run.
    fn fill_random_access_gates(&mut self) {
        let zero = self.zero();
        for (vec_size, (_, i)) in self.free_random_access.clone() {
            let max_copies = RandomAccessGate::<F, D>::max_num_copies(
                self.config.num_routed_wires,
                self.config.num_wires,
                vec_size,
            );
            for _ in i..max_copies {
                self.random_access(zero, zero, vec![zero; vec_size]);
            }
        }
    }

    /// Fill the remaining unused switch gates with dummy values, so that all
    /// `SwitchGenerator` are run.
    fn fill_switch_gates(&mut self) {
        let zero = self.zero();

        for chunk_size in 1..=self.current_switch_gates.len() {
            if let Some((gate, gate_index, mut copy)) =
                self.current_switch_gates[chunk_size - 1].clone()
            {
                while copy < gate.num_copies {
                    for element in 0..chunk_size {
                        let wire_first_input =
                            Target::wire(gate_index, gate.wire_first_input(copy, element));
                        let wire_second_input =
                            Target::wire(gate_index, gate.wire_second_input(copy, element));
                        let wire_switch_bool =
                            Target::wire(gate_index, gate.wire_switch_bool(copy));
                        self.connect(zero, wire_first_input);
                        self.connect(zero, wire_second_input);
                        self.connect(zero, wire_switch_bool);
                    }
                    copy += 1;
                }
            }
        }
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
    pub fn build<C: GenericConfig<D, F = F>>(mut self) -> CircuitData<F, C, D> {
        let mut timing = TimingTree::new("preprocess", Level::Trace);
        let start = Instant::now();

        self.fill_arithmetic_gates();
        self.fill_random_access_gates();
        self.fill_switch_gates();

        // Hash the public inputs, and route them to a `PublicInputGate` which will enforce that
        // those hash wires match the claimed public inputs.
        let public_inputs_hash =
            self.hash_n_to_hash::<C::InnerHasher>(self.public_inputs.clone(), true);
        let pi_gate = self.add_gate(PublicInputGate, vec![]);
        for (&hash_part, wire) in public_inputs_hash
            .elements
            .iter()
            .zip(PublicInputGate::wires_public_inputs_hash())
        {
            self.connect(hash_part, Target::wire(pi_gate, wire))
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
            fri_params.total_arities() <= degree_bits,
            "FRI total reduction arity is too large.",
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
        let (sigma_vecs, forest) = self.sigma_vecs(&k_is, &subgroup);

        // Precompute FFT roots.
        let max_fft_points =
            1 << degree_bits + max(self.config.rate_bits, log2_ceil(quotient_degree_factor));
        let fft_root_table = fft_root_table(max_fft_points);

        let constants_sigmas_vecs = [constant_vecs, sigma_vecs.clone()].concat();
        let constants_sigmas_commitment = PolynomialBatchCommitment::from_values(
            constants_sigmas_vecs,
            self.config.rate_bits,
            self.config.zero_knowledge & PlonkPolynomials::CONSTANTS_SIGMAS.blinding,
            self.config.cap_height,
            &mut timing,
            Some(&fft_root_table),
        );

        let constants_sigmas_cap = constants_sigmas_commitment.merkle_tree.cap.clone();
        let verifier_only = VerifierOnlyCircuitData {
            constants_sigmas_cap: constants_sigmas_cap.clone(),
        };

        // Index generator indices by their watched targets.
        let mut generator_indices_by_watches = BTreeMap::new();
        for (i, generator) in self.generators.iter().enumerate() {
            for watch in generator.watch_list() {
                let watch_index = forest.target_index(watch);
                let watch_rep_index = forest.parents[watch_index];
                generator_indices_by_watches
                    .entry(watch_rep_index)
                    .or_insert(vec![])
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
            marked_targets: self.marked_targets,
            representative_map: forest.parents,
            fft_root_table: Some(fft_root_table),
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
        let circuit_digest = C::Hasher::hash(circuit_digest_parts.concat(), false);

        let common = CommonCircuitData {
            config: self.config,
            fri_params,
            degree_bits,
            gates: prefixed_gates,
            quotient_degree_factor,
            num_gate_constraints,
            num_constants,
            num_virtual_targets: self.virtual_target_index,
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
    pub fn build_prover<C: GenericConfig<D, F = F>>(self) -> ProverCircuitData<F, C, D> {
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
    pub fn build_verifier<C: GenericConfig<D, F = F>>(self) -> VerifierCircuitData<F, C, D> {
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
