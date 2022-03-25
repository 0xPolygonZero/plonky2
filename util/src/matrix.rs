use std::iter::{DoubleEndedIterator, ExactSizeIterator, FusedIterator, Iterator};
use std::marker::PhantomData;
use std::mem::{ManuallyDrop, MaybeUninit};
use std::ops::{Index, IndexMut};
use std::ptr;
use std::slice;

/// Calculates the number of elements in a matrix with the given height and width, panicking on
/// overflow.
fn size_checked(height: usize, width: usize) -> usize {
    height.checked_mul(width).expect("memory overflow")
}

/// Returns an uninitialized `Vec` of lenth `len`.
fn uninit_vec<T>(len: usize) -> Vec<MaybeUninit<T>> {
    let mut res: Vec<MaybeUninit<T>> = Vec::with_capacity(len);
    unsafe {
        // SAFETY: `with_capacity` guarantees that `res.capacity() == len`, so we're not violating
        // memory rules. `MaybeUninit` doesn't need initialization, so the values are already valid.
        debug_assert_eq!(res.capacity(), len);
        res.set_len(len);
    }
    res
}

/// A non-resizeable matrix.
pub struct Matrix<T> {
    data: *mut T,
    height: usize,
    width: usize,
    _phantom: PhantomData<T>,
}

impl<T> Matrix<T> {
    /// Creates a `Matrix<T>` directly from the raw components of another matrix. The matrix takes
    /// ownership of `data` and becomes responsible for deallocating it.
    ///
    /// # Safety
    ///
    /// This is unsafe because a number of invariants are assumed and not checked:
    /// * `height * width * size_of::<T>()` must not overflow `usize`.
    /// * `data` must be correctly aligned.
    /// * `data` must point to the start of an allocation (i.e., returned by `std::alloc::alloc` or
    ///   similar), and that allocation must hold exactly `height * width` instances of `T`.
    /// * All `height * width` elements pointed to by `data` must be correctly initialized.
    ///
    /// Note that these invariants hold when `data` was allocated by a `Vec<T>` whose length and
    /// capacity both equal `height * width` (and the latter does not overflow).
    pub unsafe fn from_raw(data: *mut T, height: usize, width: usize) -> Self {
        Matrix {
            data,
            height,
            width,
            _phantom: PhantomData,
        }
    }

    /// Creates a `Matrix<T>` from a vector. The vector is interpreted as a storing the matrix in a
    /// row-major order. In other words, `v` is split into contiguous chunks, with each chunk
    /// treated as a row of the matrix.
    pub fn from_flat_vec(height: usize, width: usize, v: Vec<T>) -> Self {
        // NB: We ask for the height _and_ the width to handle edge cases when `v.len() == 0`.

        let size = size_checked(height, width);
        assert_eq!(
            size,
            v.len(),
            "matrix dimensions do not match vector length"
        );

        // Get the buffer as a box. This is guaranteed to drop excess capacity; this is important
        // because to correctly deallocate memory we need to know its allocated size.
        let buf = v.into_boxed_slice();

        // SAFETY: The pointer comes from a vector with `len()` and `capacity()` both equal to
        // `height * width`. We can assume ownership of the memory since `Box::into_raw`
        // relinguishes it.
        unsafe { Self::from_raw(Box::into_raw(buf).cast(), height, width) }
    }

    /// Creates an uninitialized matrix of specified height and width.
    pub fn new_uninit(height: usize, width: usize) -> Matrix<MaybeUninit<T>> {
        let size = size_checked(height, width);
        let buf_vec = uninit_vec::<T>(size);

        // We don't need `buf_vec.capacity() == size` for safety, it to prevents reallocation.
        debug_assert_eq!(buf_vec.capacity(), size);
        Matrix::from_flat_vec(height, width, buf_vec)
    }

    /// Creates a new matrix with a specified height and width, initialized with a provided closure.
    ///
    /// This function allocates an uninitialized matrix of with a given `height` and `width`. A
    /// reference to this uninitialized array is passed to the user-provided closure, which is
    /// responsible for initializing all elements.
    ///
    /// # Safety
    ///
    /// This function is unsafe as `f` must initialization on every element of the matrix.
    pub unsafe fn new_with<F: FnOnce(&mut Matrix<MaybeUninit<T>>)>(
        height: usize,
        width: usize,
        f: F,
    ) -> Self {
        let mut res = Self::new_uninit(height, width);
        f(&mut res);
        // SAFETY: it's up to the user to ensure that `f` correctly initializes `res`.
        res.assume_init()
    }

    /// The height of the matrix.
    pub const fn height(&self) -> usize {
        self.height
    }

    /// The width of the matrix.
    pub const fn width(&self) -> usize {
        self.width
    }

    /// The total size of the matrix, in terms of instances of `T`.
    pub const fn size(&self) -> usize {
        self.height * self.width
    }

    /// Get a reference to the matrix as a row-major slice.
    pub fn as_flat(&self) -> &[T] {
        // SAFETY: self.data is correctly aligned and of length `self.size()`.
        unsafe { slice::from_raw_parts(self.data, self.size()) }
    }

    /// Get a mutable reference to the matrix as a row-major slice.
    pub fn as_flat_mut(&mut self) -> &mut [T] {
        // SAFETY: self.data is correctly aligned and of length `self.size()`.
        unsafe { slice::from_raw_parts_mut(self.data, self.size()) }
    }

    /// Get a reference to a row of the matrix as a slice, returning `None` if out of bounds.
    pub fn get(&self, index: usize) -> Option<&[T]> {
        if index < self.height {
            // SAFETY: we just verified that `index` is valid.
            Some(unsafe { self.get_unchecked(index) })
        } else {
            None
        }
    }

    /// Get a mutable reference to a row of the matrix as a slice, returning `None` if out of
    /// bounds.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut [T]> {
        if index < self.height {
            // SAFETY: we just verified that `index` is valid.
            Some(unsafe { self.get_unchecked_mut(index) })
        } else {
            None
        }
    }

    /// Get a reference to a row of the matrix as a slice, without bounds checks.
    ///
    /// # Safety
    ///
    /// The `index` must be valid, i.e. `index < matrix.height()`.
    pub unsafe fn get_unchecked(&self, index: usize) -> &[T] {
        self.as_flat()
            .get_unchecked(index * self.width..)
            .get_unchecked(..self.width)
    }

    /// Get a mutable reference to a row of the matrix as a slice, without bounds checks.
    ///
    /// # Safety
    ///
    /// The `index` must be valid, i.e. `index < matrix.height()`.
    pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> &mut [T] {
        let width = self.width;
        self.as_flat_mut()
            .get_unchecked_mut(index * width..)
            .get_unchecked_mut(..width)
    }

    /// Iterate over all rows of the matrix as references.
    pub fn iter(
        &self,
    ) -> impl Iterator<Item = &[T]> + DoubleEndedIterator + ExactSizeIterator + FusedIterator {
        self.as_flat().chunks(self.width)
    }

    /// Iterate over all rows of the matrix as mutable references.
    pub fn iter_mut(
        &mut self,
    ) -> impl Iterator<Item = &mut [T]> + DoubleEndedIterator + ExactSizeIterator + FusedIterator
    {
        let width = self.width;
        self.as_flat_mut().chunks_mut(width)
    }

    /// Transpose the matrix.
    ///
    /// This function may re-use memory but is not required to do so.
    pub fn transpose(self) -> Self {
        todo!();
    }
}

impl<T> Matrix<MaybeUninit<T>> {
    /// Convert `Matrix<MaybeUninit<T>>` to `Matrix<T>`, under the assumption that all elements have
    /// been correcly initialized.
    ///
    /// # Safety
    ///
    /// All elements of the matrix must have been correctly initialized, such that they can be
    /// safely transmuted to `T`.
    pub unsafe fn assume_init(self) -> Matrix<T> {
        // Prevent `self` from getting dropped.
        // Warning: must ensure that we can't crash before making a new Matrix or we will leak!
        let me = ManuallyDrop::new(self);
        Matrix::from_raw(me.data.cast(), me.height, me.width)
    }
}

impl<T: Clone> Clone for Matrix<T> {
    fn clone(&self) -> Self {
        Self::from_flat_vec(self.height, self.width, self.as_flat().to_vec())
    }
}

impl<T> Drop for Matrix<T> {
    fn drop(&mut self) {
        let sized_data = ptr::slice_from_raw_parts_mut(self.data, self.size());
        unsafe {
            // Transfer ownership to a `Box`, which will deallocate when dropped.
            Box::from_raw(sized_data);
        }
    }
}

impl<T> Index<usize> for Matrix<T> {
    type Output = [T];

    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).expect("out of bounds")
    }
}

impl<T> IndexMut<usize> for Matrix<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.get_mut(index).expect("out of bounds")
    }
}

impl<T, R: IntoIterator<Item = T>> FromIterator<R> for Matrix<T> {
    /// Create a matrix from an iterator of iterators of `T`. Each inner iterator corresponds to a
    /// row of the matrix.
    fn from_iter<I: IntoIterator<Item = R>>(iter: I) -> Self {
        let mut iter = iter.into_iter();

        // We cannot create a matrix if the outer iterator is empty. Note that it is not sufficient
        // to set `height = 0` as we don't know what `width` is.
        let first_row = iter
            .next()
            .expect("cannot create matrix from empty iterator");

        // Collect contents of all iterators into a buffer. Use a `Vec` so we don't have to
        // pre-specify a capacity.
        // TODO: we can reserve capacity using `first_row.size_hint()` and `iter.size_hint()`.
        let mut buf: Vec<T> = first_row.into_iter().collect();
        // We will check that every row contains `width` elements.
        let width = buf.len();
        let mut height = 0;
        for row in iter {
            let old_len = buf.len();
            buf.extend(row);
            assert_eq!(
                buf.len() - old_len,
                width,
                "matrix rows have unequal length"
            );
            height += 1;
        }

        // Unflatten the buffer into a matrix
        Self::from_flat_vec(height, width, buf)
    }
}
