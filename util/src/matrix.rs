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
        let height = self.height;
        let width = self.width;

        let initializer_fn = |res: &mut Matrix<MaybeUninit<T>>| {
            // We're essentially moving all contents of `self` to `res`. We need to ensure that they
            // don't get dropped when `self` gets dropped, so we wrap them in `ManuallyDrop`.
            let mut me = self.manually_drop_elements();

            // Important that we don't panic here. We've disabled the destructors of all our
            // elements, so they won't be called if we have to unwind, potentially causing a leak.

            // Easy optimization: ensure the larger loop is outside.
            unsafe {
                if width >= height {
                    for i in 0..width {
                        for j in 0..height {
                            // SAFETY: The indices are valid. The pointee of `in_ref` is not used
                            // again (so we're effectively just moving it).
                            // idk why `in_ref` has to be `mut`, but `ManuallyDrop::take` requires
                            // it.
                            let in_ref = me.get_unchecked_mut(j).get_unchecked_mut(i);
                            let out_ref = res.get_unchecked_mut(i).get_unchecked_mut(j);
                            out_ref.write(ManuallyDrop::take(in_ref));
                        }
                    }
                } else {
                    for j in 0..height {
                        for i in 0..width {
                            // SAFETY: see above.
                            let in_ref = me.get_unchecked_mut(j).get_unchecked_mut(i);
                            let out_ref = res.get_unchecked_mut(i).get_unchecked_mut(j);
                            out_ref.write(ManuallyDrop::take(in_ref));
                        }
                    }
                };
            }
        };

        unsafe {
            // SAFETY: The closure initializes every element.
            Self::new_with(width, height, initializer_fn)
        }
    }

    /// Wrap elements of the matrix in `ManuallyDrop`, preventing their drop code being run when
    /// the matrix is dropped.
    ///
    /// Although this function is safe, it can cause memory leaks. To avoid them, ensure that all
    /// resources are correctly dropped in some other way.
    fn manually_drop_elements(self) -> Matrix<ManuallyDrop<T>> {
        // Prevent `self` from getting dropped.
        // Warning: must ensure that we can't crash before making a new Matrix or we will leak!
        let me = ManuallyDrop::new(self);
        // SAFETY: The pointers and sizes are valid, as element size and alignment does not change.
        unsafe { Matrix::from_raw(me.data.cast(), me.height, me.width) }
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
        let mut height = 1;
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

#[cfg(test)]
mod tests {
    use super::*;

    const HEIGHT: usize = 3;
    const WIDTH: usize = 5;

    fn check_matrix(m: Matrix<u64>) {
        assert_eq!(m.height(), HEIGHT);
        assert_eq!(m.width(), WIDTH);
        for i in 0..HEIGHT {
            for j in 0..WIDTH {
                assert_eq!(m[i][j], (WIDTH * i + j) as u64);
            }
        }
    }

    #[test]
    fn test_from_raw() {
        let buf: Vec<u64> = (0..(HEIGHT * WIDTH) as u64).into_iter().collect();
        let m: Matrix<u64> = unsafe {
            Matrix::from_raw(Box::into_raw(buf.into_boxed_slice()).cast(), HEIGHT, WIDTH)
        };
        check_matrix(m);
    }

    #[test]
    fn test_from_flat_vec() {
        let v: Vec<u64> = (0..(HEIGHT * WIDTH) as u64).into_iter().collect();
        let m = Matrix::from_flat_vec(HEIGHT, WIDTH, v);
        check_matrix(m);
    }

    #[test]
    fn test_new_uninit() {
        let m: Matrix<MaybeUninit<u64>> = Matrix::new_uninit(HEIGHT, WIDTH);
        assert_eq!(m.height(), HEIGHT);
        assert_eq!(m.width(), WIDTH);
    }

    #[test]
    fn test_assume_init() {
        let mut m = Matrix::new_uninit(HEIGHT, WIDTH);
        for i in 0..HEIGHT {
            for j in 0..WIDTH {
                m[i][j].write((WIDTH * i + j) as u64);
            }
        }
        let m = unsafe { m.assume_init() };
        check_matrix(m);
    }

    #[test]
    #[allow(clippy::redundant_clone)]
    fn test_clone() {
        let v: Vec<u64> = (0..(HEIGHT * WIDTH) as u64).into_iter().collect();
        let m = Matrix::from_flat_vec(HEIGHT, WIDTH, v);
        let m = m.clone();
        check_matrix(m);
    }

    #[test]
    fn test_new_with() {
        let m = unsafe {
            Matrix::new_with(HEIGHT, WIDTH, |res| {
                for i in 0..HEIGHT {
                    for j in 0..WIDTH {
                        res[i][j].write((WIDTH * i + j) as u64);
                    }
                }
            })
        };
        check_matrix(m);
    }

    #[test]
    fn test_from_iter() {
        let m =
            Matrix::from_iter((0..HEIGHT).map(|i| (WIDTH * i..WIDTH * (i + 1)).map(|i| i as u64)));
        check_matrix(m);
    }

    #[test]
    fn test_iter() {
        let v: Vec<u64> = (0..(HEIGHT * WIDTH) as u64).into_iter().collect();
        let m = Matrix::from_flat_vec(HEIGHT, WIDTH, v);
        let m_iter: Vec<_> = m.iter().collect();
        assert_eq!(m_iter.len(), HEIGHT);
        assert!(m_iter.iter().map(|row| row.len() == WIDTH).all(|x| x));
        let m_iter = m_iter.into_iter().flat_map(|row| row.iter()).copied();
        assert!((0..(HEIGHT * WIDTH) as u64).into_iter().eq(m_iter));
    }

    #[test]
    fn test_iter_mut() {
        let v: Vec<u64> = (0..(HEIGHT * WIDTH) as u64).into_iter().collect();
        let mut m = Matrix::from_flat_vec(HEIGHT, WIDTH, v);
        let m_iter: Vec<_> = m.iter_mut().collect();
        assert_eq!(m_iter.len(), HEIGHT);
        assert!(m_iter.iter().map(|row| row.len() == WIDTH).all(|x| x));
        let m_iter = m_iter.into_iter().flat_map(|row| row.iter()).copied();
        assert!((0..(HEIGHT * WIDTH) as u64).into_iter().eq(m_iter));
    }

    #[test]
    fn test_size() {
        let m: Matrix<MaybeUninit<u64>> = Matrix::new_uninit(HEIGHT, WIDTH);
        assert_eq!(m.size(), HEIGHT * WIDTH);
    }

    #[test]
    fn test_transpose() {
        let m = unsafe {
            Matrix::new_with(WIDTH, HEIGHT, |res| {
                for i in 0..HEIGHT {
                    for j in 0..WIDTH {
                        res[j][i].write((WIDTH * i + j) as u64);
                    }
                }
            })
        };
        let m = m.transpose();
        check_matrix(m);
    }

    #[test]
    fn test_get() {
        let v: Vec<u64> = (0..(HEIGHT * WIDTH) as u64).into_iter().collect();
        let m = Matrix::from_flat_vec(HEIGHT, WIDTH, v);
        for i in 0..HEIGHT {
            let target: Vec<_> = (i * WIDTH..(i + 1) * WIDTH).map(|x| x as u64).collect();
            let result = m.get(i).unwrap().to_vec();
            assert_eq!(target, result);
        }
        assert_eq!(m.get(HEIGHT), None);
    }

    #[test]
    fn test_get_mut() {
        let v: Vec<u64> = (0..(HEIGHT * WIDTH) as u64).into_iter().collect();
        let mut m = Matrix::from_flat_vec(HEIGHT, WIDTH, v);
        for i in 0..HEIGHT {
            let target: Vec<_> = (i * WIDTH..(i + 1) * WIDTH).map(|x| x as u64).collect();
            let result = m.get_mut(i).unwrap().to_vec();
            assert_eq!(target, result);
        }
        assert_eq!(m.get_mut(HEIGHT), None);
    }

    #[test]
    fn test_get_unchecked() {
        let v: Vec<u64> = (0..(HEIGHT * WIDTH) as u64).into_iter().collect();
        let m = Matrix::from_flat_vec(HEIGHT, WIDTH, v);
        for i in 0..HEIGHT {
            let target: Vec<_> = (i * WIDTH..(i + 1) * WIDTH).map(|x| x as u64).collect();
            let result = unsafe { m.get_unchecked(i) }.to_vec();
            assert_eq!(target, result);
        }
    }

    #[test]
    fn test_get_unchecked_mut() {
        let v: Vec<u64> = (0..(HEIGHT * WIDTH) as u64).into_iter().collect();
        let mut m = Matrix::from_flat_vec(HEIGHT, WIDTH, v);
        for i in 0..HEIGHT {
            let target: Vec<_> = (i * WIDTH..(i + 1) * WIDTH).map(|x| x as u64).collect();
            let result = unsafe { m.get_unchecked_mut(i) }.to_vec();
            assert_eq!(target, result);
        }
    }

    #[test]
    #[should_panic]
    fn test_index_out_of_bounds() {
        let v: Vec<u64> = (0..(HEIGHT * WIDTH) as u64).into_iter().collect();
        let m = Matrix::from_flat_vec(HEIGHT, WIDTH, v);
        let _ = &m[HEIGHT];
    }

    #[test]
    #[should_panic]
    fn test_index_mut_out_of_bounds() {
        let v: Vec<u64> = (0..(HEIGHT * WIDTH) as u64).into_iter().collect();
        let mut m = Matrix::from_flat_vec(HEIGHT, WIDTH, v);
        let _ = &mut m[HEIGHT];
    }

    #[test]
    fn test_as_flat() {
        let v: Vec<u64> = (0..(HEIGHT * WIDTH) as u64).into_iter().collect();
        let m = Matrix::from_flat_vec(HEIGHT, WIDTH, v.clone());
        assert_eq!(m.as_flat().to_vec(), v);
    }

    #[test]
    fn test_as_flat_mut() {
        let v: Vec<u64> = (0..(HEIGHT * WIDTH) as u64).into_iter().collect();
        let mut m = Matrix::from_flat_vec(HEIGHT, WIDTH, v.clone());
        assert_eq!(m.as_flat_mut().to_vec(), v);
    }
}
