use std::iter::Product;
use std::ops::Sub;

pub fn partial_products<T: Product + Copy>(v: &[T], max_degree: usize) -> (Vec<T>, usize) {
    let mut res = Vec::new();
    let mut remainder = v.to_vec();
    while remainder.len() >= max_degree {
        let new_partials = remainder
            .chunks(max_degree)
            // TODO: If `chunk.len()=1`, there's some redundant data.
            .map(|chunk| chunk.iter().copied().product())
            .collect::<Vec<_>>();
        res.extend_from_slice(&new_partials);
        remainder = new_partials;
    }

    (res, remainder.len())
}

pub fn check_partial_products<T: Product + Copy + Sub<Output = T>>(
    v: &[T],
    partials: &[T],
    max_degree: usize,
) -> Vec<T> {
    let mut res = Vec::new();
    let mut remainder = v.to_vec();
    let mut partials = partials.to_vec();
    while remainder.len() >= max_degree {
        let products = remainder
            .chunks(max_degree)
            .map(|chunk| chunk.iter().copied().product())
            .collect::<Vec<T>>();
        res.extend(products.iter().zip(&partials).map(|(&a, &b)| a - b));
        remainder = partials.drain(..products.len()).collect();
    }

    res
}

#[cfg(test)]
mod tests {
    use num::Zero;

    use super::*;

    #[test]
    fn test_partial_products() {
        let v = vec![1, 2, 3, 4, 5, 6];
        let p = partial_products(&v, 2);
        assert_eq!(p, (vec![2, 12, 30, 24, 30, 720], 1));
        assert!(check_partial_products(&v, &p.0, 2)
            .iter()
            .all(|x| x.is_zero()));

        let v = vec![1, 2, 3, 4, 5, 6];
        let p = partial_products(&v, 3);
        assert_eq!(p, (vec![6, 120], 2));
        assert!(check_partial_products(&v, &p.0, 3)
            .iter()
            .all(|x| x.is_zero()));
    }
}
