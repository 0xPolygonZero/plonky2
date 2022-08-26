use itertools::Itertools;
use plonky2::field::extension::Extendable;
use plonky2::field::types::Field;
use plonky2::fri::witness_util::set_fri_proof_target;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::iop::target::Target;
use plonky2::iop::witness::Witness;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::config::{AlgebraicHasher, GenericConfig};
use plonky2::util::reducing::ReducingFactorTarget;
use plonky2::with_context;

use crate::all_stark::{AllStark, Table};
use crate::config::StarkConfig;
use crate::constraint_consumer::RecursiveConstraintConsumer;
use crate::cpu::cpu_stark::CpuStark;
use crate::cross_table_lookup::{verify_cross_table_lookups_circuit, CtlCheckVarsTarget};
use crate::keccak::keccak_stark::KeccakStark;
use crate::keccak_memory::keccak_memory_stark::KeccakMemoryStark;
use crate::logic::LogicStark;
use crate::memory::memory_stark::MemoryStark;
use crate::permutation::PermutationCheckDataTarget;
use crate::proof::{
    AllProof, AllProofChallengesTarget, AllProofTarget, BlockMetadata, BlockMetadataTarget,
    PublicValues, PublicValuesTarget, StarkOpeningSetTarget, StarkProof,
    StarkProofChallengesTarget, StarkProofTarget, TrieRoots, TrieRootsTarget,
};
use crate::stark::Stark;
use crate::util::{h160_limbs, u256_limbs};
use crate::vanishing_poly::eval_vanishing_poly_circuit;
use crate::vars::StarkEvaluationTargets;

pub fn verify_proof_circuit<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(
    builder: &mut CircuitBuilder<F, D>,
    all_stark: AllStark<F, D>,
    all_proof: AllProofTarget<D>,
    inner_config: &StarkConfig,
) where
    [(); CpuStark::<F, D>::COLUMNS]:,
    [(); KeccakStark::<F, D>::COLUMNS]:,
    [(); KeccakMemoryStark::<F, D>::COLUMNS]:,
    [(); LogicStark::<F, D>::COLUMNS]:,
    [(); MemoryStark::<F, D>::COLUMNS]:,
    C::Hasher: AlgebraicHasher<F>,
{
    let AllProofChallengesTarget {
        stark_challenges,
        ctl_challenges,
    } = all_proof.get_challenges::<F, C>(builder, &all_stark, inner_config);

    let nums_permutation_zs = all_stark.nums_permutation_zs(inner_config);

    let AllStark {
        cpu_stark,
        keccak_stark,
        keccak_memory_stark,
        logic_stark,
        memory_stark,
        cross_table_lookups,
    } = all_stark;

    let ctl_vars_per_table = CtlCheckVarsTarget::from_proofs(
        &all_proof.stark_proofs,
        &cross_table_lookups,
        &ctl_challenges,
        &nums_permutation_zs,
    );

    with_context!(
        builder,
        "verify CPU proof",
        verify_stark_proof_with_challenges_circuit::<F, C, _, D>(
            builder,
            cpu_stark,
            &all_proof.stark_proofs[Table::Cpu as usize],
            &stark_challenges[Table::Cpu as usize],
            &ctl_vars_per_table[Table::Cpu as usize],
            inner_config,
        )
    );
    with_context!(
        builder,
        "verify Keccak proof",
        verify_stark_proof_with_challenges_circuit::<F, C, _, D>(
            builder,
            keccak_stark,
            &all_proof.stark_proofs[Table::Keccak as usize],
            &stark_challenges[Table::Keccak as usize],
            &ctl_vars_per_table[Table::Keccak as usize],
            inner_config,
        )
    );
    with_context!(
        builder,
        "verify Keccak memory proof",
        verify_stark_proof_with_challenges_circuit::<F, C, _, D>(
            builder,
            keccak_memory_stark,
            &all_proof.stark_proofs[Table::KeccakMemory as usize],
            &stark_challenges[Table::KeccakMemory as usize],
            &ctl_vars_per_table[Table::KeccakMemory as usize],
            inner_config,
        )
    );
    with_context!(
        builder,
        "verify logic proof",
        verify_stark_proof_with_challenges_circuit::<F, C, _, D>(
            builder,
            logic_stark,
            &all_proof.stark_proofs[Table::Logic as usize],
            &stark_challenges[Table::Logic as usize],
            &ctl_vars_per_table[Table::Logic as usize],
            inner_config,
        )
    );
    with_context!(
        builder,
        "verify memory proof",
        verify_stark_proof_with_challenges_circuit::<F, C, _, D>(
            builder,
            memory_stark,
            &all_proof.stark_proofs[Table::Memory as usize],
            &stark_challenges[Table::Memory as usize],
            &ctl_vars_per_table[Table::Memory as usize],
            inner_config,
        )
    );

    with_context!(
        builder,
        "verify cross-table lookups",
        verify_cross_table_lookups_circuit::<F, C, D>(
            builder,
            cross_table_lookups,
            &all_proof.stark_proofs,
            ctl_challenges,
            inner_config,
        )
    );
}

/// Recursively verifies an inner proof.
fn verify_stark_proof_with_challenges_circuit<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
    const D: usize,
>(
    builder: &mut CircuitBuilder<F, D>,
    stark: S,
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
    let (l_1, l_last) =
        eval_l_1_and_l_last_circuit(builder, degree_bits, challenges.stark_zeta, z_h_zeta);
    let last =
        builder.constant_extension(F::Extension::primitive_root_of_unity(degree_bits).inverse());
    let z_last = builder.sub_extension(challenges.stark_zeta, last);

    let mut consumer = RecursiveConstraintConsumer::<F, D>::new(
        builder.zero_extension(),
        challenges.stark_alphas.clone(),
        z_last,
        l_1,
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
            &stark,
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

fn eval_l_1_and_l_last_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    log_n: usize,
    x: ExtensionTarget<D>,
    z_x: ExtensionTarget<D>,
) -> (ExtensionTarget<D>, ExtensionTarget<D>) {
    let n = builder.constant_extension(F::Extension::from_canonical_usize(1 << log_n));
    let g = builder.constant_extension(F::Extension::primitive_root_of_unity(log_n));
    let one = builder.one_extension();
    let l_1_deno = builder.mul_sub_extension(n, x, n);
    let l_last_deno = builder.mul_sub_extension(g, x, one);
    let l_last_deno = builder.mul_extension(n, l_last_deno);

    (
        builder.div_extension(z_x, l_1_deno),
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
            all_stark.cpu_stark,
            config,
            degree_bits[Table::Cpu as usize],
            nums_ctl_zs[Table::Cpu as usize],
        ),
        add_virtual_stark_proof(
            builder,
            all_stark.keccak_stark,
            config,
            degree_bits[Table::Keccak as usize],
            nums_ctl_zs[Table::Keccak as usize],
        ),
        add_virtual_stark_proof(
            builder,
            all_stark.keccak_memory_stark,
            config,
            degree_bits[Table::KeccakMemory as usize],
            nums_ctl_zs[Table::KeccakMemory as usize],
        ),
        add_virtual_stark_proof(
            builder,
            all_stark.logic_stark,
            config,
            degree_bits[Table::Logic as usize],
            nums_ctl_zs[Table::Logic as usize],
        ),
        add_virtual_stark_proof(
            builder,
            all_stark.memory_stark,
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
    stark: S,
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
        openings: add_stark_opening_set::<F, S, D>(builder, stark, num_ctl_zs, config),
        opening_proof: builder.add_virtual_fri_proof(&num_leaves_per_oracle, &fri_params),
    }
}

fn add_stark_opening_set<F: RichField + Extendable<D>, S: Stark<F, D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    stark: S,
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
        u256_limbs(trie_roots.state_root),
    );
    witness.set_target_arr(
        trie_roots_target.transactions_root,
        u256_limbs(trie_roots.transactions_root),
    );
    witness.set_target_arr(
        trie_roots_target.receipts_root,
        u256_limbs(trie_roots.receipts_root),
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
