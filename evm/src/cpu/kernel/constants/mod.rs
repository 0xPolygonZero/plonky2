use std::collections::HashMap;

use ethereum_types::U256;
use hex_literal::hex;
use static_assertions::const_assert;

use crate::cpu::kernel::constants::context_metadata::ContextMetadata;
use crate::cpu::kernel::constants::global_metadata::GlobalMetadata;
use crate::cpu::kernel::constants::journal_entry::JournalEntry;
use crate::cpu::kernel::constants::trie_type::PartialTrieType;
use crate::cpu::kernel::constants::txn_fields::NormalizedTxnField;
use crate::memory::segments::Segment;

pub(crate) mod context_metadata;
mod exc_bitfields;
pub(crate) mod global_metadata;
pub(crate) mod journal_entry;
pub(crate) mod trie_type;
pub(crate) mod txn_fields;

/// Constants that are accessible to our kernel assembly code.
pub(crate) fn evm_constants() -> HashMap<String, U256> {
    let mut c = HashMap::new();

    let hex_constants = MISC_CONSTANTS
        .iter()
        .chain(EC_CONSTANTS.iter())
        .chain(HASH_CONSTANTS.iter())
        .cloned();
    for (name, value) in hex_constants {
        c.insert(name.into(), U256::from_big_endian(&value));
    }

    for (name, value) in GAS_CONSTANTS {
        c.insert(name.into(), U256::from(value));
    }

    for (name, value) in REFUND_CONSTANTS {
        c.insert(name.into(), U256::from(value));
    }

    for (name, value) in PRECOMPILES {
        c.insert(name.into(), U256::from(value));
    }

    for (name, value) in PRECOMPILES_GAS {
        c.insert(name.into(), U256::from(value));
    }

    for (name, value) in CODE_SIZE_LIMIT {
        c.insert(name.into(), U256::from(value));
    }

    for (name, value) in SNARKV_POINTERS {
        c.insert(name.into(), U256::from(value));
    }

    c.insert(MAX_NONCE.0.into(), U256::from(MAX_NONCE.1));
    c.insert(CALL_STACK_LIMIT.0.into(), U256::from(CALL_STACK_LIMIT.1));

    for segment in Segment::all() {
        c.insert(segment.var_name().into(), (segment as usize).into());
    }
    for txn_field in NormalizedTxnField::all() {
        // These offsets are already scaled by their respective segment.
        c.insert(txn_field.var_name().into(), (txn_field as usize).into());
    }
    for txn_field in GlobalMetadata::all() {
        // These offsets are already scaled by their respective segment.
        c.insert(txn_field.var_name().into(), (txn_field as usize).into());
    }
    for txn_field in ContextMetadata::all() {
        // These offsets are already scaled by their respective segment.
        c.insert(txn_field.var_name().into(), (txn_field as usize).into());
    }
    for trie_type in PartialTrieType::all() {
        c.insert(trie_type.var_name().into(), (trie_type as u32).into());
    }
    for entry in JournalEntry::all() {
        c.insert(entry.var_name().into(), (entry as u32).into());
    }
    c.insert(
        "INVALID_OPCODES_USER".into(),
        exc_bitfields::INVALID_OPCODES_USER,
    );
    c.insert(
        "STACK_LENGTH_INCREASING_OPCODES_USER".into(),
        exc_bitfields::STACK_LENGTH_INCREASING_OPCODES_USER,
    );
    c
}

const MISC_CONSTANTS: [(&str, [u8; 32]); 3] = [
    // Base for limbs used in bignum arithmetic.
    (
        "BIGNUM_LIMB_BASE",
        hex!("0000000000000000000000000000000100000000000000000000000000000000"),
    ),
    // Position in SEGMENT_RLP_RAW where the empty node encoding is stored. It is
    // equal to u32::MAX + @SEGMENT_RLP_RAW so that all rlp pointers are much smaller than that.
    (
        "ENCODED_EMPTY_NODE_POS",
        hex!("0000000000000000000000000000000000000000000000000000000CFFFFFFFF"),
    ),
    // 0x10000 = 2^16 bytes, much larger than any RLP blob the EVM could possibly create.
    (
        "MAX_RLP_BLOB_SIZE",
        hex!("0000000000000000000000000000000000000000000000000000000000010000"),
    ),
];

const HASH_CONSTANTS: [(&str, [u8; 32]); 2] = [
    // Hash of an empty string: keccak(b'').hex()
    (
        "EMPTY_STRING_HASH",
        hex!("c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"),
    ),
    // Hash of an empty node: keccak(rlp.encode(b'')).hex()
    (
        "EMPTY_NODE_HASH",
        hex!("56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421"),
    ),
];

const EC_CONSTANTS: [(&str, [u8; 32]); 20] = [
    (
        "U256_MAX",
        hex!("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"),
    ),
    (
        "BN_BASE",
        hex!("30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47"),
    ),
    (
        "BN_TWISTED_RE",
        hex!("2b149d40ceb8aaae81be18991be06ac3b5b4c5e559dbefa33267e6dc24a138e5"),
    ),
    (
        "BN_TWISTED_IM",
        hex!("009713b03af0fed4cd2cafadeed8fdf4a74fa084e52d1852e4a2bd0685c315d2"),
    ),
    (
        "BN_SCALAR",
        hex!("30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001"),
    ),
    (
        "BN_GLV_BETA",
        hex!("000000000000000059e26bcea0d48bacd4f263f1acdb5c4f5763473177fffffe"),
    ),
    (
        "BN_GLV_S",
        hex!("0000000000000000b3c4d79d41a917585bfc41088d8daaa78b17ea66b99c90dd"),
    ),
    (
        "BN_GLV_MINUS_G1",
        hex!("000000000000000000000000000000024ccef014a773d2cf7a7bd9d4391eb18d"),
    ),
    (
        "BN_GLV_G2",
        hex!("000000000000000000000000000000000000000000000002d91d232ec7e0b3d7"),
    ),
    (
        "BN_GLV_B1",
        hex!("30644e72e131a029b85045b68181585cb8e665ff8b011694c1d039a872b0eed9"),
    ),
    (
        "BN_GLV_B2",
        hex!("00000000000000000000000000000000000000000000000089d3256894d213e3"),
    ),
    (
        "BN_BNEG_LOC",
        // This just needs to be large enough to not interfere with anything else in SEGMENT_BN_TABLE_Q.
        hex!("0000000000000000000000000000000000000000000000000000000000001337"),
    ),
    (
        "SECP_BASE",
        hex!("fffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f"),
    ),
    (
        "SECP_SCALAR",
        hex!("fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141"),
    ),
    (
        "SECP_GLV_BETA",
        hex!("7ae96a2b657c07106e64479eac3434e99cf0497512f58995c1396c28719501ee"),
    ),
    (
        "SECP_GLV_S",
        hex!("5363ad4cc05c30e0a5261c028812645a122e22ea20816678df02967c1b23bd72"),
    ),
    (
        "SECP_GLV_MINUS_G1",
        hex!("00000000000000000000000000000000e4437ed6010e88286f547fa90abfe4c4"),
    ),
    (
        "SECP_GLV_G2",
        hex!("000000000000000000000000000000003086d221a7d46bcde86c90e49284eb15"),
    ),
    (
        "SECP_GLV_B1",
        hex!("fffffffffffffffffffffffffffffffdd66b5e10ae3a1813507ddee3c5765c7e"),
    ),
    (
        "SECP_GLV_B2",
        hex!("000000000000000000000000000000003086d221a7d46bcde86c90e49284eb15"),
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
    ("GAS_COLDACCOUNTACCESS_MINUS_WARMACCESS", 2_500),
    ("GAS_COLDSLOAD", 2_100),
    ("GAS_COLDSLOAD_MINUS_WARMACCESS", 2_000),
    ("GAS_SSET", 20_000),
    ("GAS_SRESET", 2_900),
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

const REFUND_CONSTANTS: [(&str, u16); 2] = [("REFUND_SCLEAR", 4_800), ("MAX_REFUND_QUOTIENT", 5)];

const PRECOMPILES: [(&str, u16); 9] = [
    ("ECREC", 1),
    ("SHA256", 2),
    ("RIP160", 3),
    ("ID", 4),
    ("EXPMOD", 5),
    ("BN_ADD", 6),
    ("BN_MUL", 7),
    ("SNARKV", 8),
    ("BLAKE2_F", 9),
];

const PRECOMPILES_GAS: [(&str, u16); 13] = [
    ("ECREC_GAS", 3_000),
    ("SHA256_STATIC_GAS", 60),
    ("SHA256_DYNAMIC_GAS", 12),
    ("RIP160_STATIC_GAS", 600),
    ("RIP160_DYNAMIC_GAS", 120),
    ("ID_STATIC_GAS", 15),
    ("ID_DYNAMIC_GAS", 3),
    ("EXPMOD_MIN_GAS", 200),
    ("BN_ADD_GAS", 150),
    ("BN_MUL_GAS", 6_000),
    ("SNARKV_STATIC_GAS", 45_000),
    ("SNARKV_DYNAMIC_GAS", 34_000),
    ("BLAKE2_F__GAS", 1),
];

const SNARKV_POINTERS: [(&str, u64); 2] = [("SNARKV_INP", 112), ("SNARKV_OUT", 100)];

const CODE_SIZE_LIMIT: [(&str, u64); 3] = [
    ("MAX_CODE_SIZE", 0x6000),
    ("MAX_INITCODE_SIZE", 0xc000),
    ("INITCODE_WORD_COST", 2),
];

const MAX_NONCE: (&str, u64) = ("MAX_NONCE", 0xffffffffffffffff);
const CALL_STACK_LIMIT: (&str, u64) = ("CALL_STACK_LIMIT", 1024);
