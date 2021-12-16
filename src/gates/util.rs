use std::marker::PhantomData;

use crate::field::field_types::Field;

/// Writes constraints yielded by a gate to a buffer, with a given stride.
/// Permits us to abstract the underlying memory layout. In particular, we can make a matrix of
/// constraints where every column is an evaluation point and every row is a constraint index, with
/// the matrix stored in row-contiguous form.
pub struct StridedConstraintConsumer<'a, F: Field> {
    // This is a particularly neat way of doing this, more so than a slice. We increase start by
    // stride at every step and terminate when it equals end.
    start: *mut F,
    end: *mut F,
    stride: usize,
    _phantom: PhantomData<&'a mut [F]>,
}

impl<'a, F: Field> StridedConstraintConsumer<'a, F> {
    pub fn new(buffer: &'a mut [F], stride: usize, offset: usize) -> Self {
        assert!(offset < stride);
        assert_eq!(buffer.len() % stride, 0);
        let ptr_range = buffer.as_mut_ptr_range();
        // `wrapping_add` is needed to avoid undefined behavior. Plain `add` causes UB if 'the ...
        // resulting pointer [is neither] in bounds or one byte past the end of the same allocated
        // object'; the UB results even if the pointer is not dereferenced. `end` will be more than
        // one byte past the buffer unless `offset` is 0. The same applies to `start` if the buffer
        // has length 0 and the offset is not 0.
        // We _could_ do pointer arithmetic without `wrapping_add`, but the logic would be
        // unnecessarily complicated.
        let start = ptr_range.start.wrapping_add(offset);
        let end = ptr_range.end.wrapping_add(offset);
        Self {
            start,
            end,
            stride,
            _phantom: PhantomData,
        }
    }

    /// Emit one constraint.
    pub fn one(&mut self, constraint: F) {
        if self.start != self.end {
            // # Safety
            // The checks in `new` guarantee that this points to valid space.
            unsafe {
                *self.start = constraint;
            }
            // See the comment in `new`. `wrapping_add` is needed to avoid UB if we've just
            // exhausted our buffer (and hence we're setting `self.start` to point past the end).
            self.start = self.start.wrapping_add(self.stride);
        } else {
            panic!("gate produced too many constraints");
        }
    }

    /// Convenience method that calls `.one()` multiple times.
    pub fn many<I: IntoIterator<Item = F>>(&mut self, constraints: I) {
        constraints
            .into_iter()
            .for_each(|constraint| self.one(constraint));
    }
}
