use std::iter::{DoubleEndedIterator, ExactSizeIterator, FusedIterator, Iterator};
use std::marker::PhantomData;
use std::mem::{ManuallyDrop, MaybeUninit};
use std::ops::{Index, IndexMut};
use std::ptr;
use std::slice;

fn size_checked(height: usize, width: usize) -> usize {
    height.checked_mul(width).expect("memory overflow")
}

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

pub struct Matrix<T> {
    data: *mut T,
    height: usize,
    width: usize,
    _phantom: PhantomData<T>,
}

impl<T> Matrix<T> {
    pub unsafe fn from_raw(data: *mut T, height: usize, width: usize) -> Self {
        Matrix {
            data,
            height,
            width,
            _phantom: PhantomData,
        }
    }

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

        unsafe { Self::from_raw(Box::into_raw(buf).cast(), height, width) }
    }

    pub fn new_uninit(height: usize, width: usize) -> Matrix<MaybeUninit<T>> {
        let size = size_checked(height, width);
        let buf_vec = uninit_vec::<T>(size);

        // We don't need `buf_vec.capacity() == size` for safety, it to prevents reallocation.
        debug_assert_eq!(buf_vec.capacity(), size);
        Matrix::from_flat_vec(height, width, buf_vec)
    }

    pub unsafe fn new_with<F: FnOnce(&mut Matrix<MaybeUninit<T>>)>(
        height: usize,
        width: usize,
        f: F,
    ) -> Self {
        let mut res = Self::new_uninit(height, width);
        f(&mut res);
        res.assume_init()
    }

    pub const fn height(&self) -> usize {
        self.height
    }

    pub const fn width(&self) -> usize {
        self.width
    }

    pub const fn size(&self) -> usize {
        self.height * self.width
    }

    pub fn as_flat(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.data, self.size()) }
    }

    pub fn as_flat_mut(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(self.data, self.size()) }
    }

    pub fn get(&self, index: usize) -> Option<&[T]> {
        if index < self.height {
            Some(unsafe { self.get_unchecked(index) })
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut [T]> {
        if index < self.height {
            Some(unsafe { self.get_unchecked_mut(index) })
        } else {
            None
        }
    }

    pub unsafe fn get_unchecked(&self, index: usize) -> &[T] {
        self.as_flat()
            .get_unchecked(index * self.width..)
            .get_unchecked(..self.width)
    }

    pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> &mut [T] {
        let width = self.width;
        self.as_flat_mut()
            .get_unchecked_mut(index * width..)
            .get_unchecked_mut(..width)
    }

    pub fn iter<'a>(
        &'a self,
    ) -> impl Iterator<Item = &'a [T]> + DoubleEndedIterator + ExactSizeIterator + FusedIterator
    {
        self.as_flat().chunks(self.width)
    }

    pub fn iter_mut<'a>(
        &'a mut self,
    ) -> impl Iterator<Item = &'a mut [T]> + DoubleEndedIterator + ExactSizeIterator + FusedIterator
    {
        let width = self.width;
        self.as_flat_mut().chunks_mut(width)
    }

    pub fn transpose(self) -> Self {
        todo!();
    }
}

impl<T> Matrix<MaybeUninit<T>> {
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
    fn from_iter<I: IntoIterator<Item = R>>(iter: I) -> Self {
        let mut iter = iter.into_iter();
        let first_row = iter
            .next()
            .expect("cannot create matrix from empty iterator");
        let mut buf: Vec<T> = first_row.into_iter().collect();
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
        Self::from_flat_vec(height, width, buf)
    }
}
