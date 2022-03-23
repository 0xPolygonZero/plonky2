use plonky2_field::extension_field::Extendable;
use plonky2_field::polynomial::PolynomialValues;

use crate::gates::gate::{GateInstance, GateRef};
use crate::hash::hash_types::RichField;

pub(crate) fn compute_selectors<F: RichField + Extendable<D>, const D: usize>(
    mut gates: Vec<GateRef<F, D>>,
    instances: &[GateInstance<F, D>],
    max_degree: usize,
) -> (Vec<PolynomialValues<F>>, Vec<usize>, Vec<(usize, usize)>) {
    let n = instances.len();

    let mut combinations = Vec::new();
    let mut pos = 0;

    while pos < gates.len() {
        let mut i = 0;
        while (pos + i < gates.len()) && (i + gates[pos + i].0.degree() <= max_degree + 1) {
            i += 1;
        }
        combinations.push((pos, pos + i));
        pos += i;
    }

    let num_constants_polynomials =
        0.max(gates.iter().map(|g| g.0.num_constants()).max().unwrap() - combinations.len() + 1);
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
        let mut k = 0;
        let mut constant_ind = 0;
        while k < constants.len() {
            if constant_ind == comb {
                constant_ind += 1;
            } else {
                polynomials[constant_ind].values[j] = constants[k];
                constant_ind += 1;
                k += 1;
            }
        }
    }
    (polynomials, selector_indices, combination_ranges)
}
