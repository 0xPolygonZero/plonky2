use core::any::type_name;

use anyhow::{ensure, Result};
use ethereum_types::{BigEndianHash, U256};
use itertools::Itertools;
use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::types::Field;
use plonky2::fri::verifier::verify_fri_proof;
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::config::GenericConfig;
use plonky2::plonk::plonk_common::reduce_with_powers;
use starky::cross_table_lookup::get_ctl_vars_from_proofs;
use starky::proof::MultiProof;
use starky::verifier::verify_stark_proof_with_challenges;

use crate::all_stark::{AllStark, Table, NUM_TABLES};
use crate::config::StarkConfig;
use crate::constraint_consumer::ConstraintConsumer;
use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::constants::global_metadata::GlobalMetadata;
use crate::cross_table_lookup::verify_cross_table_lookups;
use crate::evaluation_frame::StarkEvaluationFrame;
use crate::lookup::{GrandProductChallenge, LookupCheckVars};
use crate::memory::segments::Segment;
use crate::memory::VALUE_LIMBS;
use crate::proof::{
    AllProof, AllProofChallenges, PublicValues, StarkOpeningSet, StarkProof, StarkProofChallenges,
};
use crate::stark::Stark;
use crate::util::h2u;

pub fn verify_proof<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>(
    all_stark: &AllStark<F, D>,
    all_proof: AllProof<F, C, D>,
    config: &StarkConfig,
) -> Result<()>
where
{
    let AllProofChallenges {
        stark_challenges,
        ctl_challenges,
    } = all_proof
        .get_challenges(config)
        .map_err(|_| anyhow::Error::msg("Invalid sampling of proof challenges."))?;

    let num_lookup_columns = all_stark.num_lookups_helper_columns(config);

    let AllStark {
        arithmetic_stark,
        byte_packing_stark,
        cpu_stark,
        keccak_stark,
        keccak_sponge_stark,
        logic_stark,
        memory_stark,
        cross_table_lookups,
    } = all_stark;

    let ctl_vars_per_table = get_ctl_vars_from_proofs(
        &all_proof.multi_proof,
        cross_table_lookups,
        &ctl_challenges,
        &num_lookup_columns,
        all_stark.arithmetic_stark.constraint_degree(),
    );

    let stark_proofs = &all_proof.multi_proof.stark_proofs;

    verify_stark_proof_with_challenges(
        arithmetic_stark,
        &stark_proofs[Table::Arithmetic as usize].proof,
        &stark_challenges[Table::Arithmetic as usize],
        Some(&ctl_vars_per_table[Table::Arithmetic as usize]),
        Some(&ctl_challenges),
        &[],
        config,
    )?;

    verify_stark_proof_with_challenges(
        byte_packing_stark,
        &stark_proofs[Table::BytePacking as usize].proof,
        &stark_challenges[Table::BytePacking as usize],
        Some(&ctl_vars_per_table[Table::BytePacking as usize]),
        Some(&ctl_challenges),
        &[],
        config,
    )?;
    verify_stark_proof_with_challenges(
        cpu_stark,
        &stark_proofs[Table::Cpu as usize].proof,
        &stark_challenges[Table::Cpu as usize],
        Some(&ctl_vars_per_table[Table::Cpu as usize]),
        Some(&ctl_challenges),
        &[],
        config,
    )?;
    verify_stark_proof_with_challenges(
        keccak_stark,
        &stark_proofs[Table::Keccak as usize].proof,
        &stark_challenges[Table::Keccak as usize],
        Some(&ctl_vars_per_table[Table::Keccak as usize]),
        Some(&ctl_challenges),
        &[],
        config,
    )?;
    verify_stark_proof_with_challenges(
        keccak_sponge_stark,
        &stark_proofs[Table::KeccakSponge as usize].proof,
        &stark_challenges[Table::KeccakSponge as usize],
        Some(&ctl_vars_per_table[Table::KeccakSponge as usize]),
        Some(&ctl_challenges),
        &[],
        config,
    )?;
    verify_stark_proof_with_challenges(
        logic_stark,
        &stark_proofs[Table::Logic as usize].proof,
        &stark_challenges[Table::Logic as usize],
        Some(&ctl_vars_per_table[Table::Logic as usize]),
        Some(&ctl_challenges),
        &[],
        config,
    )?;
    verify_stark_proof_with_challenges(
        memory_stark,
        &stark_proofs[Table::Memory as usize].proof,
        &stark_challenges[Table::Memory as usize],
        Some(&ctl_vars_per_table[Table::Memory as usize]),
        Some(&ctl_challenges),
        &[],
        config,
    )?;

    let public_values = all_proof.public_values;

    // Extra sums to add to the looked last value.
    // Only necessary for the Memory values.
    let mut extra_looking_sums = vec![vec![F::ZERO; config.num_challenges]; NUM_TABLES];

    // Memory
    extra_looking_sums[Table::Memory as usize] = (0..config.num_challenges)
        .map(|i| get_memory_extra_looking_sum(&public_values, ctl_challenges.challenges[i]))
        .collect_vec();

    verify_cross_table_lookups::<F, D, NUM_TABLES>(
        cross_table_lookups,
        all_proof
            .multi_proof
            .stark_proofs
            .map(|p| p.proof.openings.ctl_zs_first.unwrap()),
        Some(&extra_looking_sums),
        config,
    )
}

/// Computes the extra product to multiply to the looked value. It contains memory operations not in the CPU trace:
/// - block metadata writes,
/// - trie roots writes.
pub(crate) fn get_memory_extra_looking_sum<F, const D: usize>(
    public_values: &PublicValues,
    challenge: GrandProductChallenge<F>,
) -> F
where
    F: RichField + Extendable<D>,
{
    let mut sum = F::ZERO;

    // Add metadata and tries writes.
    let fields = [
        (
            GlobalMetadata::BlockBeneficiary,
            U256::from_big_endian(&public_values.block_metadata.block_beneficiary.0),
        ),
        (
            GlobalMetadata::BlockTimestamp,
            public_values.block_metadata.block_timestamp,
        ),
        (
            GlobalMetadata::BlockNumber,
            public_values.block_metadata.block_number,
        ),
        (
            GlobalMetadata::BlockRandom,
            public_values.block_metadata.block_random.into_uint(),
        ),
        (
            GlobalMetadata::BlockDifficulty,
            public_values.block_metadata.block_difficulty,
        ),
        (
            GlobalMetadata::BlockGasLimit,
            public_values.block_metadata.block_gaslimit,
        ),
        (
            GlobalMetadata::BlockChainId,
            public_values.block_metadata.block_chain_id,
        ),
        (
            GlobalMetadata::BlockBaseFee,
            public_values.block_metadata.block_base_fee,
        ),
        (
            GlobalMetadata::BlockCurrentHash,
            h2u(public_values.block_hashes.cur_hash),
        ),
        (
            GlobalMetadata::BlockGasUsed,
            public_values.block_metadata.block_gas_used,
        ),
        (
            GlobalMetadata::TxnNumberBefore,
            public_values.extra_block_data.txn_number_before,
        ),
        (
            GlobalMetadata::TxnNumberAfter,
            public_values.extra_block_data.txn_number_after,
        ),
        (
            GlobalMetadata::BlockGasUsedBefore,
            public_values.extra_block_data.gas_used_before,
        ),
        (
            GlobalMetadata::BlockGasUsedAfter,
            public_values.extra_block_data.gas_used_after,
        ),
        (
            GlobalMetadata::StateTrieRootDigestBefore,
            h2u(public_values.trie_roots_before.state_root),
        ),
        (
            GlobalMetadata::TransactionTrieRootDigestBefore,
            h2u(public_values.trie_roots_before.transactions_root),
        ),
        (
            GlobalMetadata::ReceiptTrieRootDigestBefore,
            h2u(public_values.trie_roots_before.receipts_root),
        ),
        (
            GlobalMetadata::StateTrieRootDigestAfter,
            h2u(public_values.trie_roots_after.state_root),
        ),
        (
            GlobalMetadata::TransactionTrieRootDigestAfter,
            h2u(public_values.trie_roots_after.transactions_root),
        ),
        (
            GlobalMetadata::ReceiptTrieRootDigestAfter,
            h2u(public_values.trie_roots_after.receipts_root),
        ),
        (GlobalMetadata::KernelHash, h2u(KERNEL.code_hash)),
        (GlobalMetadata::KernelLen, KERNEL.code.len().into()),
    ];

    let segment = F::from_canonical_usize(Segment::GlobalMetadata.unscale());

    fields.map(|(field, val)| {
        // These fields are already scaled by their segment, and are in context 0 (kernel).
        sum = add_data_write(challenge, segment, sum, field.unscale(), val)
    });

    // Add block bloom writes.
    let bloom_segment = F::from_canonical_usize(Segment::GlobalBlockBloom.unscale());
    for index in 0..8 {
        let val = public_values.block_metadata.block_bloom[index];
        sum = add_data_write(challenge, bloom_segment, sum, index, val);
    }

    // Add Blockhashes writes.
    let block_hashes_segment = F::from_canonical_usize(Segment::BlockHashes.unscale());
    for index in 0..256 {
        let val = h2u(public_values.block_hashes.prev_hashes[index]);
        sum = add_data_write(challenge, block_hashes_segment, sum, index, val);
    }

    sum
}

fn add_data_write<F, const D: usize>(
    challenge: GrandProductChallenge<F>,
    segment: F,
    running_sum: F,
    index: usize,
    val: U256,
) -> F
where
    F: RichField + Extendable<D>,
{
    let mut row = [F::ZERO; 13];
    row[0] = F::ZERO; // is_read
    row[1] = F::ZERO; // context
    row[2] = segment;
    row[3] = F::from_canonical_usize(index);

    for j in 0..VALUE_LIMBS {
        row[j + 4] = F::from_canonical_u32((val >> (j * 32)).low_u32());
    }
    row[12] = F::ONE; // timestamp
    running_sum + challenge.combine(row.iter()).inverse()
}

#[cfg(test)]
pub(crate) mod testutils {
    use super::*;

    /// Output all the extra memory rows that don't appear in the CPU trace but are
    /// necessary to correctly check the MemoryStark CTL.
    pub(crate) fn get_memory_extra_looking_values<F, const D: usize>(
        public_values: &PublicValues,
    ) -> Vec<Vec<F>>
    where
        F: RichField + Extendable<D>,
    {
        // Add metadata and tries writes.
        let fields = [
            (
                GlobalMetadata::BlockBeneficiary,
                U256::from_big_endian(&public_values.block_metadata.block_beneficiary.0),
            ),
            (
                GlobalMetadata::BlockTimestamp,
                public_values.block_metadata.block_timestamp,
            ),
            (
                GlobalMetadata::BlockNumber,
                public_values.block_metadata.block_number,
            ),
            (
                GlobalMetadata::BlockRandom,
                public_values.block_metadata.block_random.into_uint(),
            ),
            (
                GlobalMetadata::BlockDifficulty,
                public_values.block_metadata.block_difficulty,
            ),
            (
                GlobalMetadata::BlockGasLimit,
                public_values.block_metadata.block_gaslimit,
            ),
            (
                GlobalMetadata::BlockChainId,
                public_values.block_metadata.block_chain_id,
            ),
            (
                GlobalMetadata::BlockBaseFee,
                public_values.block_metadata.block_base_fee,
            ),
            (
                GlobalMetadata::BlockCurrentHash,
                h2u(public_values.block_hashes.cur_hash),
            ),
            (
                GlobalMetadata::BlockGasUsed,
                public_values.block_metadata.block_gas_used,
            ),
            (
                GlobalMetadata::TxnNumberBefore,
                public_values.extra_block_data.txn_number_before,
            ),
            (
                GlobalMetadata::TxnNumberAfter,
                public_values.extra_block_data.txn_number_after,
            ),
            (
                GlobalMetadata::BlockGasUsedBefore,
                public_values.extra_block_data.gas_used_before,
            ),
            (
                GlobalMetadata::BlockGasUsedAfter,
                public_values.extra_block_data.gas_used_after,
            ),
            (
                GlobalMetadata::StateTrieRootDigestBefore,
                h2u(public_values.trie_roots_before.state_root),
            ),
            (
                GlobalMetadata::TransactionTrieRootDigestBefore,
                h2u(public_values.trie_roots_before.transactions_root),
            ),
            (
                GlobalMetadata::ReceiptTrieRootDigestBefore,
                h2u(public_values.trie_roots_before.receipts_root),
            ),
            (
                GlobalMetadata::StateTrieRootDigestAfter,
                h2u(public_values.trie_roots_after.state_root),
            ),
            (
                GlobalMetadata::TransactionTrieRootDigestAfter,
                h2u(public_values.trie_roots_after.transactions_root),
            ),
            (
                GlobalMetadata::ReceiptTrieRootDigestAfter,
                h2u(public_values.trie_roots_after.receipts_root),
            ),
            (GlobalMetadata::KernelHash, h2u(KERNEL.code_hash)),
            (GlobalMetadata::KernelLen, KERNEL.code.len().into()),
        ];

        let segment = F::from_canonical_usize(Segment::GlobalMetadata.unscale());
        let mut extra_looking_rows = Vec::new();

        fields.map(|(field, val)| {
            extra_looking_rows.push(add_extra_looking_row(segment, field.unscale(), val))
        });

        // Add block bloom writes.
        let bloom_segment = F::from_canonical_usize(Segment::GlobalBlockBloom.unscale());
        for index in 0..8 {
            let val = public_values.block_metadata.block_bloom[index];
            extra_looking_rows.push(add_extra_looking_row(bloom_segment, index, val));
        }

        // Add Blockhashes writes.
        let block_hashes_segment = F::from_canonical_usize(Segment::BlockHashes.unscale());
        for index in 0..256 {
            let val = h2u(public_values.block_hashes.prev_hashes[index]);
            extra_looking_rows.push(add_extra_looking_row(block_hashes_segment, index, val));
        }

        extra_looking_rows
    }

    fn add_extra_looking_row<F, const D: usize>(segment: F, index: usize, val: U256) -> Vec<F>
    where
        F: RichField + Extendable<D>,
    {
        let mut row = vec![F::ZERO; 13];
        row[0] = F::ZERO; // is_read
        row[1] = F::ZERO; // context
        row[2] = segment;
        row[3] = F::from_canonical_usize(index);

        for j in 0..VALUE_LIMBS {
            row[j + 4] = F::from_canonical_u32((val >> (j * 32)).low_u32());
        }
        row[12] = F::ONE; // timestamp
        row
    }
}
