def lb_exact(n):
    """Returns `log2(n)`, raising if `n` is not a power of 2."""
    lb = n.bit_length() - 1
    if lb < 0 or n != 1 << lb:
        raise ValueError(f"{n} is not a power of 2")
    return lb
