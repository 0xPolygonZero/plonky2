use core::marker::PhantomData;
use core::mem::size_of;
use core::ops::{Index, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive};

use crate::field::packed::PackedField;

/// Imagine a slice, but with a stride (a la a NumPy array).
///
/// For example, if the stride is 3,
///     `packed_strided_view[0]` is `data[0]`,
///     `packed_strided_view[1]` is `data[3]`,
///     `packed_strided_view[2]` is `data[6]`,
/// and so on. An offset may be specified. With an offset of 1, we get
///     `packed_strided_view[0]` is `data[1]`,
///     `packed_strided_view[1]` is `data[4]`,
///     `packed_strided_view[2]` is `data[7]`,
/// and so on.
///
/// Additionally, this view is *packed*, which means that it may yield a packing of the underlying
/// field slice. With a packing of width 4 and a stride of 5, the accesses are
///     `packed_strided_view[0]` is `data[0..4]`, transmuted to the packing,
///     `packed_strided_view[1]` is `data[5..9]`, transmuted to the packing,
///     `packed_strided_view[2]` is `data[10..14]`, transmuted to the packing,
/// and so on.
#[derive(Debug, Copy, Clone)]
pub struct PackedStridedView<'a, P: PackedField> {
    // This type has to be a struct, which means that it is not itself a reference (in the sense
    // that a slice is a reference so we can return it from e.g. `Index::index`).

    // Raw pointers rarely appear in good Rust code, but I think this is the most elegant way to
    // implement this. The alternative would be to replace `start_ptr` and `length` with one slice
    // (`&[P::Scalar]`). Unfortunately, with a slice, an empty view becomes an edge case that
    // necessitates separate handling. It _could_ be done but it would also be uglier.
    start_ptr: *const P::Scalar,
    /// This is the total length of elements accessible through the view. In other words, valid
    /// indices are in `0..length`.
    length: usize,
    /// This stride is in units of `P::Scalar` (NOT in bytes and NOT in `P`).
    stride: usize,
    _phantom: PhantomData<&'a [P::Scalar]>,
}

impl<'a, P: PackedField> PackedStridedView<'a, P> {
    // `wrapping_add` is needed throughout to avoid undefined behavior. Plain `add` causes UB if
    // '[either] the starting [or] resulting pointer [is neither] in bounds or one byte past the
    // end of the same allocated object'; the UB results even if the pointer is not dereferenced.

    #[inline]
    pub fn new(data: &'a [P::Scalar], stride: usize, offset: usize) -> Self {
        assert!(
            stride >= P::WIDTH,
            "stride (got {}) must be at least P::WIDTH ({})",
            stride,
            P::WIDTH
        );
        assert_eq!(
            data.len() % stride,
            0,
            "data.len() ({}) must be a multiple of stride (got {})",
            data.len(),
            stride
        );

        // This requirement means that stride divides data into slices of `data.len() / stride`
        // elements. Every access must fit entirely within one of those slices.
        assert!(
            offset + P::WIDTH <= stride,
            "offset (got {}) + P::WIDTH ({}) cannot be greater than stride (got {})",
            offset,
            P::WIDTH,
            stride
        );

        // See comment above. `start_ptr` will be more than one byte past the buffer if `data` has
        // length 0 and `offset` is not 0.
        let start_ptr = data.as_ptr().wrapping_add(offset);

        Self {
            start_ptr,
            length: data.len() / stride,
            stride,
            _phantom: PhantomData,
        }
    }

    #[inline]
    pub const fn get(&self, index: usize) -> Option<&'a P> {
        if index < self.length {
            // Cast scalar pointer to vector pointer.
            let res_ptr = unsafe { self.start_ptr.add(index * self.stride) }.cast();
            // This transmutation is safe by the spec in `PackedField`.
            Some(unsafe { &*res_ptr })
        } else {
            None
        }
    }

    /// Take a range of `PackedStridedView` indices, as `PackedStridedView`.
    #[inline]
    pub fn view<I>(&self, index: I) -> Self
    where
        Self: Viewable<I, View = Self>,
    {
        // We cannot implement `Index` as `PackedStridedView` is a struct, not a reference.

        // The `Viewable` trait is needed for overloading.
        // Re-export `Viewable::view` so users don't have to import `Viewable`.
        <Self as Viewable<I>>::view(self, index)
    }

    #[inline]
    pub const fn iter(&self) -> PackedStridedViewIter<'a, P> {
        PackedStridedViewIter::new(
            self.start_ptr,
            // See comment at the top of the `impl`. Below will point more than one byte past the
            // end of the buffer (unless `offset` is 0) so `wrapping_add` is needed.
            self.start_ptr.wrapping_add(self.length * self.stride),
            self.stride,
        )
    }

    #[inline]
    pub const fn len(&self) -> usize {
        self.length
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<P: PackedField> Index<usize> for PackedStridedView<'_, P> {
    type Output = P;
    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        self.get(index)
            .expect("invalid memory access in PackedStridedView")
    }
}

impl<'a, P: PackedField> IntoIterator for PackedStridedView<'a, P> {
    type Item = &'a P;
    type IntoIter = PackedStridedViewIter<'a, P>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TryFromPackedStridedViewError;

impl<P: PackedField, const N: usize> TryInto<[P; N]> for PackedStridedView<'_, P> {
    type Error = TryFromPackedStridedViewError;
    fn try_into(self) -> Result<[P; N], Self::Error> {
        if N == self.len() {
            let mut res = [P::default(); N];
            for i in 0..N {
                res[i] = *self.get(i).unwrap();
            }
            Ok(res)
        } else {
            Err(TryFromPackedStridedViewError)
        }
    }
}

// Not deriving `Copy`. An implicit copy of an iterator is likely a bug.
#[derive(Clone, Debug)]
pub struct PackedStridedViewIter<'a, P: PackedField> {
    // Again, a pair of pointers is a neater solution than a slice. `start` and `end` are always
    // separated by a multiple of stride elements. To advance the iterator from the front, we
    // advance `start` by `stride` elements. To advance it from the end, we subtract `stride`
    // elements. Iteration is done when they meet.
    // A slice cannot recreate the same pattern. The end pointer may point past the underlying
    // buffer (this is okay as we do not dereference it in that case); it becomes valid as soon as
    // it is decreased by `stride`. On the other hand, a slice that ends on invalid memory is
    // instant undefined behavior.
    start: *const P::Scalar,
    end: *const P::Scalar,
    stride: usize,
    _phantom: PhantomData<&'a [P::Scalar]>,
}

impl<P: PackedField> PackedStridedViewIter<'_, P> {
    pub(self) const fn new(start: *const P::Scalar, end: *const P::Scalar, stride: usize) -> Self {
        Self {
            start,
            end,
            stride,
            _phantom: PhantomData,
        }
    }
}

impl<'a, P: PackedField> Iterator for PackedStridedViewIter<'a, P> {
    type Item = &'a P;
    fn next(&mut self) -> Option<Self::Item> {
        debug_assert_eq!(
            (self.end as usize).wrapping_sub(self.start as usize)
                % (self.stride * size_of::<P::Scalar>()),
            0,
            "start and end pointers should be separated by a multiple of stride"
        );

        if !core::ptr::eq(self.start, self.end) {
            let res = unsafe { &*self.start.cast() };
            // See comment in `PackedStridedView`. Below will point more than one byte past the end
            // of the buffer if the offset is not 0 and we've reached the end.
            self.start = self.start.wrapping_add(self.stride);
            Some(res)
        } else {
            None
        }
    }
}

impl<P: PackedField> DoubleEndedIterator for PackedStridedViewIter<'_, P> {
    fn next_back(&mut self) -> Option<Self::Item> {
        debug_assert_eq!(
            (self.end as usize).wrapping_sub(self.start as usize)
                % (self.stride * size_of::<P::Scalar>()),
            0,
            "start and end pointers should be separated by a multiple of stride"
        );

        if !core::ptr::eq(self.start, self.end) {
            // See comment in `PackedStridedView`. `self.end` starts off pointing more than one byte
            // past the end of the buffer unless `offset` is 0.
            self.end = self.end.wrapping_sub(self.stride);
            Some(unsafe { &*self.end.cast() })
        } else {
            None
        }
    }
}

pub trait Viewable<F> {
    // We cannot implement `Index` as `PackedStridedView` is a struct, not a reference.
    type View;
    fn view(&self, index: F) -> Self::View;
}

impl<P: PackedField> Viewable<Range<usize>> for PackedStridedView<'_, P> {
    type View = Self;
    fn view(&self, range: Range<usize>) -> Self::View {
        assert!(range.start <= self.len(), "Invalid access");
        assert!(range.end <= self.len(), "Invalid access");
        Self {
            // See comment in `PackedStridedView`. `self.start_ptr` will point more than one byte
            // past the end of the buffer if the offset is not 0 and the buffer has length 0.
            start_ptr: self.start_ptr.wrapping_add(self.stride * range.start),
            length: range.end - range.start,
            stride: self.stride,
            _phantom: PhantomData,
        }
    }
}

impl<P: PackedField> Viewable<RangeFrom<usize>> for PackedStridedView<'_, P> {
    type View = Self;
    fn view(&self, range: RangeFrom<usize>) -> Self::View {
        assert!(range.start <= self.len(), "Invalid access");
        Self {
            // See comment in `PackedStridedView`. `self.start_ptr` will point more than one byte
            // past the end of the buffer if the offset is not 0 and the buffer has length 0.
            start_ptr: self.start_ptr.wrapping_add(self.stride * range.start),
            length: self.len() - range.start,
            stride: self.stride,
            _phantom: PhantomData,
        }
    }
}

impl<P: PackedField> Viewable<RangeFull> for PackedStridedView<'_, P> {
    type View = Self;
    fn view(&self, _range: RangeFull) -> Self::View {
        *self
    }
}

impl<P: PackedField> Viewable<RangeInclusive<usize>> for PackedStridedView<'_, P> {
    type View = Self;
    fn view(&self, range: RangeInclusive<usize>) -> Self::View {
        assert!(*range.start() <= self.len(), "Invalid access");
        assert!(*range.end() < self.len(), "Invalid access");
        Self {
            // See comment in `PackedStridedView`. `self.start_ptr` will point more than one byte
            // past the end of the buffer if the offset is not 0 and the buffer has length 0.
            start_ptr: self.start_ptr.wrapping_add(self.stride * range.start()),
            length: range.end() - range.start() + 1,
            stride: self.stride,
            _phantom: PhantomData,
        }
    }
}

impl<P: PackedField> Viewable<RangeTo<usize>> for PackedStridedView<'_, P> {
    type View = Self;
    fn view(&self, range: RangeTo<usize>) -> Self::View {
        assert!(range.end <= self.len(), "Invalid access");
        Self {
            start_ptr: self.start_ptr,
            length: range.end,
            stride: self.stride,
            _phantom: PhantomData,
        }
    }
}

impl<P: PackedField> Viewable<RangeToInclusive<usize>> for PackedStridedView<'_, P> {
    type View = Self;
    fn view(&self, range: RangeToInclusive<usize>) -> Self::View {
        assert!(range.end < self.len(), "Invalid access");
        Self {
            start_ptr: self.start_ptr,
            length: range.end + 1,
            stride: self.stride,
            _phantom: PhantomData,
        }
    }
}
