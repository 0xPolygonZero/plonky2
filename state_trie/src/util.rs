use plonky2::field::field_types::Field;
use plonky2::field::goldilocks_field::GoldilocksField;
use primitive_types::{H160, H256, U256};

type F = GoldilocksField;

pub fn get_bit(h: &H256, bit: usize) -> bool {
    todo!()
}

pub fn h256_to_u32s_le(h: &H256) -> [u32; 8] {
    todo!()
}

pub fn h160_to_u32s_le(h: &H160) -> [u32; 5] {
    todo!()
}

pub fn u256_to_u32s_le(u: &U256) -> [u32; 8] {
    todo!()
}

pub fn h256_to_u32s_le_f(h: &H256) -> [F; 8] {
    h256_to_u32s_le(h).map(F::from_canonical_u32)
}

pub fn h160_to_u32s_le_f(h: &H160) -> [F; 5] {
    h160_to_u32s_le(h).map(F::from_canonical_u32)
}

pub fn u256_to_u32s_le_f(u: &U256) -> [F; 8] {
    u256_to_u32s_le(u).map(F::from_canonical_u32)
}
