use ethereum_types::{BigEndianHash, H256, U256};
use plonky2::field::extension::Extendable;
use plonky2::fri::proof::{FriProof, FriProofTarget};
use plonky2::hash::hash_types::RichField;
use plonky2::iop::challenger::{Challenger, RecursiveChallenger};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::config::{AlgebraicHasher, GenericConfig};

use crate::config::StarkConfig;
use crate::cross_table_lookup::get_grand_product_challenge_set;
use crate::proof::*;
use crate::util::{h256_limbs, u256_limbs, u256_to_u32, u256_to_u64};
use crate::witness::errors::ProgramError;

fn observe_root<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>(
    challenger: &mut Challenger::<F, C::InnerHasher>,
    root: H256,
) {
    for limb in root.into_uint().0.into_iter() {
        challenger.observe_element(F::from_canonical_u32(limb as u32));
        challenger.observe_element(F::from_canonical_u32((limb >> 32) as u32));
    }
}

fn observe_trie_roots<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>(
    challenger: &mut Challenger::<F, C::InnerHasher>,
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
    challenger: &mut Challenger::<F, C::InnerHasher>,
    block_metadata: &BlockMetadata,
) -> Result<(), ProgramError> {
    challenger.observe_elements(
        &u256_limbs::<F>(U256::from_big_endian(&block_metadata.block_beneficiary.0))[..5],
    );
    challenger.observe_element(u256_to_u32(block_metadata.block_timestamp)?);
    challenger.observe_element(u256_to_u32(block_metadata.block_number)?);
    challenger.observe_element(u256_to_u32(block_metadata.block_difficulty)?);
    challenger.observe_elements(&h256_limbs::<F>(block_metadata.block_random));
    challenger.observe_element(u256_to_u32(block_metadata.block_gaslimit)?);
    challenger.observe_element(u256_to_u32(block_metadata.block_chain_id)?);
    let basefee = u256_to_u64(block_metadata.block_base_fee)?;
    challenger.observe_element(basefee.0);
    challenger.observe_element(basefee.1);
    challenger.observe_element(u256_to_u32(block_metadata.block_gas_used)?);
    for i in 0..8 {
        challenger.observe_elements(&u256_limbs(block_metadata.block_bloom[i]));
    }

    Ok(())
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
    challenger.observe_elements(&block_metadata.block_random);
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
    challenger: &mut Challenger::<F, C::InnerHasher>,
    extra_data: &ExtraBlockData,
) -> Result<(), ProgramError> {
    challenger.observe_elements(&h256_limbs(extra_data.checkpoint_state_trie_root));
    challenger.observe_element(u256_to_u32(extra_data.txn_number_before)?);
    challenger.observe_element(u256_to_u32(extra_data.txn_number_after)?);
    challenger.observe_element(u256_to_u32(extra_data.gas_used_before)?);
    challenger.observe_element(u256_to_u32(extra_data.gas_used_after)?);

    Ok(())
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
    challenger.observe_elements(&extra_data.checkpoint_state_trie_root);
    challenger.observe_element(extra_data.txn_number_before);
    challenger.observe_element(extra_data.txn_number_after);
    challenger.observe_element(extra_data.gas_used_before);
    challenger.observe_element(extra_data.gas_used_after);
}

fn observe_block_hashes<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(
    challenger: &mut Challenger::<F, C::InnerHasher>,
    block_hashes: &BlockHashes,
) {
    for i in 0..256 {
        challenger.observe_elements(&h256_limbs::<F>(block_hashes.prev_hashes[i]));
    }
    challenger.observe_elements(&h256_limbs::<F>(block_hashes.cur_hash));
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
    challenger: &mut Challenger::<F, C::InnerHasher>,
    public_values: &PublicValues,
) -> Result<(), ProgramError> {
    observe_trie_roots::<F, C, D>(challenger, &public_values.trie_roots_before);
    observe_trie_roots::<F, C, D>(challenger, &public_values.trie_roots_after);
    observe_block_metadata::<F, C, D>(challenger, &public_values.block_metadata)?;
    observe_block_hashes::<F, C, D>(challenger, &public_values.block_hashes);
    observe_extra_block_data::<F, C, D>(challenger, &public_values.extra_block_data)
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
        config: &StarkConfig,
    ) -> Result<AllProofChallenges<F, D>, ProgramError> {
        let mut challenger = Challenger::<F, C::InnerHasher>::new();

        for proof in &self.stark_proofs {
            challenger.observe_cap(&proof.proof.trace_cap);
        }

        observe_public_values::<F, C, D>(&mut challenger, &self.public_values)?;

        let ctl_challenges =
            get_grand_product_challenge_set(&mut challenger, config.num_challenges);

        Ok(AllProofChallenges {
            stark_challenges: core::array::from_fn(|i| {
                challenger.compact();
                self.stark_proofs[i]
                    .proof
                    .get_challenges(&mut challenger, config)
            }),
            ctl_challenges,
        })
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
        challenger: &mut Challenger::<F, C::InnerHasher>,
        config: &StarkConfig,
    ) -> StarkProofChallenges<F, D> {
        let degree_bits = self.recover_degree_bits(config);

        let StarkProof {
            auxiliary_polys_cap,
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

        challenger.observe_cap(auxiliary_polys_cap);

        let stark_alphas = challenger.get_n_challenges(num_challenges);

        challenger.observe_cap(quotient_polys_cap);
        let stark_zeta = challenger.get_extension_challenge::<D>();

        challenger.observe_openings(&openings.to_fri_openings());

        StarkProofChallenges {
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
        config: &StarkConfig,
    ) -> StarkProofChallengesTarget<D>
    where
        C::Hasher: AlgebraicHasher<F>,
    {
        let StarkProofTarget {
            auxiliary_polys_cap: auxiliary_polys,
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

        challenger.observe_cap(auxiliary_polys);

        let stark_alphas = challenger.get_n_challenges(builder, num_challenges);

        challenger.observe_cap(quotient_polys_cap);
        let stark_zeta = challenger.get_extension_challenge(builder);

        challenger.observe_openings(&openings.to_fri_openings(builder.zero()));

        StarkProofChallengesTarget {
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
