use std::ptr::swap;

const LB_BLOCK_SIZE: usize = 3;

unsafe fn transpose_in_place_square_small<T>(
    arr: &mut [T],
    lb_stride: usize,
    lb_size: usize,
    x: usize,
) {
    for i in x..x + (1 << lb_size) {
        for j in x..i {
            swap(
                arr.get_unchecked_mut(i + (j << lb_stride)),
                arr.get_unchecked_mut((i << lb_stride) + j),
            );
        }
    }
}

unsafe fn transpose_swap_square_small<T>(
    arr: &mut [T],
    lb_stride: usize,
    lb_size: usize,
    x: usize,
    y: usize,
) {
    for i in x..x + (1 << lb_size) {
        for j in y..y + (1 << lb_size) {
            swap(
                arr.get_unchecked_mut(i + (j << lb_stride)),
                arr.get_unchecked_mut((i << lb_stride) + j),
            );
        }
    }
}

unsafe fn transpose_swap_square<T>(
    arr: &mut [T],
    lb_stride: usize,
    lb_size: usize,
    x: usize,
    y: usize,
) {
    if lb_size <= LB_BLOCK_SIZE {
        transpose_swap_square_small(arr, lb_stride, lb_size, x, y);
    } else {
        let lb_block_size = lb_size - 1;
        let block_size = 1 << lb_block_size;
        transpose_swap_square(arr, lb_stride, lb_block_size, x, y);
        transpose_swap_square(arr, lb_stride, lb_block_size, x + block_size, y);
        transpose_swap_square(arr, lb_stride, lb_block_size, x, y + block_size);
        transpose_swap_square(
            arr,
            lb_stride,
            lb_block_size,
            x + block_size,
            y + block_size,
        );
    }
}

pub(crate) unsafe fn transpose_in_place_square<T>(
    arr: &mut [T],
    lb_stride: usize,
    lb_size: usize,
    x: usize,
) {
    if lb_size <= LB_BLOCK_SIZE {
        transpose_in_place_square_small(arr, lb_stride, lb_size, x);
    } else {
        let lb_block_size = lb_size - 1;
        let block_size = 1 << lb_block_size;
        transpose_in_place_square(arr, lb_stride, lb_block_size, x);
        transpose_swap_square(arr, lb_stride, lb_block_size, x, x + block_size);
        transpose_in_place_square(arr, lb_stride, lb_block_size, x + block_size);
    }
}
