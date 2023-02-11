use std::mem::{size_of, transmute_copy, ManuallyDrop};

use ethereum_types::{H160, H256, U256};
use itertools::Itertools;
use num::BigUint;
use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::polynomial::PolynomialValues;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::util::transpose;

/// Construct an integer from its constituent bits (in little-endian order)
pub fn limb_from_bits_le<P: PackedField>(iter: impl IntoIterator<Item = P>) -> P {
    // TODO: This is technically wrong, as 1 << i won't be canonical for all fields...
    iter.into_iter()
        .enumerate()
        .map(|(i, bit)| bit * P::Scalar::from_canonical_u64(1 << i))
        .sum()
}

/// Construct an integer from its constituent bits (in little-endian order): recursive edition
pub fn limb_from_bits_le_recursive<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    iter: impl IntoIterator<Item = ExtensionTarget<D>>,
) -> ExtensionTarget<D> {
    iter.into_iter()
        .enumerate()
        .fold(builder.zero_extension(), |acc, (i, bit)| {
            // TODO: This is technically wrong, as 1 << i won't be canonical for all fields...
            builder.mul_const_add_extension(F::from_canonical_u64(1 << i), bit, acc)
        })
}

/// A helper function to transpose a row-wise trace and put it in the format that `prove` expects.
pub fn trace_rows_to_poly_values<F: Field, const COLUMNS: usize>(
    trace_rows: Vec<[F; COLUMNS]>,
) -> Vec<PolynomialValues<F>> {
    let trace_row_vecs = trace_rows.into_iter().map(|row| row.to_vec()).collect_vec();
    let trace_col_vecs: Vec<Vec<F>> = transpose(&trace_row_vecs);
    trace_col_vecs
        .into_iter()
        .map(|column| PolynomialValues::new(column))
        .collect()
}

#[allow(unused)] // TODO: Remove?
/// Returns the 32-bit little-endian limbs of a `U256`.
pub(crate) fn u256_limbs<F: Field>(u256: U256) -> [F; 8] {
    u256.0
        .into_iter()
        .flat_map(|limb_64| {
            let lo = limb_64 as u32;
            let hi = (limb_64 >> 32) as u32;
            [lo, hi]
        })
        .map(F::from_canonical_u32)
        .collect_vec()
        .try_into()
        .unwrap()
}

/// Returns the 32-bit little-endian limbs of a `H256`.
pub(crate) fn h256_limbs<F: Field>(h256: H256) -> [F; 8] {
    h256.0
        .chunks(4)
        .map(|chunk| u32::from_le_bytes(chunk.try_into().unwrap()))
        .map(F::from_canonical_u32)
        .collect_vec()
        .try_into()
        .unwrap()
}

/// Returns the 32-bit limbs of a `U160`.
pub(crate) fn h160_limbs<F: Field>(h160: H160) -> [F; 5] {
    h160.0
        .chunks(4)
        .map(|chunk| u32::from_le_bytes(chunk.try_into().unwrap()))
        .map(F::from_canonical_u32)
        .collect_vec()
        .try_into()
        .unwrap()
}

pub(crate) const fn indices_arr<const N: usize>() -> [usize; N] {
    let mut indices_arr = [0; N];
    let mut i = 0;
    while i < N {
        indices_arr[i] = i;
        i += 1;
    }
    indices_arr
}

pub(crate) unsafe fn transmute_no_compile_time_size_checks<T, U>(value: T) -> U {
    debug_assert_eq!(size_of::<T>(), size_of::<U>());
    // Need ManuallyDrop so that `value` is not dropped by this function.
    let value = ManuallyDrop::new(value);
    // Copy the bit pattern. The original value is no longer safe to use.
    transmute_copy(&value)
}

pub(crate) fn addmod(x: U256, y: U256, m: U256) -> U256 {
    if m.is_zero() {
        return m;
    }
    let x = u256_to_biguint(x);
    let y = u256_to_biguint(y);
    let m = u256_to_biguint(m);
    biguint_to_u256((x + y) % m)
}

pub(crate) fn mulmod(x: U256, y: U256, m: U256) -> U256 {
    if m.is_zero() {
        return m;
    }
    let x = u256_to_biguint(x);
    let y = u256_to_biguint(y);
    let m = u256_to_biguint(m);
    biguint_to_u256(x * y % m)
}

pub(crate) fn submod(x: U256, y: U256, m: U256) -> U256 {
    if m.is_zero() {
        return m;
    }
    let mut x = u256_to_biguint(x);
    let y = u256_to_biguint(y);
    let m = u256_to_biguint(m);
    while x < y {
        x += &m;
    }
    biguint_to_u256((x - y) % m)
}

pub(crate) fn u256_to_biguint(x: U256) -> BigUint {
    let mut bytes = [0u8; 32];
    x.to_little_endian(&mut bytes);
    BigUint::from_bytes_le(&bytes)
}

pub(crate) fn biguint_to_u256(x: BigUint) -> U256 {
    let bytes = x.to_bytes_le();
    U256::from_little_endian(&bytes)
}

pub(crate) fn le_limbs_to_biguint(x: &[u128]) -> BigUint {
    BigUint::from_slice(
        &x.iter()
            .flat_map(|&a| {
                [
                    (a % (1 << 32)) as u32,
                    ((a >> 32) % (1 << 32)) as u32,
                    ((a >> 64) % (1 << 32)) as u32,
                    ((a >> 96) % (1 << 32)) as u32,
                ]
            })
            .collect::<Vec<u32>>(),
    )
}

pub(crate) fn mem_vec_to_biguint(x: &[U256]) -> BigUint {
    le_limbs_to_biguint(&x.iter().map(|&n| n.try_into().unwrap()).collect_vec())
}

pub(crate) fn biguint_to_le_limbs(x: BigUint) -> Vec<u128> {
    let mut digits = x.to_u32_digits();

    // Pad to a multiple of 8.
    digits.resize((digits.len() + 7) / 8 * 8, 0);

    digits
        .chunks(4)
        .map(|c| (c[3] as u128) << 96 | (c[2] as u128) << 64 | (c[1] as u128) << 32 | c[0] as u128)
        .collect()
}

pub(crate) fn biguint_to_mem_vec(x: BigUint) -> Vec<U256> {
    biguint_to_le_limbs(x)
        .into_iter()
        .map(|n| n.into())
        .collect()
}
