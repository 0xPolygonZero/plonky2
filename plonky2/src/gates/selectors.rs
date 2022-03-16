use plonky2_field::extension_field::Extendable;
use plonky2_field::polynomial::PolynomialValues;

use crate::gates::gate::{GateInstance, GateRef};
use crate::hash::hash_types::RichField;

pub(crate) fn compute_selectors<F: RichField + Extendable<D>, const D: usize>(
    mut gates: Vec<GateRef<F, D>>,
    instances: &[GateInstance<F, D>],
    max_degree: usize,
) {
    let n = instances.len();
    gates.sort_unstable_by_key(|g| g.0.degree());

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
        0.max(gates.iter().map(|g| g.0.num_constants()).max().unwrap() - combinations.len() - 1);
    let mut polynomials =
        vec![PolynomialValues::zero(n); combinations.len() + num_constants_polynomials];

    let index = |id| gates.iter().position(|g| g.0.id() == id).unwrap();
    let combination = |i| combinations.iter().position(|&(a, _)| a <= i).unwrap();

    for (j, g) in instances.iter().enumerate() {
        let i = index(g.gate_ref.0.id());
        let comb = combination(i);
        polynomials[comb].values[j] = i - combinations[comb].0;
    }
}
