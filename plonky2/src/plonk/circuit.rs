use std::cmp::max;
use std::collections::{BTreeMap, HashSet};
use std::time::Instant;

use log::{debug, info, Level};
use plonky2_field::cosets::get_unique_coset_shifts;
use plonky2_field::extension_field::Extendable;
use plonky2_field::fft::fft_root_table;
use plonky2_field::polynomial::PolynomialValues;
use plonky2_util::{log2_ceil, log2_strict};

use crate::fri::oracle::PolynomialBatch;
use crate::fri::FriParams;
use crate::gates::gate::{GateInstance, GateRef, PrefixedGate};
use crate::gates::gate_tree::Tree;
use crate::hash::hash_types::RichField;
use crate::plonk::circuit_data::{
    CircuitConfig, CircuitData, CommonCircuitData, ProverCircuitData, ProverOnlyCircuitData,
    VerifierCircuitData, VerifierOnlyCircuitData,
};
use crate::plonk::config::{GenericConfig, Hasher};
use crate::plonk::permutation_argument::Forest;
use crate::plonk::plonk_common::PlonkOracle;
use crate::util::partial_products::num_partial_products;
use crate::util::timing::TimingTree;
use crate::util::transpose_poly_values;

#[derive(Debug, Clone)]
pub struct Circuit<F: RichField + Extendable<D>, const D: usize> {
    pub(crate) config: CircuitConfig,

    /// The types of gates used in this circuit.
    gates: HashSet<GateRef<F, D>>,

    /// The concrete placement of each gate.
    pub(crate) gate_instances: Vec<GateInstance<F, D>>,
}

impl<F: RichField + Extendable<D>, const D: usize> Circuit<F, D> {
    fn fri_params(&self, degree_bits: usize) -> FriParams {
        let fri_config = &self.config.fri_config;
        let reduction_arity_bits = fri_config.reduction_strategy.reduction_arity_bits(
            degree_bits,
            self.config.fri_config.rate_bits,
            fri_config.num_query_rounds,
        );
        FriParams {
            config: fri_config.clone(),
            hiding: self.config.zero_knowledge,
            degree_bits,
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
        //
        // let num_gates = self.gate_instances.len();
        // let mut degree_estimate = 1 << log2_ceil(num_gates);
        //
        // loop {
        //     let (regular_poly_openings, z_openings) = self.num_blinding_gates(degree_estimate);
        //
        //     // For most polynomials, we add one random element to offset each opened value.
        //     // But blinding Z is separate. For that, we add two random elements with a copy
        //     // constraint between them.
        //     let total_blinding_count = regular_poly_openings + 2 * z_openings;
        //
        //     if num_gates + total_blinding_count <= degree_estimate {
        //         return (regular_poly_openings, z_openings);
        //     }
        //
        //     // The blinding gates do not fit within our estimated degree; increase our estimate.
        //     degree_estimate *= 2;
        // }
        todo!()
    }
    fn blind_and_pad(&mut self) {
        // if self.config.zero_knowledge {
        //     self.blind();
        // }
        //
        // while !self.gate_instances.len().is_power_of_two() {
        //     self.add_gate(NoopGate, vec![]);
        // }
        todo!()
    }

    fn blind(&mut self) {
        // let (regular_poly_openings, z_openings) = self.blinding_counts();
        // info!(
        //     "Adding {} blinding terms for witness polynomials, and {}*2 for Z polynomials",
        //     regular_poly_openings, z_openings
        // );
        //
        // let num_routed_wires = self.config.num_routed_wires;
        // let num_wires = self.config.num_wires;
        //
        // // For each "regular" blinding factor, we simply add a no-op gate, and insert a random value
        // // for each wire.
        // for _ in 0..regular_poly_openings {
        //     let gate = self.add_gate(NoopGate, vec![]);
        //     for w in 0..num_wires {
        //         self.add_simple_generator(RandomValueGenerator {
        //             target: Target::Wire(Wire { gate, input: w }),
        //         });
        //     }
        // }
        //
        // // For each z poly blinding factor, we add two new gates with the same random value, and
        // // enforce a copy constraint between them.
        // // See https://mirprotocol.org/blog/Adding-zero-knowledge-to-Plonk-Halo
        // for _ in 0..z_openings {
        //     let gate_1 = self.add_gate(NoopGate, vec![]);
        //     let gate_2 = self.add_gate(NoopGate, vec![]);
        //
        //     for w in 0..num_routed_wires {
        //         self.add_simple_generator(RandomValueGenerator {
        //             target: Target::Wire(Wire {
        //                 gate: gate_1,
        //                 input: w,
        //             }),
        //         });
        //         self.generate_copy(
        //             Target::Wire(Wire {
        //                 gate: gate_1,
        //                 input: w,
        //             }),
        //             Target::Wire(Wire {
        //                 gate: gate_2,
        //                 input: w,
        //             }),
        //         );
        //     }
        // }
        todo!()
    }

    fn constant_polys(
        &self,
        gates: &[PrefixedGate<F, D>],
        num_constants: usize,
    ) -> Vec<PolynomialValues<F>> {
        // let constants_per_gate = self
        //     .gate_instances
        //     .iter()
        //     .map(|gate| {
        //         let prefix = &gates
        //             .iter()
        //             .find(|g| g.gate.0.id() == gate.gate_ref.0.id())
        //             .unwrap()
        //             .prefix;
        //         let mut prefixed_constants = Vec::with_capacity(num_constants);
        //         prefixed_constants.extend(prefix.iter().map(|&b| if b { F::ONE } else { F::ZERO }));
        //         prefixed_constants.extend_from_slice(&gate.constants);
        //         prefixed_constants.resize(num_constants, F::ZERO);
        //         prefixed_constants
        //     })
        //     .collect::<Vec<_>>();
        //
        // transpose(&constants_per_gate)
        //     .into_iter()
        //     .map(PolynomialValues::new)
        //     .collect()
        todo!()
    }

    fn sigma_vecs(&self, k_is: &[F], subgroup: &[F]) -> (Vec<PolynomialValues<F>>, Forest) {
        // let degree = self.gate_instances.len();
        // let degree_log = log2_strict(degree);
        // let config = &self.config;
        // let mut forest = Forest::new(
        //     config.num_wires,
        //     config.num_routed_wires,
        //     degree,
        //     self.virtual_target_index,
        // );
        //
        // for gate in 0..degree {
        //     for input in 0..config.num_wires {
        //         forest.add(Target::Wire(Wire { gate, input }));
        //     }
        // }
        //
        // for index in 0..self.virtual_target_index {
        //     forest.add(Target::VirtualTarget { index });
        // }
        //
        // for &CopyConstraint { pair: (a, b), .. } in &self.copy_constraints {
        //     forest.merge(a, b);
        // }
        //
        // forest.compress_paths();
        //
        // let wire_partition = forest.wire_partition();
        // (
        //     wire_partition.get_sigma_polys(degree_log, k_is, subgroup),
        //     forest,
        // )
        todo!()
    }

    pub fn build<C: GenericConfig<D, F = F>>(mut self) -> CircuitData<F, C, D> {
        let mut timing = TimingTree::new("preprocess", Level::Trace);
        let start = Instant::now();
        let rate_bits = self.config.fri_config.rate_bits;

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
        let prefixed_gates = PrefixedGate::from_tree(gate_tree);

        // `quotient_degree_factor` has to be between `max_filtered_constraint_degree-1` and `1<<rate_bits`.
        // We find the value that minimizes `num_partial_product + quotient_degree_factor`.
        let min_quotient_degree_factor = max_filtered_constraint_degree - 1;
        let max_quotient_degree_factor = self.config.max_quotient_degree_factor.min(1 << rate_bits);
        let quotient_degree_factor = (min_quotient_degree_factor..=max_quotient_degree_factor)
            .min_by_key(|&q| num_partial_products(self.config.num_routed_wires, q).0 + q)
            .unwrap();
        debug!("Quotient degree factor set to: {}.", quotient_degree_factor);

        let subgroup = F::two_adic_subgroup(degree_bits);

        let constant_vecs = self.constant_polys(&prefixed_gates, num_constants);

        let k_is = get_unique_coset_shifts(degree, self.config.num_routed_wires);
        let (sigma_vecs, forest) = self.sigma_vecs(&k_is, &subgroup);

        // Precompute FFT roots.
        let max_fft_points = 1 << (degree_bits + max(rate_bits, log2_ceil(quotient_degree_factor)));
        let fft_root_table = fft_root_table(max_fft_points);

        let constants_sigmas_vecs = [constant_vecs, sigma_vecs.clone()].concat();
        let constants_sigmas_commitment = PolynomialBatch::from_values(
            constants_sigmas_vecs,
            rate_bits,
            PlonkOracle::CONSTANTS_SIGMAS.blinding,
            self.config.fri_config.cap_height,
            &mut timing,
            Some(&fft_root_table),
        );

        let constants_sigmas_cap = constants_sigmas_commitment.merkle_tree.cap.clone();
        let verifier_only = VerifierOnlyCircuitData {
            constants_sigmas_cap: constants_sigmas_cap.clone(),
        };

        // Add gate generators.
        self.add_generators(
            self.gate_instances
                .iter()
                .enumerate()
                .flat_map(|(index, gate)| gate.gate_ref.0.generators(index, &gate.constants))
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

        debug!("Building circuit took {}s", start.elapsed().as_secs_f32());
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
