use plonky2::hash::hash_types::RichField;
use plonky2::iop::target::{BoolTarget};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2_ecdsa::gadgets::biguint::CircuitBuilderBiguint;
use plonky2_field::extension::Extendable;

use crate::helper::{biguint_to_bits_target, bits_to_biguint_target};

/*
a ^ b ^ c = a+b+c - 2*a*b - 2*a*c - 2*b*c + 4*a*b*c
          = a*( 1 - 2*b - 2*c + 4*b*c ) + b + c - 2*b*c
          = a*( 1 - 2*b -2*c + 4*m ) + b + c - 2*m
where m = b*c
 */
pub fn xor3<F: RichField + Extendable<D>, const D: usize>(
    a: BoolTarget,
    b: BoolTarget,
    c: BoolTarget,
    builder: &mut CircuitBuilder<F, D>
) -> BoolTarget {
    let m = builder.mul(b.target, c.target);
    let two_b = builder.add(b.target, b.target);
    let two_c = builder.add(c.target, c.target);
    let two_m = builder.add(m, m);
    let four_m = builder.add(two_m, two_m);
    let one = builder.one();
    let one_sub_two_b = builder.sub(one, two_b);
    let one_sub_two_b_sub_two_c = builder.sub(one_sub_two_b, two_c);
    let one_sub_two_b_sub_two_c_add_four_m = builder.add(one_sub_two_b_sub_two_c, four_m);
    let mut res = builder.mul(a.target, one_sub_two_b_sub_two_c_add_four_m);
    res = builder.add(res, b.target);
    res = builder.add(res, c.target);

    BoolTarget::new_unsafe(builder.sub(res, two_m))
}

pub fn xor3_arr<F: RichField + Extendable<D>, const D: usize, const S: usize>(
    a: [BoolTarget; S],
    b: [BoolTarget; S],
    c: [BoolTarget; S],
    builder: &mut CircuitBuilder<F, D>
) -> [BoolTarget; S] {
    let mut res = [None; S];
    for i in 0..S {
        res[i] = Some(xor3(a[i], b[i], c[i], builder));
    }
    res.map(|x| x.unwrap())
}

pub fn xor2_arr<F: RichField + Extendable<D>, const D: usize, const S: usize>(
    a: [BoolTarget; S],
    b: [BoolTarget; S],
    builder: &mut CircuitBuilder<F, D>
) -> [BoolTarget; S] {
    let mut res = [None; S];
    for i in 0..S {
        let a_b = builder.mul(a[i].target, b[i].target);
        let two_a_b = builder.mul_const(F::ONE + F::ONE, a_b);
        let a_plus_b = builder.add(a[i].target, b[i].target);
        res[i] = Some(BoolTarget::new_unsafe(builder.sub(a_plus_b, two_a_b)));
    }
    res.map(|x| x.unwrap())
}

pub fn and_arr<F: RichField + Extendable<D>, const D: usize, const S: usize>(    
    a: [BoolTarget; S], b: [BoolTarget; S],
    builder: &mut CircuitBuilder<F, D>
) -> [BoolTarget; S] {
    let mut res = [None; S];
    for i in 0..S {
        res[i] = Some(builder.and(a[i], b[i]));
    }
    res.map(|x| x.unwrap())
}

pub fn not_arr<F: RichField + Extendable<D>, const D: usize, const S: usize>(    
    a: [BoolTarget; S],
    builder: &mut CircuitBuilder<F, D>
) -> [BoolTarget; S] {
    let mut res = [None; S];
    for i in 0..S {
        res[i] = Some(builder.not(a[i]));
    }
    res.map(|x| x.unwrap())
}

pub fn zip_add<F: RichField + Extendable<D>, const D: usize, const S: usize>(
    a: [[BoolTarget; S]; 8],
    b: [[BoolTarget; S]; 8],
    builder: &mut CircuitBuilder<F, D>
) -> [[BoolTarget; S]; 8] {
    let mut res = [None; 8];
    for i in 0..8 {
        res[i] = Some(add_arr(a[i], b[i], builder));
    }
    res.map(|x| x.unwrap())
}

pub fn add_arr<F:RichField + Extendable<D>, const D:usize, const S: usize>(a: [BoolTarget; S], b: [BoolTarget; S], builder: &mut CircuitBuilder<F, D>) -> [BoolTarget; S] {
    // First convert a, b into biguint with limbs of 32 bits each
    let a_biguint = bits_to_biguint_target(builder, a.to_vec());
    let b_biguint = bits_to_biguint_target(builder, b.to_vec());
    // Then add a and b are big uints
    let sum_biguint = builder.add_biguint(&a_biguint, &b_biguint);
    let mut sum_bits = biguint_to_bits_target::<F, D, 2>(builder, &sum_biguint);

    // sum_bits is in big-endian format.
    // we need to return the S least significant bits in big-endian format
    let mut res = [None; S];
    sum_bits.reverse();
    for i in 0..S {
        res[i] = Some(sum_bits[S-1-i]);
    }
    res.map(|x| x.unwrap())
}