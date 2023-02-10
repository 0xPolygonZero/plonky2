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
use rand::Rng;

pub(crate) const LIMB_BASE: U256 = U256([0, 0, 1, 0]);

pub(crate) fn u256ify<'a>(hexes: impl IntoIterator<Item = &'a str>) -> Result<Vec<U256>> {
    Ok(hexes
        .into_iter()
        .map(U256::from_str)
        .collect::<Result<Vec<_>, _>>()?)
}

pub(crate) fn u256_to_le_limbs(x: U256) -> [u128; 2] {
    let mut bytes = vec![0; 32];
    x.to_little_endian(&mut bytes);

    let lower_bytes: [u8; 16] = bytes[..16].try_into().unwrap();
    let lower = u128::from_le_bytes(lower_bytes);
    let upper_bytes: [u8; 16] = bytes[16..].try_into().unwrap();
    let upper = u128::from_le_bytes(upper_bytes);

    [lower, upper]
}

fn gen_u256_limbs<R: Rng>(rng: &mut R, num_bits: usize) -> [u64; 4] {
    let remaining = num_bits % 64;
    let top_limb: u64 = rng.gen_range(0..(1 << remaining));
    if num_bits < 64 {
        [top_limb, 0, 0, 0]
    } else if num_bits < 128 {
        [rng.gen(), top_limb, 0, 0]
    } else if num_bits < 192 {
        [rng.gen(), rng.gen(), top_limb, 0]
    } else {
        [rng.gen(), rng.gen(), rng.gen(), top_limb]
    }
}

pub(crate) fn gen_random_u256(max: U256) -> U256 {
    let mut rng = rand::thread_rng();

    let num_bits = max.bits();

    let mut x: U256 = {
        let limbs = gen_u256_limbs(&mut rng, num_bits);
        U256(limbs)
    };
    while x > max {
        x = {
            let limbs = gen_u256_limbs(&mut rng, num_bits);
            U256(limbs)
        };
    }
    x
}
