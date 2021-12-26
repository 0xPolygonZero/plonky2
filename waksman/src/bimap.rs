use std::collections::HashMap;
use std::hash::Hash;

use bimap::BiMap;

/// Given two lists which are permutations of one another, creates a BiMap which maps an index in
/// one list to an index in the other list with the same associated value.
///
/// If the lists contain duplicates, then multiple permutations with this property exist, and an
/// arbitrary one of them will be returned.
pub fn bimap_from_lists<T: Eq + Hash>(a: Vec<T>, b: Vec<T>) -> BiMap<usize, usize> {
    assert_eq!(a.len(), b.len(), "Vectors differ in length");

    let mut b_values_to_indices = HashMap::new();
    for (i, value) in b.iter().enumerate() {
        b_values_to_indices
            .entry(value)
            .or_insert_with(Vec::new)
            .push(i);
    }

    let mut bimap = BiMap::new();
    for (i, value) in a.iter().enumerate() {
        if let Some(j) = b_values_to_indices.get_mut(&value).and_then(Vec::pop) {
            bimap.insert(i, j);
        } else {
            panic!("Value in first list not found in second list");
        }
    }

    bimap
}

#[cfg(test)]
mod tests {
    use crate::bimap::bimap_from_lists;

    #[test]
    fn empty_lists() {
        let empty: Vec<char> = Vec::new();
        let bimap = bimap_from_lists(empty.clone(), empty);
        assert!(bimap.is_empty());
    }

    #[test]
    fn without_duplicates() {
        let bimap = bimap_from_lists(vec!['a', 'b', 'c'], vec!['b', 'c', 'a']);
        assert_eq!(bimap.get_by_left(&0), Some(&2));
        assert_eq!(bimap.get_by_left(&1), Some(&0));
        assert_eq!(bimap.get_by_left(&2), Some(&1));
    }

    #[test]
    fn with_duplicates() {
        let first = vec!['a', 'a', 'b'];
        let second = vec!['a', 'b', 'a'];
        let bimap = bimap_from_lists(first.clone(), second.clone());
        for i in 0..3 {
            let j = *bimap.get_by_left(&i).unwrap();
            assert_eq!(first[i], second[j]);
        }
    }

    #[test]
    #[should_panic]
    fn lengths_differ() {
        bimap_from_lists(vec!['a', 'a', 'b'], vec!['a', 'b']);
    }

    #[test]
    #[should_panic]
    fn not_a_permutation() {
        bimap_from_lists(vec!['a', 'a', 'b'], vec!['a', 'b', 'b']);
    }
}
