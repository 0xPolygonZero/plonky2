use std::collections::HashMap;

use ethereum_types::U256;
use hex_literal::hex;

use crate::cpu::decode::invalid_opcodes_user;
use crate::cpu::kernel::constants::context_metadata::ContextMetadata;
use crate::cpu::kernel::constants::global_metadata::GlobalMetadata;
use crate::cpu::kernel::constants::trie_type::PartialTrieType;
use crate::cpu::kernel::constants::txn_fields::NormalizedTxnField;
use crate::memory::segments::Segment;

pub(crate) mod context_metadata;
pub(crate) mod global_metadata;
pub(crate) mod trie_type;
pub(crate) mod txn_fields;

/// Constants that are accessible to our kernel assembly code.
pub fn evm_constants() -> HashMap<String, U256> {
    let mut c = HashMap::new();
    for (name, value) in EC_CONSTANTS {
        c.insert(name.into(), U256::from_big_endian(&value));
    }
    for (name, value) in HASH_CONSTANTS {
        c.insert(name.into(), U256::from_big_endian(&value));
    }
    for (name, value) in GAS_CONSTANTS {
        c.insert(name.into(), U256::from(value));
    }
    for segment in Segment::all() {
        c.insert(segment.var_name().into(), (segment as u32).into());
    }
    for txn_field in NormalizedTxnField::all() {
        c.insert(txn_field.var_name().into(), (txn_field as u32).into());
    }
    for txn_field in GlobalMetadata::all() {
        c.insert(txn_field.var_name().into(), (txn_field as u32).into());
    }
    for txn_field in ContextMetadata::all() {
        c.insert(txn_field.var_name().into(), (txn_field as u32).into());
    }
    for trie_type in PartialTrieType::all() {
        c.insert(trie_type.var_name().into(), (trie_type as u32).into());
    }
    c.insert(
        "INVALID_OPCODES_USER".into(),
        U256::from_little_endian(&invalid_opcodes_user()),
    );
    c
}

const HASH_CONSTANTS: [(&str, [u8; 32]); 1] = [
    // Hash of an empty node: keccak(rlp.encode(b'')).hex()
    (
        "EMPTY_NODE_HASH",
        hex!("56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421"),
    ),
];

const EC_CONSTANTS: [(&str, [u8; 32]); 3] = [
    (
        "BN_BASE",
        hex!("30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47"),
    ),
    (
        "SECP_BASE",
        hex!("fffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f"),
    ),
    (
        "SECP_SCALAR",
        hex!("fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141"),
    ),
];

const GAS_CONSTANTS: [(&str, u16); 36] = [
    ("GAS_ZERO", 0),
    ("GAS_JUMPDEST", 1),
    ("GAS_BASE", 2),
    ("GAS_VERYLOW", 3),
    ("GAS_LOW", 5),
    ("GAS_MID", 8),
    ("GAS_HIGH", 10),
    ("GAS_WARMACCESS", 100),
    ("GAS_ACCESSLISTADDRESS", 2_400),
    ("GAS_ACCESSLISTSTORAGE", 1_900),
    ("GAS_COLDACCOUNTACCESS", 2_600),
    ("GAS_COLDSLOAD", 2_100),
    ("GAS_SSET", 20_000),
    ("GAS_SRESET", 2_900),
    ("REFUND_SCLEAR", 15_000),
    ("REFUND_SELFDESTRUCT", 24_000),
    ("GAS_SELFDESTRUCT", 5_000),
    ("GAS_CREATE", 32_000),
    ("GAS_CODEDEPOSIT", 200),
    ("GAS_CALLVALUE", 9_000),
    ("GAS_CALLSTIPEND", 2_300),
    ("GAS_NEWACCOUNT", 25_000),
    ("GAS_EXP", 10),
    ("GAS_EXPBYTE", 50),
    ("GAS_MEMORY", 3),
    ("GAS_TXCREATE", 32_000),
    ("GAS_TXDATAZERO", 4),
    ("GAS_TXDATANONZERO", 16),
    ("GAS_TRANSACTION", 21_000),
    ("GAS_LOG", 375),
    ("GAS_LOGDATA", 8),
    ("GAS_LOGTOPIC", 375),
    ("GAS_KECCAK256", 30),
    ("GAS_KECCAK256WORD", 6),
    ("GAS_COPY", 3),
    ("GAS_BLOCKHASH", 20),
];
