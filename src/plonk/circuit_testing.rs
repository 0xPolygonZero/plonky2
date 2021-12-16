use std::cmp::max;
use std::collections::BTreeMap;

use anyhow::ensure;
use anyhow::Result;
use log::{debug, info};
use rayon::prelude::*;

use crate::field::cosets::get_unique_coset_shifts;
use crate::field::extension_field::Extendable;
use crate::field::fft::fft_root_table;
use crate::field::field_types::RichField;
use crate::fri::commitment::PolynomialBatchCommitment;
use crate::gates::gate::PrefixedGate;
use crate::gates::gate_tree::Tree;
use crate::gates::public_input::PublicInputGate;
use crate::hash::hash_types::HashOut;
use crate::hash::hashing::hash_n_to_hash;
use crate::hash::merkle_tree::{MerkleCap, MerkleTree};
use crate::iop::challenger::Challenger;
use crate::iop::generator::generate_partial_witness;
use crate::iop::target::Target;
use crate::iop::witness::{PartialWitness, Witness};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::circuit_data::{
    CircuitData, CommonCircuitData, ProverOnlyCircuitData, VerifierOnlyCircuitData,
};
use crate::plonk::config::{AlgebraicHasher, GenericConfig, Hasher};
use crate::plonk::plonk_common::PlonkPolynomials;
use crate::plonk::vanishing_poly::{
    evaluate_gate_constraints, evaluate_gate_constraints_base_batch,
};
use crate::plonk::vars::EvaluationVarsBase;
use crate::polynomial::PolynomialValues;
use crate::timed;
use crate::util::partial_products::num_partial_products;
use crate::util::{log2_ceil, log2_strict, transpose_poly_values};

pub fn check_constraints<F: Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>(
    mut builder: CircuitBuilder<F, D>,
    pw: PartialWitness<F>,
) -> Result<()> {
    builder.fill_batched_gates();

    // Hash the public inputs, and route them to a `PublicInputGate` which will enforce that
    // those hash wires match the claimed public inputs.
    let public_inputs_hash =
        builder.hash_n_to_hash::<C::InnerHasher>(builder.public_inputs.clone(), true);
    let pi_gate = builder.add_gate(PublicInputGate, vec![]);
    for (&hash_part, wire) in public_inputs_hash
        .elements
        .iter()
        .zip(PublicInputGate::wires_public_inputs_hash())
    {
        builder.connect(hash_part, Target::wire(pi_gate, wire))
    }

    info!(
        "Degree before blinding & padding: {}",
        builder.gate_instances.len()
    );
    builder.blind_and_pad();
    let degree = builder.gate_instances.len();
    info!("Degree after blinding & padding: {}", degree);
    let degree_bits = log2_strict(degree);
    let fri_params = builder.fri_params(degree_bits);
    assert!(
        fri_params.total_arities() <= degree_bits,
        "FRI total reduction arity is too large.",
    );

    let gates = builder.gates.iter().cloned().collect();
    let (gate_tree, max_filtered_constraint_degree, num_constants) = Tree::from_gates(gates);
    // `quotient_degree_factor` has to be between `max_filtered_constraint_degree-1` and `1<<rate_bits`.
    // We find the value that minimizes `num_partial_product + quotient_degree_factor`.
    let quotient_degree_factor = (max_filtered_constraint_degree - 1
        ..=1 << builder.config.rate_bits)
        .min_by_key(|&q| num_partial_products(builder.config.num_routed_wires, q).0 + q)
        .unwrap();
    debug!("Quotient degree factor set to: {}.", quotient_degree_factor);
    let prefixed_gates = PrefixedGate::from_tree(gate_tree);

    let subgroup = F::two_adic_subgroup(degree_bits);

    let constant_vecs = builder.constant_polys(&prefixed_gates, num_constants);

    let k_is = get_unique_coset_shifts(degree, builder.config.num_routed_wires);
    let (sigma_vecs, forest) = builder.sigma_vecs(&k_is, &subgroup);

    // Precompute FFT roots.
    let max_fft_points =
        1 << (degree_bits + max(builder.config.rate_bits, log2_ceil(quotient_degree_factor)));
    let fft_root_table = fft_root_table(max_fft_points);

    // Add gate generators.
    builder.add_generators(
        builder
            .gate_instances
            .iter()
            .enumerate()
            .flat_map(|(index, gate)| gate.gate_ref.0.generators(index, &gate.constants))
            .collect(),
    );

    // Index generator indices by their watched targets.
    let mut generator_indices_by_watches = BTreeMap::new();
    for (i, generator) in builder.generators.iter().enumerate() {
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

    let prover_data = ProverOnlyCircuitData::<F, C, D> {
        generators: builder.generators,
        generator_indices_by_watches,
        constants_sigmas_commitment: PolynomialBatchCommitment {
            degree_log: 0,
            polynomials: vec![],
            rate_bits: 0,
            merkle_tree: MerkleTree {
                leaves: vec![],
                layers: vec![],
                cap: MerkleCap(vec![]),
            },
            blinding: false,
        },
        sigmas: transpose_poly_values(sigma_vecs),
        subgroup,
        public_inputs: builder.public_inputs,
        marked_targets: builder.marked_targets,
        representative_map: forest.parents,
        fft_root_table: Some(fft_root_table),
    };

    // The HashSet of gates will have a non-deterministic order. When converting to a Vec, we
    // sort by ID to make the ordering deterministic.
    let mut gates = builder.gates.iter().cloned().collect::<Vec<_>>();
    gates.sort_unstable_by_key(|gate| gate.0.id());

    let num_gate_constraints = gates
        .iter()
        .map(|gate| gate.0.num_constraints())
        .max()
        .expect("No gates?");

    let num_partial_products =
        num_partial_products(builder.config.num_routed_wires, quotient_degree_factor);

    let common_data = CommonCircuitData {
        config: builder.config.clone(),
        fri_params,
        degree_bits,
        gates: prefixed_gates,
        quotient_degree_factor,
        num_gate_constraints,
        num_constants,
        num_virtual_targets: builder.virtual_target_index,
        k_is,
        num_partial_products,
        circuit_digest: <C::Hasher as Hasher<F>>::Hash::from(vec![0; 32]),
    };

    let config = &builder.config;
    let num_challenges = config.num_challenges;
    let quotient_degree = quotient_degree_factor * degree;

    let partition_witness = generate_partial_witness(pw, &prover_data, &common_data);

    let public_inputs = partition_witness.get_targets(&prover_data.public_inputs);
    let public_inputs_hash = hash_n_to_hash::<F, <C::InnerHasher as AlgebraicHasher<F>>::Permutation>(
        public_inputs.clone(),
        true,
    );

    if cfg!(debug_assertions) {
        // Display the marked targets for debugging purposes.
        for m in &prover_data.marked_targets {
            m.display(&partition_witness);
        }
    }

    let witness = partition_witness.full_witness();

    for i in 0..degree {
        let local_constants = &constant_vecs
            .iter()
            .map(|pv| pv.values[i])
            .collect::<Vec<_>>();
        let local_wires = &witness
            .wire_values
            .iter()
            .map(|column| column[i])
            .collect::<Vec<_>>();
        let vars = EvaluationVarsBase {
            local_constants,
            local_wires,
            public_inputs_hash: &public_inputs_hash,
        };
        ensure!(
            evaluate_gate_constraints_base_batch(
                &common_data.gates,
                common_data.num_gate_constraints,
                &[vars],
            )
            .into_iter()
            .all(|c| c.is_zero()),
            "{}-th gate's constraints are not satisfied.",
            i
        );
    }
    Ok(())
}
