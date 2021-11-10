use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::field::field_types::{Field, RichField};
use crate::plonk::circuit_builder::CircuitBuilder;

/// Compute partial products of the original vector `v` such that all products consist of `max_degree`
/// or less elements. This is done until we've computed the product `P` of all elements in the vector.
pub fn partial_products<F: Field>(v: &[F], max_degree: usize) -> Vec<F> {
    debug_assert!(max_degree > 1);
    let mut res = Vec::new();
    let mut acc = F::ONE;
    let chunk_size = max_degree;
    for chunk in v.chunks_exact(chunk_size) {
        acc *= chunk.iter().copied().product();
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

/// Checks that the partial products of `v` are coherent with those in `partials` by only computing
/// products of size `max_degree` or less.
pub fn check_partial_products<F: Field>(v: &[F], partials: &[F], max_degree: usize) -> Vec<F> {
    debug_assert!(max_degree > 1);
    let mut partials = partials.iter();
    let mut res = Vec::new();
    let mut acc = F::ONE;
    let chunk_size = max_degree;
    for chunk in v.chunks_exact(chunk_size) {
        acc *= chunk.iter().copied().product();
        let new_acc = *partials.next().unwrap();
        res.push(acc - new_acc);
        acc = new_acc;
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
    let mut acc = builder.one_extension();
    let chunk_size = max_degree;
    for chunk in v.chunks_exact(chunk_size) {
        let chunk_product = builder.mul_many_extension(chunk);
        let new_acc = *partials.next().unwrap();
        // Assert that new_acc = acc * chunk_product.
        res.push(builder.mul_sub_extension(acc, chunk_product, new_acc));
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
        let v = [1, 2, 3, 4, 5, 6]
            .into_iter()
            .map(|&i| F::from_canonical_u64(i))
            .collect::<Vec<_>>();
        let p = partial_products(&v, 2);
        assert_eq!(
            p,
            [2, 24, 720]
                .into_iter()
                .map(|&i| F::from_canonical_u64(i))
                .collect::<Vec<_>>()
        );

        let nums = num_partial_products(v.len(), 2);
        assert_eq!(p.len(), nums.0);
        assert!(check_partial_products(&v, &p, 2)
            .iter()
            .all(|x| x.is_zero()));
        assert_eq!(
            *p.last().unwrap() * v[nums.1..].iter().copied().product::<F>(),
            v.into_iter().product::<F>(),
        );

        let v = [1, 2, 3, 4, 5, 6]
            .into_iter()
            .map(|&i| F::from_canonical_u64(i))
            .collect::<Vec<_>>();
        let p = partial_products(&v, 3);
        assert_eq!(
            p,
            [6, 720]
                .into_iter()
                .map(|&i| F::from_canonical_u64(i))
                .collect::<Vec<_>>()
        );
        let nums = num_partial_products(v.len(), 3);
        assert_eq!(p.len(), nums.0);
        assert!(check_partial_products(&v, &p, 3)
            .iter()
            .all(|x| x.is_zero()));
        assert_eq!(
            *p.last().unwrap() * v[nums.1..].iter().copied().product::<F>(),
            v.into_iter().product::<F>(),
        );
    }
}
