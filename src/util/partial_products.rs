use std::iter::Product;

pub fn partial_products<T: Product + Copy>(v: Vec<T>, max_degree: usize) -> Vec<T> {
    let mut res = Vec::new();
    let mut remainder = v;
    while remainder.len() > max_degree {
        let new_partials = remainder
            .chunks(max_degree)
            .filter(|chunk| chunk.len() != 1) // Don't need to compute the product in this case.
            .map(|chunk| chunk.iter().copied().product())
            .collect::<Vec<_>>();
        res.extend_from_slice(&new_partials);
        remainder = new_partials;
    }

    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_partial_products() {
        assert_eq!(
            partial_products(vec![1, 2, 3, 4, 5, 6], 2),
            vec![2, 12, 30, 24]
        );
    }
}
