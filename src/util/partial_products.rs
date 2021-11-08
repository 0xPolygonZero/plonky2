use std::iter::Product;
use std::ops::{MulAssign, Sub};

use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::field::field_types::RichField;
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::util::ceil_div_usize;

/// Compute partial products of the original vector `v` such that all products consist of `max_degree`
/// or less elements. This is done until we've computed the product `P` of all elements in the vector.
pub fn partial_products<T: MulAssign + Product + Copy>(v: &[T], max_degree: usize) -> Vec<T> {
    debug_assert!(max_degree > 1);
    let mut res = Vec::new();
    let mut acc = v[0];
    let chunk_size = max_degree - 1;
    let num_chunks = ceil_div_usize(v.len() - 1, chunk_size) - 1;
    for i in 0..num_chunks {
        acc *= v[1 + i * chunk_size..1 + (i + 1) * chunk_size]
            .iter()
            .copied()
            .product();
        res.push(acc);
    }

    res
}

/// Returns a tuple `(a,b)`, where `a` is the length of the output of `partial_products()` on a
/// vector of length `n`, and `b` is the number of elements needed to compute the final product.
pub fn num_partial_products(n: usize, max_degree: usize) -> (usize, usize) {
    debug_assert!(max_degree > 1);
    let chunk_size = max_degree - 1;
    let num_chunks = ceil_div_usize(n - 1, chunk_size) - 1;

    (num_chunks, 1 + num_chunks * chunk_size)
}

/// Checks that the partial products of `v` are coherent with those in `partials` by only computing
/// products of size `max_degree` or less.
pub fn check_partial_products<T: MulAssign + Product + Copy + Sub<Output = T>>(
    v: &[T],
    mut partials: &[T],
    max_degree: usize,
) -> Vec<T> {
    debug_assert!(max_degree > 1);
    let mut partials = partials.iter();
    let mut res = Vec::new();
    let mut acc = v[0];
    let chunk_size = max_degree - 1;
    let num_chunks = ceil_div_usize(v.len() - 1, chunk_size) - 1;
    for i in 0..num_chunks {
        acc *= v[1 + i * chunk_size..1 + (i + 1) * chunk_size]
            .iter()
            .copied()
            .product();
        res.push(acc - *partials.next().unwrap());
    }
    debug_assert!(partials.next().is_none());

    res
}

pub fn check_partial_products_recursively<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    v: &[ExtensionTarget<D>],
    partials: &[ExtensionTarget<D>],
    max_degree: usize,
) -> Vec<ExtensionTarget<D>> {
    debug_assert!(max_degree > 1);
    let mut partials = partials.iter();
    let mut res = Vec::new();
    let mut acc = v[0];
    let chunk_size = max_degree - 1;
    let num_chunks = ceil_div_usize(v.len() - 1, chunk_size) - 1;
    for i in 0..num_chunks {
        let mut chunk = v[1 + i * chunk_size..1 + (i + 1) * chunk_size].to_vec();
        chunk.push(acc);
        acc = builder.mul_many_extension(&chunk);

        res.push(builder.sub_extension(acc, *partials.next().unwrap()));
    }
    debug_assert!(partials.next().is_none());

    res
}

#[cfg(test)]
mod tests {
    use num::Zero;

    use super::*;

    #[test]
    fn test_partial_products() {
        let v = vec![1, 2, 3, 4, 5, 6];
        let p = partial_products(&v, 2);
        assert_eq!(p, vec![2, 6, 24, 120]);
        let nums = num_partial_products(v.len(), 2);
        assert_eq!(p.len(), nums.0);
        assert!(check_partial_products(&v, &p, 2)
            .iter()
            .all(|x| x.is_zero()));
        assert_eq!(
            *p.last().unwrap() * v[nums.1..].iter().copied().product::<i32>(),
            v.into_iter().product::<i32>(),
        );

        let v = vec![1, 2, 3, 4, 5, 6];
        let p = partial_products(&v, 3);
        assert_eq!(p, vec![6, 120]);
        let nums = num_partial_products(v.len(), 3);
        assert_eq!(p.len(), nums.0);
        assert!(check_partial_products(&v, &p, 3)
            .iter()
            .all(|x| x.is_zero()));
        assert_eq!(
            *p.last().unwrap() * v[nums.1..].iter().copied().product::<i32>(),
            v.into_iter().product::<i32>(),
        );
    }
}
