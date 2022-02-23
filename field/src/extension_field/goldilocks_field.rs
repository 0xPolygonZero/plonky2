use plonky2_util::branch_hint;

use crate::extension_field::Extendable;
use crate::goldilocks_field::{GoldilocksField, reduce128};
use crate::field_types::Field64;

// FIXME: reduce160 should be marked unsafe, or the type of x_hi
// changed, since the argument x_hi is assumed to actually fit in a
// u32.
//
// FIXME: Need a test that triggers the carry branch
#[inline(always)]
fn reduce160(x_lo: u128, x_hi: u64) -> GoldilocksField {
    debug_assert!(x_hi < (1 << 32) - 1);

    // for t = 1 .. 2^32-1, t*2^128 % p == p - (t << 32)
    let hi = <GoldilocksField as Field64>::ORDER - (x_hi << 32);
    // hi is not reduced if x_hi was 0.
    let (lo, cy) = x_lo.overflowing_add(hi as u128);
    if cy {
        // cy = true is very rare. The only way it can happen is if
        // x_lo is at least 2^128 - (2^64 - 2^63), i.e.
        // 0xFFFFFFFF FFFFFFFF 80000000 00000000
        // which for randomly distributed values will only happen with
        // probability about 2^-64.
        branch_hint();
        let lo = reduce128(lo).0;
        let cy_red = <GoldilocksField as Field64>::ORDER - (1u64 << 32);
        reduce128(lo as u128 + cy_red as u128)
    } else {
        reduce128(lo)
    }
}

/*
 * The functions add_prods[0-4] and add_sqrs[0-4] are helper functions
 * for computing products and squares for the quintic extension over the
 * Goldilocks field. They are faster than the generic method because all
 * reductions are delayed until the end which means only one is necessary.
 */

#[inline(always)]
fn u128_times_2(x: u128) -> (u128, u64) {
    (x << 1, (x >> 127) as u64)
}

#[inline(always)]
fn u128_times_3(x: u128) -> (u128, u64) {
    let (s, cy) = x.overflowing_add(x << 1);
    (s, (x >> 127) as u64 + cy as u64)
}

#[inline(always)]
fn u128_times_7(x: u128) -> (u128, u64) {
    let (d, br) = (x << 3).overflowing_sub(x);
    // TODO: Check that subtracting the borrow can't underflow
    (d, (x >> (128 - 3)) as u64 - br as u64)
}

/*
 * Quadratic multiplication and squaring
 */

#[inline(always)]
fn ext2_add_prods0(a: &[u64; 2], b: &[u64; 2]) -> GoldilocksField {
    // Computes a0 * b0 + W * a1 * b1;
    let [a0, a1] = *a;
    let [b0, b1] = *b;

    let mut cumul_lo: u128;
    let mut cumul_hi: u64;
    let cy;

    // W * a1 * b1
    (cumul_lo, cumul_hi) = u128_times_7((a1 as u128) * (b1 as u128));

    // a0 * b0
    (cumul_lo, cy) = cumul_lo.overflowing_add((a0 as u128) * (b0 as u128));
    cumul_hi += cy as u64;

    reduce160(cumul_lo, cumul_hi)
}

#[inline(always)]
fn ext2_add_prods1(a: &[u64; 2], b: &[u64; 2]) -> GoldilocksField {
    // Computes a0 * b1 + a1 * b0;
    let [a0, a1] = *a;
    let [b0, b1] = *b;

    let mut cumul_lo: u128;
    let cumul_hi: u64;
    let cy;

    // a0 * b1
    cumul_lo = (a0 as u128) * (b1 as u128);

    // a1 * b0
    (cumul_lo, cy) = cumul_lo.overflowing_add((a1 as u128) * (b0 as u128));
    cumul_hi = cy as u64;

    reduce160(cumul_lo, cumul_hi)
}

/// Multiply a and b considered as elements of GF(p^2).
#[inline(always)]
pub(crate) fn ext2_mul(a: [u64; 2], b: [u64; 2]) -> [GoldilocksField; 2] {
    let c0 = ext2_add_prods0(&a, &b);
    let c1 = ext2_add_prods1(&a, &b);
    [c0, c1]
}

#[inline(always)]
pub(crate) fn ext2_add_sqrs0(a: &[u64; 2]) -> GoldilocksField {
    let [a0, a1] = *a;

    let mut cumul_lo: u128;
    let mut cumul_hi: u64;
    let cy;

    // W * a1 * a1
    (cumul_lo, cumul_hi) = u128_times_7((a1 as u128) * (a1 as u128));

    // a0 * a0
    (cumul_lo, cy) = cumul_lo.overflowing_add((a0 as u128) * (a0 as u128));
    cumul_hi += cy as u64;

    reduce160(cumul_lo, cumul_hi)
}

#[inline(always)]
pub(crate) fn ext2_sqr(a: [u64; 2]) -> [GoldilocksField; 2] {
    let [a0, a1] = a;

    let c0 = ext2_add_sqrs0(&a);
    let (t, cy) = u128_times_2((a0 as u128) * (a1 as u128));
    let c1 = reduce160(t, cy as u64);

    [c0, c1]
}


/*
 * Quintic multiplication and squaring
 */

#[inline(always)]
fn ext5_add_prods0(a: &[u64; 5], b: &[u64; 5]) -> GoldilocksField {
    // Computes c0 = a0 * b0 + w * (a1 * b4 + a2 * b3 + a3 * b2 + a4 * b1)

    const W: u64 = <GoldilocksField as Extendable<5>>::W.0;

    let [a0, a1, a2, a3, a4] = *a;
    let [b0, b1, b2, b3, b4] = *b;

    let mut cumul_lo: u128;
    let mut cumul_hi: u64;
    let mut cy;

    // a1 * b4
    cumul_lo = (a1 as u128) * (b4 as u128);

    // a2 * b3
    (cumul_lo, cy) = cumul_lo.overflowing_add((a2 as u128) * (b3 as u128));
    cumul_hi = cy as u64;

    // a3 * b2
    (cumul_lo, cy) = cumul_lo.overflowing_add((a3 as u128) * (b2 as u128));
    cumul_hi += cy as u64;

    // a4 * b1
    (cumul_lo, cy) = cumul_lo.overflowing_add((a4 as u128) * (b1 as u128));
    cumul_hi += cy as u64;

    // * W
    let over;
    (cumul_lo, over) = u128_times_3(cumul_lo);
    cumul_hi = W * cumul_hi + over;

    // a0 * b0
    (cumul_lo, cy) = cumul_lo.overflowing_add((a0 as u128) * (b0 as u128));
    cumul_hi += cy as u64;

    reduce160(cumul_lo, cumul_hi)
}

#[inline(always)]
fn ext5_add_prods1(a: &[u64; 5], b: &[u64; 5]) -> GoldilocksField {
    // Computes c1 = a0 * b1 + a1 * b0 + w * (a2 * b4 + a3 * b3 + a4 * b2);

    const W: u64 = <GoldilocksField as Extendable<5>>::W.0;

    let [a0, a1, a2, a3, a4] = *a;
    let [b0, b1, b2, b3, b4] = *b;

    let mut cumul_lo: u128;
    let mut cumul_hi: u64;
    let mut cy;

    // a2 * b4
    cumul_lo = (a2 as u128) * (b4 as u128);

    // a3 * b3
    (cumul_lo, cy) = cumul_lo.overflowing_add((a3 as u128) * (b3 as u128));
    cumul_hi = cy as u64;

    // a4 * b2
    (cumul_lo, cy) = cumul_lo.overflowing_add((a4 as u128) * (b2 as u128));
    cumul_hi += cy as u64;

    // * W
    let over;
    (cumul_lo, over) = u128_times_3(cumul_lo);
    cumul_hi = W * cumul_hi + over;

    // a0 * b1
    (cumul_lo, cy) = cumul_lo.overflowing_add((a0 as u128) * (b1 as u128));
    cumul_hi += cy as u64;

    // a1 * b0
    (cumul_lo, cy) = cumul_lo.overflowing_add((a1 as u128) * (b0 as u128));
    cumul_hi += cy as u64;

    reduce160(cumul_lo, cumul_hi)
}

#[inline(always)]
fn ext5_add_prods2(a: &[u64; 5], b: &[u64; 5]) -> GoldilocksField {
    // Computes c2 = a0 * b2 + a1 * b1 + a2 * b0 + w * (a3 * b4 + a4 * b3);

    const W: u64 = <GoldilocksField as Extendable<5>>::W.0;

    let [a0, a1, a2, a3, a4] = *a;
    let [b0, b1, b2, b3, b4] = *b;

    let mut cumul_lo: u128;
    let mut cumul_hi: u64;
    let mut cy;

    // a3 * b4
    cumul_lo = (a3 as u128) * (b4 as u128);

    // a4 * b3
    (cumul_lo, cy) = cumul_lo.overflowing_add((a4 as u128) * (b3 as u128));
    cumul_hi = cy as u64;

    // * W
    let over;
    (cumul_lo, over) = u128_times_3(cumul_lo);
    cumul_hi = W * cumul_hi + over;

    // a0 * b2
    (cumul_lo, cy) = cumul_lo.overflowing_add((a0 as u128) * (b2 as u128));
    cumul_hi += cy as u64;

    // a1 * b1
    (cumul_lo, cy) = cumul_lo.overflowing_add((a1 as u128) * (b1 as u128));
    cumul_hi += cy as u64;

    // a2 * b0
    (cumul_lo, cy) = cumul_lo.overflowing_add((a2 as u128) * (b0 as u128));
    cumul_hi += cy as u64;

    reduce160(cumul_lo, cumul_hi)
}

#[inline(always)]
fn ext5_add_prods3(a: &[u64; 5], b: &[u64; 5]) -> GoldilocksField {
    // Computes c3 = a0 * b3 + a1 * b2 + a2 * b1 + a3 * b0 + w * a4 * b4;

    let [a0, a1, a2, a3, a4] = *a;
    let [b0, b1, b2, b3, b4] = *b;

    let mut cumul_lo: u128;
    let mut cumul_hi: u64;
    let mut cy;

    // a4 * b4
    cumul_lo = (a4 as u128) * (b4 as u128);

    // * W
    (cumul_lo, cumul_hi) = u128_times_3(cumul_lo);

    // a0 * b3
    (cumul_lo, cy) = cumul_lo.overflowing_add((a0 as u128) * (b3 as u128));
    cumul_hi += cy as u64;

    // a1 * b2
    (cumul_lo, cy) = cumul_lo.overflowing_add((a1 as u128) * (b2 as u128));
    cumul_hi += cy as u64;

    // a2 * b1
    (cumul_lo, cy) = cumul_lo.overflowing_add((a2 as u128) * (b1 as u128));
    cumul_hi += cy as u64;

    // a3 * b0
    (cumul_lo, cy) = cumul_lo.overflowing_add((a3 as u128) * (b0 as u128));
    cumul_hi += cy as u64;

    reduce160(cumul_lo, cumul_hi)
}

#[inline(always)]
fn ext5_add_prods4(a: &[u64; 5], b: &[u64; 5]) -> GoldilocksField {
    // Computes c4 = a0 * b4 + a1 * b3 + a2 * b2 + a3 * b1 + a4 * b0;

    let [a0, a1, a2, a3, a4] = *a;
    let [b0, b1, b2, b3, b4] = *b;

    let mut cumul_lo: u128;
    let mut cumul_hi: u64;
    let mut cy;

    // a0 * b4
    cumul_lo = (a0 as u128) * (b4 as u128);

    // a1 * b3
    (cumul_lo, cy) = cumul_lo.overflowing_add((a1 as u128) * (b3 as u128));
    cumul_hi = cy as u64;

    // a2 * b2
    (cumul_lo, cy) = cumul_lo.overflowing_add((a2 as u128) * (b2 as u128));
    cumul_hi += cy as u64;

    // a3 * b1
    (cumul_lo, cy) = cumul_lo.overflowing_add((a3 as u128) * (b1 as u128));
    cumul_hi += cy as u64;

    // a4 * b0
    (cumul_lo, cy) = cumul_lo.overflowing_add((a4 as u128) * (b0 as u128));
    cumul_hi += cy as u64;

    reduce160(cumul_lo, cumul_hi)
}

/// Multiply a and b considered as elements of GF(p^5).
#[inline(always)]
pub(crate) fn ext5_mul(a: [u64; 5], b: [u64; 5]) -> [GoldilocksField; 5] {
    let c0 = ext5_add_prods0(&a, &b);
    let c1 = ext5_add_prods1(&a, &b);
    let c2 = ext5_add_prods2(&a, &b);
    let c3 = ext5_add_prods3(&a, &b);
    let c4 = ext5_add_prods4(&a, &b);
    [c0, c1, c2, c3, c4]
}

#[inline(always)]
fn ext5_add_sqrs0(a: &[u64; 5]) -> GoldilocksField {
    // Compute c0 = a0^2 + 2 * w * (a1 * a4 + a2 * a3);

    const W: u64 = <GoldilocksField as Extendable<5>>::W.0;

    let [a0, a1, a2, a3, a4] = *a;

    let mut cumul_lo: u128;
    let mut cumul_hi: u64;
    let mut cy;

    // a1 * a4
    cumul_lo = (a1 as u128) * (a4 as u128);

    // a2 * a3
    (cumul_lo, cy) = cumul_lo.overflowing_add((a2 as u128) * (a3 as u128));
    cumul_hi = cy as u64;

    // * 2 * W
    let over1;
    let over2;
    (cumul_lo, over1) = u128_times_2(cumul_lo);
    cumul_hi = 2 * cumul_hi + over1;
    (cumul_lo, over2) = u128_times_3(cumul_lo);
    cumul_hi = W * cumul_hi + over2;

    // a0 * a0
    (cumul_lo, cy) = cumul_lo.overflowing_add((a0 as u128) * (a0 as u128));
    cumul_hi += cy as u64;

    reduce160(cumul_lo, cumul_hi)
}

#[inline(always)]
fn ext5_add_sqrs1(a: &[u64; 5]) -> GoldilocksField {
    // Compute c1 = 2 * a0 * a1 + 2 * w * a2 * a4 + w * a3 * a3;

    const W: u64 = <GoldilocksField as Extendable<5>>::W.0;

    let [a0, a1, a2, a3, a4] = *a;

    let mut cumul_lo: u128;
    let mut cumul_hi: u64;
    let mut cy;

    // a3 * a3
    cumul_lo = (a3 as u128) * (a3 as u128);

    // 2 * a2 * a4
    let (prod, top_bit) = u128_times_2((a2 as u128) * (a4 as u128));
    (cumul_lo, cy) = cumul_lo.overflowing_add(prod);
    cumul_hi = cy as u64 + top_bit;

    // * W
    let over;
    (cumul_lo, over) = u128_times_3(cumul_lo);
    cumul_hi = W * cumul_hi + over;

    // 2 * a0 * a1
    let (prod, top_bit) = u128_times_2((a0 as u128) * (a1 as u128));
    (cumul_lo, cy) = cumul_lo.overflowing_add(prod);
    cumul_hi += cy as u64 + top_bit;

    reduce160(cumul_lo, cumul_hi)
}

#[inline(always)]
fn ext5_add_sqrs2(a: &[u64; 5]) -> GoldilocksField {
    // Compute c2 = 2 * a0 * a2 + a1 * a1 + 2 * w * a4 * a3;

    const W: u64 = <GoldilocksField as Extendable<5>>::W.0;

    let [a0, a1, a2, a3, a4] = *a;

    let mut cumul_lo: u128;
    let mut cumul_hi: u64;
    let mut cy;

    // 2 * W * a3 * a4
    let over;
    (cumul_lo, cumul_hi) = u128_times_2((a3 as u128) * (a4 as u128));
    (cumul_lo, over) = u128_times_3(cumul_lo);
    cumul_hi = W * cumul_hi + over;

    // a1 * a1
    (cumul_lo, cy) = cumul_lo.overflowing_add((a1 as u128) * (a1 as u128));
    cumul_hi += cy as u64;

    // 2 * a0 * a2
    let (prod, top_bit) = u128_times_2((a0 as u128) * (a2 as u128));
    (cumul_lo, cy) = cumul_lo.overflowing_add(prod);
    cumul_hi += cy as u64 + top_bit;

    reduce160(cumul_lo, cumul_hi)
}

#[inline(always)]
fn ext5_add_sqrs3(a: &[u64; 5]) -> GoldilocksField {
    // Compute c3 = 2 * a0 * a3 + 2 * a1 * a2 + w * a4 * a4;

    let [a0, a1, a2, a3, a4] = *a;

    let mut cumul_lo: u128;
    let mut cumul_hi: u64;
    let mut cy;

    // a1 * a2
    cumul_lo = (a1 as u128) * (a2 as u128);

    // a0 * a3
    (cumul_lo, cy) = cumul_lo.overflowing_add((a0 as u128) * (a3 as u128));
    cumul_hi = cy as u64;

    // * 2
    let over;
    (cumul_lo, over) = u128_times_2(cumul_lo);
    cumul_hi = 2 * cumul_hi + over;

    // W * a4 * a4
    let (prod, over) = u128_times_3((a4 as u128) * (a4 as u128));
    (cumul_lo, cy) = cumul_lo.overflowing_add(prod);
    cumul_hi += cy as u64 + over;

    reduce160(cumul_lo, cumul_hi)
}

#[inline(always)]
fn ext5_add_sqrs4(a: &[u64; 5]) -> GoldilocksField {
    // Compute c4 = 2 * a0 * a4 + 2 * a1 * a3 + a2 * a2;

    let [a0, a1, a2, a3, a4] = *a;

    let mut cumul_lo: u128;
    let mut cumul_hi: u64;
    let mut cy;

    // a0 * a4
    cumul_lo = (a0 as u128) * (a4 as u128);

    // a1 * a3
    (cumul_lo, cy) = cumul_lo.overflowing_add((a1 as u128) * (a3 as u128));
    cumul_hi = cy as u64;

    // * 2
    let over;
    (cumul_lo, over) = u128_times_2(cumul_lo);
    cumul_hi = 2 * cumul_hi + over;

    // a2 * a2
    (cumul_lo, cy) = cumul_lo.overflowing_add((a2 as u128) * (a2 as u128));
    cumul_hi += cy as u64;

    reduce160(cumul_lo, cumul_hi)
}

/// Square a considered as an element of GF(p^5).
#[inline(always)]
pub(crate) fn ext5_sqr(a: [u64; 5]) -> [GoldilocksField; 5] {
    let c0 = ext5_add_sqrs0(&a);
    let c1 = ext5_add_sqrs1(&a);
    let c2 = ext5_add_sqrs2(&a);
    let c3 = ext5_add_sqrs3(&a);
    let c4 = ext5_add_sqrs4(&a);

    [c0, c1, c2, c3, c4]
}
