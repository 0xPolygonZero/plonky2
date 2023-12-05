use crate::packable::Packable;
use crate::packed::PackedField;
use crate::types::Field;

const fn pack_with_leftovers_split_point<P: PackedField>(slice: &[P::Scalar]) -> usize {
    let n = slice.len();
    let n_leftover = n % P::WIDTH;
    n - n_leftover
}

fn pack_slice_with_leftovers<P: PackedField>(slice: &[P::Scalar]) -> (&[P], &[P::Scalar]) {
    let split_point = pack_with_leftovers_split_point::<P>(slice);
    let (slice_packable, slice_leftovers) = slice.split_at(split_point);
    let slice_packed = P::pack_slice(slice_packable);
    (slice_packed, slice_leftovers)
}

fn pack_slice_with_leftovers_mut<P: PackedField>(
    slice: &mut [P::Scalar],
) -> (&mut [P], &mut [P::Scalar]) {
    let split_point = pack_with_leftovers_split_point::<P>(slice);
    let (slice_packable, slice_leftovers) = slice.split_at_mut(split_point);
    let slice_packed = P::pack_slice_mut(slice_packable);
    (slice_packed, slice_leftovers)
}

/// Elementwise inplace multiplication of two slices of field elements.
/// Implementation be faster than the trivial for loop.
pub fn batch_multiply_inplace<F: Field>(out: &mut [F], a: &[F]) {
    let n = out.len();
    assert_eq!(n, a.len(), "both arrays must have the same length");

    // Split out slice of vectors, leaving leftovers as scalars
    let (out_packed, out_leftovers) =
        pack_slice_with_leftovers_mut::<<F as Packable>::Packing>(out);
    let (a_packed, a_leftovers) = pack_slice_with_leftovers::<<F as Packable>::Packing>(a);

    // Multiply packed and the leftovers
    for (x_out, x_a) in out_packed.iter_mut().zip(a_packed) {
        *x_out *= *x_a;
    }
    for (x_out, x_a) in out_leftovers.iter_mut().zip(a_leftovers) {
        *x_out *= *x_a;
    }
}

/// Elementwise inplace addition of two slices of field elements.
/// Implementation be faster than the trivial for loop.
pub fn batch_add_inplace<F: Field>(out: &mut [F], a: &[F]) {
    let n = out.len();
    assert_eq!(n, a.len(), "both arrays must have the same length");

    // Split out slice of vectors, leaving leftovers as scalars
    let (out_packed, out_leftovers) =
        pack_slice_with_leftovers_mut::<<F as Packable>::Packing>(out);
    let (a_packed, a_leftovers) = pack_slice_with_leftovers::<<F as Packable>::Packing>(a);

    // Add packed and the leftovers
    for (x_out, x_a) in out_packed.iter_mut().zip(a_packed) {
        *x_out += *x_a;
    }
    for (x_out, x_a) in out_leftovers.iter_mut().zip(a_leftovers) {
        *x_out += *x_a;
    }
}
