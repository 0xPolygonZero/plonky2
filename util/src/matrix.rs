use std::alloc;
use std::iter::{DoubleEndedIterator, ExactSizeIterator, FusedIterator, Iterator};
use std::marker::PhantomData;
use std::mem::{ManuallyDrop, MaybeUninit};
use std::ops::{Index, IndexMut};
use std::ptr;
use std::slice;

pub struct Matrix<T> {
    data: *mut T,
    height: usize,
    width: usize,
    _phantom: PhantomData<T>,
}

impl<T> Matrix<T> {
    unsafe fn new_raw(data: *mut T, height: usize, width: usize) -> Self {
        Matrix {
            data,
            height,
            width,
            _phantom: PhantomData,
        }
    }

    pub fn new_uninit(height: usize, width: usize) -> Matrix<MaybeUninit<T>> {
        let size = height
            .checked_mul(width)
            .expect("arithmetic overflow when allocating matrix");
        let layout =
            alloc::Layout::array::<T>(size).expect("unable to construct `Layout` for matrix");
        let data = unsafe { alloc::alloc(layout) };
        if data.is_null() {
            alloc::handle_alloc_error(layout);
        }
        unsafe { Matrix::new_raw(data.cast(), height, width) }
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

    pub fn from_flat_vec(height: usize, width: usize, v: Vec<T>) -> Self {
        // NB: We ask for the height _and_ the width to handle edge cases when `v.len() == 0`.

        let size = height
            .checked_mul(width)
            .expect("arithmetic overflow when creating matrix");
        assert_eq!(
            size,
            v.len(),
            "matrix dimensions ({}x{}={}) do not match vector length ({})",
            height,
            width,
            size,
            v.len()
        );

        // Get the buffer as a box. This is guaranteed to drop excess capacity; this is important
        // because to correctly deallocate memory we need to know its allocated size.
        let buf = v.into_boxed_slice();

        unsafe { Self::new_raw(Box::into_raw(buf).cast(), height, width) }
    }

    pub const fn height(&self) -> usize {
        self.height
    }

    pub const fn width(&self) -> usize {
        self.width
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
        let row_ptr = self.data.add(index * self.width);
        slice::from_raw_parts(row_ptr, self.width)
    }

    pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> &mut [T] {
        let row_ptr = self.data.add(index * self.width);
        slice::from_raw_parts_mut(row_ptr, self.width)
    }

    pub fn iter(&self) -> MatrixIterator<T> {
        MatrixIterator::new(self.data, self.height, self.width)
    }

    pub fn iter_mut(&mut self) -> MatrixIteratorMut<T> {
        MatrixIteratorMut::new(self.data, self.height, self.width)
    }

    pub fn transpose(self) -> Self {
        todo!();
    }
}

impl<T> Matrix<MaybeUninit<T>> {
    pub unsafe fn assume_init(self) -> Matrix<T> {
        let me = ManuallyDrop::new(self);
        Matrix::new_raw(me.data.cast(), me.height, me.width)
    }
}

impl<T: Clone> Clone for Matrix<T> {
    fn clone(&self) -> Self {
        todo!();
    }

    fn clone_from(&mut self, _source: &Self) {
        todo!();
    }
}

impl<T> Drop for Matrix<T> {
    fn drop(&mut self) {
        let size = self
            .height
            .checked_mul(self.width)
            .expect("arithmetic overflow when dropping matrix");
        unsafe {
            // Recursively drop
            ptr::drop_in_place(ptr::slice_from_raw_parts_mut(self.data, size));
        }
        let layout =
            alloc::Layout::array::<T>(size).expect("unable to construct `Layout` for matrix");
        unsafe {
            alloc::dealloc(self.data.cast(), layout);
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
    fn from_iter<I: IntoIterator<Item = R>>(_iter: I) -> Self {
        todo!();
    }
}

pub struct MatrixIterator<'a, T> {
    _data: *const T,
    height: usize,
    _width: usize,
    _phantom: PhantomData<&'a Matrix<T>>,
}

impl<T> MatrixIterator<'_, T> {
    pub(self) fn new(_data: *const T, _height: usize, _width: usize) -> Self {
        todo!();
    }
}

impl<'a, T> Iterator for MatrixIterator<'a, T> {
    type Item = &'a [T];

    fn next(&mut self) -> Option<Self::Item> {
        todo!();
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl<T> DoubleEndedIterator for MatrixIterator<'_, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        todo!();
    }
}

impl<T> FusedIterator for MatrixIterator<'_, T> {}

impl<T> ExactSizeIterator for MatrixIterator<'_, T> {
    fn len(&self) -> usize {
        self.height
    }
}

pub struct MatrixIteratorMut<'a, T> {
    _data: *mut T,
    height: usize,
    _width: usize,
    _phantom: PhantomData<&'a mut Matrix<T>>,
}

impl<T> MatrixIteratorMut<'_, T> {
    pub(self) fn new(_data: *const T, _height: usize, _width: usize) -> Self {
        todo!();
    }
}

impl<'a, T> Iterator for MatrixIteratorMut<'a, T> {
    type Item = &'a mut [T];

    fn next(&mut self) -> Option<Self::Item> {
        todo!();
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl<T> DoubleEndedIterator for MatrixIteratorMut<'_, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        todo!();
    }
}

impl<T> FusedIterator for MatrixIteratorMut<'_, T> {}

impl<T> ExactSizeIterator for MatrixIteratorMut<'_, T> {
    fn len(&self) -> usize {
        self.height
    }
}
