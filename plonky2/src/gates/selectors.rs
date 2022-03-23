use plonky2_field::extension_field::Extendable;
use plonky2_field::polynomial::PolynomialValues;

use crate::gates::gate::{GateInstance, GateRef};
use crate::hash::hash_types::RichField;

pub(crate) fn compute_selectors<F: RichField + Extendable<D>, const D: usize>(
    mut gates: Vec<GateRef<F, D>>,
    instances: &[GateInstance<F, D>],
    max_degree: usize,
) -> (
    Vec<PolynomialValues<F>>,
    Vec<usize>,
    Vec<(usize, usize)>,
    usize,
) {
    let n = instances.len();

    let mut combinations = Vec::new();
    let mut pos = 0;

    while pos < gates.len() {
        let mut i = 0;
        while (pos + i < gates.len()) && (i + gates[pos + i].0.degree() <= max_degree) {
            i += 1;
        }
        combinations.push((pos, pos + i));
        pos += i;
    }
    dbg!(&combinations);
    let bad = F::from_canonical_usize(u32::MAX as usize);

    let num_constants_polynomials = gates.iter().map(|g| g.0.num_constants()).max().unwrap();
    let mut polynomials =
        vec![PolynomialValues::zero(n); combinations.len() + num_constants_polynomials];

    let index = |id| gates.iter().position(|g| g.0.id() == id).unwrap();
    let combination = |i| {
        combinations
            .iter()
            .position(|&(a, b)| a <= i && i < b)
            .unwrap()
    };

    let selector_indices = gates
        .iter()
        .map(|g| combination(index(g.0.id())))
        .collect::<Vec<_>>();
    let combination_ranges = selector_indices
        .iter()
        .map(|&i| (combinations[i].0, combinations[i].1))
        .collect();

    for (j, g) in instances.iter().enumerate() {
        let GateInstance {
            gate_ref,
            constants,
        } = g;
        let i = index(gate_ref.0.id());
        let comb = combination(i);
        polynomials[comb].values[j] = F::from_canonical_usize(i);

        for combis in (0..combinations.len()).filter(|&combis| combis != comb) {
            polynomials[combis].values[j] = bad;
        }

        for k in 0..constants.len() {
            polynomials[combinations.len() + k].values[j] = constants[k];
        }
    }

    (
        polynomials,
        selector_indices,
        combination_ranges,
        combinations.len(),
    )
}
