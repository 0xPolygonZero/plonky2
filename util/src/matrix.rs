use std::iter::{DoubleEndedIterator, ExactSizeIterator, FusedIterator, Iterator};
use std::mem::{ManuallyDrop, MaybeUninit};
use std::ops::{Index, IndexMut};
use std::ptr;

/// Calculates the number of elements in a matrix with the given height and width, panicking on
/// overflow.
fn size_checked(height: usize, width: usize) -> usize {
    height.checked_mul(width).expect("memory overflow")
}

/// Returns an uninitialized boxed slice of length `len`.
fn uninit_box<T>(len: usize) -> Box<[MaybeUninit<T>]> {
    let mut buf: Vec<MaybeUninit<T>> = Vec::with_capacity(len);
    debug_assert_eq!(buf.capacity(), len);
    unsafe {
        // SAFETY: `with_capacity` guarantees that `buf.capacity() == len`, so we're not violating
        // memory rules. `MaybeUninit` doesn't need initialization, so the values are already valid.
        buf.set_len(len);
    }
    buf.into_boxed_slice()
}

/// A non-resizeable matrix.
#[derive(Clone)]
pub struct Matrix<T> {
    data: Box<[T]>,
    height: usize,
    width: usize,
}

impl<T> Matrix<T> {
    /// Creates a `Matrix<T>` from a boxed slice. The boxed slice is interpreted as a storing the
    /// matrix in a row-major order. In other words, `data` is split into contiguous chunks, with
    /// each chunk treated as a row of the matrix.
    pub fn from_flat_box(height: usize, width: usize, data: Box<[T]>) -> Self {
        // NB: We ask for the height _and_ the width to handle edge cases when `data.len() == 0`.

        let size = size_checked(height, width);
        assert_eq!(
            size,
            data.len(),
            "matrix dimensions do not match boxed slice length"
        );

        Self {
            data,
            height,
            width,
        }
    }

    /// Creates a `Matrix<T>` from a vector. The vector is interpreted as a storing the matrix in a
    /// row-major order. In other words, `v` is split into contiguous chunks, with each chunk
    /// treated as a row of the matrix.
    ///
    /// This is a convenience wrapper of `from_flat_box`.
    pub fn from_flat_vec(height: usize, width: usize, v: Vec<T>) -> Self {
        Self::from_flat_box(height, width, v.into_boxed_slice())
    }

    /// Creates an uninitialized matrix of specified height and width.
    pub fn new_uninit(height: usize, width: usize) -> Matrix<MaybeUninit<T>> {
        let size = size_checked(height, width);
        let buf_box = uninit_box::<T>(size);
        Matrix::from_flat_box(height, width, buf_box)
    }

    /// The height of the matrix.
    pub fn height(&self) -> usize {
        self.height
    }

    /// The width of the matrix.
    pub fn width(&self) -> usize {
        self.width
    }

    /// The total size of the matrix, in terms of instances of `T`.
    pub fn size(&self) -> usize {
        debug_assert_eq!(self.height * self.width, self.data.len());
        self.data.len()
    }

    /// Get a reference to the matrix as a row-major slice.
    pub fn as_flat(&self) -> &[T] {
        &self.data
    }

    /// Get a mutable reference to the matrix as a row-major slice.
    pub fn as_flat_mut(&mut self) -> &mut [T] {
        &mut self.data
    }

    /// Get a reference to a row of the matrix as a slice, returning `None` if out of bounds.
    pub fn get(&self, index: usize) -> Option<&[T]> {
        self.as_flat()
            .get(index * self.width..(index + 1) * self.width)
    }

    /// Get a mutable reference to a row of the matrix as a slice, returning `None` if out of
    /// bounds.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut [T]> {
        let width = self.width; // borrow checker is not very smart
        self.as_flat_mut()
            .get_mut(index * width..(index + 1) * width)
    }

    /// Get a reference to a row of the matrix as a slice, without bounds checks.
    ///
    /// # Safety
    ///
    /// The `index` must be valid, i.e. `index < matrix.height()`.
    pub unsafe fn get_unchecked(&self, index: usize) -> &[T] {
        self.as_flat()
            .get_unchecked(index * self.width..(index + 1) * self.width)
    }

    /// Get a mutable reference to a row of the matrix as a slice, without bounds checks.
    ///
    /// # Safety
    ///
    /// The `index` must be valid, i.e. `index < matrix.height()`.
    pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> &mut [T] {
        let width = self.width; // borrow checker again
        self.as_flat_mut()
            .get_unchecked_mut(index * width..(index + 1) * width)
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

        let mut res = Matrix::new_uninit(width, height);

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

        unsafe {
            // SAFETY: All elements are initialized above.
            res.assume_init()
        }
    }

    /// Transmute each element of the matrix from `T` to `U`.
    ///
    /// # Safety
    ///
    /// Due to interactions with the allocator, this method has stricter requirements than the
    /// `mem::transmute` function in the library.
    ///
    /// Same as `mem::transmute`:
    /// - `T` and `U` must have the same size.
    /// - All elements of the matrix must be safe to interpret bitwise as `U`.
    ///
    /// Furthermore:
    /// - `T` and `U` must have the same alignment.
    unsafe fn transmute<U>(self) -> Matrix<U> {
        let Matrix {
            data,
            height,
            width,
        } = self;
        let size = data.len();

        let ptr: *mut [T] = Box::into_raw(data);
        // Warning: we cannot panic from now until we've constructed another box. The buffer is
        // currently not owned by any object, so it would leak if we have to unwind the stack.

        // Rust inexplicably can't cast to a fat pointer (??) so we cast to a thin pointer to `U`,
        // then we construct a fat pointer to `U`. (lol)
        let ptr: *mut U = ptr.cast::<U>();
        let ptr: *mut [U] = ptr::slice_from_raw_parts_mut(ptr, size);

        // SAFETY: The caller guarantees that the preconditions hold.
        let data: Box<[U]> = Box::from_raw(ptr);

        Matrix {
            data,
            height,
            width,
        }
    }

    /// Wrap elements of the matrix in `ManuallyDrop`, preventing their drop code being run when
    /// the matrix is dropped.
    ///
    /// Although this function is safe, it can cause memory leaks. To avoid them, ensure that all
    /// resources are correctly dropped in some other way.
    fn manually_drop_elements(self) -> Matrix<ManuallyDrop<T>> {
        // SAFETY: `ManuallyDrop<T>` has the same layout as `T`. `T` can be safely reinterpreted as
        // `ManuallyDrop<T>`.
        unsafe { self.transmute::<ManuallyDrop<T>>() }
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
        // SAFETY: `MaybeUninit<T>` has the same layout as `T`. `MaybeUninit<T>` can be safely
        // reinterpreted as `T` when initialized. The caller guarantees that initialization took
        // place.
        self.transmute::<T>()
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
        let m = {
            let mut m = Matrix::new_uninit(WIDTH, HEIGHT);
            for i in 0..HEIGHT {
                for j in 0..WIDTH {
                    m[j][i].write((WIDTH * i + j) as u64);
                }
            }
            unsafe { m.assume_init() }
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
