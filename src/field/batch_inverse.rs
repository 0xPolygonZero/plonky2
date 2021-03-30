use crate::field::field::Field;

fn batch_multiplicative_inverse<F: Field>(x: &[F]) -> Vec<F> {
    // This is Montgomery's trick. At a high level, we invert the product of the given field
    // elements, then derive the individual inverses from that via multiplication.

    let n = x.len();
    if n == 0 {
        return Vec::new();
    }

    let mut a = Vec::with_capacity(n);
    a.push(x[0]);
    for i in 1..n {
        a.push(a[i - 1] * x[i]);
    }

    let mut a_inv = vec![F::ZERO; n];
    a_inv[n - 1] = a[n - 1].try_inverse().expect("No inverse");
    for i in (0..n - 1).rev() {
        a_inv[i] = x[i + 1] * a_inv[i + 1];
    }

    let mut x_inv = Vec::with_capacity(n);
    x_inv.push(a_inv[0]);
    for i in 1..n {
        x_inv.push(a[i - 1] * a_inv[i]);
    }
    x_inv
}
