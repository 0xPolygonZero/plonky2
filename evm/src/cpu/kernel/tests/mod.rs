mod account_code;
mod add11;
mod balance;
mod bignum;
mod blake2_f;
mod block_hash;
mod bls381;
mod bn254;
mod core;
mod ecc;
mod exp;
mod hash;
mod kernel_consistency;
mod log;
mod mpt;
mod packing;
mod receipt;
mod rlp;
mod signed_syscalls;
mod transaction_parsing;

use std::str::FromStr;

use anyhow::Result;
use ethereum_types::U256;

pub(crate) fn u256ify<'a>(hexes: impl IntoIterator<Item = &'a str>) -> Result<Vec<U256>> {
    Ok(hexes
        .into_iter()
        .map(U256::from_str)
        .collect::<Result<Vec<_>, _>>()?)
}
