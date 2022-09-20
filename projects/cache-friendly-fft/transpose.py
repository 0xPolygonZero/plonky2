from util import lb_exact


def _swap_transpose_square(a, b):
    """Transpose two square matrices in-place and swap them.

    The matrices must be a of shape `(n, n, m)`, where the `m` dimension
    may be of arbitrary length and is not moved.
    """
    assert len(a.shape) == len(b.shape) == 3
    n = a.shape[0]
    m = a.shape[2]
    assert n == a.shape[1] == b.shape[0] == b.shape[1]
    assert m == b.shape[2]

    if n == 0:
        return
    if n == 1:
        # Swap the two matrices (transposition is a no-op).
        a = a[0, 0]
        b = b[0, 0]
        # Recall that each element of the matrix is an `m`-vector. Swap
        # all `m` elements.
        for i in range(m):
            a[i], b[i] = b[i], a[i]
        return

    half_n = n >> 1
    # Transpose and swap top-left of `a` with top-left of `b`.
    _swap_transpose_square(a[:half_n, :half_n], b[:half_n, :half_n])
    # ...top-right of `a` with bottom-left of `b`.
    _swap_transpose_square(a[:half_n, half_n:], b[half_n:, :half_n])
    # ...bottom-left of `a` with top-right of `b`.
    _swap_transpose_square(a[half_n:, :half_n], b[:half_n, half_n:])
    # ...bottom-right of `a` with bottom-right of `b`.
    _swap_transpose_square(a[half_n:, half_n:], b[half_n:, half_n:])


def transpose_square(a):
    """In-place transpose of a square matrix.

    The matrix must be a of shape `(n, n, m)`, where the `m` dimension
    may be of arbitrary length and is not moved.
    """
    if len(a.shape) != 3:
        raise ValueError("a must be a matrix of batches")
    n, n_, _ = a.shape
    if n != n_:
        raise ValueError("a must be square")
    lb_exact(n)

    if n <= 1:
        return  # Base case: no-op

    half_n = n >> 1
    # Transpose top-left quarter in-place.
    transpose_square(a[:half_n, :half_n])
    # Transpose top-right and bottom-left quarters and swap them.
    _swap_transpose_square(a[:half_n, half_n:], a[half_n:, :half_n])
    # Transpose bottom-right quarter in-place.
    transpose_square(a[half_n:, half_n:])
