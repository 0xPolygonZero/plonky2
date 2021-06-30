use std::iter::Product;

pub fn partial_products<T: Product + Copy>(v: Vec<T>, max_degree: usize) -> (Vec<T>, Vec<T>) {
    let mut res = Vec::new();
    let mut remainder = v;
    while remainder.len() >= max_degree {
        let new_partials = remainder
            .chunks(max_degree)
            // TODO: If `chunk.len()=1`, there's some redundant data.
            .map(|chunk| chunk.iter().copied().product())
            .collect::<Vec<_>>();
        res.extend_from_slice(&new_partials);
        remainder = new_partials;
    }

    (res, remainder)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_partial_products() {
        assert_eq!(
            partial_products(vec![1, 2, 3, 4, 5, 6], 2),
            (vec![2, 12, 30, 24, 30], vec![24, 30])
        );
    }
}
