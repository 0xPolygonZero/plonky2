use ethereum_types::{BigEndianHash, H256, U256};
use plonky2::field::extension::Extendable;
use plonky2::fri::proof::{FriProof, FriProofTarget};
use plonky2::hash::hash_types::RichField;
use plonky2::iop::challenger::{Challenger, RecursiveChallenger};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::config::{AlgebraicHasher, GenericConfig};

use crate::all_stark::{AllStark, NUM_TABLES};
use crate::config::StarkConfig;
use crate::permutation::{
    get_grand_product_challenge_set, get_n_grand_product_challenge_sets,
    get_n_grand_product_challenge_sets_target,
};
use crate::proof::*;
use crate::util::{h256_limbs, u256_limbs};

fn observe_root<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>(
    challenger: &mut Challenger<F, C::Hasher>,
    root: H256,
) {
    for limb in root.into_uint().0.into_iter() {
        challenger.observe_element(F::from_canonical_u32(limb as u32));
        challenger.observe_element(F::from_canonical_u32((limb >> 32) as u32));
    }
}

fn observe_trie_roots<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>(
    challenger: &mut Challenger<F, C::Hasher>,
    trie_roots: &TrieRoots,
) {
    observe_root::<F, C, D>(challenger, trie_roots.state_root);
    observe_root::<F, C, D>(challenger, trie_roots.transactions_root);
    observe_root::<F, C, D>(challenger, trie_roots.receipts_root);
}

fn observe_trie_roots_target<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(
    challenger: &mut RecursiveChallenger<F, C::Hasher, D>,
    trie_roots: &TrieRootsTarget,
) where
    C::Hasher: AlgebraicHasher<F>,
{
    challenger.observe_elements(&trie_roots.state_root);
    challenger.observe_elements(&trie_roots.transactions_root);
    challenger.observe_elements(&trie_roots.receipts_root);
}

fn observe_block_metadata<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(
    challenger: &mut Challenger<F, C::Hasher>,
    block_metadata: &BlockMetadata,
) {
    challenger.observe_elements(
        &u256_limbs::<F>(U256::from_big_endian(&block_metadata.block_beneficiary.0))[..5],
    );
    challenger.observe_element(F::from_canonical_u32(
        block_metadata.block_timestamp.as_u32(),
    ));
    challenger.observe_element(F::from_canonical_u32(block_metadata.block_number.as_u32()));
    challenger.observe_element(F::from_canonical_u32(
        block_metadata.block_difficulty.as_u32(),
    ));
    challenger.observe_element(F::from_canonical_u32(
        block_metadata.block_gaslimit.as_u32(),
    ));
    challenger.observe_element(F::from_canonical_u32(
        block_metadata.block_chain_id.as_u32(),
    ));
    challenger.observe_element(F::from_canonical_u32(
        block_metadata.block_base_fee.as_u64() as u32,
    ));
    challenger.observe_element(F::from_canonical_u32(
        (block_metadata.block_base_fee.as_u64() >> 32) as u32,
    ));
    challenger.observe_element(F::from_canonical_u32(
        block_metadata.block_gas_used.as_u32(),
    ));
    for i in 0..8 {
        challenger.observe_elements(&u256_limbs(block_metadata.block_bloom[i]));
    }
}

fn observe_block_metadata_target<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(
    challenger: &mut RecursiveChallenger<F, C::Hasher, D>,
    block_metadata: &BlockMetadataTarget,
) where
    C::Hasher: AlgebraicHasher<F>,
{
    challenger.observe_elements(&block_metadata.block_beneficiary);
    challenger.observe_element(block_metadata.block_timestamp);
    challenger.observe_element(block_metadata.block_number);
    challenger.observe_element(block_metadata.block_difficulty);
    challenger.observe_element(block_metadata.block_gaslimit);
    challenger.observe_element(block_metadata.block_chain_id);
    challenger.observe_elements(&block_metadata.block_base_fee);
    challenger.observe_element(block_metadata.block_gas_used);
    challenger.observe_elements(&block_metadata.block_bloom);
}

fn observe_extra_block_data<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(
    challenger: &mut Challenger<F, C::Hasher>,
    extra_data: &ExtraBlockData,
) {
    challenger.observe_element(F::from_canonical_u32(extra_data.txn_number_before.as_u32()));
    challenger.observe_element(F::from_canonical_u32(extra_data.txn_number_after.as_u32()));
    challenger.observe_element(F::from_canonical_u32(extra_data.gas_used_before.as_u32()));
    challenger.observe_element(F::from_canonical_u32(extra_data.gas_used_after.as_u32()));
    for i in 0..8 {
        challenger.observe_elements(&u256_limbs(extra_data.block_bloom_before[i]));
    }
    for i in 0..8 {
        challenger.observe_elements(&u256_limbs(extra_data.block_bloom_after[i]));
    }
}

fn observe_extra_block_data_target<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(
    challenger: &mut RecursiveChallenger<F, C::Hasher, D>,
    extra_data: &ExtraBlockDataTarget,
) where
    C::Hasher: AlgebraicHasher<F>,
{
    challenger.observe_element(extra_data.txn_number_before);
    challenger.observe_element(extra_data.txn_number_after);
    challenger.observe_element(extra_data.gas_used_before);
    challenger.observe_element(extra_data.gas_used_after);
    challenger.observe_elements(&extra_data.block_bloom_before);
    challenger.observe_elements(&extra_data.block_bloom_after);
}

fn observe_block_hashes<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(
    challenger: &mut Challenger<F, C::Hasher>,
    block_hashes: &BlockHashes,
) {
    for i in 0..256 {
        challenger.observe_elements(&h256_limbs::<F>(block_hashes.prev_hashes[i])[0..8]);
    }
    challenger.observe_elements(&h256_limbs::<F>(block_hashes.cur_hash)[0..8])
}

fn observe_block_hashes_target<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(
    challenger: &mut RecursiveChallenger<F, C::Hasher, D>,
    block_hashes: &BlockHashesTarget,
) where
    C::Hasher: AlgebraicHasher<F>,
{
    challenger.observe_elements(&block_hashes.prev_hashes);
    challenger.observe_elements(&block_hashes.cur_hash);
}

pub(crate) fn observe_public_values<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(
    challenger: &mut Challenger<F, C::Hasher>,
    public_values: &PublicValues,
) {
    observe_trie_roots::<F, C, D>(challenger, &public_values.trie_roots_before);
    observe_trie_roots::<F, C, D>(challenger, &public_values.trie_roots_after);
    observe_block_metadata::<F, C, D>(challenger, &public_values.block_metadata);
    observe_block_hashes::<F, C, D>(challenger, &public_values.block_hashes);
    observe_extra_block_data::<F, C, D>(challenger, &public_values.extra_block_data);
}

pub(crate) fn observe_public_values_target<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(
    challenger: &mut RecursiveChallenger<F, C::Hasher, D>,
    public_values: &PublicValuesTarget,
) where
    C::Hasher: AlgebraicHasher<F>,
{
    observe_trie_roots_target::<F, C, D>(challenger, &public_values.trie_roots_before);
    observe_trie_roots_target::<F, C, D>(challenger, &public_values.trie_roots_after);
    observe_block_metadata_target::<F, C, D>(challenger, &public_values.block_metadata);
    observe_block_hashes_target::<F, C, D>(challenger, &public_values.block_hashes);
    observe_extra_block_data_target::<F, C, D>(challenger, &public_values.extra_block_data);
}

impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize> AllProof<F, C, D> {
    /// Computes all Fiat-Shamir challenges used in the STARK proof.
    pub(crate) fn get_challenges(
        &self,
        all_stark: &AllStark<F, D>,
        config: &StarkConfig,
    ) -> AllProofChallenges<F, D> {
        let mut challenger = Challenger::<F, C::Hasher>::new();

        for proof in &self.stark_proofs {
            challenger.observe_cap(&proof.proof.trace_cap);
        }

        observe_public_values::<F, C, D>(&mut challenger, &self.public_values);

        let ctl_challenges =
            get_grand_product_challenge_set(&mut challenger, config.num_challenges);

        let num_permutation_zs = all_stark.nums_permutation_zs(config);
        let num_permutation_batch_sizes = all_stark.permutation_batch_sizes();

        AllProofChallenges {
            stark_challenges: core::array::from_fn(|i| {
                challenger.compact();
                self.stark_proofs[i].proof.get_challenges(
                    &mut challenger,
                    num_permutation_zs[i] > 0,
                    num_permutation_batch_sizes[i],
                    config,
                )
            }),
            ctl_challenges,
        }
    }

    #[allow(unused)] // TODO: should be used soon
    pub(crate) fn get_challenger_states(
        &self,
        all_stark: &AllStark<F, D>,
        config: &StarkConfig,
    ) -> AllChallengerState<F, C::Hasher, D> {
        let mut challenger = Challenger::<F, C::Hasher>::new();

        for proof in &self.stark_proofs {
            challenger.observe_cap(&proof.proof.trace_cap);
        }

        observe_public_values::<F, C, D>(&mut challenger, &self.public_values);

        let ctl_challenges =
            get_grand_product_challenge_set(&mut challenger, config.num_challenges);

        let num_permutation_zs = all_stark.nums_permutation_zs(config);
        let num_permutation_batch_sizes = all_stark.permutation_batch_sizes();

        let mut challenger_states = vec![challenger.compact()];
        for i in 0..NUM_TABLES {
            self.stark_proofs[i].proof.get_challenges(
                &mut challenger,
                num_permutation_zs[i] > 0,
                num_permutation_batch_sizes[i],
                config,
            );
            challenger_states.push(challenger.compact());
        }

        AllChallengerState {
            states: challenger_states.try_into().unwrap(),
            ctl_challenges,
        }
    }
}

impl<F, C, const D: usize> StarkProof<F, C, D>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
{
    /// Computes all Fiat-Shamir challenges used in the STARK proof.
    pub(crate) fn get_challenges(
        &self,
        challenger: &mut Challenger<F, C::Hasher>,
        stark_use_permutation: bool,
        stark_permutation_batch_size: usize,
        config: &StarkConfig,
    ) -> StarkProofChallenges<F, D> {
        let degree_bits = self.recover_degree_bits(config);

        let StarkProof {
            permutation_ctl_zs_cap,
            quotient_polys_cap,
            openings,
            opening_proof:
                FriProof {
                    commit_phase_merkle_caps,
                    final_poly,
                    pow_witness,
                    ..
                },
            ..
        } = &self;

        let num_challenges = config.num_challenges;

        let permutation_challenge_sets = stark_use_permutation.then(|| {
            get_n_grand_product_challenge_sets(
                challenger,
                num_challenges,
                stark_permutation_batch_size,
            )
        });

        challenger.observe_cap(permutation_ctl_zs_cap);

        let stark_alphas = challenger.get_n_challenges(num_challenges);

        challenger.observe_cap(quotient_polys_cap);
        let stark_zeta = challenger.get_extension_challenge::<D>();

        challenger.observe_openings(&openings.to_fri_openings());

        StarkProofChallenges {
            permutation_challenge_sets,
            stark_alphas,
            stark_zeta,
            fri_challenges: challenger.fri_challenges::<C, D>(
                commit_phase_merkle_caps,
                final_poly,
                *pow_witness,
                degree_bits,
                &config.fri_config,
            ),
        }
    }
}

impl<const D: usize> StarkProofTarget<D> {
    pub(crate) fn get_challenges<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>>(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        challenger: &mut RecursiveChallenger<F, C::Hasher, D>,
        stark_use_permutation: bool,
        stark_permutation_batch_size: usize,
        config: &StarkConfig,
    ) -> StarkProofChallengesTarget<D>
    where
        C::Hasher: AlgebraicHasher<F>,
    {
        let StarkProofTarget {
            permutation_ctl_zs_cap,
            quotient_polys_cap,
            openings,
            opening_proof:
                FriProofTarget {
                    commit_phase_merkle_caps,
                    final_poly,
                    pow_witness,
                    ..
                },
            ..
        } = &self;

        let num_challenges = config.num_challenges;

        let permutation_challenge_sets = stark_use_permutation.then(|| {
            get_n_grand_product_challenge_sets_target(
                builder,
                challenger,
                num_challenges,
                stark_permutation_batch_size,
            )
        });

        challenger.observe_cap(permutation_ctl_zs_cap);

        let stark_alphas = challenger.get_n_challenges(builder, num_challenges);

        challenger.observe_cap(quotient_polys_cap);
        let stark_zeta = challenger.get_extension_challenge(builder);

        challenger.observe_openings(&openings.to_fri_openings(builder.zero()));

        StarkProofChallengesTarget {
            permutation_challenge_sets,
            stark_alphas,
            stark_zeta,
            fri_challenges: challenger.fri_challenges(
                builder,
                commit_phase_merkle_caps,
                final_poly,
                *pow_witness,
                &config.fri_config,
            ),
        }
    }
}
