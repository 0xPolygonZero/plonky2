use std::iter;

use crate::constraint_polynomial::{ConstraintPolynomial};
use crate::field::field::Field;
use std::collections::HashMap;
use num::BigUint;

/// Represents a set of deterministic gate outputs, expressed as polynomials over witness
/// values.
pub struct OutputGraph<F: Field> {
    pub(crate) outputs: Vec<(GateOutputLocation, ConstraintPolynomial<F>)>
}

/// Represents an output location of a deterministic gate.
#[derive(Copy, Clone)]
pub enum GateOutputLocation {
    /// A wire belonging to the gate itself.
    LocalWire(usize),
    /// A wire belonging to the following gate.
    NextWire(usize),
}

impl<F: Field> OutputGraph<F> {
    /// Creates an output graph with a single output.
    pub fn single_output(loc: GateOutputLocation, out: ConstraintPolynomial<F>) -> Self {
        Self { outputs: vec![(loc, out)] }
    }

    /// Compiles an output graph with potentially high-degree polynomials to one with low-degree
    /// polynomials by introducing extra wires for some intermediate values.
    ///
    /// Note that this uses a simple greedy algorithm, so the result may not be optimal in terms of wire
    /// count.
    // TODO: This doesn't yet work with large exponentiations, i.e. x^n where n > new_degree. Not an
    // immediate problem since our gates don't use those.
    pub fn shrink_degree(&self, new_degree: usize) -> Self {
        todo!()
    }

    fn degree_map(&self) -> HashMap<ConstraintPolynomial<F>, BigUint> {
        let mut degrees = HashMap::new();
        for (_loc, out) in &self.outputs {
            out.populate_degree_map(&mut degrees);
        }
        degrees
    }

    /// Allocate a new wire for the given target polynomial, and return a new output graph with
    /// references to the target polynomial replaced with references to that wire.
    fn allocate_wire(&self, target: ConstraintPolynomial<F>) -> Self {
        let new_wire_index = self.outputs.iter()
            .flat_map(|(loc, out)| out.max_wire_input_index())
            .max()
            .map_or(0, |i| i + 1);

        let new_wire = ConstraintPolynomial::local_wire_value(new_wire_index);

        let outputs = self.outputs.iter()
            .map(|(loc, out)| (*loc, out.replace_all(target.clone(), new_wire.clone())))
            .chain(iter::once((GateOutputLocation::LocalWire(new_wire_index), target.clone())))
            .collect();
        Self { outputs }
    }
}

#[cfg(test)]
mod tests {
    use crate::constraint_polynomial::ConstraintPolynomial;
    use crate::gates::output_graph::shrink_degree;

    #[test]
    fn shrink_exp() {
        let original = ConstraintPolynomial::local_wire_value(0).exp(10);
        let shrunk = shrink_degree(original, 3);
        // `shrunk` should be something similar to (wire0^3)^3 * wire0.
        assert_eq!(shrunk.max_wire_input_index(), Some(2))
    }
}
