use plonky2_field::extension_field::Extendable;
use plonky2_field::polynomial::PolynomialValues;

use crate::gates::gate::{GateInstance, GateRef};
use crate::hash::hash_types::RichField;

/// Placeholder value to indicate that a gate doesn't use a selector polynomial.
pub(crate) const UNUSED_SELECTOR: usize = u32::MAX as usize;

#[derive(Debug, Clone)]
pub(crate) struct SelectorsInfo {
    pub(crate) selector_indices: Vec<usize>,
    pub(crate) groups: Vec<(usize, usize)>,
    pub(crate) num_selectors: usize,
}

/// Returns the selector polynomials and related information.
///
/// Selector polynomials are computed as follows:
/// Partition the gates into (the smallest amount of) groups `{ G_i }`, such that for each group `G`
/// `|G| + max_{g in G} g.degree() <= max_degree`. These groups are constructed greedily from
/// the list of gates sorted by degree.
/// We build a selector polynomial `S_i` for each group `G_i`, with
/// ```
/// S_i[j] =
///     if j-th row gate=g_k in G_i
///         k
///     else
///         UNUSED_SELECTOR
/// ```
pub(crate) fn selector_polynomials<F: RichField + Extendable<D>, const D: usize>(
    gates: &[GateRef<F, D>],
    instances: &[GateInstance<F, D>],
    max_degree: usize,
) -> (Vec<PolynomialValues<F>>, SelectorsInfo) {
    let n = instances.len();

    // Greedily construct the groups.
    let mut groups = Vec::new();
    let mut pos = 0;
    while pos < gates.len() {
        let mut i = 0;
        while (pos + i < gates.len()) && (i + gates[pos + i].0.degree() < max_degree) {
            i += 1;
        }
        groups.push((pos, pos + i));
        pos += i;
    }

    let index = |id| gates.iter().position(|g| g.0.id() == id).unwrap();
    let group = |i| groups.iter().position(|&(a, b)| a <= i && i < b).unwrap();

    // `selector_indices[i] = j` iff the `i`-th gate uses the `j`-th selector polynomial.
    let selector_indices = gates
        .iter()
        .map(|g| group(index(g.0.id())))
        .collect::<Vec<_>>();

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
            num_selectors: groups.len(),
            groups,
        },
    )
}
