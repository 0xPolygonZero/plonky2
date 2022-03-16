use std::collections::HashMap;
use std::hash::Hash;
use std::iter::repeat;

/// Converts a slice to a "hash bag", i.e. a `HashMap` whose values correspond to the number of
/// times each value appears in the given `Vec`.
pub(crate) fn create_hash_bag<T: Eq + Hash + Clone>(values: &[T]) -> HashMap<T, usize> {
    let mut counts = HashMap::with_capacity(values.len());
    for v in values {
        counts.entry(v.clone()).and_modify(|c| *c += 1).or_insert(1);
    }
    counts
}

/// Convert a "hash bag" to a flat `Vec` of values.
///
/// The resulting ordering is undefined, except that multiple instances the same value are
/// guaranteed to be grouped together.
pub(crate) fn flatten_hash_bag<T: Clone>(count_map: &HashMap<T, usize>) -> Vec<T> {
    count_map
        .iter()
        .flat_map(|(val, &count)| repeat(val.clone()).take(count))
        .collect()
}
