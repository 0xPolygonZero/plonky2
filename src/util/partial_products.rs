use std::iter::Product;
use std::ops::Sub;

use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::field::field_types::RichField;
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::util::ceil_div_usize;

/// Compute partial products of the original vector `v` such that all products consist of `max_degree`
/// or less elements. This is done until we've computed the product `P` of all elements in the vector.
pub fn partial_products<T: Product + Copy>(v: &[T], max_degree: usize) -> Vec<T> {
    let mut res = Vec::new();
    let mut remainder = v.to_vec();
    while remainder.len() > max_degree {
        let new_partials = remainder
            .chunks(max_degree)
            // TODO: can filter out chunks of length 1.
            .map(|chunk| chunk.iter().copied().product())
            .collect::<Vec<_>>();
        res.extend_from_slice(&new_partials);
        remainder = new_partials;
    }

    res
}

/// Returns a tuple `(a,b)`, where `a` is the length of the output of `partial_products()` on a
/// vector of length `n`, and `b` is the number of elements needed to compute the final product.
pub fn num_partial_products(n: usize, max_degree: usize) -> (usize, usize) {
    debug_assert!(max_degree > 1);
    let mut res = 0;
    let mut remainder = n;
    while remainder > max_degree {
        let new_partials_len = ceil_div_usize(remainder, max_degree);
        res += new_partials_len;
        remainder = new_partials_len;
    }

    (res, remainder)
}

/// Checks that the partial products of `v` are coherent with those in `partials` by only computing
/// products of size `max_degree` or less.
pub fn check_partial_products<T: Product + Copy + Sub<Output = T>>(
    v: &[T],
    mut partials: &[T],
    max_degree: usize,
) -> Vec<T> {
    let mut res = Vec::new();
    let mut remainder = v;
    while remainder.len() > max_degree {
        let products = remainder
            .chunks(max_degree)
            .map(|chunk| chunk.iter().copied().product::<T>());
        let products_len = products.len();
        res.extend(products.zip(partials).map(|(a, &b)| a - b));
        (remainder, partials) = partials.split_at(products_len);
    }

    res
}

pub fn check_partial_products_recursively<F: Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    v: &[ExtensionTarget<D>],
    partials: &[ExtensionTarget<D>],
    max_degree: usize,
) -> Vec<ExtensionTarget<D>> {
    let mut res = Vec::new();
    let mut remainder = v.to_vec();
    let mut partials = partials.to_vec();
    while remainder.len() > max_degree {
        let products = remainder
            .chunks(max_degree)
            .map(|chunk| builder.mul_many_extension(chunk))
            .collect::<Vec<_>>();
        res.extend(
            products
                .iter()
                .zip(&partials)
                .map(|(&a, &b)| builder.sub_extension(a, b)),
        );
        remainder = partials.drain(..products.len()).collect();
    }

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
        assert_eq!(p, vec![2, 12, 30, 24, 30]);
        let nums = num_partial_products(v.len(), 2);
        assert_eq!(p.len(), nums.0);
        assert!(check_partial_products(&v, &p, 2)
            .iter()
            .all(|x| x.is_zero()));
        assert_eq!(
            v.into_iter().product::<i32>(),
            p[p.len() - nums.1..].iter().copied().product(),
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
            v.into_iter().product::<i32>(),
            p[p.len() - nums.1..].iter().copied().product(),
        );
    }
}
