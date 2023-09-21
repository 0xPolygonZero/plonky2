use ethereum_types::{Address, H256, U256};
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

/// A STARK proof for each table, plus some metadata used to create recursive wrapper proofs.
#[derive(Debug, Clone)]
pub struct AllProof<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize> {
    pub stark_proofs: [StarkProofWithMetadata<F, C, D>; NUM_TABLES],
    pub(crate) ctl_challenges: GrandProductChallengeSet<F>,
    pub public_values: PublicValues,
}

impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize> AllProof<F, C, D> {
    pub fn degree_bits(&self, config: &StarkConfig) -> [usize; NUM_TABLES] {
        core::array::from_fn(|i| self.stark_proofs[i].proof.recover_degree_bits(config))
    }
}

pub(crate) struct AllProofChallenges<F: RichField + Extendable<D>, const D: usize> {
    pub stark_challenges: [StarkProofChallenges<F, D>; NUM_TABLES],
    pub ctl_challenges: GrandProductChallengeSet<F>,
}

/// Memory values which are public.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PublicValues {
    pub trie_roots_before: TrieRoots,
    pub trie_roots_after: TrieRoots,
    pub block_metadata: BlockMetadata,
    pub block_hashes: BlockHashes,
    pub extra_block_data: ExtraBlockData,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TrieRoots {
    pub state_root: H256,
    pub transactions_root: H256,
    pub receipts_root: H256,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHashes {
    /// The previous 256 hashes to the current block. The leftmost hash, i.e. `prev_hashes[0]`,
    /// is the oldest, and the rightmost, i.e. `prev_hashes[255]` is the hash of the parent block.
    pub prev_hashes: Vec<H256>,
    // The hash of the current block.
    pub cur_hash: H256,
}

// TODO: Before going into production, `block_gas_used` and `block_gaslimit` here
// as well as `gas_used_before` / `gas_used_after` in `ExtraBlockData` should be
// updated to fit in a single 32-bit limb, as supporting 64-bit values for those
// fields is only necessary for testing purposes.
/// Metadata contained in a block header. Those are identical between
/// all state transition proofs within the same block.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct BlockMetadata {
    /// The address of this block's producer.
    pub block_beneficiary: Address,
    /// The timestamp of this block. It must fit in a `u32`.
    pub block_timestamp: U256,
    /// The index of this block. It must fit in a `u32`.
    pub block_number: U256,
    /// The difficulty (before PoS transition) of this block.
    pub block_difficulty: U256,
    /// The `mix_hash` value of this block.
    pub block_random: H256,
    /// The gas limit of this block. It must fit in a `u64`.
    pub block_gaslimit: U256,
    /// The chain id of this block. It must fit in a `u32`.
    pub block_chain_id: U256,
    /// The base fee of this block. It must fit in a `u64`.
    pub block_base_fee: U256,
    /// The total gas used in this block. It must fit in a `u64`.
    pub block_gas_used: U256,
    /// The block bloom of this block, represented as the consecutive
    /// 32-byte chunks of a block's final bloom filter string.
    pub block_bloom: [U256; 8],
}

/// Additional block data that are specific to the local transaction being proven,
/// unlike `BlockMetadata`.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ExtraBlockData {
    /// The state trie digest of the genesis block.
    pub genesis_state_trie_root: H256,
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
    /// The accumulated bloom filter of this block prior execution of the local state transition,
    /// starting with all zeros for the initial transaction of a block.
    pub block_bloom_before: [U256; 8],
    /// The accumulated bloom filter after execution of the local state transition. It should
    /// match the `block_bloom` value after execution of the last transaction in a block.
    pub block_bloom_after: [U256; 8],
}

/// Memory values which are public.
/// Note: All the larger integers are encoded with 32-bit limbs in little-endian order.
#[derive(Eq, PartialEq, Debug)]
pub struct PublicValuesTarget {
    pub trie_roots_before: TrieRootsTarget,
    pub trie_roots_after: TrieRootsTarget,
    pub block_metadata: BlockMetadataTarget,
    pub block_hashes: BlockHashesTarget,
    pub extra_block_data: ExtraBlockDataTarget,
}

impl PublicValuesTarget {
    pub fn to_buffer(&self, buffer: &mut Vec<u8>) -> IoResult<()> {
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
        buffer.write_target_array(&block_gaslimit)?;
        buffer.write_target(block_chain_id)?;
        buffer.write_target_array(&block_base_fee)?;
        buffer.write_target_array(&block_gas_used)?;
        buffer.write_target_array(&block_bloom)?;

        let BlockHashesTarget {
            prev_hashes,
            cur_hash,
        } = self.block_hashes;
        buffer.write_target_array(&prev_hashes)?;
        buffer.write_target_array(&cur_hash)?;

        let ExtraBlockDataTarget {
            genesis_state_trie_root: genesis_state_root,
            txn_number_before,
            txn_number_after,
            gas_used_before,
            gas_used_after,
            block_bloom_before,
            block_bloom_after,
        } = self.extra_block_data;
        buffer.write_target_array(&genesis_state_root)?;
        buffer.write_target(txn_number_before)?;
        buffer.write_target(txn_number_after)?;
        buffer.write_target_array(&gas_used_before)?;
        buffer.write_target_array(&gas_used_after)?;
        buffer.write_target_array(&block_bloom_before)?;
        buffer.write_target_array(&block_bloom_after)?;

        Ok(())
    }

    pub fn from_buffer(buffer: &mut Buffer) -> IoResult<Self> {
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
            block_gaslimit: buffer.read_target_array()?,
            block_chain_id: buffer.read_target()?,
            block_base_fee: buffer.read_target_array()?,
            block_gas_used: buffer.read_target_array()?,
            block_bloom: buffer.read_target_array()?,
        };

        let block_hashes = BlockHashesTarget {
            prev_hashes: buffer.read_target_array()?,
            cur_hash: buffer.read_target_array()?,
        };

        let extra_block_data = ExtraBlockDataTarget {
            genesis_state_trie_root: buffer.read_target_array()?,
            txn_number_before: buffer.read_target()?,
            txn_number_after: buffer.read_target()?,
            gas_used_before: buffer.read_target_array()?,
            gas_used_after: buffer.read_target_array()?,
            block_bloom_before: buffer.read_target_array()?,
            block_bloom_after: buffer.read_target_array()?,
        };

        Ok(Self {
            trie_roots_before,
            trie_roots_after,
            block_metadata,
            block_hashes,
            extra_block_data,
        })
    }

    pub fn from_public_inputs(pis: &[Target]) -> Self {
        assert!(
            pis.len()
                > TrieRootsTarget::SIZE * 2
                    + BlockMetadataTarget::SIZE
                    + BlockHashesTarget::BLOCK_HASHES_SIZE
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
                        + BlockHashesTarget::BLOCK_HASHES_SIZE],
            ),
            extra_block_data: ExtraBlockDataTarget::from_public_inputs(
                &pis[TrieRootsTarget::SIZE * 2
                    + BlockMetadataTarget::SIZE
                    + BlockHashesTarget::BLOCK_HASHES_SIZE
                    ..TrieRootsTarget::SIZE * 2
                        + BlockMetadataTarget::SIZE
                        + BlockHashesTarget::BLOCK_HASHES_SIZE
                        + ExtraBlockDataTarget::SIZE],
            ),
        }
    }

    pub fn select<F: RichField + Extendable<D>, const D: usize>(
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

#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub struct TrieRootsTarget {
    pub state_root: [Target; 8],
    pub transactions_root: [Target; 8],
    pub receipts_root: [Target; 8],
}

impl TrieRootsTarget {
    pub const SIZE: usize = 24;

    pub fn from_public_inputs(pis: &[Target]) -> Self {
        let state_root = pis[0..8].try_into().unwrap();
        let transactions_root = pis[8..16].try_into().unwrap();
        let receipts_root = pis[16..24].try_into().unwrap();

        Self {
            state_root,
            transactions_root,
            receipts_root,
        }
    }

    pub fn select<F: RichField + Extendable<D>, const D: usize>(
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

    pub fn connect<F: RichField + Extendable<D>, const D: usize>(
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

#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub struct BlockMetadataTarget {
    pub block_beneficiary: [Target; 5],
    pub block_timestamp: Target,
    pub block_number: Target,
    pub block_difficulty: Target,
    pub block_random: [Target; 8],
    pub block_gaslimit: [Target; 2],
    pub block_chain_id: Target,
    pub block_base_fee: [Target; 2],
    pub block_gas_used: [Target; 2],
    pub block_bloom: [Target; 64],
}

impl BlockMetadataTarget {
    pub const SIZE: usize = 87;

    pub fn from_public_inputs(pis: &[Target]) -> Self {
        let block_beneficiary = pis[0..5].try_into().unwrap();
        let block_timestamp = pis[5];
        let block_number = pis[6];
        let block_difficulty = pis[7];
        let block_random = pis[8..16].try_into().unwrap();
        let block_gaslimit = pis[16..18].try_into().unwrap();
        let block_chain_id = pis[18];
        let block_base_fee = pis[19..21].try_into().unwrap();
        let block_gas_used = pis[21..23].try_into().unwrap();
        let block_bloom = pis[23..87].try_into().unwrap();

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

    pub fn select<F: RichField + Extendable<D>, const D: usize>(
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
            block_gaslimit: core::array::from_fn(|i| {
                builder.select(condition, bm0.block_gaslimit[i], bm1.block_gaslimit[i])
            }),
            block_chain_id: builder.select(condition, bm0.block_chain_id, bm1.block_chain_id),
            block_base_fee: core::array::from_fn(|i| {
                builder.select(condition, bm0.block_base_fee[i], bm1.block_base_fee[i])
            }),
            block_gas_used: core::array::from_fn(|i| {
                builder.select(condition, bm0.block_gas_used[i], bm1.block_gas_used[i])
            }),
            block_bloom: core::array::from_fn(|i| {
                builder.select(condition, bm0.block_bloom[i], bm1.block_bloom[i])
            }),
        }
    }

    pub fn connect<F: RichField + Extendable<D>, const D: usize>(
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
        for i in 0..2 {
            builder.connect(bm0.block_gaslimit[i], bm1.block_gaslimit[i])
        }
        builder.connect(bm0.block_chain_id, bm1.block_chain_id);
        for i in 0..2 {
            builder.connect(bm0.block_base_fee[i], bm1.block_base_fee[i])
        }
        for i in 0..2 {
            builder.connect(bm0.block_gas_used[i], bm1.block_gas_used[i])
        }
        for i in 0..64 {
            builder.connect(bm0.block_bloom[i], bm1.block_bloom[i])
        }
    }
}

#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub struct BlockHashesTarget {
    pub prev_hashes: [Target; 2048],
    pub cur_hash: [Target; 8],
}

impl BlockHashesTarget {
    pub const BLOCK_HASHES_SIZE: usize = 2056;
    pub fn from_public_inputs(pis: &[Target]) -> Self {
        Self {
            prev_hashes: pis[0..2048].try_into().unwrap(),
            cur_hash: pis[2048..2056].try_into().unwrap(),
        }
    }

    pub fn select<F: RichField + Extendable<D>, const D: usize>(
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

    pub fn connect<F: RichField + Extendable<D>, const D: usize>(
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

#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub struct ExtraBlockDataTarget {
    pub genesis_state_trie_root: [Target; 8],
    pub txn_number_before: Target,
    pub txn_number_after: Target,
    pub gas_used_before: [Target; 2],
    pub gas_used_after: [Target; 2],
    pub block_bloom_before: [Target; 64],
    pub block_bloom_after: [Target; 64],
}

impl ExtraBlockDataTarget {
    const SIZE: usize = 142;

    pub fn from_public_inputs(pis: &[Target]) -> Self {
        let genesis_state_trie_root = pis[0..8].try_into().unwrap();
        let txn_number_before = pis[8];
        let txn_number_after = pis[9];
        let gas_used_before = pis[10..12].try_into().unwrap();
        let gas_used_after = pis[12..14].try_into().unwrap();
        let block_bloom_before = pis[14..78].try_into().unwrap();
        let block_bloom_after = pis[78..142].try_into().unwrap();

        Self {
            genesis_state_trie_root,
            txn_number_before,
            txn_number_after,
            gas_used_before,
            gas_used_after,
            block_bloom_before,
            block_bloom_after,
        }
    }

    pub fn select<F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
        condition: BoolTarget,
        ed0: Self,
        ed1: Self,
    ) -> Self {
        Self {
            genesis_state_trie_root: core::array::from_fn(|i| {
                builder.select(
                    condition,
                    ed0.genesis_state_trie_root[i],
                    ed1.genesis_state_trie_root[i],
                )
            }),
            txn_number_before: builder.select(
                condition,
                ed0.txn_number_before,
                ed1.txn_number_before,
            ),
            txn_number_after: builder.select(condition, ed0.txn_number_after, ed1.txn_number_after),
            gas_used_before: core::array::from_fn(|i| {
                builder.select(condition, ed0.gas_used_before[i], ed1.gas_used_before[i])
            }),
            gas_used_after: core::array::from_fn(|i| {
                builder.select(condition, ed0.gas_used_after[i], ed1.gas_used_after[i])
            }),
            block_bloom_before: core::array::from_fn(|i| {
                builder.select(
                    condition,
                    ed0.block_bloom_before[i],
                    ed1.block_bloom_before[i],
                )
            }),
            block_bloom_after: core::array::from_fn(|i| {
                builder.select(
                    condition,
                    ed0.block_bloom_after[i],
                    ed1.block_bloom_after[i],
                )
            }),
        }
    }

    pub fn connect<F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
        ed0: Self,
        ed1: Self,
    ) {
        for i in 0..8 {
            builder.connect(
                ed0.genesis_state_trie_root[i],
                ed1.genesis_state_trie_root[i],
            );
        }
        builder.connect(ed0.txn_number_before, ed1.txn_number_before);
        builder.connect(ed0.txn_number_after, ed1.txn_number_after);
        for i in 0..2 {
            builder.connect(ed0.gas_used_before[i], ed1.gas_used_before[i]);
        }
        for i in 0..2 {
            builder.connect(ed1.gas_used_after[i], ed1.gas_used_after[i]);
        }
        for i in 0..64 {
            builder.connect(ed0.block_bloom_before[i], ed1.block_bloom_before[i]);
        }
        for i in 0..64 {
            builder.connect(ed0.block_bloom_after[i], ed1.block_bloom_after[i]);
        }
    }
}

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
    pub(crate) init_challenger_state: <C::Hasher as Hasher<F>>::Permutation,
    // TODO: set it back to pub(crate) when cpu trace len is a public input
    pub proof: StarkProof<F, C, D>,
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

    pub fn num_ctl_zs(&self) -> usize {
        self.openings.ctl_zs_first.len()
    }
}

#[derive(Eq, PartialEq, Debug)]
pub struct StarkProofTarget<const D: usize> {
    pub trace_cap: MerkleCapTarget,
    pub auxiliary_polys_cap: MerkleCapTarget,
    pub quotient_polys_cap: MerkleCapTarget,
    pub openings: StarkOpeningSetTarget<D>,
    pub opening_proof: FriProofTarget<D>,
}

impl<const D: usize> StarkProofTarget<D> {
    pub fn to_buffer(&self, buffer: &mut Vec<u8>) -> IoResult<()> {
        buffer.write_target_merkle_cap(&self.trace_cap)?;
        buffer.write_target_merkle_cap(&self.auxiliary_polys_cap)?;
        buffer.write_target_merkle_cap(&self.quotient_polys_cap)?;
        buffer.write_target_fri_proof(&self.opening_proof)?;
        self.openings.to_buffer(buffer)?;
        Ok(())
    }

    pub fn from_buffer(buffer: &mut Buffer) -> IoResult<Self> {
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
    pub fn recover_degree_bits(&self, config: &StarkConfig) -> usize {
        let initial_merkle_proof = &self.opening_proof.query_round_proofs[0]
            .initial_trees_proof
            .evals_proofs[0]
            .1;
        let lde_bits = config.fri_config.cap_height + initial_merkle_proof.siblings.len();
        lde_bits - config.fri_config.rate_bits
    }
}

pub(crate) struct StarkProofChallenges<F: RichField + Extendable<D>, const D: usize> {
    /// Random values used to combine STARK constraints.
    pub stark_alphas: Vec<F>,

    /// Point at which the STARK polynomials are opened.
    pub stark_zeta: F::Extension,

    pub fri_challenges: FriChallenges<F, D>,
}

pub(crate) struct StarkProofChallengesTarget<const D: usize> {
    pub stark_alphas: Vec<Target>,
    pub stark_zeta: ExtensionTarget<D>,
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
    pub fn new<C: GenericConfig<D, F = F>>(
        zeta: F::Extension,
        g: F,
        trace_commitment: &PolynomialBatch<F, C, D>,
        auxiliary_polys_commitment: &PolynomialBatch<F, C, D>,
        quotient_commitment: &PolynomialBatch<F, C, D>,
        num_lookup_columns: usize,
    ) -> Self {
        let eval_commitment = |z: F::Extension, c: &PolynomialBatch<F, C, D>| {
            c.polynomials
                .par_iter()
                .map(|p| p.to_extension().eval(z))
                .collect::<Vec<_>>()
        };
        let eval_commitment_base = |z: F, c: &PolynomialBatch<F, C, D>| {
            c.polynomials
                .par_iter()
                .map(|p| p.eval(z))
                .collect::<Vec<_>>()
        };
        let zeta_next = zeta.scalar_mul(g);
        Self {
            local_values: eval_commitment(zeta, trace_commitment),
            next_values: eval_commitment(zeta_next, trace_commitment),
            auxiliary_polys: eval_commitment(zeta, auxiliary_polys_commitment),
            auxiliary_polys_next: eval_commitment(zeta_next, auxiliary_polys_commitment),
            ctl_zs_first: eval_commitment_base(F::ONE, auxiliary_polys_commitment)
                [num_lookup_columns..]
                .to_vec(),
            quotient_polys: eval_commitment(zeta, quotient_commitment),
        }
    }

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

#[derive(Eq, PartialEq, Debug)]
pub struct StarkOpeningSetTarget<const D: usize> {
    pub local_values: Vec<ExtensionTarget<D>>,
    pub next_values: Vec<ExtensionTarget<D>>,
    pub auxiliary_polys: Vec<ExtensionTarget<D>>,
    pub auxiliary_polys_next: Vec<ExtensionTarget<D>>,
    pub ctl_zs_first: Vec<Target>,
    pub quotient_polys: Vec<ExtensionTarget<D>>,
}

impl<const D: usize> StarkOpeningSetTarget<D> {
    pub fn to_buffer(&self, buffer: &mut Vec<u8>) -> IoResult<()> {
        buffer.write_target_ext_vec(&self.local_values)?;
        buffer.write_target_ext_vec(&self.next_values)?;
        buffer.write_target_ext_vec(&self.auxiliary_polys)?;
        buffer.write_target_ext_vec(&self.auxiliary_polys_next)?;
        buffer.write_target_vec(&self.ctl_zs_first)?;
        buffer.write_target_ext_vec(&self.quotient_polys)?;
        Ok(())
    }

    pub fn from_buffer(buffer: &mut Buffer) -> IoResult<Self> {
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
