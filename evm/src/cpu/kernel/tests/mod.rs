mod account_code;
mod balance;
mod bignum;
mod core;
mod ecc;
mod exp;
mod fields;
mod hash;
mod mpt;
mod packing;
mod ripemd;
mod rlp;
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
