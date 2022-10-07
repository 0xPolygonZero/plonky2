use std::fmt::Debug;

use anyhow::{ensure, Result};
use itertools::Itertools;
use plonky2::field::extension::Extendable;
use plonky2::field::types::Field;
use plonky2::fri::witness_util::set_fri_proof_target;
use plonky2::hash::hash_types::{HashOut, RichField};
use plonky2::hash::hashing::SPONGE_WIDTH;
use plonky2::iop::challenger::{Challenger, RecursiveChallenger};
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::iop::target::Target;
use plonky2::iop::witness::Witness;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::circuit_data::{CircuitConfig, VerifierCircuitData, VerifierCircuitTarget};
use plonky2::plonk::config::Hasher;
use plonky2::plonk::config::{AlgebraicHasher, GenericConfig};
use plonky2::plonk::proof::{ProofWithPublicInputs, ProofWithPublicInputsTarget};
use plonky2::util::reducing::ReducingFactorTarget;
use plonky2::with_context;

use crate::all_stark::NUM_TABLES;
use crate::config::StarkConfig;
use crate::constraint_consumer::RecursiveConstraintConsumer;
use crate::cpu::cpu_stark::CpuStark;
use crate::cross_table_lookup::{
    verify_cross_table_lookups, verify_cross_table_lookups_circuit, CrossTableLookup,
    CtlCheckVarsTarget,
};
use crate::keccak::keccak_stark::KeccakStark;
use crate::keccak_memory::keccak_memory_stark::KeccakMemoryStark;
use crate::logic::LogicStark;
use crate::memory::memory_stark::MemoryStark;
use crate::permutation::{
    get_grand_product_challenge_set, get_grand_product_challenge_set_target, GrandProductChallenge,
    GrandProductChallengeSet, PermutationCheckDataTarget,
};
use crate::proof::{
    AllProof, AllProofTarget, BlockMetadata, BlockMetadataTarget, PublicValues, PublicValuesTarget,
    StarkOpeningSetTarget, StarkProof, StarkProofChallengesTarget, StarkProofTarget, TrieRoots,
    TrieRootsTarget,
};
use crate::stark::Stark;
use crate::util::h160_limbs;
use crate::vanishing_poly::eval_vanishing_poly_circuit;
use crate::vars::StarkEvaluationTargets;
use crate::{
    all_stark::{AllStark, Table},
    util::h256_limbs,
};

/// Table-wise recursive proofs of an `AllProof`.
pub struct RecursiveAllProof<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
> {
    pub recursive_proofs: [ProofWithPublicInputs<F, C, D>; NUM_TABLES],
}

pub struct RecursiveAllProofTargetWithData<const D: usize> {
    pub recursive_proofs: [ProofWithPublicInputsTarget<D>; NUM_TABLES],
    pub verifier_data: [VerifierCircuitTarget; NUM_TABLES],
}

struct PublicInputs<T: Copy + Eq + PartialEq + Debug> {
    trace_cap: Vec<Vec<T>>,
    ctl_zs_last: Vec<T>,
    ctl_challenges: GrandProductChallengeSet<T>,
    challenger_state_before: [T; SPONGE_WIDTH],
    challenger_state_after: [T; SPONGE_WIDTH],
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
    fn from_vec(v: &[T], config: &StarkConfig) -> Self {
        let mut iter = v.iter().copied();
        let trace_cap = (0..1 << config.fri_config.cap_height)
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
        let ctl_zs_last = iter.collect();

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
        let degrees_bits = std::array::from_fn(|i| verifier_data[i].common.degree_bits);
        verify_cross_table_lookups::<F, C, D>(
            cross_table_lookups,
            pis.map(|p| p.ctl_zs_last),
            degrees_bits,
            ctl_challenges,
            inner_config,
        )?;

        // Verify the proofs.
        for (proof, verifier_data) in self.recursive_proofs.into_iter().zip(verifier_data) {
            verifier_data.verify(proof)?;
        }
        Ok(())
    }

    /// Recursively verify every recursive proof.
    pub fn verify_circuit(
        builder: &mut CircuitBuilder<F, D>,
        recursive_all_proof_target: RecursiveAllProofTargetWithData<D>,
        verifier_data: &[VerifierCircuitData<F, C, D>; NUM_TABLES],
        cross_table_lookups: Vec<CrossTableLookup<F>>,
        inner_config: &StarkConfig,
    ) where
        [(); C::Hasher::HASH_SIZE]:,
        <C as GenericConfig<D>>::Hasher: AlgebraicHasher<F>,
    {
        let RecursiveAllProofTargetWithData {
            recursive_proofs,
            verifier_data: verifier_data_target,
        } = recursive_all_proof_target;
        let pis: [_; NUM_TABLES] = std::array::from_fn(|i| {
            PublicInputs::from_vec(&recursive_proofs[i].public_inputs, inner_config)
        });

        let mut challenger = RecursiveChallenger::<F, C::Hasher, D>::new(builder);
        for pi in &pis {
            for h in &pi.trace_cap {
                challenger.observe_elements(h);
            }
        }
        let ctl_challenges = get_grand_product_challenge_set_target(
            builder,
            &mut challenger,
            inner_config.num_challenges,
        );
        // Check that the correct CTL challenges are used in every proof.
        for pi in &pis {
            for i in 0..inner_config.num_challenges {
                builder.connect(
                    ctl_challenges.challenges[i].beta,
                    pi.ctl_challenges.challenges[i].beta,
                );
                builder.connect(
                    ctl_challenges.challenges[i].gamma,
                    pi.ctl_challenges.challenges[i].gamma,
                );
            }
        }

        let state = challenger.compact(builder);
        for k in 0..SPONGE_WIDTH {
            builder.connect(state[k], pis[0].challenger_state_before[k]);
        }
        // Check that the challenger state is consistent between proofs.
        for i in 1..NUM_TABLES {
            for k in 0..SPONGE_WIDTH {
                builder.connect(
                    pis[i].challenger_state_before[k],
                    pis[i - 1].challenger_state_after[k],
                );
            }
        }

        // Verify the CTL checks.
        let degrees_bits = std::array::from_fn(|i| verifier_data[i].common.degree_bits);
        verify_cross_table_lookups_circuit::<F, C, D>(
            builder,
            cross_table_lookups,
            pis.map(|p| p.ctl_zs_last),
            degrees_bits,
            ctl_challenges,
            inner_config,
        );
        for (i, (recursive_proof, verifier_data_target)) in recursive_proofs
            .into_iter()
            .zip(verifier_data_target)
            .enumerate()
        {
            builder.verify_proof(
                recursive_proof,
                &verifier_data_target,
                &verifier_data[i].common,
            );
        }
    }
}

/// Returns the verifier data for the recursive Stark circuit.
fn verifier_data_recursive_stark_proof<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
    const D: usize,
>(
    table: Table,
    stark: S,
    degree_bits: usize,
    cross_table_lookups: &[CrossTableLookup<F>],
    inner_config: &StarkConfig,
    circuit_config: &CircuitConfig,
) -> VerifierCircuitData<F, C, D>
where
    [(); S::COLUMNS]:,
    [(); C::Hasher::HASH_SIZE]:,
    C::Hasher: AlgebraicHasher<F>,
{
    let mut builder = CircuitBuilder::<F, D>::new(circuit_config.clone());

    let num_permutation_zs = stark.num_permutation_batches(inner_config);
    let num_permutation_batch_size = stark.permutation_batch_size();
    let num_ctl_zs =
        CrossTableLookup::num_ctl_zs(cross_table_lookups, table, inner_config.num_challenges);
    let proof_target =
        add_virtual_stark_proof(&mut builder, &stark, inner_config, degree_bits, num_ctl_zs);
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

    let challenger_state = std::array::from_fn(|_| builder.add_virtual_public_input());
    let mut challenger = RecursiveChallenger::<F, C::Hasher, D>::from_state(challenger_state);
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
        &stark,
        &proof_target,
        &challenges,
        &ctl_vars,
        inner_config,
    );

    builder.build_verifier::<C>()
}

/// Returns the recursive Stark circuit verifier data for every Stark in `AllStark`.
pub fn all_verifier_data_recursive_stark_proof<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(
    all_stark: &AllStark<F, D>,
    degree_bits: [usize; NUM_TABLES],
    inner_config: &StarkConfig,
    circuit_config: &CircuitConfig,
) -> [VerifierCircuitData<F, C, D>; NUM_TABLES]
where
    [(); CpuStark::<F, D>::COLUMNS]:,
    [(); KeccakStark::<F, D>::COLUMNS]:,
    [(); KeccakMemoryStark::<F, D>::COLUMNS]:,
    [(); LogicStark::<F, D>::COLUMNS]:,
    [(); MemoryStark::<F, D>::COLUMNS]:,
    [(); C::Hasher::HASH_SIZE]:,
    C::Hasher: AlgebraicHasher<F>,
{
    [
        verifier_data_recursive_stark_proof(
            Table::Cpu,
            all_stark.cpu_stark,
            degree_bits[Table::Cpu as usize],
            &all_stark.cross_table_lookups,
            inner_config,
            circuit_config,
        ),
        verifier_data_recursive_stark_proof(
            Table::Keccak,
            all_stark.keccak_stark,
            degree_bits[Table::Keccak as usize],
            &all_stark.cross_table_lookups,
            inner_config,
            circuit_config,
        ),
        verifier_data_recursive_stark_proof(
            Table::KeccakMemory,
            all_stark.keccak_memory_stark,
            degree_bits[Table::KeccakMemory as usize],
            &all_stark.cross_table_lookups,
            inner_config,
            circuit_config,
        ),
        verifier_data_recursive_stark_proof(
            Table::Logic,
            all_stark.logic_stark,
            degree_bits[Table::Logic as usize],
            &all_stark.cross_table_lookups,
            inner_config,
            circuit_config,
        ),
        verifier_data_recursive_stark_proof(
            Table::Memory,
            all_stark.memory_stark,
            degree_bits[Table::Memory as usize],
            &all_stark.cross_table_lookups,
            inner_config,
            circuit_config,
        ),
    ]
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

pub fn add_virtual_all_proof<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    all_stark: &AllStark<F, D>,
    config: &StarkConfig,
    degree_bits: &[usize],
    nums_ctl_zs: &[usize],
) -> AllProofTarget<D> {
    let stark_proofs = [
        add_virtual_stark_proof(
            builder,
            &all_stark.cpu_stark,
            config,
            degree_bits[Table::Cpu as usize],
            nums_ctl_zs[Table::Cpu as usize],
        ),
        add_virtual_stark_proof(
            builder,
            &all_stark.keccak_stark,
            config,
            degree_bits[Table::Keccak as usize],
            nums_ctl_zs[Table::Keccak as usize],
        ),
        add_virtual_stark_proof(
            builder,
            &all_stark.keccak_memory_stark,
            config,
            degree_bits[Table::KeccakMemory as usize],
            nums_ctl_zs[Table::KeccakMemory as usize],
        ),
        add_virtual_stark_proof(
            builder,
            &all_stark.logic_stark,
            config,
            degree_bits[Table::Logic as usize],
            nums_ctl_zs[Table::Logic as usize],
        ),
        add_virtual_stark_proof(
            builder,
            &all_stark.memory_stark,
            config,
            degree_bits[Table::Memory as usize],
            nums_ctl_zs[Table::Memory as usize],
        ),
    ];

    let public_values = add_virtual_public_values(builder);
    AllProofTarget {
        stark_proofs,
        public_values,
    }
}

/// Returns `RecursiveAllProofTargetWithData` where the proofs targets are virtual and the
/// verifier data targets are constants.
pub fn add_virtual_recursive_all_proof<F: RichField + Extendable<D>, H, C, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    verifier_data: &[VerifierCircuitData<F, C, D>; NUM_TABLES],
) -> RecursiveAllProofTargetWithData<D>
where
    H: Hasher<F, Hash = HashOut<F>>,
    C: GenericConfig<D, F = F, Hasher = H>,
{
    let recursive_proofs = std::array::from_fn(|i| {
        let verifier_data = &verifier_data[i];
        builder.add_virtual_proof_with_pis(&verifier_data.common)
    });
    let verifier_data = std::array::from_fn(|i| {
        let verifier_data = &verifier_data[i];
        VerifierCircuitTarget {
            constants_sigmas_cap: builder
                .constant_merkle_cap(&verifier_data.verifier_only.constants_sigmas_cap),
        }
    });
    RecursiveAllProofTargetWithData {
        recursive_proofs,
        verifier_data,
    }
}

pub fn add_virtual_public_values<F: RichField + Extendable<D>, const D: usize>(
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

pub fn add_virtual_trie_roots<F: RichField + Extendable<D>, const D: usize>(
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

pub fn add_virtual_block_metadata<F: RichField + Extendable<D>, const D: usize>(
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

pub fn add_virtual_stark_proof<F: RichField + Extendable<D>, S: Stark<F, D>, const D: usize>(
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

pub fn set_recursive_all_proof_target<F, C: GenericConfig<D, F = F>, W, const D: usize>(
    witness: &mut W,
    recursive_all_proof_target: &RecursiveAllProofTargetWithData<D>,
    all_proof: &RecursiveAllProof<F, C, D>,
) where
    F: RichField + Extendable<D>,
    C::Hasher: AlgebraicHasher<F>,
    W: Witness<F>,
{
    for i in 0..NUM_TABLES {
        witness.set_proof_with_pis_target(
            &recursive_all_proof_target.recursive_proofs[i],
            &all_proof.recursive_proofs[i],
        );
    }
}
pub fn set_all_proof_target<F, C: GenericConfig<D, F = F>, W, const D: usize>(
    witness: &mut W,
    all_proof_target: &AllProofTarget<D>,
    all_proof: &AllProof<F, C, D>,
    zero: Target,
) where
    F: RichField + Extendable<D>,
    C::Hasher: AlgebraicHasher<F>,
    W: Witness<F>,
{
    for (pt, p) in all_proof_target
        .stark_proofs
        .iter()
        .zip_eq(&all_proof.stark_proofs)
    {
        set_stark_proof_target(witness, pt, p, zero);
    }
    set_public_value_targets(
        witness,
        &all_proof_target.public_values,
        &all_proof.public_values,
    )
}

pub fn set_stark_proof_target<F, C: GenericConfig<D, F = F>, W, const D: usize>(
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

pub fn set_public_value_targets<F, W, const D: usize>(
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

pub fn set_trie_roots_target<F, W, const D: usize>(
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

pub fn set_block_metadata_target<F, W, const D: usize>(
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

#[cfg(test)]
pub(crate) mod tests {
    use anyhow::Result;
    use plonky2::field::extension::Extendable;
    use plonky2::hash::hash_types::RichField;
    use plonky2::hash::hashing::SPONGE_WIDTH;
    use plonky2::iop::challenger::RecursiveChallenger;
    use plonky2::iop::witness::{PartialWitness, Witness};
    use plonky2::plonk::circuit_builder::CircuitBuilder;
    use plonky2::plonk::circuit_data::{CircuitConfig, VerifierCircuitData};
    use plonky2::plonk::config::Hasher;
    use plonky2::plonk::config::{AlgebraicHasher, GenericConfig};
    use plonky2::plonk::proof::ProofWithPublicInputs;

    use crate::all_stark::{AllStark, Table};
    use crate::config::StarkConfig;
    use crate::cpu::cpu_stark::CpuStark;
    use crate::cross_table_lookup::{CrossTableLookup, CtlCheckVarsTarget};
    use crate::keccak::keccak_stark::KeccakStark;
    use crate::keccak_memory::keccak_memory_stark::KeccakMemoryStark;
    use crate::logic::LogicStark;
    use crate::memory::memory_stark::MemoryStark;
    use crate::permutation::{GrandProductChallenge, GrandProductChallengeSet};
    use crate::proof::{AllChallengerState, AllProof, StarkProof};
    use crate::recursive_verifier::{
        add_virtual_stark_proof, set_stark_proof_target,
        verify_stark_proof_with_challenges_circuit, RecursiveAllProof,
    };
    use crate::stark::Stark;

    /// Recursively verify a Stark proof.
    /// Outputs the recursive proof and the associated verifier data.
    fn recursively_verify_stark_proof<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        S: Stark<F, D>,
        const D: usize,
    >(
        table: Table,
        stark: S,
        proof: &StarkProof<F, C, D>,
        cross_table_lookups: &[CrossTableLookup<F>],
        ctl_challenges: &GrandProductChallengeSet<F>,
        challenger_state_before_vals: [F; SPONGE_WIDTH],
        inner_config: &StarkConfig,
        circuit_config: &CircuitConfig,
    ) -> Result<(ProofWithPublicInputs<F, C, D>, VerifierCircuitData<F, C, D>)>
    where
        [(); S::COLUMNS]:,
        [(); C::Hasher::HASH_SIZE]:,
        C::Hasher: AlgebraicHasher<F>,
    {
        let mut builder = CircuitBuilder::<F, D>::new(circuit_config.clone());
        let mut pw = PartialWitness::new();

        let num_permutation_zs = stark.num_permutation_batches(inner_config);
        let num_permutation_batch_size = stark.permutation_batch_size();
        let proof_target = add_virtual_stark_proof(
            &mut builder,
            &stark,
            inner_config,
            proof.recover_degree_bits(inner_config),
            proof.num_ctl_zs(),
        );
        set_stark_proof_target(&mut pw, &proof_target, proof, builder.zero());
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
        for i in 0..inner_config.num_challenges {
            pw.set_target(
                ctl_challenges_target.challenges[i].beta,
                ctl_challenges.challenges[i].beta,
            );
            pw.set_target(
                ctl_challenges_target.challenges[i].gamma,
                ctl_challenges.challenges[i].gamma,
            );
        }

        let ctl_vars = CtlCheckVarsTarget::from_proof(
            table,
            &proof_target,
            cross_table_lookups,
            &ctl_challenges_target,
            num_permutation_zs,
        );

        let challenger_state_before = std::array::from_fn(|_| builder.add_virtual_public_input());
        pw.set_target_arr(challenger_state_before, challenger_state_before_vals);
        let mut challenger =
            RecursiveChallenger::<F, C::Hasher, D>::from_state(challenger_state_before);
        let challenges = proof_target.get_challenges::<F, C>(
            &mut builder,
            &mut challenger,
            num_permutation_zs > 0,
            num_permutation_batch_size,
            inner_config,
        );
        let challenger_state_after = challenger.compact(&mut builder);
        builder.register_public_inputs(&challenger_state_after);

        builder.register_public_inputs(&proof_target.openings.ctl_zs_last);

        verify_stark_proof_with_challenges_circuit::<F, C, _, D>(
            &mut builder,
            &stark,
            &proof_target,
            &challenges,
            &ctl_vars,
            inner_config,
        );

        let data = builder.build::<C>();
        Ok((data.prove(pw)?, data.verifier_data()))
    }

    /// Recursively verify every Stark proof in an `AllProof`.
    pub fn recursively_verify_all_proof<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
    >(
        all_stark: &AllStark<F, D>,
        all_proof: &AllProof<F, C, D>,
        inner_config: &StarkConfig,
        circuit_config: &CircuitConfig,
    ) -> Result<RecursiveAllProof<F, C, D>>
    where
        [(); CpuStark::<F, D>::COLUMNS]:,
        [(); KeccakStark::<F, D>::COLUMNS]:,
        [(); KeccakMemoryStark::<F, D>::COLUMNS]:,
        [(); LogicStark::<F, D>::COLUMNS]:,
        [(); MemoryStark::<F, D>::COLUMNS]:,
        [(); C::Hasher::HASH_SIZE]:,
        C::Hasher: AlgebraicHasher<F>,
    {
        let AllChallengerState {
            states,
            ctl_challenges,
        } = all_proof.get_challenger_states(all_stark, inner_config);
        Ok(RecursiveAllProof {
            recursive_proofs: [
                recursively_verify_stark_proof(
                    Table::Cpu,
                    all_stark.cpu_stark,
                    &all_proof.stark_proofs[Table::Cpu as usize],
                    &all_stark.cross_table_lookups,
                    &ctl_challenges,
                    states[0],
                    inner_config,
                    circuit_config,
                )?
                .0,
                recursively_verify_stark_proof(
                    Table::Keccak,
                    all_stark.keccak_stark,
                    &all_proof.stark_proofs[Table::Keccak as usize],
                    &all_stark.cross_table_lookups,
                    &ctl_challenges,
                    states[1],
                    inner_config,
                    circuit_config,
                )?
                .0,
                recursively_verify_stark_proof(
                    Table::KeccakMemory,
                    all_stark.keccak_memory_stark,
                    &all_proof.stark_proofs[Table::KeccakMemory as usize],
                    &all_stark.cross_table_lookups,
                    &ctl_challenges,
                    states[2],
                    inner_config,
                    circuit_config,
                )?
                .0,
                recursively_verify_stark_proof(
                    Table::Logic,
                    all_stark.logic_stark,
                    &all_proof.stark_proofs[Table::Logic as usize],
                    &all_stark.cross_table_lookups,
                    &ctl_challenges,
                    states[3],
                    inner_config,
                    circuit_config,
                )?
                .0,
                recursively_verify_stark_proof(
                    Table::Memory,
                    all_stark.memory_stark,
                    &all_proof.stark_proofs[Table::Memory as usize],
                    &all_stark.cross_table_lookups,
                    &ctl_challenges,
                    states[4],
                    inner_config,
                    circuit_config,
                )?
                .0,
            ],
        })
    }
}
