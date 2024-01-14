use ethereum_types::{Address, H160, H256, U256};
use itertools::Itertools;
use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::fri::oracle::PolynomialBatch;
use plonky2::fri::proof::{FriChallenges, FriChallengesTarget, FriProof, FriProofTarget};
use plonky2::fri::structure::{
    FriOpeningBatch, FriOpeningBatchTarget, FriOpenings, FriOpeningsTarget,
};
use plonky2::hash::hash_types::{MerkleCapTarget, RichField};
use plonky2::hash::merkle_tree::MerkleCap;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::iop::target::{BoolTarget, Target};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::config::{GenericConfig, Hasher};
use plonky2::util::serialization::{Buffer, IoResult, Read, Write};
use plonky2_maybe_rayon::*;
use serde::{Deserialize, Serialize};

use crate::all_stark::NUM_TABLES;
use crate::config::StarkConfig;
use crate::cross_table_lookup::GrandProductChallengeSet;
use crate::generation::mpt::TrieRootPtrs;
use crate::util::{get_h160, get_h256, h2u};

/// A STARK proof for each table, plus some metadata used to create recursive wrapper proofs.
#[derive(Debug, Clone)]
pub struct AllProof<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize> {
    /// Proofs for all the different STARK modules.
    pub stark_proofs: [StarkProofWithMetadata<F, C, D>; NUM_TABLES],
    /// Cross-table lookup challenges.
    pub(crate) ctl_challenges: GrandProductChallengeSet<F>,
    /// Public memory values used for the recursive proofs.
    pub public_values: PublicValues,
}

impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize> AllProof<F, C, D> {
    /// Returns the degree (i.e. the trace length) of each STARK.
    pub fn degree_bits(&self, config: &StarkConfig) -> [usize; NUM_TABLES] {
        core::array::from_fn(|i| self.stark_proofs[i].proof.recover_degree_bits(config))
    }
}

/// Randomness for all STARKs.
pub(crate) struct AllProofChallenges<F: RichField + Extendable<D>, const D: usize> {
    /// Randomness used in each STARK proof.
    pub stark_challenges: [StarkProofChallenges<F, D>; NUM_TABLES],
    /// Randomness used for cross-table lookups. It is shared by all STARKs.
    pub ctl_challenges: GrandProductChallengeSet<F>,
}

/// Memory values which are public.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct PublicValues {
    /// Trie hashes before the execution of the local state transition
    pub trie_roots_before: TrieRoots,
    /// Trie hashes after the execution of the local state transition.
    pub trie_roots_after: TrieRoots,
    /// Block metadata: it remains unchanged within a block.
    pub block_metadata: BlockMetadata,
    /// 256 previous block hashes and current block's hash.
    pub block_hashes: BlockHashes,
    /// Extra block data that is specific to the current proof.
    pub extra_block_data: ExtraBlockData,
}

impl PublicValues {
    /// Extracts public values from the given public inputs of a proof.
    /// Public values are always the first public inputs added to the circuit,
    /// so we can start extracting at index 0.
    pub fn from_public_inputs<F: RichField>(pis: &[F]) -> Self {
        assert!(
            pis.len()
                > TrieRootsTarget::SIZE * 2
                    + BlockMetadataTarget::SIZE
                    + BlockHashesTarget::SIZE
                    + ExtraBlockDataTarget::SIZE
                    - 1
        );

        let trie_roots_before = TrieRoots::from_public_inputs(&pis[0..TrieRootsTarget::SIZE]);
        let trie_roots_after =
            TrieRoots::from_public_inputs(&pis[TrieRootsTarget::SIZE..TrieRootsTarget::SIZE * 2]);
        let block_metadata = BlockMetadata::from_public_inputs(
            &pis[TrieRootsTarget::SIZE * 2..TrieRootsTarget::SIZE * 2 + BlockMetadataTarget::SIZE],
        );
        let block_hashes = BlockHashes::from_public_inputs(
            &pis[TrieRootsTarget::SIZE * 2 + BlockMetadataTarget::SIZE
                ..TrieRootsTarget::SIZE * 2 + BlockMetadataTarget::SIZE + BlockHashesTarget::SIZE],
        );
        let extra_block_data = ExtraBlockData::from_public_inputs(
            &pis[TrieRootsTarget::SIZE * 2 + BlockMetadataTarget::SIZE + BlockHashesTarget::SIZE
                ..TrieRootsTarget::SIZE * 2
                    + BlockMetadataTarget::SIZE
                    + BlockHashesTarget::SIZE
                    + ExtraBlockDataTarget::SIZE],
        );

        Self {
            trie_roots_before,
            trie_roots_after,
            block_metadata,
            block_hashes,
            extra_block_data,
        }
    }
}

/// Trie hashes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrieRoots {
    /// State trie hash.
    pub state_root: H256,
    /// Transaction trie hash.
    pub transactions_root: H256,
    /// Receipts trie hash.
    pub receipts_root: H256,
}

impl TrieRoots {
    pub fn from_public_inputs<F: RichField>(pis: &[F]) -> Self {
        assert!(pis.len() == TrieRootsTarget::SIZE);

        let state_root = get_h256(&pis[0..8]);
        let transactions_root = get_h256(&pis[8..16]);
        let receipts_root = get_h256(&pis[16..24]);

        Self {
            state_root,
            transactions_root,
            receipts_root,
        }
    }
}

// There should be 256 previous hashes stored, so the default should also contain 256 values.
impl Default for BlockHashes {
    fn default() -> Self {
        Self {
            prev_hashes: vec![H256::default(); 256],
            cur_hash: H256::default(),
        }
    }
}

/// User-provided helper values to compute the `BLOCKHASH` opcode.
/// The proofs across consecutive blocks ensure that these values
/// are consistent (i.e. shifted by one to the left).
///
/// When the block number is less than 256, dummy values, i.e. `H256::default()`,
/// should be used for the additional block hashes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockHashes {
    /// The previous 256 hashes to the current block. The leftmost hash, i.e. `prev_hashes[0]`,
    /// is the oldest, and the rightmost, i.e. `prev_hashes[255]` is the hash of the parent block.
    pub prev_hashes: Vec<H256>,
    // The hash of the current block.
    pub cur_hash: H256,
}

impl BlockHashes {
    pub fn from_public_inputs<F: RichField>(pis: &[F]) -> Self {
        assert!(pis.len() == BlockHashesTarget::SIZE);

        let prev_hashes: [H256; 256] = core::array::from_fn(|i| get_h256(&pis[8 * i..8 + 8 * i]));
        let cur_hash = get_h256(&pis[2048..2056]);

        Self {
            prev_hashes: prev_hashes.to_vec(),
            cur_hash,
        }
    }
}

/// Metadata contained in a block header. Those are identical between
/// all state transition proofs within the same block.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct BlockMetadata {
    /// The address of this block's producer.
    pub block_beneficiary: Address,
    /// The timestamp of this block.
    pub block_timestamp: U256,
    /// The index of this block.
    pub block_number: U256,
    /// The difficulty (before PoS transition) of this block.
    pub block_difficulty: U256,
    pub block_random: H256,
    /// The gas limit of this block. It must fit in a `u32`.
    pub block_gaslimit: U256,
    /// The chain id of this block.
    pub block_chain_id: U256,
    /// The base fee of this block.
    pub block_base_fee: U256,
    /// The total gas used in this block. It must fit in a `u32`.
    pub block_gas_used: U256,
    /// The block bloom of this block, represented as the consecutive
    /// 32-byte chunks of a block's final bloom filter string.
    pub block_bloom: [U256; 8],
}

impl BlockMetadata {
    pub fn from_public_inputs<F: RichField>(pis: &[F]) -> Self {
        assert!(pis.len() == BlockMetadataTarget::SIZE);

        let block_beneficiary = get_h160(&pis[0..5]);
        let block_timestamp = pis[5].to_canonical_u64().into();
        let block_number = pis[6].to_canonical_u64().into();
        let block_difficulty = pis[7].to_canonical_u64().into();
        let block_random = get_h256(&pis[8..16]);
        let block_gaslimit = pis[16].to_canonical_u64().into();
        let block_chain_id = pis[17].to_canonical_u64().into();
        let block_base_fee =
            (pis[18].to_canonical_u64() + (pis[19].to_canonical_u64() << 32)).into();
        let block_gas_used = pis[20].to_canonical_u64().into();
        let block_bloom = core::array::from_fn(|i| h2u(get_h256(&pis[21 + 8 * i..29 + 8 * i])));

        Self {
            block_beneficiary,
            block_timestamp,
            block_number,
            block_difficulty,
            block_random,
            block_gaslimit,
            block_chain_id,
            block_base_fee,
            block_gas_used,
            block_bloom,
        }
    }
}

/// Additional block data that are specific to the local transaction being proven,
/// unlike `BlockMetadata`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct ExtraBlockData {
    /// The state trie digest of the checkpoint block.
    pub checkpoint_state_trie_root: H256,
    /// The transaction count prior execution of the local state transition, starting
    /// at 0 for the initial transaction of a block.
    pub txn_number_before: U256,
    /// The transaction count after execution of the local state transition.
    pub txn_number_after: U256,
    /// The accumulated gas used prior execution of the local state transition, starting
    /// at 0 for the initial transaction of a block.
    pub gas_used_before: U256,
    /// The accumulated gas used after execution of the local state transition. It should
    /// match the `block_gas_used` value after execution of the last transaction in a block.
    pub gas_used_after: U256,
}

impl ExtraBlockData {
    pub fn from_public_inputs<F: RichField>(pis: &[F]) -> Self {
        assert!(pis.len() == ExtraBlockDataTarget::SIZE);

        let checkpoint_state_trie_root = get_h256(&pis[0..8]);
        let txn_number_before = pis[8].to_canonical_u64().into();
        let txn_number_after = pis[9].to_canonical_u64().into();
        let gas_used_before = pis[10].to_canonical_u64().into();
        let gas_used_after = pis[11].to_canonical_u64().into();

        Self {
            checkpoint_state_trie_root,
            txn_number_before,
            txn_number_after,
            gas_used_before,
            gas_used_after,
        }
    }
}

/// Memory values which are public.
/// Note: All the larger integers are encoded with 32-bit limbs in little-endian order.
#[derive(Eq, PartialEq, Debug)]
pub struct PublicValuesTarget {
    /// Trie hashes before the execution of the local state transition.
    pub trie_roots_before: TrieRootsTarget,
    /// Trie hashes after the execution of the local state transition.
    pub trie_roots_after: TrieRootsTarget,
    /// Block metadata: it remains unchanged within a block.
    pub block_metadata: BlockMetadataTarget,
    /// 256 previous block hashes and current block's hash.
    pub block_hashes: BlockHashesTarget,
    /// Extra block data that is specific to the current proof.
    pub extra_block_data: ExtraBlockDataTarget,
}

impl PublicValuesTarget {
    /// Serializes public value targets.
    pub(crate) fn to_buffer(&self, buffer: &mut Vec<u8>) -> IoResult<()> {
        let TrieRootsTarget {
            state_root: state_root_before,
            transactions_root: transactions_root_before,
            receipts_root: receipts_root_before,
        } = self.trie_roots_before;

        buffer.write_target_array(&state_root_before)?;
        buffer.write_target_array(&transactions_root_before)?;
        buffer.write_target_array(&receipts_root_before)?;

        let TrieRootsTarget {
            state_root: state_root_after,
            transactions_root: transactions_root_after,
            receipts_root: receipts_root_after,
        } = self.trie_roots_after;

        buffer.write_target_array(&state_root_after)?;
        buffer.write_target_array(&transactions_root_after)?;
        buffer.write_target_array(&receipts_root_after)?;

        let BlockMetadataTarget {
            block_beneficiary,
            block_timestamp,
            block_number,
            block_difficulty,
            block_random,
            block_gaslimit,
            block_chain_id,
            block_base_fee,
            block_gas_used,
            block_bloom,
        } = self.block_metadata;

        buffer.write_target_array(&block_beneficiary)?;
        buffer.write_target(block_timestamp)?;
        buffer.write_target(block_number)?;
        buffer.write_target(block_difficulty)?;
        buffer.write_target_array(&block_random)?;
        buffer.write_target(block_gaslimit)?;
        buffer.write_target(block_chain_id)?;
        buffer.write_target_array(&block_base_fee)?;
        buffer.write_target(block_gas_used)?;
        buffer.write_target_array(&block_bloom)?;

        let BlockHashesTarget {
            prev_hashes,
            cur_hash,
        } = self.block_hashes;
        buffer.write_target_array(&prev_hashes)?;
        buffer.write_target_array(&cur_hash)?;

        let ExtraBlockDataTarget {
            checkpoint_state_trie_root,
            txn_number_before,
            txn_number_after,
            gas_used_before,
            gas_used_after,
        } = self.extra_block_data;
        buffer.write_target_array(&checkpoint_state_trie_root)?;
        buffer.write_target(txn_number_before)?;
        buffer.write_target(txn_number_after)?;
        buffer.write_target(gas_used_before)?;
        buffer.write_target(gas_used_after)?;

        Ok(())
    }

    /// Deserializes public value targets.
    pub(crate) fn from_buffer(buffer: &mut Buffer) -> IoResult<Self> {
        let trie_roots_before = TrieRootsTarget {
            state_root: buffer.read_target_array()?,
            transactions_root: buffer.read_target_array()?,
            receipts_root: buffer.read_target_array()?,
        };

        let trie_roots_after = TrieRootsTarget {
            state_root: buffer.read_target_array()?,
            transactions_root: buffer.read_target_array()?,
            receipts_root: buffer.read_target_array()?,
        };

        let block_metadata = BlockMetadataTarget {
            block_beneficiary: buffer.read_target_array()?,
            block_timestamp: buffer.read_target()?,
            block_number: buffer.read_target()?,
            block_difficulty: buffer.read_target()?,
            block_random: buffer.read_target_array()?,
            block_gaslimit: buffer.read_target()?,
            block_chain_id: buffer.read_target()?,
            block_base_fee: buffer.read_target_array()?,
            block_gas_used: buffer.read_target()?,
            block_bloom: buffer.read_target_array()?,
        };

        let block_hashes = BlockHashesTarget {
            prev_hashes: buffer.read_target_array()?,
            cur_hash: buffer.read_target_array()?,
        };

        let extra_block_data = ExtraBlockDataTarget {
            checkpoint_state_trie_root: buffer.read_target_array()?,
            txn_number_before: buffer.read_target()?,
            txn_number_after: buffer.read_target()?,
            gas_used_before: buffer.read_target()?,
            gas_used_after: buffer.read_target()?,
        };

        Ok(Self {
            trie_roots_before,
            trie_roots_after,
            block_metadata,
            block_hashes,
            extra_block_data,
        })
    }

    /// Extracts public value `Target`s from the given public input `Target`s.
    /// Public values are always the first public inputs added to the circuit,
    /// so we can start extracting at index 0.
    pub(crate) fn from_public_inputs(pis: &[Target]) -> Self {
        assert!(
            pis.len()
                > TrieRootsTarget::SIZE * 2
                    + BlockMetadataTarget::SIZE
                    + BlockHashesTarget::SIZE
                    + ExtraBlockDataTarget::SIZE
                    - 1
        );

        Self {
            trie_roots_before: TrieRootsTarget::from_public_inputs(&pis[0..TrieRootsTarget::SIZE]),
            trie_roots_after: TrieRootsTarget::from_public_inputs(
                &pis[TrieRootsTarget::SIZE..TrieRootsTarget::SIZE * 2],
            ),
            block_metadata: BlockMetadataTarget::from_public_inputs(
                &pis[TrieRootsTarget::SIZE * 2
                    ..TrieRootsTarget::SIZE * 2 + BlockMetadataTarget::SIZE],
            ),
            block_hashes: BlockHashesTarget::from_public_inputs(
                &pis[TrieRootsTarget::SIZE * 2 + BlockMetadataTarget::SIZE
                    ..TrieRootsTarget::SIZE * 2
                        + BlockMetadataTarget::SIZE
                        + BlockHashesTarget::SIZE],
            ),
            extra_block_data: ExtraBlockDataTarget::from_public_inputs(
                &pis[TrieRootsTarget::SIZE * 2 + BlockMetadataTarget::SIZE + BlockHashesTarget::SIZE
                    ..TrieRootsTarget::SIZE * 2
                        + BlockMetadataTarget::SIZE
                        + BlockHashesTarget::SIZE
                        + ExtraBlockDataTarget::SIZE],
            ),
        }
    }

    /// Returns the public values in `pv0` or `pv1` depening on `condition`.
    pub(crate) fn select<F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
        condition: BoolTarget,
        pv0: Self,
        pv1: Self,
    ) -> Self {
        Self {
            trie_roots_before: TrieRootsTarget::select(
                builder,
                condition,
                pv0.trie_roots_before,
                pv1.trie_roots_before,
            ),
            trie_roots_after: TrieRootsTarget::select(
                builder,
                condition,
                pv0.trie_roots_after,
                pv1.trie_roots_after,
            ),
            block_metadata: BlockMetadataTarget::select(
                builder,
                condition,
                pv0.block_metadata,
                pv1.block_metadata,
            ),
            block_hashes: BlockHashesTarget::select(
                builder,
                condition,
                pv0.block_hashes,
                pv1.block_hashes,
            ),
            extra_block_data: ExtraBlockDataTarget::select(
                builder,
                condition,
                pv0.extra_block_data,
                pv1.extra_block_data,
            ),
        }
    }
}

/// Circuit version of `TrieRoots`.
/// `Target`s for trie hashes. Since a `Target` holds a 32-bit limb, each hash requires 8 `Target`s.
#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub struct TrieRootsTarget {
    /// Targets for the state trie hash.
    pub(crate) state_root: [Target; 8],
    /// Targets for the transactions trie hash.
    pub(crate) transactions_root: [Target; 8],
    /// Targets for the receipts trie hash.
    pub(crate) receipts_root: [Target; 8],
}

impl TrieRootsTarget {
    /// Number of `Target`s required for all trie hashes.
    pub(crate) const HASH_SIZE: usize = 8;
    pub(crate) const SIZE: usize = Self::HASH_SIZE * 3;

    /// Extracts trie hash `Target`s for all tries from the provided public input `Target`s.
    /// The provided `pis` should start with the trie hashes.
    pub(crate) fn from_public_inputs(pis: &[Target]) -> Self {
        let state_root = pis[0..8].try_into().unwrap();
        let transactions_root = pis[8..16].try_into().unwrap();
        let receipts_root = pis[16..24].try_into().unwrap();

        Self {
            state_root,
            transactions_root,
            receipts_root,
        }
    }

    /// If `condition`, returns the trie hashes in `tr0`,
    /// otherwise returns the trie hashes in `tr1`.
    pub(crate) fn select<F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
        condition: BoolTarget,
        tr0: Self,
        tr1: Self,
    ) -> Self {
        Self {
            state_root: core::array::from_fn(|i| {
                builder.select(condition, tr0.state_root[i], tr1.state_root[i])
            }),
            transactions_root: core::array::from_fn(|i| {
                builder.select(
                    condition,
                    tr0.transactions_root[i],
                    tr1.transactions_root[i],
                )
            }),
            receipts_root: core::array::from_fn(|i| {
                builder.select(condition, tr0.receipts_root[i], tr1.receipts_root[i])
            }),
        }
    }

    /// Connects the trie hashes in `tr0` and in `tr1`.
    pub(crate) fn connect<F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
        tr0: Self,
        tr1: Self,
    ) {
        for i in 0..8 {
            builder.connect(tr0.state_root[i], tr1.state_root[i]);
            builder.connect(tr0.transactions_root[i], tr1.transactions_root[i]);
            builder.connect(tr0.receipts_root[i], tr1.receipts_root[i]);
        }
    }
}

/// Circuit version of `BlockMetadata`.
/// Metadata contained in a block header. Those are identical between
/// all state transition proofs within the same block.
#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub struct BlockMetadataTarget {
    /// `Target`s for the address of this block's producer.
    pub(crate) block_beneficiary: [Target; 5],
    /// `Target` for the timestamp of this block.
    pub(crate) block_timestamp: Target,
    /// `Target` for the index of this block.
    pub(crate) block_number: Target,
    /// `Target` for the difficulty (before PoS transition) of this block.
    pub(crate) block_difficulty: Target,
    /// `Target`s for the `mix_hash` value of this block.
    pub(crate) block_random: [Target; 8],
    /// `Target`s for the gas limit of this block.
    pub(crate) block_gaslimit: Target,
    /// `Target` for the chain id of this block.
    pub(crate) block_chain_id: Target,
    /// `Target`s for the base fee of this block.
    pub(crate) block_base_fee: [Target; 2],
    /// `Target`s for the gas used of this block.
    pub(crate) block_gas_used: Target,
    /// `Target`s for the block bloom of this block.
    pub(crate) block_bloom: [Target; 64],
}

impl BlockMetadataTarget {
    /// Number of `Target`s required for the block metadata.
    pub(crate) const SIZE: usize = 85;

    /// Extracts block metadata `Target`s from the provided public input `Target`s.
    /// The provided `pis` should start with the block metadata.
    pub(crate) fn from_public_inputs(pis: &[Target]) -> Self {
        let block_beneficiary = pis[0..5].try_into().unwrap();
        let block_timestamp = pis[5];
        let block_number = pis[6];
        let block_difficulty = pis[7];
        let block_random = pis[8..16].try_into().unwrap();
        let block_gaslimit = pis[16];
        let block_chain_id = pis[17];
        let block_base_fee = pis[18..20].try_into().unwrap();
        let block_gas_used = pis[20];
        let block_bloom = pis[21..85].try_into().unwrap();

        Self {
            block_beneficiary,
            block_timestamp,
            block_number,
            block_difficulty,
            block_random,
            block_gaslimit,
            block_chain_id,
            block_base_fee,
            block_gas_used,
            block_bloom,
        }
    }

    /// If `condition`, returns the block metadata in `bm0`,
    /// otherwise returns the block metadata in `bm1`.
    pub(crate) fn select<F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
        condition: BoolTarget,
        bm0: Self,
        bm1: Self,
    ) -> Self {
        Self {
            block_beneficiary: core::array::from_fn(|i| {
                builder.select(
                    condition,
                    bm0.block_beneficiary[i],
                    bm1.block_beneficiary[i],
                )
            }),
            block_timestamp: builder.select(condition, bm0.block_timestamp, bm1.block_timestamp),
            block_number: builder.select(condition, bm0.block_number, bm1.block_number),
            block_difficulty: builder.select(condition, bm0.block_difficulty, bm1.block_difficulty),
            block_random: core::array::from_fn(|i| {
                builder.select(condition, bm0.block_random[i], bm1.block_random[i])
            }),
            block_gaslimit: builder.select(condition, bm0.block_gaslimit, bm1.block_gaslimit),
            block_chain_id: builder.select(condition, bm0.block_chain_id, bm1.block_chain_id),
            block_base_fee: core::array::from_fn(|i| {
                builder.select(condition, bm0.block_base_fee[i], bm1.block_base_fee[i])
            }),
            block_gas_used: builder.select(condition, bm0.block_gas_used, bm1.block_gas_used),
            block_bloom: core::array::from_fn(|i| {
                builder.select(condition, bm0.block_bloom[i], bm1.block_bloom[i])
            }),
        }
    }

    /// Connects the block metadata in `bm0` to the block metadata in `bm1`.
    pub(crate) fn connect<F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
        bm0: Self,
        bm1: Self,
    ) {
        for i in 0..5 {
            builder.connect(bm0.block_beneficiary[i], bm1.block_beneficiary[i]);
        }
        builder.connect(bm0.block_timestamp, bm1.block_timestamp);
        builder.connect(bm0.block_number, bm1.block_number);
        builder.connect(bm0.block_difficulty, bm1.block_difficulty);
        for i in 0..8 {
            builder.connect(bm0.block_random[i], bm1.block_random[i]);
        }
        builder.connect(bm0.block_gaslimit, bm1.block_gaslimit);
        builder.connect(bm0.block_chain_id, bm1.block_chain_id);
        for i in 0..2 {
            builder.connect(bm0.block_base_fee[i], bm1.block_base_fee[i])
        }
        builder.connect(bm0.block_gas_used, bm1.block_gas_used);
        for i in 0..64 {
            builder.connect(bm0.block_bloom[i], bm1.block_bloom[i])
        }
    }
}

/// Circuit version of `BlockHashes`.
/// `Target`s for the user-provided previous 256 block hashes and current block hash.
/// Each block hash requires 8 `Target`s.
/// The proofs across consecutive blocks ensure that these values
/// are consistent (i.e. shifted by eight `Target`s to the left).
///
/// When the block number is less than 256, dummy values, i.e. `H256::default()`,
/// should be used for the additional block hashes.
#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub struct BlockHashesTarget {
    /// `Target`s for the previous 256 hashes to the current block. The leftmost hash, i.e. `prev_hashes[0..8]`,
    /// is the oldest, and the rightmost, i.e. `prev_hashes[255 * 7..255 * 8]` is the hash of the parent block.
    pub(crate) prev_hashes: [Target; 2048],
    // `Target` for the hash of the current block.
    pub(crate) cur_hash: [Target; 8],
}

impl BlockHashesTarget {
    /// Number of `Target`s required for previous and current block hashes.
    pub(crate) const SIZE: usize = 2056;

    /// Extracts the previous and current block hash `Target`s from the public input `Target`s.
    /// The provided `pis` should start with the block hashes.
    pub(crate) fn from_public_inputs(pis: &[Target]) -> Self {
        Self {
            prev_hashes: pis[0..2048].try_into().unwrap(),
            cur_hash: pis[2048..2056].try_into().unwrap(),
        }
    }

    /// If `condition`, returns the block hashes in `bm0`,
    /// otherwise returns the block hashes in `bm1`.
    pub(crate) fn select<F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
        condition: BoolTarget,
        bm0: Self,
        bm1: Self,
    ) -> Self {
        Self {
            prev_hashes: core::array::from_fn(|i| {
                builder.select(condition, bm0.prev_hashes[i], bm1.prev_hashes[i])
            }),
            cur_hash: core::array::from_fn(|i| {
                builder.select(condition, bm0.cur_hash[i], bm1.cur_hash[i])
            }),
        }
    }

    /// Connects the block hashes in `bm0` to the block hashes in `bm1`.
    pub(crate) fn connect<F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
        bm0: Self,
        bm1: Self,
    ) {
        for i in 0..2048 {
            builder.connect(bm0.prev_hashes[i], bm1.prev_hashes[i]);
        }
        for i in 0..8 {
            builder.connect(bm0.cur_hash[i], bm1.cur_hash[i]);
        }
    }
}

/// Circuit version of `ExtraBlockData`.
/// Additional block data that are specific to the local transaction being proven,
/// unlike `BlockMetadata`.
#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub struct ExtraBlockDataTarget {
    /// `Target`s for the state trie digest of the checkpoint block.
    pub checkpoint_state_trie_root: [Target; 8],
    /// `Target` for the transaction count prior execution of the local state transition, starting
    /// at 0 for the initial trnasaction of a block.
    pub txn_number_before: Target,
    /// `Target` for the transaction count after execution of the local state transition.
    pub txn_number_after: Target,
    /// `Target` for the accumulated gas used prior execution of the local state transition, starting
    /// at 0 for the initial transaction of a block.
    pub gas_used_before: Target,
    /// `Target` for the accumulated gas used after execution of the local state transition. It should
    /// match the `block_gas_used` value after execution of the last transaction in a block.
    pub gas_used_after: Target,
}

impl ExtraBlockDataTarget {
    /// Number of `Target`s required for the extra block data.
    const SIZE: usize = 12;

    /// Extracts the extra block data `Target`s from the public input `Target`s.
    /// The provided `pis` should start with the extra vblock data.
    pub(crate) fn from_public_inputs(pis: &[Target]) -> Self {
        let checkpoint_state_trie_root = pis[0..8].try_into().unwrap();
        let txn_number_before = pis[8];
        let txn_number_after = pis[9];
        let gas_used_before = pis[10];
        let gas_used_after = pis[11];

        Self {
            checkpoint_state_trie_root,
            txn_number_before,
            txn_number_after,
            gas_used_before,
            gas_used_after,
        }
    }

    /// If `condition`, returns the extra block data in `ed0`,
    /// otherwise returns the extra block data in `ed1`.
    pub(crate) fn select<F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
        condition: BoolTarget,
        ed0: Self,
        ed1: Self,
    ) -> Self {
        Self {
            checkpoint_state_trie_root: core::array::from_fn(|i| {
                builder.select(
                    condition,
                    ed0.checkpoint_state_trie_root[i],
                    ed1.checkpoint_state_trie_root[i],
                )
            }),
            txn_number_before: builder.select(
                condition,
                ed0.txn_number_before,
                ed1.txn_number_before,
            ),
            txn_number_after: builder.select(condition, ed0.txn_number_after, ed1.txn_number_after),
            gas_used_before: builder.select(condition, ed0.gas_used_before, ed1.gas_used_before),
            gas_used_after: builder.select(condition, ed0.gas_used_after, ed1.gas_used_after),
        }
    }

    /// Connects the extra block data in `ed0` with the extra block data in `ed1`.
    pub(crate) fn connect<F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
        ed0: Self,
        ed1: Self,
    ) {
        for i in 0..8 {
            builder.connect(
                ed0.checkpoint_state_trie_root[i],
                ed1.checkpoint_state_trie_root[i],
            );
        }
        builder.connect(ed0.txn_number_before, ed1.txn_number_before);
        builder.connect(ed0.txn_number_after, ed1.txn_number_after);
        builder.connect(ed0.gas_used_before, ed1.gas_used_before);
        builder.connect(ed0.gas_used_after, ed1.gas_used_after);
    }
}

/// Merkle caps and openings that form the proof of a single STARK.
#[derive(Debug, Clone)]
pub struct StarkProof<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize> {
    /// Merkle cap of LDEs of trace values.
    pub trace_cap: MerkleCap<F, C::Hasher>,
    /// Merkle cap of LDEs of lookup helper and CTL columns.
    pub auxiliary_polys_cap: MerkleCap<F, C::Hasher>,
    /// Merkle cap of LDEs of quotient polynomial evaluations.
    pub quotient_polys_cap: MerkleCap<F, C::Hasher>,
    /// Purported values of each polynomial at the challenge point.
    pub openings: StarkOpeningSet<F, D>,
    /// A batch FRI argument for all openings.
    pub opening_proof: FriProof<F, C::Hasher, D>,
}

/// A `StarkProof` along with some metadata about the initial Fiat-Shamir state, which is used when
/// creating a recursive wrapper proof around a STARK proof.
#[derive(Debug, Clone)]
pub struct StarkProofWithMetadata<F, C, const D: usize>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
{
    /// Initial Fiat-Shamir state.
    pub(crate) init_challenger_state: <C::InnerHasher as Hasher<F>>::Permutation,
    /// Proof for a single STARK.
    pub(crate) proof: StarkProof<F, C, D>,
}

impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize> StarkProof<F, C, D> {
    /// Recover the length of the trace from a STARK proof and a STARK config.
    pub fn recover_degree_bits(&self, config: &StarkConfig) -> usize {
        let initial_merkle_proof = &self.opening_proof.query_round_proofs[0]
            .initial_trees_proof
            .evals_proofs[0]
            .1;
        let lde_bits = config.fri_config.cap_height + initial_merkle_proof.siblings.len();
        lde_bits - config.fri_config.rate_bits
    }

    /// Returns the number of cross-table lookup polynomials computed for the current STARK.
    pub fn num_ctl_zs(&self) -> usize {
        self.openings.ctl_zs_first.len()
    }
}

/// Circuit version of `StarkProof`.
/// Merkle caps and openings that form the proof of a single STARK.
#[derive(Eq, PartialEq, Debug)]
pub(crate) struct StarkProofTarget<const D: usize> {
    /// `Target` for the Merkle cap if LDEs of trace values.
    pub trace_cap: MerkleCapTarget,
    /// `Target` for the Merkle cap of LDEs of lookup helper and CTL columns.
    pub auxiliary_polys_cap: MerkleCapTarget,
    /// `Target` for the Merkle cap of LDEs of quotient polynomial evaluations.
    pub quotient_polys_cap: MerkleCapTarget,
    /// `Target`s for the purported values of each polynomial at the challenge point.
    pub openings: StarkOpeningSetTarget<D>,
    /// `Target`s for the batch FRI argument for all openings.
    pub opening_proof: FriProofTarget<D>,
}

impl<const D: usize> StarkProofTarget<D> {
    /// Serializes a STARK proof.
    pub(crate) fn to_buffer(&self, buffer: &mut Vec<u8>) -> IoResult<()> {
        buffer.write_target_merkle_cap(&self.trace_cap)?;
        buffer.write_target_merkle_cap(&self.auxiliary_polys_cap)?;
        buffer.write_target_merkle_cap(&self.quotient_polys_cap)?;
        buffer.write_target_fri_proof(&self.opening_proof)?;
        self.openings.to_buffer(buffer)?;
        Ok(())
    }

    /// Deserializes a STARK proof.
    pub(crate) fn from_buffer(buffer: &mut Buffer) -> IoResult<Self> {
        let trace_cap = buffer.read_target_merkle_cap()?;
        let auxiliary_polys_cap = buffer.read_target_merkle_cap()?;
        let quotient_polys_cap = buffer.read_target_merkle_cap()?;
        let opening_proof = buffer.read_target_fri_proof()?;
        let openings = StarkOpeningSetTarget::from_buffer(buffer)?;

        Ok(Self {
            trace_cap,
            auxiliary_polys_cap,
            quotient_polys_cap,
            openings,
            opening_proof,
        })
    }

    /// Recover the length of the trace from a STARK proof and a STARK config.
    pub(crate) fn recover_degree_bits(&self, config: &StarkConfig) -> usize {
        let initial_merkle_proof = &self.opening_proof.query_round_proofs[0]
            .initial_trees_proof
            .evals_proofs[0]
            .1;
        let lde_bits = config.fri_config.cap_height + initial_merkle_proof.siblings.len();
        lde_bits - config.fri_config.rate_bits
    }
}

/// Randomness used for a STARK proof.
pub(crate) struct StarkProofChallenges<F: RichField + Extendable<D>, const D: usize> {
    /// Random values used to combine STARK constraints.
    pub stark_alphas: Vec<F>,

    /// Point at which the STARK polynomials are opened.
    pub stark_zeta: F::Extension,

    /// Randomness used in FRI.
    pub fri_challenges: FriChallenges<F, D>,
}

/// Circuit version of `StarkProofChallenges`.
pub(crate) struct StarkProofChallengesTarget<const D: usize> {
    /// `Target`s for the random values used to combine STARK constraints.
    pub stark_alphas: Vec<Target>,
    /// `ExtensionTarget` for the point at which the STARK polynomials are opened.
    pub stark_zeta: ExtensionTarget<D>,
    /// `Target`s for the randomness used in FRI.
    pub fri_challenges: FriChallengesTarget<D>,
}

/// Purported values of each polynomial at the challenge point.
#[derive(Debug, Clone)]
pub struct StarkOpeningSet<F: RichField + Extendable<D>, const D: usize> {
    /// Openings of trace polynomials at `zeta`.
    pub local_values: Vec<F::Extension>,
    /// Openings of trace polynomials at `g * zeta`.
    pub next_values: Vec<F::Extension>,
    /// Openings of lookups and cross-table lookups `Z` polynomials at `zeta`.
    pub auxiliary_polys: Vec<F::Extension>,
    /// Openings of lookups and cross-table lookups `Z` polynomials at `g * zeta`.
    pub auxiliary_polys_next: Vec<F::Extension>,
    /// Openings of cross-table lookups `Z` polynomials at `1`.
    pub ctl_zs_first: Vec<F>,
    /// Openings of quotient polynomials at `zeta`.
    pub quotient_polys: Vec<F::Extension>,
}

impl<F: RichField + Extendable<D>, const D: usize> StarkOpeningSet<F, D> {
    /// Returns a `StarkOpeningSet` given all the polynomial commitments, the number of permutation `Z`polynomials,
    /// the evaluation point and a generator `g`.
    /// Polynomials are evaluated at point `zeta` and, if necessary, at `g * zeta`.
    pub fn new<C: GenericConfig<D, F = F>>(
        zeta: F::Extension,
        g: F,
        trace_commitment: &PolynomialBatch<F, C, D>,
        auxiliary_polys_commitment: &PolynomialBatch<F, C, D>,
        quotient_commitment: &PolynomialBatch<F, C, D>,
        num_lookup_columns: usize,
        num_ctl_polys: &[usize],
    ) -> Self {
        let total_num_helper_cols: usize = num_ctl_polys.iter().sum();

        // Batch evaluates polynomials on the LDE, at a point `z`.
        let eval_commitment = |z: F::Extension, c: &PolynomialBatch<F, C, D>| {
            c.polynomials
                .par_iter()
                .map(|p| p.to_extension().eval(z))
                .collect::<Vec<_>>()
        };
        // Batch evaluates polynomials at a base field point `z`.
        let eval_commitment_base = |z: F, c: &PolynomialBatch<F, C, D>| {
            c.polynomials
                .par_iter()
                .map(|p| p.eval(z))
                .collect::<Vec<_>>()
        };

        let auxiliary_first = eval_commitment_base(F::ONE, auxiliary_polys_commitment);
        let ctl_zs_first = auxiliary_first[num_lookup_columns + total_num_helper_cols..].to_vec();
        // `g * zeta`.
        let zeta_next = zeta.scalar_mul(g);
        Self {
            local_values: eval_commitment(zeta, trace_commitment),
            next_values: eval_commitment(zeta_next, trace_commitment),
            auxiliary_polys: eval_commitment(zeta, auxiliary_polys_commitment),
            auxiliary_polys_next: eval_commitment(zeta_next, auxiliary_polys_commitment),
            ctl_zs_first,
            quotient_polys: eval_commitment(zeta, quotient_commitment),
        }
    }

    /// Constructs the openings required by FRI.
    /// All openings but `ctl_zs_first` are grouped together.
    pub(crate) fn to_fri_openings(&self) -> FriOpenings<F, D> {
        let zeta_batch = FriOpeningBatch {
            values: self
                .local_values
                .iter()
                .chain(&self.auxiliary_polys)
                .chain(&self.quotient_polys)
                .copied()
                .collect_vec(),
        };
        let zeta_next_batch = FriOpeningBatch {
            values: self
                .next_values
                .iter()
                .chain(&self.auxiliary_polys_next)
                .copied()
                .collect_vec(),
        };
        debug_assert!(!self.ctl_zs_first.is_empty());
        let ctl_first_batch = FriOpeningBatch {
            values: self
                .ctl_zs_first
                .iter()
                .copied()
                .map(F::Extension::from_basefield)
                .collect(),
        };

        FriOpenings {
            batches: vec![zeta_batch, zeta_next_batch, ctl_first_batch],
        }
    }
}

/// Circuit version of `StarkOpeningSet`.
/// `Target`s for the purported values of each polynomial at the challenge point.
#[derive(Eq, PartialEq, Debug)]
pub(crate) struct StarkOpeningSetTarget<const D: usize> {
    /// `ExtensionTarget`s for the openings of trace polynomials at `zeta`.
    pub local_values: Vec<ExtensionTarget<D>>,
    /// `ExtensionTarget`s for the opening of trace polynomials at `g * zeta`.
    pub next_values: Vec<ExtensionTarget<D>>,
    /// `ExtensionTarget`s for the opening of lookups and cross-table lookups `Z` polynomials at `zeta`.
    pub auxiliary_polys: Vec<ExtensionTarget<D>>,
    /// `ExtensionTarget`s for the opening of lookups and cross-table lookups `Z` polynomials at `g * zeta`.
    pub auxiliary_polys_next: Vec<ExtensionTarget<D>>,
    /// `ExtensionTarget`s for the opening of lookups and cross-table lookups `Z` polynomials at 1.
    pub ctl_zs_first: Vec<Target>,
    /// `ExtensionTarget`s for the opening of quotient polynomials at `zeta`.
    pub quotient_polys: Vec<ExtensionTarget<D>>,
}

impl<const D: usize> StarkOpeningSetTarget<D> {
    /// Serializes a STARK's opening set.
    pub(crate) fn to_buffer(&self, buffer: &mut Vec<u8>) -> IoResult<()> {
        buffer.write_target_ext_vec(&self.local_values)?;
        buffer.write_target_ext_vec(&self.next_values)?;
        buffer.write_target_ext_vec(&self.auxiliary_polys)?;
        buffer.write_target_ext_vec(&self.auxiliary_polys_next)?;
        buffer.write_target_vec(&self.ctl_zs_first)?;
        buffer.write_target_ext_vec(&self.quotient_polys)?;
        Ok(())
    }

    /// Deserializes a STARK's opening set.
    pub(crate) fn from_buffer(buffer: &mut Buffer) -> IoResult<Self> {
        let local_values = buffer.read_target_ext_vec::<D>()?;
        let next_values = buffer.read_target_ext_vec::<D>()?;
        let auxiliary_polys = buffer.read_target_ext_vec::<D>()?;
        let auxiliary_polys_next = buffer.read_target_ext_vec::<D>()?;
        let ctl_zs_first = buffer.read_target_vec()?;
        let quotient_polys = buffer.read_target_ext_vec::<D>()?;

        Ok(Self {
            local_values,
            next_values,
            auxiliary_polys,
            auxiliary_polys_next,
            ctl_zs_first,
            quotient_polys,
        })
    }

    /// Circuit version of `to_fri_openings`for `FriOpenings`.
    /// Constructs the `Target`s the circuit version of FRI.
    /// All openings but `ctl_zs_first` are grouped together.
    pub(crate) fn to_fri_openings(&self, zero: Target) -> FriOpeningsTarget<D> {
        let zeta_batch = FriOpeningBatchTarget {
            values: self
                .local_values
                .iter()
                .chain(&self.auxiliary_polys)
                .chain(&self.quotient_polys)
                .copied()
                .collect_vec(),
        };
        let zeta_next_batch = FriOpeningBatchTarget {
            values: self
                .next_values
                .iter()
                .chain(&self.auxiliary_polys_next)
                .copied()
                .collect_vec(),
        };
        debug_assert!(!self.ctl_zs_first.is_empty());
        let ctl_first_batch = FriOpeningBatchTarget {
            values: self
                .ctl_zs_first
                .iter()
                .copied()
                .map(|t| t.to_ext_target(zero))
                .collect(),
        };

        FriOpeningsTarget {
            batches: vec![zeta_batch, zeta_next_batch, ctl_first_batch],
        }
    }
}
