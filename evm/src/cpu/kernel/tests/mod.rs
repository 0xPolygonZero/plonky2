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
use num::BigUint;
use rand::Rng;

pub(crate) fn u256ify<'a>(hexes: impl IntoIterator<Item = &'a str>) -> Result<Vec<U256>> {
    Ok(hexes
        .into_iter()
        .map(U256::from_str)
        .collect::<Result<Vec<_>, _>>()?)
}

pub(crate) fn biguint_to_le_limbs(x: BigUint) -> Vec<u128> {
    let mut bytes = x.to_bytes_le();
    let padded_len = (bytes.len() + 15) * 16 / 16;
    bytes.resize(padded_len, 0);

    let mut result = Vec::with_capacity(padded_len / 16);
    for i in (0..bytes.len()).step_by(16) {
        let these_bytes: [u8; 16] = bytes[i..i + 16].try_into().unwrap();
        let this = u128::from_le_bytes(these_bytes);
        result.push(this);
    }
    result
}
