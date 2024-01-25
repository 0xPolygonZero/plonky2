use std::collections::HashMap;
use std::ops::Deref;

use bytes::Bytes;
use eth_trie_utils::nibbles::Nibbles;
use eth_trie_utils::partial_trie::{HashedPartialTrie, PartialTrie};
use ethereum_types::{Address, BigEndianHash, H256, U256, U512};
use keccak_hash::keccak;
use rlp::{Decodable, DecoderError, Encodable, PayloadInfo, Rlp, RlpStream};
use rlp_derive::{RlpDecodable, RlpEncodable};

use crate::cpu::kernel::constants::global_metadata::GlobalMetadata;
use crate::cpu::kernel::constants::trie_type::PartialTrieType;
use crate::generation::TrieInputs;
use crate::memory::segments::Segment;
use crate::util::h2u;
use crate::witness::errors::{ProgramError, ProverInputError};
use crate::witness::memory::MemoryAddress;
use crate::Node;

#[derive(RlpEncodable, RlpDecodable, Debug)]
pub struct AccountRlp {
    pub nonce: U256,
    pub balance: U256,
    pub storage_root: H256,
    pub code_hash: H256,
}

#[derive(Clone, Debug)]
pub struct TrieRootPtrs {
    pub state_root_ptr: usize,
    pub txn_root_ptr: usize,
    pub receipt_root_ptr: usize,
}

impl Default for AccountRlp {
    fn default() -> Self {
        Self {
            nonce: U256::zero(),
            balance: U256::zero(),
            storage_root: HashedPartialTrie::from(Node::Empty).hash(),
            code_hash: keccak([]),
        }
    }
}

#[derive(RlpEncodable, RlpDecodable, Debug, Clone)]
pub struct LogRlp {
    pub address: Address,
    pub topics: Vec<H256>,
    pub data: Bytes,
}

#[derive(RlpEncodable, RlpDecodable, Debug, Clone)]
pub struct LegacyReceiptRlp {
    pub status: bool,
    pub cum_gas_used: U256,
    pub bloom: Bytes,
    pub logs: Vec<LogRlp>,
}

impl LegacyReceiptRlp {
    // RLP encode the receipt and prepend the tx type.
    pub fn encode(&self, tx_type: u8) -> Vec<u8> {
        let mut bytes = rlp::encode(self).to_vec();
        if tx_type != 0 {
            bytes.insert(0, tx_type);
        }
        bytes
    }
}

pub(crate) fn parse_receipts(rlp: &[u8]) -> Result<Vec<U256>, ProgramError> {
    let txn_type = match rlp.first().ok_or(ProgramError::InvalidRlp)? {
        1 => 1,
        2 => 2,
        _ => 0,
    };

    // If this is not a legacy transaction, we skip the leading byte.
    let rlp = if txn_type == 0 { rlp } else { &rlp[1..] };

    let payload_info = PayloadInfo::from(rlp).map_err(|_| ProgramError::InvalidRlp)?;
    let decoded_receipt: LegacyReceiptRlp =
        rlp::decode(rlp).map_err(|_| ProgramError::InvalidRlp)?;

    let mut parsed_receipt = if txn_type == 0 {
        Vec::new()
    } else {
        vec![txn_type.into()]
    };

    parsed_receipt.push(payload_info.value_len.into()); // payload_len of the entire receipt
    parsed_receipt.push((decoded_receipt.status as u8).into());
    parsed_receipt.push(decoded_receipt.cum_gas_used);
    parsed_receipt.extend(decoded_receipt.bloom.iter().map(|byte| U256::from(*byte)));
    let encoded_logs = rlp::encode_list(&decoded_receipt.logs);
    let logs_payload_info =
        PayloadInfo::from(&encoded_logs).map_err(|_| ProgramError::InvalidRlp)?;
    parsed_receipt.push(logs_payload_info.value_len.into()); // payload_len of all the logs
    parsed_receipt.push(decoded_receipt.logs.len().into());

    for log in decoded_receipt.logs {
        let encoded_log = rlp::encode(&log);
        let log_payload_info =
            PayloadInfo::from(&encoded_log).map_err(|_| ProgramError::InvalidRlp)?;
        parsed_receipt.push(log_payload_info.value_len.into()); // payload of one log
        parsed_receipt.push(U256::from_big_endian(&log.address.to_fixed_bytes()));
        parsed_receipt.push(log.topics.len().into());
        parsed_receipt.extend(log.topics.iter().map(|topic| U256::from(topic.as_bytes())));
        parsed_receipt.push(log.data.len().into());
        parsed_receipt.extend(log.data.iter().map(|byte| U256::from(*byte)));
    }

    Ok(parsed_receipt)
}

fn parse_storage_value(value_rlp: &[u8]) -> Result<Vec<U256>, ProgramError> {
    let value: U256 = rlp::decode(value_rlp).map_err(|_| ProgramError::InvalidRlp)?;
    Ok(vec![value])
}

const fn empty_nibbles() -> Nibbles {
    Nibbles {
        count: 0,
        packed: U512::zero(),
    }
}

fn load_mpt<F>(
    trie: &HashedPartialTrie,
    trie_data: &mut Vec<U256>,
    parse_value: &F,
) -> Result<usize, ProgramError>
where
    F: Fn(&[u8]) -> Result<Vec<U256>, ProgramError>,
{
    let node_ptr = trie_data.len();
    let type_of_trie = PartialTrieType::of(trie) as u32;
    if type_of_trie > 0 {
        trie_data.push(type_of_trie.into());
    }

    match trie.deref() {
        Node::Empty => Ok(0),
        Node::Hash(h) => {
            trie_data.push(h2u(*h));

            Ok(node_ptr)
        }
        Node::Branch { children, value } => {
            // First, set children pointers to 0.
            let first_child_ptr = trie_data.len();
            trie_data.extend(vec![U256::zero(); 16]);
            // Then, set value.
            if value.is_empty() {
                trie_data.push(U256::zero());
            } else {
                let parsed_value = parse_value(value)?;
                trie_data.push((trie_data.len() + 1).into());
                trie_data.extend(parsed_value);
            }

            // Now, load all children and update their pointers.
            for (i, child) in children.iter().enumerate() {
                let child_ptr = load_mpt(child, trie_data, parse_value)?;
                trie_data[first_child_ptr + i] = child_ptr.into();
            }

            Ok(node_ptr)
        }

        Node::Extension { nibbles, child } => {
            trie_data.push(nibbles.count.into());
            trie_data.push(
                nibbles
                    .try_into_u256()
                    .map_err(|_| ProgramError::IntegerTooLarge)?,
            );
            trie_data.push((trie_data.len() + 1).into());

            let child_ptr = load_mpt(child, trie_data, parse_value)?;
            if child_ptr == 0 {
                trie_data.push(0.into());
            }

            Ok(node_ptr)
        }
        Node::Leaf { nibbles, value } => {
            trie_data.push(nibbles.count.into());
            trie_data.push(
                nibbles
                    .try_into_u256()
                    .map_err(|_| ProgramError::IntegerTooLarge)?,
            );

            // Set `value_ptr_ptr`.
            trie_data.push((trie_data.len() + 1).into());

            let leaf = parse_value(value)?;
            trie_data.extend(leaf);

            Ok(node_ptr)
        }
    }
}

fn load_state_trie(
    trie: &HashedPartialTrie,
    key: Nibbles,
    trie_data: &mut Vec<U256>,
    storage_tries_by_state_key: &HashMap<Nibbles, &HashedPartialTrie>,
) -> Result<usize, ProgramError> {
    let node_ptr = trie_data.len();
    let type_of_trie = PartialTrieType::of(trie) as u32;
    if type_of_trie > 0 {
        trie_data.push(type_of_trie.into());
    }
    match trie.deref() {
        Node::Empty => Ok(0),
        Node::Hash(h) => {
            trie_data.push(h2u(*h));

            Ok(node_ptr)
        }
        Node::Branch { children, value } => {
            if !value.is_empty() {
                return Err(ProgramError::ProverInputError(
                    ProverInputError::InvalidMptInput,
                ));
            }
            // First, set children pointers to 0.
            let first_child_ptr = trie_data.len();
            trie_data.extend(vec![U256::zero(); 16]);
            // Then, set value pointer to 0.
            trie_data.push(U256::zero());

            // Now, load all children and update their pointers.
            for (i, child) in children.iter().enumerate() {
                let extended_key = key.merge_nibbles(&Nibbles {
                    count: 1,
                    packed: i.into(),
                });
                let child_ptr =
                    load_state_trie(child, extended_key, trie_data, storage_tries_by_state_key)?;

                trie_data[first_child_ptr + i] = child_ptr.into();
            }

            Ok(node_ptr)
        }
        Node::Extension { nibbles, child } => {
            trie_data.push(nibbles.count.into());
            trie_data.push(
                nibbles
                    .try_into_u256()
                    .map_err(|_| ProgramError::IntegerTooLarge)?,
            );
            // Set `value_ptr_ptr`.
            trie_data.push((trie_data.len() + 1).into());
            let extended_key = key.merge_nibbles(nibbles);
            let child_ptr =
                load_state_trie(child, extended_key, trie_data, storage_tries_by_state_key)?;
            if child_ptr == 0 {
                trie_data.push(0.into());
            }

            Ok(node_ptr)
        }
        Node::Leaf { nibbles, value } => {
            let account: AccountRlp = rlp::decode(value).map_err(|_| ProgramError::InvalidRlp)?;
            let AccountRlp {
                nonce,
                balance,
                storage_root,
                code_hash,
            } = account;

            let storage_hash_only = HashedPartialTrie::new(Node::Hash(storage_root));
            let merged_key = key.merge_nibbles(nibbles);
            let storage_trie: &HashedPartialTrie = storage_tries_by_state_key
                .get(&merged_key)
                .copied()
                .unwrap_or(&storage_hash_only);

            assert_eq!(storage_trie.hash(), storage_root,
                "In TrieInputs, an account's storage_root didn't match the associated storage trie hash");

            trie_data.push(nibbles.count.into());
            trie_data.push(
                nibbles
                    .try_into_u256()
                    .map_err(|_| ProgramError::IntegerTooLarge)?,
            );
            // Set `value_ptr_ptr`.
            trie_data.push((trie_data.len() + 1).into());

            trie_data.push(nonce);
            trie_data.push(balance);
            // Storage trie ptr.
            let storage_ptr_ptr = trie_data.len();
            trie_data.push((trie_data.len() + 2).into());
            trie_data.push(code_hash.into_uint());
            let storage_ptr = load_mpt(storage_trie, trie_data, &parse_storage_value)?;
            if storage_ptr == 0 {
                trie_data[storage_ptr_ptr] = 0.into();
            }

            Ok(node_ptr)
        }
    }
}

pub(crate) fn load_all_mpts(
    trie_inputs: &TrieInputs,
) -> Result<(TrieRootPtrs, Vec<U256>), ProgramError> {
    let mut trie_data = vec![U256::zero()];
    let storage_tries_by_state_key = trie_inputs
        .storage_tries
        .iter()
        .map(|(hashed_address, storage_trie)| {
            let key = Nibbles::from_bytes_be(hashed_address.as_bytes())
                .expect("An H256 is 32 bytes long");
            (key, storage_trie)
        })
        .collect();

    let state_root_ptr = load_state_trie(
        &trie_inputs.state_trie,
        empty_nibbles(),
        &mut trie_data,
        &storage_tries_by_state_key,
    )?;

    let txn_root_ptr = load_mpt(&trie_inputs.transactions_trie, &mut trie_data, &|rlp| {
        let mut parsed_txn = vec![U256::from(rlp.len())];
        parsed_txn.extend(rlp.iter().copied().map(U256::from));
        Ok(parsed_txn)
    })?;

    let receipt_root_ptr = load_mpt(&trie_inputs.receipts_trie, &mut trie_data, &parse_receipts)?;

    let trie_root_ptrs = TrieRootPtrs {
        state_root_ptr,
        txn_root_ptr,
        receipt_root_ptr,
    };

    Ok((trie_root_ptrs, trie_data))
}

pub mod transaction_testing {
    use super::*;

    #[derive(RlpEncodable, RlpDecodable, Debug, Clone, PartialEq, Eq)]
    pub struct AccessListItemRlp {
        pub address: Address,
        pub storage_keys: Vec<U256>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct AddressOption(pub Option<Address>);

    impl Encodable for AddressOption {
        fn rlp_append(&self, s: &mut RlpStream) {
            match self.0 {
                None => s.encoder().encode_value(&[]),
                Some(value) => {
                    s.encoder().encode_value(&value.to_fixed_bytes());
                }
            }
        }
    }

    impl Decodable for AddressOption {
        fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
            if rlp.is_int() && rlp.is_empty() {
                return Ok(AddressOption(None));
            }
            if rlp.is_data() && rlp.size() == 20 {
                return Ok(AddressOption(Some(Address::decode(rlp)?)));
            }
            Err(DecoderError::RlpExpectedToBeData)
        }
    }

    #[derive(RlpEncodable, RlpDecodable, Debug, Clone, PartialEq, Eq)]
    pub struct LegacyTransactionRlp {
        pub nonce: U256,
        pub gas_price: U256,
        pub gas: U256,
        pub to: AddressOption,
        pub value: U256,
        pub data: Bytes,
        pub v: U256,
        pub r: U256,
        pub s: U256,
    }

    #[derive(RlpEncodable, RlpDecodable, Debug, Clone, PartialEq, Eq)]
    pub struct AccessListTransactionRlp {
        pub chain_id: u64,
        pub nonce: U256,
        pub gas_price: U256,
        pub gas: U256,
        pub to: AddressOption,
        pub value: U256,
        pub data: Bytes,
        pub access_list: Vec<AccessListItemRlp>,
        pub y_parity: U256,
        pub r: U256,
        pub s: U256,
    }

    #[derive(RlpEncodable, RlpDecodable, Debug, Clone, PartialEq, Eq)]
    pub struct FeeMarketTransactionRlp {
        pub chain_id: u64,
        pub nonce: U256,
        pub max_priority_fee_per_gas: U256,
        pub max_fee_per_gas: U256,
        pub gas: U256,
        pub to: AddressOption,
        pub value: U256,
        pub data: Bytes,
        pub access_list: Vec<AccessListItemRlp>,
        pub y_parity: U256,
        pub r: U256,
        pub s: U256,
    }
}
