use std::ops::Range;

use plonky2_field::extension::Extendable;
use plonky2_field::polynomial::PolynomialValues;

use crate::gates::gate::{GateInstance, GateRef};
use crate::hash::hash_types::RichField;

/// Placeholder value to indicate that a gate doesn't use a selector polynomial.
pub(crate) const UNUSED_SELECTOR: usize = u32::MAX as usize;

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct SelectorsInfo {
    pub(crate) selector_indices: Vec<usize>,
    pub(crate) groups: Vec<Range<usize>>,
}

impl SelectorsInfo {
    pub fn num_selectors(&self) -> usize {
        self.groups.len()
    }
}

/// Returns the selector polynomials and related information.
///
/// Selector polynomials are computed as follows:
/// Partition the gates into (the smallest amount of) groups `{ G_i }`, such that for each group `G`
/// `|G| + max_{g in G} g.degree() <= max_degree`. These groups are constructed greedily from
/// the list of gates sorted by degree.
/// We build a selector polynomial `S_i` for each group `G_i`, with
/// S_i[j] =
///     if j-th row gate=g_k in G_i
///         k
///     else
///         UNUSED_SELECTOR
pub(crate) fn selector_polynomials<F: RichField + Extendable<D>, const D: usize>(
    gates: &[GateRef<F, D>],
    instances: &[GateInstance<F, D>],
    max_degree: usize,
) -> (Vec<PolynomialValues<F>>, SelectorsInfo) {
    let n = instances.len();
    let num_gates = gates.len();
    let max_gate_degree = gates.last().expect("No gates?").0.degree();

    let index = |id| gates.iter().position(|g| g.0.id() == id).unwrap();

    // Special case if we can use only one selector polynomial.
    if max_gate_degree + num_gates - 1 <= max_degree {
        return (
            vec![PolynomialValues::new(
                instances
                    .iter()
                    .map(|g| F::from_canonical_usize(index(g.gate_ref.0.id())))
                    .collect(),
            )],
            SelectorsInfo {
                selector_indices: vec![0; num_gates],
                groups: vec![0..num_gates],
            },
        );
    }

    if max_gate_degree >= max_degree {
        panic!(
            "{} has too high degree. Consider increasing `quotient_degree_factor`.",
            gates.last().unwrap().0.id()
        );
    }

    // Greedily construct the groups.
    let mut groups = Vec::new();
    let mut start = 0;
    while start < num_gates {
        let mut size = 0;
        while (start + size < gates.len()) && (size + gates[start + size].0.degree() < max_degree) {
            size += 1;
        }
        groups.push(start..start + size);
        start += size;
    }

    let group = |i| groups.iter().position(|range| range.contains(&i)).unwrap();

    // `selector_indices[i] = j` iff the `i`-th gate uses the `j`-th selector polynomial.
    let selector_indices = (0..num_gates).map(group).collect();

    // Placeholder value to indicate that a gate doesn't use a selector polynomial.
    let unused = F::from_canonical_usize(UNUSED_SELECTOR);

    let mut polynomials = vec![PolynomialValues::zero(n); groups.len()];
    for (j, g) in instances.iter().enumerate() {
        let GateInstance { gate_ref, .. } = g;
        let i = index(gate_ref.0.id());
        let gr = group(i);
        for g in 0..groups.len() {
            polynomials[g].values[j] = if g == gr {
                F::from_canonical_usize(i)
            } else {
                unused
            };
        }
    }

    (
        polynomials,
        SelectorsInfo {
            selector_indices,
            groups,
        },
    )
}
