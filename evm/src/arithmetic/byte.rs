//! Support for the EVM BYTE instruction
//!
//! This crate verifies the EVM BYTE instruction, defined as follows:
//!
//! INPUTS: 256-bit values X = \sum_{i=0}^31 X_i B^i and I,
//! where B = 2^8 and 0 <= X_i < B for all i.
//!
//! OUTPUT: X_{31-I} if 0 <= I < 32, otherwise 0.
//!
//! NB: index I=0 corresponds to byte X_31, i.e. the most significant
//! byte. This is exactly the opposite of anyone would expect; who
//! knows what the EVM designers were thinking. Anyway, if anything
//! below seems confusing, first check to ensure you're counting from
//! the wrong end of X, as the spec requires.
//!
//! Wlog consider 0 <= I < 32, so I has five bits b0,...,b4. We are
//! given X as an array of 16-bit limbs; write X := \sum_{i=0}^15 Y_i
//! 2^{16i} where 0 <= Y_i < 2^16.
//!
//! The technique (hat tip to Jacqui for the idea) is to store a tree
//! of limbs of X that are selected according to the bits in I.  The
//! main observation is that each bit bi halves the number of
//! candidate bytes that we might return: If b4 is 0, then I < 16 and
//! the possible bytes are in the top half of X: Y_8,..,Y_15
//! (corresponding to bytes X_16,..,X_31), and if b4 is 1 then I >= 16
//! and the possible bytes are the bottom half of X: Y_0,..,Y_7
//! (corresponding to bytes X_0,..,X_15).
//!
//! Let Z_0,..,Z_7 be the bytes selected in the first step. Then, in
//! the next step, if b3 is 0, we select Z_4,..,Z_7 and if it's 1 we
//! select Z_0,..,Z_3. Together, b4 and b3 divide the bytes of X into
//! 4 equal-sized chunks of 4 limbs, and the byte we're after will be
//! among the limbs 4 selected limbs.
//!
//! Repeating for b2 and b1, we reduce to a single 16-bit limb
//! L=L0+L1*256; the desired byte will be L0 if b0 is 1 and L1 if b0
//! is 0.

use ethereum_types::U256;
use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::{Field, PrimeField64};
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::plonk::circuit_builder::CircuitBuilder;

use crate::arithmetic::columns::*;
use crate::arithmetic::utils::u256_to_array;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};

// Give meaningful names to the columns of AUX_INPUT_REGISTER_0 that
// we're using
const BYTE_LAST_LIMB_LO: usize = AUX_INPUT_REGISTER_0.start + 7;
const BYTE_LAST_LIMB_HI: usize = AUX_INPUT_REGISTER_0.start + 8;
const BYTE_IDX_IS_LARGE: usize = AUX_INPUT_REGISTER_0.start + 9;
const BYTE_IDX_HI_LIMB_SUM_INV_0: usize = AUX_INPUT_REGISTER_0.start + 10;
const BYTE_IDX_HI_LIMB_SUM_INV_1: usize = AUX_INPUT_REGISTER_0.start + 11;
const BYTE_IDX_HI_LIMB_SUM_INV_2: usize = AUX_INPUT_REGISTER_0.start + 12;
const BYTE_IDX_HI_LIMB_SUM_INV_3: usize = AUX_INPUT_REGISTER_0.start + 13;

/// Decompose `idx` into bits and bobs and store in `idx_decomp`.
///
/// Specifically, write
///
///     idx = idx0_lo5 + idx0_hi * 2^5 + \sum_i idx[i] * 2^(16i),
///
/// where `0 <= idx0_lo5 < 32` and `0 <= idx0_hi < 2^11`.  Store the
/// 5 bits of `idx0_lo5` in `idx_decomp[0..5]`; we don't explicitly need
/// the higher 11 bits of the first limb, so we put them in
/// `idx_decomp[5]`. The rest of `idx_decomp` is set to 0.
fn set_idx_decomp<F: PrimeField64>(idx_decomp: &mut [F], idx: &U256) {
    for i in 0..5 {
        idx_decomp[i] = F::from_bool(idx.bit(i));
    }
    idx_decomp[5] = F::from_canonical_u16((idx.low_u64() as u16) >> 5);
    for i in 6..N_LIMBS {
        idx_decomp[i] = F::ZERO;
    }
}

pub(crate) fn generate<F: PrimeField64>(lv: &mut [F], val: U256, idx: U256) {
    u256_to_array(&mut lv[INPUT_REGISTER_0], val);
    u256_to_array(&mut lv[INPUT_REGISTER_1], idx);
    set_idx_decomp(&mut lv[AUX_INPUT_REGISTER_0], &idx);

    let idx0_hi = lv[AUX_INPUT_REGISTER_0][5];
    let hi_limb_sum = lv[INPUT_REGISTER_1][1..]
        .iter()
        .fold(idx0_hi, |acc, &x| acc + x);
    let hi_limb_sum_inv = hi_limb_sum
        .try_inverse()
        .unwrap_or(F::ONE)
        .to_canonical_u64();
    // It's a bit silly that we have to split this value, which
    // doesn't need to be range-checked, into 16-bit limbs so that it
    // can be range-checked; but the rigidity of the range-checking
    // mechanism means we can't optionally switch it off for some
    // instructions.
    lv[BYTE_IDX_HI_LIMB_SUM_INV_0] = F::from_canonical_u16(hi_limb_sum_inv as u16);
    lv[BYTE_IDX_HI_LIMB_SUM_INV_1] = F::from_canonical_u16((hi_limb_sum_inv >> 16) as u16);
    lv[BYTE_IDX_HI_LIMB_SUM_INV_2] = F::from_canonical_u16((hi_limb_sum_inv >> 32) as u16);
    lv[BYTE_IDX_HI_LIMB_SUM_INV_3] = F::from_canonical_u16((hi_limb_sum_inv >> 48) as u16);
    lv[BYTE_IDX_IS_LARGE] = F::from_bool(!hi_limb_sum.is_zero());

    // Set the tree values according to the low 5 bits of idx, even
    // when idx >= 32.

    // Use the bits of idx0 to build a multiplexor that selects
    // the correct byte of val. Each level of the tree uses one
    // bit to halve the set of possible bytes from the previous
    // level. The tree stores limbs rather than bytes though, so
    // the last value must be handled specially.

    // Morally, offset at i is 2^i * bit[i], but because of the
    // reversed indexing and handling of the last element
    // separately, the offset is 2^i * ( ! bit[i + 1]). (The !bit
    // corresponds to calculating 31 - bits which is just bitwise NOT.)

    // Conceptually we want to initialise the tree using something
    // like this:
    //
    //   let tree = &mut lv[AUX_INPUT_REGISTER_1];
    //   let val_limbs = &lv[INPUT_REGISTER_0];
    //   tree[..8].copy_from_slice(&val_limbs[offset..offset + 8]);
    //
    // but we can't borrow both tree and val_limbs
    // simultaneously. Apparently the solution is to use
    // `split_at_mut()`; below we assume that the val registers are
    // earlier in the row than the tree registers, so we enforce that
    // assumption here.
    let val_idx = INPUT_REGISTER_0.start;
    let tree_idx = AUX_INPUT_REGISTER_1.start;
    assert!(val_idx + N_LIMBS < tree_idx);

    // `lvl_len` is the number of elements of the current level of the
    // "tree". Can think of `val_limbs` as level 0, with length =
    // N_LIMBS = 16.
    assert!(N_LIMBS == 16); // Enforce assumption

    let (prev, next) = lv[val_idx..].split_at_mut(tree_idx - val_idx);
    let lvl_len = 8;
    let offset = (!idx.bit(4) as usize) * lvl_len;
    next[..lvl_len].copy_from_slice(&prev[offset..offset + lvl_len]);

    let (prev, next) = next.split_at_mut(lvl_len);
    let lvl_len = 4;
    let offset = (!idx.bit(3) as usize) * lvl_len;
    next[..lvl_len].copy_from_slice(&prev[offset..offset + lvl_len]);

    let (prev, next) = next.split_at_mut(lvl_len);
    let lvl_len = 2;
    let offset = (!idx.bit(2) as usize) * lvl_len;
    next[..lvl_len].copy_from_slice(&prev[offset..offset + lvl_len]);

    let (prev, next) = next.split_at_mut(lvl_len);
    // lvl_len = 1
    let offset = !idx.bit(1) as usize;
    next[0] = prev[offset]; // tree[14] = tree[offset + 12]

    // Handle the last bit; i.e. pick a byte of the final limb.
    let t = next[0].to_canonical_u64();
    let lo = t as u8 as u64;
    let hi = t >> 8;

    // Store 256 * lo rather than lo:
    lv[BYTE_LAST_LIMB_LO] = F::from_canonical_u64(lo << 8);
    lv[BYTE_LAST_LIMB_HI] = F::from_canonical_u64(hi);

    let tree = &mut lv[AUX_INPUT_REGISTER_1];
    let output = if idx.bit(0) {
        tree[15] = F::from_canonical_u64(lo);
        lo.into()
    } else {
        tree[15] = F::from_canonical_u64(hi);
        hi.into()
    };

    u256_to_array(
        &mut lv[OUTPUT_REGISTER],
        if idx < 32.into() {
            output
        } else {
            U256::zero()
        },
    );
}

pub fn eval_packed<P: PackedField>(
    lv: &[P; NUM_ARITH_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let is_byte = lv[IS_BYTE];

    let val = &lv[INPUT_REGISTER_0];
    let idx = &lv[INPUT_REGISTER_1];
    let out = &lv[OUTPUT_REGISTER];
    let idx_decomp = &lv[AUX_INPUT_REGISTER_0];
    let tree = &lv[AUX_INPUT_REGISTER_1];

    // low 5 bits of the first limb of idx:
    let mut idx0_lo5 = P::ZEROS;
    for i in 0..5 {
        let bit = idx_decomp[i];
        yield_constr.constraint(is_byte * (bit * bit - bit));
        idx0_lo5 += bit * P::Scalar::from_canonical_u64(1 << i);
    }
    // high (11) bits of the first limb of idx:
    let idx0_hi = idx_decomp[5] * P::Scalar::from_canonical_u64(32u64);
    yield_constr.constraint(is_byte * (idx[0] - (idx0_lo5 + idx0_hi)));

    // Verify the layers of the tree
    // NB: Each of the bit values is negated in place to account for
    // the reversed indexing.
    let bit = idx_decomp[4];
    for i in 0..8 {
        let limb = bit * val[i] + (P::ONES - bit) * val[i + 8];
        yield_constr.constraint(is_byte * (tree[i] - limb));
    }

    let bit = idx_decomp[3];
    for i in 0..4 {
        let limb = bit * tree[i] + (P::ONES - bit) * tree[i + 4];
        yield_constr.constraint(is_byte * (tree[i + 8] - limb));
    }

    let bit = idx_decomp[2];
    for i in 0..2 {
        let limb = bit * tree[i + 8] + (P::ONES - bit) * tree[i + 10];
        yield_constr.constraint(is_byte * (tree[i + 12] - limb));
    }

    let bit = idx_decomp[1];
    let limb = bit * tree[12] + (P::ONES - bit) * tree[13];
    yield_constr.constraint(is_byte * (tree[14] - limb));

    // Check byte decomposition of last limb:

    let base8 = P::Scalar::from_canonical_u64(1 << 8);
    // base8_inv is Goldilocks-specific
    debug_assert!(P::Scalar::order() == 0xffff_ffff_0000_0001u64.into());
    let base8_inv = P::Scalar::from_canonical_u64(0xfeffffff01000001);
    debug_assert!(base8 * base8_inv == P::Scalar::ONE);

    let lo_byte = lv[BYTE_LAST_LIMB_LO];
    let hi_byte = lv[BYTE_LAST_LIMB_HI];
    yield_constr.constraint(is_byte * (base8_inv * lo_byte + base8 * hi_byte - limb));

    let bit = idx_decomp[0];
    let t = bit * base8_inv * lo_byte + (P::ONES - bit) * hi_byte;
    yield_constr.constraint(is_byte * (tree[15] - t));
    let expected_out_byte = tree[15];

    // Sum all higher limbs; sum will be non-zero iff idx >= 32.
    //let hi_limb_sum: P = idx0_hi + idx[1..].iter().sum::<P>(); // doesn't work
    let hi_limb_sum = idx[1..].iter().fold(idx0_hi, |acc, &i| i + acc);
    let idx_is_large = lv[BYTE_IDX_IS_LARGE];

    // idx_is_large is 0 or 1
    yield_constr.constraint(is_byte * (idx_is_large * idx_is_large - idx_is_large));

    // If hi_limb_sum is nonzero, then idx_is_large must be one.
    yield_constr.constraint(is_byte * hi_limb_sum * (idx_is_large - P::ONES));

    let base16 = P::Scalar::from_canonical_u64(1 << 16);
    let hi_limb_sum_inv = lv[BYTE_IDX_HI_LIMB_SUM_INV_0]
        + lv[BYTE_IDX_HI_LIMB_SUM_INV_1] * base16
        + lv[BYTE_IDX_HI_LIMB_SUM_INV_2] * base16 * base16
        + lv[BYTE_IDX_HI_LIMB_SUM_INV_3] * base16 * base16 * base16;

    // If idx_is_large is 1, then hi_limb_sum_inv must be the inverse
    // of hi_limb_sum, hence hi_limb_sum is non-zero, hence idx is
    // indeed "large".
    //
    // Otherwise, if idx_is_large is 0, then hi_limb_sum * hi_limb_sum_inv
    // is zero, which is only possible if hi_limb_sum is zero, since
    // hi_limb_sum_inv is non-zero.
    yield_constr.constraint(is_byte * (hi_limb_sum * hi_limb_sum_inv - idx_is_large));

    let out_byte = out[0];
    let check = idx_is_large * out_byte + (P::ONES - idx_is_large) * (out_byte - expected_out_byte);
    yield_constr.constraint(is_byte * check);

    // Check that the rest of the output limbs are zero
    for i in 1..N_LIMBS {
        yield_constr.constraint(is_byte * out[i]);
    }
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; NUM_ARITH_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let is_byte = lv[IS_BYTE];

    let val = &lv[INPUT_REGISTER_0];
    let idx = &lv[INPUT_REGISTER_1];
    let out = &lv[OUTPUT_REGISTER];
    let idx_decomp = &lv[AUX_INPUT_REGISTER_0];
    let tree = &lv[AUX_INPUT_REGISTER_1];

    let mut idx0_lo5 = builder.zero_extension();
    for i in 0..5 {
        let bit = idx_decomp[i];
        let t = builder.mul_sub_extension(bit, bit, bit);
        let t = builder.mul_extension(t, is_byte);
        yield_constr.constraint(builder, t);
        let scale = F::Extension::from(F::from_canonical_u64(1 << i));
        let scale = builder.constant_extension(scale);
        idx0_lo5 = builder.mul_add_extension(bit, scale, idx0_lo5);
    }
    let t = F::Extension::from(F::from_canonical_u64(32));
    let t = builder.constant_extension(t);
    let idx0_hi = builder.mul_extension(idx_decomp[5], t);
    let t = builder.add_extension(idx0_lo5, idx0_hi);
    let t = builder.sub_extension(idx[0], t);
    let t = builder.mul_extension(is_byte, t);
    yield_constr.constraint(builder, t);

    let one = builder.one_extension();
    let bit = idx_decomp[4];
    for i in 0..8 {
        let t = builder.mul_extension(bit, val[i]);
        let u = builder.sub_extension(one, bit);
        let v = builder.mul_add_extension(u, val[i + 8], t);
        let t = builder.sub_extension(tree[i], v);
        let t = builder.mul_extension(is_byte, t);
        yield_constr.constraint(builder, t);
    }

    let bit = idx_decomp[3];
    for i in 0..4 {
        let t = builder.mul_extension(bit, tree[i]);
        let u = builder.sub_extension(one, bit);
        let v = builder.mul_add_extension(u, tree[i + 4], t);
        let t = builder.sub_extension(tree[i + 8], v);
        let t = builder.mul_extension(is_byte, t);
        yield_constr.constraint(builder, t);
    }

    let bit = idx_decomp[2];
    for i in 0..2 {
        let t = builder.mul_extension(bit, tree[i + 8]);
        let u = builder.sub_extension(one, bit);
        let v = builder.mul_add_extension(u, tree[i + 10], t);
        let t = builder.sub_extension(tree[i + 12], v);
        let t = builder.mul_extension(is_byte, t);
        yield_constr.constraint(builder, t);
    }

    let bit = idx_decomp[1];
    let t = builder.mul_extension(bit, tree[12]);
    let u = builder.sub_extension(one, bit);
    let limb = builder.mul_add_extension(u, tree[13], t);
    let t = builder.sub_extension(tree[14], limb);
    let t = builder.mul_extension(is_byte, t);
    yield_constr.constraint(builder, t);

    let base8 = F::Extension::from(F::from_canonical_u64(1 << 8));
    let base8 = builder.constant_extension(base8);
    let base8_inv = F::Extension::from(F::from_canonical_u64(0xfeffffff01000001));
    let base8_inv = builder.constant_extension(base8_inv);
    let lo_byte = lv[BYTE_LAST_LIMB_LO];
    let hi_byte = lv[BYTE_LAST_LIMB_HI];
    let t = builder.mul_sub_extension(base8, hi_byte, limb);
    let t = builder.mul_add_extension(base8_inv, lo_byte, t);
    let t = builder.mul_extension(is_byte, t);
    yield_constr.constraint(builder, t);

    let bit = idx_decomp[0];
    let nbit = builder.sub_extension(one, bit);
    let t = builder.mul_many_extension([bit, lo_byte, base8_inv]);
    let t = builder.mul_add_extension(nbit, hi_byte, t);
    let u = builder.sub_extension(tree[15], t);
    let t = builder.mul_extension(is_byte, u);
    yield_constr.constraint(builder, t);
    let expected_out_byte = tree[15];

    let mut hi_limb_sum = idx0_hi;
    for i in 1..N_LIMBS {
        hi_limb_sum = builder.add_extension(hi_limb_sum, idx[i]);
    }
    let idx_is_large = lv[BYTE_IDX_IS_LARGE];
    let t = builder.mul_sub_extension(idx_is_large, idx_is_large, idx_is_large);
    let t = builder.mul_extension(is_byte, t);
    yield_constr.constraint(builder, t);

    let t = builder.sub_extension(idx_is_large, one);
    let t = builder.mul_many_extension([is_byte, hi_limb_sum, t]);
    yield_constr.constraint(builder, t);

    let base16 = F::from_canonical_u64(1 << 16);
    let hi_limb_sum_inv = builder.mul_const_add_extension(
        base16,
        lv[BYTE_IDX_HI_LIMB_SUM_INV_3],
        lv[BYTE_IDX_HI_LIMB_SUM_INV_2],
    );
    let hi_limb_sum_inv =
        builder.mul_const_add_extension(base16, hi_limb_sum_inv, lv[BYTE_IDX_HI_LIMB_SUM_INV_1]);
    let hi_limb_sum_inv =
        builder.mul_const_add_extension(base16, hi_limb_sum_inv, lv[BYTE_IDX_HI_LIMB_SUM_INV_0]);
    let t = builder.mul_sub_extension(hi_limb_sum, hi_limb_sum_inv, idx_is_large);
    let t = builder.mul_extension(is_byte, t);
    yield_constr.constraint(builder, t);

    let out_byte = out[0];
    let t = builder.sub_extension(one, idx_is_large);
    let u = builder.sub_extension(out_byte, expected_out_byte);
    let t = builder.mul_extension(t, u);
    let check = builder.mul_add_extension(idx_is_large, out_byte, t);
    let t = builder.mul_extension(is_byte, check);
    yield_constr.constraint(builder, t);

    for i in 1..N_LIMBS {
        let t = builder.mul_extension(is_byte, out[i]);
        yield_constr.constraint(builder, t);
    }
}

#[cfg(test)]
mod tests {
    use plonky2::field::goldilocks_field::GoldilocksField;
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaCha8Rng;

    use super::*;
    use crate::arithmetic::columns::NUM_ARITH_COLUMNS;

    type F = GoldilocksField;

    fn verify_output(lv: &[F], expected_byte: u64) {
        let out_byte = lv[OUTPUT_REGISTER][0].to_canonical_u64();
        assert!(out_byte == expected_byte);
        for j in 1..N_LIMBS {
            assert!(lv[OUTPUT_REGISTER][j] == F::ZERO);
        }
    }

    #[test]
    fn generate_eval_consistency() {
        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);
        const N_ITERS: usize = 1000;

        for _ in 0..N_ITERS {
            // set entire row to random 16-bit values
            let mut lv =
                [F::default(); NUM_ARITH_COLUMNS].map(|_| F::from_canonical_u16(rng.gen::<u16>()));

            lv[IS_BYTE] = F::ONE;

            let val = U256::from(rng.gen::<[u8; 32]>());
            for i in 0..32 {
                let idx = i.into();
                generate(&mut lv, val, idx);

                // Check correctness
                let out_byte = val.byte(31 - i) as u64;
                verify_output(&lv, out_byte);

                let mut constrant_consumer = ConstraintConsumer::new(
                    vec![GoldilocksField(2), GoldilocksField(3), GoldilocksField(5)],
                    F::ONE,
                    F::ONE,
                    F::ONE,
                );
                eval_packed(&lv, &mut constrant_consumer);
                for &acc in &constrant_consumer.constraint_accs {
                    assert_eq!(acc, F::ZERO);
                }
            }
            // Check that output is zero when the index is big.
            let big_indices = [32.into(), 33.into(), val, U256::max_value()];
            for idx in big_indices {
                generate(&mut lv, val, idx);
                verify_output(&lv, 0);
            }
        }
    }
}
