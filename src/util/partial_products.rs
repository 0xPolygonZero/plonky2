use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::field::field_types::{Field, RichField};
use crate::plonk::circuit_builder::CircuitBuilder;
use itertools::Itertools;

pub(crate) fn quotient_chunk_products<F: Field>(
    quotient_values: &[F],
    max_degree: usize,
) -> Vec<F> {
    debug_assert!(max_degree > 1);
    assert!(quotient_values.len() > 0);
    let chunk_size = max_degree;
    quotient_values.chunks(chunk_size)
        .map(|chunk| chunk.iter().copied().product())
        .collect()
}

/// Compute partial products of the original vector `v` such that all products consist of `max_degree`
/// or less elements. This is done until we've computed the product `P` of all elements in the vector.
pub(crate) fn partial_products_and_z_gx<F: Field>(z_x: F, quotient_chunk_products: &[F]) -> Vec<F> {
    assert!(quotient_chunk_products.len() > 0);
    let mut res = Vec::new();
    let mut acc = z_x;
    for &quotient_chunk_product in quotient_chunk_products {
        acc *= quotient_chunk_product;
        res.push(acc);
    }
    res
}

/// Returns a tuple `(a,b)`, where `a` is the length of the output of `partial_products()` on a
/// vector of length `n`, and `b` is the number of original elements consumed in `partial_products()`.
pub fn num_partial_products(n: usize, max_degree: usize) -> (usize, usize) {
    debug_assert!(max_degree > 1);
    let chunk_size = max_degree;
    let num_chunks = n / chunk_size;

    (num_chunks, num_chunks * chunk_size)
}

/// Checks that the partial products of `numerators/denominators` are coherent with those in `partials` by only computing
/// products of size `max_degree` or less.
pub(crate) fn check_partial_products<F: Field>(
    numerators: &[F],
    denominators: &[F],
    partials: &[F],
    z_x: F,
    max_degree: usize,
) -> Vec<F> {
    debug_assert!(max_degree > 1);
    let mut acc = z_x;
    let mut partials = partials.iter();
    let mut res = Vec::new();
    let chunk_size = max_degree;
    for (nume_chunk, deno_chunk) in numerators
        .chunks_exact(chunk_size)
        .zip_eq(denominators.chunks_exact(chunk_size))
    {
        let num_chunk_product = nume_chunk.iter().copied().product();
        let den_chunk_product = deno_chunk.iter().copied().product();
        let new_acc = *partials.next().unwrap();
        res.push(acc * num_chunk_product - new_acc * den_chunk_product);
        acc = new_acc;
    }
    debug_assert!(partials.next().is_none());

    res
}

pub(crate) fn check_partial_products_recursively<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    numerators: &[ExtensionTarget<D>],
    denominators: &[ExtensionTarget<D>],
    partials: &[ExtensionTarget<D>],
    mut acc: ExtensionTarget<D>,
    max_degree: usize,
) -> Vec<ExtensionTarget<D>> {
    debug_assert!(max_degree > 1);
    let mut partials = partials.iter();
    let mut res = Vec::new();
    let chunk_size = max_degree;
    for (nume_chunk, deno_chunk) in numerators
        .chunks_exact(chunk_size)
        .zip(denominators.chunks_exact(chunk_size))
    {
        let nume_product = builder.mul_many_extension(nume_chunk);
        let deno_product = builder.mul_many_extension(deno_chunk);
        let new_acc = *partials.next().unwrap();
        let new_acc_deno = builder.mul_extension(new_acc, deno_product);
        // Assert that new_acc*deno_product = acc * nume_product.
        res.push(builder.mul_sub_extension(acc, nume_product, new_acc_deno));
        acc = new_acc;
    }
    debug_assert!(partials.next().is_none());

    res
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::goldilocks_field::GoldilocksField;

    #[test]
    fn test_partial_products() {
        type F = GoldilocksField;
        let denominators = vec![F::ONE; 6];
        let v = field_vec(&[1, 2, 3, 4, 5, 6]);
        let quotient_chunks_prods = quotient_chunk_products(&v, 2);
        assert_eq!(quotient_chunks_prods, field_vec(&[2, 12, 30]));
        let p = partial_products_and_z_gx(F::ONE, &quotient_chunks_prods);
        assert_eq!(p, field_vec(&[2, 24, 720]));

        let nums = num_partial_products(v.len(), 2);
        assert_eq!(p.len(), nums.0);
        assert!(check_partial_products(&v, &denominators, &p, F::ONE, 2)
            .iter()
            .all(|x| x.is_zero()));
        assert_eq!(
            *p.last().unwrap() * v[nums.1..].iter().copied().product::<F>(),
            v.into_iter().product::<F>(),
        );

        let v = field_vec(&[1, 2, 3, 4, 5, 6]);
        let quotient_chunks_prods = quotient_chunk_products(&v, 3);
        assert_eq!(quotient_chunks_prods, field_vec(&[6, 120]));
        let p = partial_products_and_z_gx(F::ONE, &quotient_chunks_prods);
        assert_eq!(p, field_vec(&[6, 720]));
        let nums = num_partial_products(v.len(), 3);
        assert_eq!(p.len(), nums.0);
        assert!(check_partial_products(&v, &denominators, &p, F::ONE, 3)
            .iter()
            .all(|x| x.is_zero()));
        assert_eq!(
            *p.last().unwrap() * v[nums.1..].iter().copied().product::<F>(),
            v.into_iter().product::<F>(),
        );
    }

    fn field_vec<F: Field>(xs: &[usize]) -> Vec<F> {
        xs.iter().map(|&x| F::from_canonical_usize(x)).collect()
    }
}
