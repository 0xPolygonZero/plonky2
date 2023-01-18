use std::fmt::Debug;

use anyhow::{ensure, Result};
use itertools::Itertools;
use plonky2::field::extension::Extendable;
use plonky2::field::types::Field;
use plonky2::fri::witness_util::set_fri_proof_target;
use plonky2::gates::exponentiation::ExponentiationGate;
use plonky2::gates::gate::GateRef;
use plonky2::gates::noop::NoopGate;
use plonky2::hash::hash_types::RichField;
use plonky2::hash::hashing::SPONGE_WIDTH;
use plonky2::iop::challenger::{Challenger, RecursiveChallenger};
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::iop::target::Target;
use plonky2::iop::witness::{PartialWitness, Witness, WitnessWrite};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::circuit_data::{CircuitConfig, CircuitData, VerifierCircuitData};
use plonky2::plonk::config::{AlgebraicHasher, GenericConfig, Hasher};
use plonky2::plonk::proof::{ProofWithPublicInputs, ProofWithPublicInputsTarget};
use plonky2::util::reducing::ReducingFactorTarget;
use plonky2::with_context;
use plonky2_util::log2_ceil;

use crate::all_stark::{Table, NUM_TABLES};
use crate::config::StarkConfig;
use crate::constraint_consumer::RecursiveConstraintConsumer;
use crate::cross_table_lookup::{verify_cross_table_lookups, CrossTableLookup, CtlCheckVarsTarget};
use crate::permutation::{
    get_grand_product_challenge_set, GrandProductChallenge, GrandProductChallengeSet,
    PermutationCheckDataTarget,
};
use crate::proof::{
    BlockMetadata, BlockMetadataTarget, PublicValues, PublicValuesTarget, StarkOpeningSetTarget,
    StarkProof, StarkProofChallengesTarget, StarkProofTarget, StarkProofWithMetadata, TrieRoots,
    TrieRootsTarget,
};
use crate::stark::Stark;
use crate::util::{h160_limbs, h256_limbs};
use crate::vanishing_poly::eval_vanishing_poly_circuit;
use crate::vars::StarkEvaluationTargets;

/// Table-wise recursive proofs of an `AllProof`.
pub struct RecursiveAllProof<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
> {
    pub recursive_proofs: [ProofWithPublicInputs<F, C, D>; NUM_TABLES],
}

pub(crate) struct PublicInputs<T: Copy + Eq + PartialEq + Debug> {
    pub(crate) trace_cap: Vec<Vec<T>>,
    pub(crate) ctl_zs_last: Vec<T>,
    pub(crate) ctl_challenges: GrandProductChallengeSet<T>,
    pub(crate) challenger_state_before: [T; SPONGE_WIDTH],
    pub(crate) challenger_state_after: [T; SPONGE_WIDTH],
}

/// Similar to the unstable `Iterator::next_chunk`. Could be replaced with that when it's stable.
fn next_chunk<T: Debug, const N: usize>(iter: &mut impl Iterator<Item = T>) -> [T; N] {
    (0..N)
        .flat_map(|_| iter.next())
        .collect_vec()
        .try_into()
        .expect("Not enough elements")
}

impl<T: Copy + Eq + PartialEq + Debug> PublicInputs<T> {
    pub(crate) fn from_vec(v: &[T], config: &StarkConfig) -> Self {
        let mut iter = v.iter().copied();
        let trace_cap = (0..config.fri_config.num_cap_elements())
            .map(|_| next_chunk::<_, 4>(&mut iter).to_vec())
            .collect();
        let ctl_challenges = GrandProductChallengeSet {
            challenges: (0..config.num_challenges)
                .map(|_| GrandProductChallenge {
                    beta: iter.next().unwrap(),
                    gamma: iter.next().unwrap(),
                })
                .collect(),
        };
        let challenger_state_before = next_chunk(&mut iter);
        let challenger_state_after = next_chunk(&mut iter);
        let ctl_zs_last: Vec<_> = iter.collect();

        Self {
            trace_cap,
            ctl_zs_last,
            ctl_challenges,
            challenger_state_before,
            challenger_state_after,
        }
    }
}

impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>
    RecursiveAllProof<F, C, D>
{
    /// Verify every recursive proof.
    pub fn verify(
        self,
        verifier_data: &[VerifierCircuitData<F, C, D>; NUM_TABLES],
        cross_table_lookups: Vec<CrossTableLookup<F>>,
        inner_config: &StarkConfig,
    ) -> Result<()>
    where
        [(); C::Hasher::HASH_SIZE]:,
    {
        let pis: [_; NUM_TABLES] = std::array::from_fn(|i| {
            PublicInputs::from_vec(&self.recursive_proofs[i].public_inputs, inner_config)
        });

        let mut challenger = Challenger::<F, C::Hasher>::new();
        for pi in &pis {
            for h in &pi.trace_cap {
                challenger.observe_elements(h);
            }
        }
        let ctl_challenges =
            get_grand_product_challenge_set(&mut challenger, inner_config.num_challenges);
        // Check that the correct CTL challenges are used in every proof.
        for pi in &pis {
            ensure!(ctl_challenges == pi.ctl_challenges);
        }

        let state = challenger.compact();
        ensure!(state == pis[0].challenger_state_before);
        // Check that the challenger state is consistent between proofs.
        for i in 1..NUM_TABLES {
            ensure!(pis[i].challenger_state_before == pis[i - 1].challenger_state_after);
        }

        // Verify the CTL checks.
        verify_cross_table_lookups::<F, C, D>(
            &cross_table_lookups,
            pis.map(|p| p.ctl_zs_last),
            inner_config,
        )?;

        // Verify the proofs.
        for (proof, verifier_data) in self.recursive_proofs.into_iter().zip(verifier_data) {
            verifier_data.verify(proof)?;
        }
        Ok(())
    }
}

/// Represents a circuit which recursively verifies a STARK proof.
pub(crate) struct StarkWrapperCircuit<F, C, const D: usize>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
{
    pub(crate) circuit: CircuitData<F, C, D>,
    pub(crate) stark_proof_target: StarkProofTarget<D>,
    pub(crate) ctl_challenges_target: GrandProductChallengeSet<Target>,
    pub(crate) init_challenger_state_target: [Target; SPONGE_WIDTH],
    pub(crate) zero_target: Target,
}

impl<F, C, const D: usize> StarkWrapperCircuit<F, C, D>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    C::Hasher: AlgebraicHasher<F>,
{
    pub(crate) fn prove(
        &self,
        proof_with_metadata: &StarkProofWithMetadata<F, C, D>,
        ctl_challenges: &GrandProductChallengeSet<F>,
    ) -> Result<ProofWithPublicInputs<F, C, D>> {
        let mut inputs = PartialWitness::new();

        set_stark_proof_target(
            &mut inputs,
            &self.stark_proof_target,
            &proof_with_metadata.proof,
            self.zero_target,
        );

        for (challenge_target, challenge) in self
            .ctl_challenges_target
            .challenges
            .iter()
            .zip(&ctl_challenges.challenges)
        {
            inputs.set_target(challenge_target.beta, challenge.beta);
            inputs.set_target(challenge_target.gamma, challenge.gamma);
        }

        inputs.set_target_arr(
            self.init_challenger_state_target,
            proof_with_metadata.init_challenger_state,
        );

        self.circuit.prove(inputs)
    }
}

/// Represents a circuit which recursively verifies a PLONK proof.
pub(crate) struct PlonkWrapperCircuit<F, C, const D: usize>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
{
    pub(crate) circuit: CircuitData<F, C, D>,
    pub(crate) proof_with_pis_target: ProofWithPublicInputsTarget<D>,
}

impl<F, C, const D: usize> PlonkWrapperCircuit<F, C, D>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    C::Hasher: AlgebraicHasher<F>,
{
    pub(crate) fn prove(
        &self,
        proof: &ProofWithPublicInputs<F, C, D>,
    ) -> Result<ProofWithPublicInputs<F, C, D>> {
        let mut inputs = PartialWitness::new();
        inputs.set_proof_with_pis_target(&self.proof_with_pis_target, proof);
        self.circuit.prove(inputs)
    }
}

/// Returns the recursive Stark circuit.
pub(crate) fn recursive_stark_circuit<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
    const D: usize,
>(
    table: Table,
    stark: &S,
    degree_bits: usize,
    cross_table_lookups: &[CrossTableLookup<F>],
    inner_config: &StarkConfig,
    circuit_config: &CircuitConfig,
    min_degree_bits: usize,
) -> StarkWrapperCircuit<F, C, D>
where
    [(); S::COLUMNS]:,
    [(); C::Hasher::HASH_SIZE]:,
    C::Hasher: AlgebraicHasher<F>,
{
    let mut builder = CircuitBuilder::<F, D>::new(circuit_config.clone());
    let zero_target = builder.zero();

    let num_permutation_zs = stark.num_permutation_batches(inner_config);
    let num_permutation_batch_size = stark.permutation_batch_size();
    let num_ctl_zs =
        CrossTableLookup::num_ctl_zs(cross_table_lookups, table, inner_config.num_challenges);
    let proof_target =
        add_virtual_stark_proof(&mut builder, stark, inner_config, degree_bits, num_ctl_zs);
    builder.register_public_inputs(
        &proof_target
            .trace_cap
            .0
            .iter()
            .flat_map(|h| h.elements)
            .collect::<Vec<_>>(),
    );

    let ctl_challenges_target = GrandProductChallengeSet {
        challenges: (0..inner_config.num_challenges)
            .map(|_| GrandProductChallenge {
                beta: builder.add_virtual_public_input(),
                gamma: builder.add_virtual_public_input(),
            })
            .collect(),
    };

    let ctl_vars = CtlCheckVarsTarget::from_proof(
        table,
        &proof_target,
        cross_table_lookups,
        &ctl_challenges_target,
        num_permutation_zs,
    );

    let init_challenger_state_target = std::array::from_fn(|_| builder.add_virtual_public_input());
    let mut challenger =
        RecursiveChallenger::<F, C::Hasher, D>::from_state(init_challenger_state_target);
    let challenges = proof_target.get_challenges::<F, C>(
        &mut builder,
        &mut challenger,
        num_permutation_zs > 0,
        num_permutation_batch_size,
        inner_config,
    );
    let challenger_state = challenger.compact(&mut builder);
    builder.register_public_inputs(&challenger_state);

    builder.register_public_inputs(&proof_target.openings.ctl_zs_last);

    verify_stark_proof_with_challenges_circuit::<F, C, _, D>(
        &mut builder,
        stark,
        &proof_target,
        &challenges,
        &ctl_vars,
        inner_config,
    );

    add_common_recursion_gates(&mut builder);

    // Pad to the minimum degree.
    while log2_ceil(builder.num_gates()) < min_degree_bits {
        builder.add_gate(NoopGate, vec![]);
    }

    let circuit = builder.build::<C>();
    StarkWrapperCircuit {
        circuit,
        stark_proof_target: proof_target,
        ctl_challenges_target,
        init_challenger_state_target,
        zero_target,
    }
}

/// Add gates that are sometimes used by recursive circuits, even if it's not actually used by this
/// particular recursive circuit. This is done for uniformity. We sometimes want all recursion
/// circuits to have the same gate set, so that we can do 1-of-n conditional recursion efficiently.
pub(crate) fn add_common_recursion_gates<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
) {
    builder.add_gate_to_gate_set(GateRef::new(ExponentiationGate::new_from_config(
        &builder.config,
    )));
}

/// Recursively verifies an inner proof.
fn verify_stark_proof_with_challenges_circuit<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
    const D: usize,
>(
    builder: &mut CircuitBuilder<F, D>,
    stark: &S,
    proof: &StarkProofTarget<D>,
    challenges: &StarkProofChallengesTarget<D>,
    ctl_vars: &[CtlCheckVarsTarget<F, D>],
    inner_config: &StarkConfig,
) where
    C::Hasher: AlgebraicHasher<F>,
    [(); S::COLUMNS]:,
{
    let zero = builder.zero();
    let one = builder.one_extension();

    let StarkOpeningSetTarget {
        local_values,
        next_values,
        permutation_ctl_zs,
        permutation_ctl_zs_next,
        ctl_zs_last,
        quotient_polys,
    } = &proof.openings;
    let vars = StarkEvaluationTargets {
        local_values: &local_values.to_vec().try_into().unwrap(),
        next_values: &next_values.to_vec().try_into().unwrap(),
    };

    let degree_bits = proof.recover_degree_bits(inner_config);
    let zeta_pow_deg = builder.exp_power_of_2_extension(challenges.stark_zeta, degree_bits);
    let z_h_zeta = builder.sub_extension(zeta_pow_deg, one);
    let (l_0, l_last) =
        eval_l_0_and_l_last_circuit(builder, degree_bits, challenges.stark_zeta, z_h_zeta);
    let last =
        builder.constant_extension(F::Extension::primitive_root_of_unity(degree_bits).inverse());
    let z_last = builder.sub_extension(challenges.stark_zeta, last);

    let mut consumer = RecursiveConstraintConsumer::<F, D>::new(
        builder.zero_extension(),
        challenges.stark_alphas.clone(),
        z_last,
        l_0,
        l_last,
    );

    let num_permutation_zs = stark.num_permutation_batches(inner_config);
    let permutation_data = stark
        .uses_permutation_args()
        .then(|| PermutationCheckDataTarget {
            local_zs: permutation_ctl_zs[..num_permutation_zs].to_vec(),
            next_zs: permutation_ctl_zs_next[..num_permutation_zs].to_vec(),
            permutation_challenge_sets: challenges.permutation_challenge_sets.clone().unwrap(),
        });

    with_context!(
        builder,
        "evaluate vanishing polynomial",
        eval_vanishing_poly_circuit::<F, C, S, D>(
            builder,
            stark,
            inner_config,
            vars,
            permutation_data,
            ctl_vars,
            &mut consumer,
        )
    );
    let vanishing_polys_zeta = consumer.accumulators();

    // Check each polynomial identity, of the form `vanishing(x) = Z_H(x) quotient(x)`, at zeta.
    let mut scale = ReducingFactorTarget::new(zeta_pow_deg);
    for (i, chunk) in quotient_polys
        .chunks(stark.quotient_degree_factor())
        .enumerate()
    {
        let recombined_quotient = scale.reduce(chunk, builder);
        let computed_vanishing_poly = builder.mul_extension(z_h_zeta, recombined_quotient);
        builder.connect_extension(vanishing_polys_zeta[i], computed_vanishing_poly);
    }

    let merkle_caps = vec![
        proof.trace_cap.clone(),
        proof.permutation_ctl_zs_cap.clone(),
        proof.quotient_polys_cap.clone(),
    ];

    let fri_instance = stark.fri_instance_target(
        builder,
        challenges.stark_zeta,
        F::primitive_root_of_unity(degree_bits),
        degree_bits,
        ctl_zs_last.len(),
        inner_config,
    );
    builder.verify_fri_proof::<C>(
        &fri_instance,
        &proof.openings.to_fri_openings(zero),
        &challenges.fri_challenges,
        &merkle_caps,
        &proof.opening_proof,
        &inner_config.fri_params(degree_bits),
    );
}

fn eval_l_0_and_l_last_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    log_n: usize,
    x: ExtensionTarget<D>,
    z_x: ExtensionTarget<D>,
) -> (ExtensionTarget<D>, ExtensionTarget<D>) {
    let n = builder.constant_extension(F::Extension::from_canonical_usize(1 << log_n));
    let g = builder.constant_extension(F::Extension::primitive_root_of_unity(log_n));
    let one = builder.one_extension();
    let l_0_deno = builder.mul_sub_extension(n, x, n);
    let l_last_deno = builder.mul_sub_extension(g, x, one);
    let l_last_deno = builder.mul_extension(n, l_last_deno);

    (
        builder.div_extension(z_x, l_0_deno),
        builder.div_extension(z_x, l_last_deno),
    )
}

#[allow(unused)] // TODO: used later?
pub(crate) fn add_virtual_public_values<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
) -> PublicValuesTarget {
    let trie_roots_before = add_virtual_trie_roots(builder);
    let trie_roots_after = add_virtual_trie_roots(builder);
    let block_metadata = add_virtual_block_metadata(builder);
    PublicValuesTarget {
        trie_roots_before,
        trie_roots_after,
        block_metadata,
    }
}

pub(crate) fn add_virtual_trie_roots<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
) -> TrieRootsTarget {
    let state_root = builder.add_virtual_target_arr();
    let transactions_root = builder.add_virtual_target_arr();
    let receipts_root = builder.add_virtual_target_arr();
    TrieRootsTarget {
        state_root,
        transactions_root,
        receipts_root,
    }
}

pub(crate) fn add_virtual_block_metadata<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
) -> BlockMetadataTarget {
    let block_beneficiary = builder.add_virtual_target_arr();
    let block_timestamp = builder.add_virtual_target();
    let block_number = builder.add_virtual_target();
    let block_difficulty = builder.add_virtual_target();
    let block_gaslimit = builder.add_virtual_target();
    let block_chain_id = builder.add_virtual_target();
    let block_base_fee = builder.add_virtual_target();
    BlockMetadataTarget {
        block_beneficiary,
        block_timestamp,
        block_number,
        block_difficulty,
        block_gaslimit,
        block_chain_id,
        block_base_fee,
    }
}

pub(crate) fn add_virtual_stark_proof<
    F: RichField + Extendable<D>,
    S: Stark<F, D>,
    const D: usize,
>(
    builder: &mut CircuitBuilder<F, D>,
    stark: &S,
    config: &StarkConfig,
    degree_bits: usize,
    num_ctl_zs: usize,
) -> StarkProofTarget<D> {
    let fri_params = config.fri_params(degree_bits);
    let cap_height = fri_params.config.cap_height;

    let num_leaves_per_oracle = vec![
        S::COLUMNS,
        stark.num_permutation_batches(config) + num_ctl_zs,
        stark.quotient_degree_factor() * config.num_challenges,
    ];

    let permutation_zs_cap = builder.add_virtual_cap(cap_height);

    StarkProofTarget {
        trace_cap: builder.add_virtual_cap(cap_height),
        permutation_ctl_zs_cap: permutation_zs_cap,
        quotient_polys_cap: builder.add_virtual_cap(cap_height),
        openings: add_virtual_stark_opening_set::<F, S, D>(builder, stark, num_ctl_zs, config),
        opening_proof: builder.add_virtual_fri_proof(&num_leaves_per_oracle, &fri_params),
    }
}

fn add_virtual_stark_opening_set<F: RichField + Extendable<D>, S: Stark<F, D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    stark: &S,
    num_ctl_zs: usize,
    config: &StarkConfig,
) -> StarkOpeningSetTarget<D> {
    let num_challenges = config.num_challenges;
    StarkOpeningSetTarget {
        local_values: builder.add_virtual_extension_targets(S::COLUMNS),
        next_values: builder.add_virtual_extension_targets(S::COLUMNS),
        permutation_ctl_zs: builder
            .add_virtual_extension_targets(stark.num_permutation_batches(config) + num_ctl_zs),
        permutation_ctl_zs_next: builder
            .add_virtual_extension_targets(stark.num_permutation_batches(config) + num_ctl_zs),
        ctl_zs_last: builder.add_virtual_targets(num_ctl_zs),
        quotient_polys: builder
            .add_virtual_extension_targets(stark.quotient_degree_factor() * num_challenges),
    }
}

pub(crate) fn set_stark_proof_target<F, C: GenericConfig<D, F = F>, W, const D: usize>(
    witness: &mut W,
    proof_target: &StarkProofTarget<D>,
    proof: &StarkProof<F, C, D>,
    zero: Target,
) where
    F: RichField + Extendable<D>,
    C::Hasher: AlgebraicHasher<F>,
    W: Witness<F>,
{
    witness.set_cap_target(&proof_target.trace_cap, &proof.trace_cap);
    witness.set_cap_target(&proof_target.quotient_polys_cap, &proof.quotient_polys_cap);

    witness.set_fri_openings(
        &proof_target.openings.to_fri_openings(zero),
        &proof.openings.to_fri_openings(),
    );

    witness.set_cap_target(
        &proof_target.permutation_ctl_zs_cap,
        &proof.permutation_ctl_zs_cap,
    );

    set_fri_proof_target(witness, &proof_target.opening_proof, &proof.opening_proof);
}

#[allow(unused)] // TODO: used later?
pub(crate) fn set_public_value_targets<F, W, const D: usize>(
    witness: &mut W,
    public_values_target: &PublicValuesTarget,
    public_values: &PublicValues,
) where
    F: RichField + Extendable<D>,
    W: Witness<F>,
{
    set_trie_roots_target(
        witness,
        &public_values_target.trie_roots_before,
        &public_values.trie_roots_before,
    );
    set_trie_roots_target(
        witness,
        &public_values_target.trie_roots_after,
        &public_values.trie_roots_after,
    );
    set_block_metadata_target(
        witness,
        &public_values_target.block_metadata,
        &public_values.block_metadata,
    );
}

pub(crate) fn set_trie_roots_target<F, W, const D: usize>(
    witness: &mut W,
    trie_roots_target: &TrieRootsTarget,
    trie_roots: &TrieRoots,
) where
    F: RichField + Extendable<D>,
    W: Witness<F>,
{
    witness.set_target_arr(
        trie_roots_target.state_root,
        h256_limbs(trie_roots.state_root),
    );
    witness.set_target_arr(
        trie_roots_target.transactions_root,
        h256_limbs(trie_roots.transactions_root),
    );
    witness.set_target_arr(
        trie_roots_target.receipts_root,
        h256_limbs(trie_roots.receipts_root),
    );
}

pub(crate) fn set_block_metadata_target<F, W, const D: usize>(
    witness: &mut W,
    block_metadata_target: &BlockMetadataTarget,
    block_metadata: &BlockMetadata,
) where
    F: RichField + Extendable<D>,
    W: Witness<F>,
{
    witness.set_target_arr(
        block_metadata_target.block_beneficiary,
        h160_limbs(block_metadata.block_beneficiary),
    );
    witness.set_target(
        block_metadata_target.block_timestamp,
        F::from_canonical_u64(block_metadata.block_timestamp.as_u64()),
    );
    witness.set_target(
        block_metadata_target.block_number,
        F::from_canonical_u64(block_metadata.block_number.as_u64()),
    );
    witness.set_target(
        block_metadata_target.block_difficulty,
        F::from_canonical_u64(block_metadata.block_difficulty.as_u64()),
    );
    witness.set_target(
        block_metadata_target.block_gaslimit,
        F::from_canonical_u64(block_metadata.block_gaslimit.as_u64()),
    );
    witness.set_target(
        block_metadata_target.block_chain_id,
        F::from_canonical_u64(block_metadata.block_chain_id.as_u64()),
    );
    witness.set_target(
        block_metadata_target.block_base_fee,
        F::from_canonical_u64(block_metadata.block_base_fee.as_u64()),
    );
}
