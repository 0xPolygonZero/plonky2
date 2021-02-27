use std::collections::HashMap;
use std::iter;

use num::{BigUint, FromPrimitive, One};

use crate::constraint_polynomial::ConstraintPolynomial;
use crate::field::field::Field;

/// Represents a set of deterministic gate outputs, expressed as polynomials over witness
/// values.
#[derive(Clone, Debug)]
pub struct OutputGraph<F: Field> {
    pub(crate) outputs: Vec<(GateOutputLocation, ConstraintPolynomial<F>)>
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
    // TODO: This doesn't yet work with large exponentiations, i.e. x^n where n > max_degree. Not an
    // immediate problem since our gates don't use those.
    pub fn shrink_degree(&self, max_degree: usize) -> Self {
        let max_degree_biguint = BigUint::from_usize(max_degree).unwrap();

        let mut current_graph = self.clone();

        while current_graph.count_high_degree_polys(max_degree) > 0 {
            // Find polynomials with a degree between 2 and the max, inclusive.
            // These are candidates for becoming new wires.
            let mut candidates = current_graph.degree_map().into_iter()
                .filter(|(_poly, deg)| deg > &BigUint::one() && deg <= &max_degree_biguint)
                .map(|(poly, _deg)| poly);

            // Pick the candidate that minimizes the number of high-degree polynomials in our graph.
            // This is just a simple heuristic; it won't always give an optimal wire count.
            let mut first = candidates.next().expect("No candidate; cannot reduce degree further");
            let mut leader_graph = current_graph.allocate_wire(first);
            let mut leader_high_deg_count = leader_graph.count_high_degree_polys(max_degree);

            for candidate in candidates {
                let candidate_graph = current_graph.allocate_wire(candidate);
                let candidate_high_deg_count = candidate_graph.count_high_degree_polys(max_degree);
                if candidate_high_deg_count < leader_high_deg_count {
                    leader_graph = candidate_graph;
                    leader_high_deg_count = candidate_high_deg_count;
                }
            }

            // println!("before {:?}", current_graph);
            // println!("after {:?}", leader_graph);
            current_graph = leader_graph;
            println!("{}", leader_high_deg_count);
        }

        current_graph
    }

    /// The number of polynomials in this graph which exceed the given maximum degree.
    fn count_high_degree_polys(&self, max_degree: usize) -> usize {
        let max_degree = BigUint::from_usize(max_degree).unwrap();
        self.degree_map().into_iter()
            .filter(|(_poly, deg)| deg > &max_degree)
            .count()
    }

    fn degree_map(&self) -> HashMap<ConstraintPolynomial<F>, BigUint> {
        let mut degrees = HashMap::new();
        for (_loc, out) in &self.outputs {
            out.populate_degree_map(&mut degrees);
        }
        degrees
    }

    /// The largest local wire index in this entire graph.
    pub(crate) fn max_wire_input_index(&self) -> Option<usize> {
        self.outputs.iter()
            .flat_map(|(loc, out)| out.max_wire_input_index())
            .max()
    }

    /// Allocate a new wire for the given target polynomial, and return a new output graph with
    /// references to the target polynomial replaced with references to that wire.
    fn allocate_wire(&self, target: ConstraintPolynomial<F>) -> Self {
        let new_wire_index = self.max_wire_input_index()
            .map_or(0, |i| i + 1);

        let new_wire = ConstraintPolynomial::local_wire_value(new_wire_index);

        let outputs = self.outputs.iter()
            .map(|(loc, out)| (*loc, out.replace_all(target.clone(), new_wire.clone())))
            .chain(iter::once((GateOutputLocation::LocalWire(new_wire_index), target.clone())))
            .collect();
        Self { outputs }
    }
}

/// Represents an output location of a deterministic gate.
#[derive(Copy, Clone, Debug)]
pub enum GateOutputLocation {
    /// A wire belonging to the gate itself.
    LocalWire(usize),
    /// A wire belonging to the following gate.
    NextWire(usize),
}

#[cfg(test)]
mod tests {
    use crate::constraint_polynomial::ConstraintPolynomial;
    use crate::field::crandall_field::CrandallField;
    use crate::gates::output_graph::{GateOutputLocation, OutputGraph};

    #[test]
    fn shrink_squaring_graph() {
        type F = CrandallField;
        let deg1 = ConstraintPolynomial::<F>::local_wire_value(0);
        let deg2 = deg1.square();
        let deg4 = deg2.square();
        let deg8 = deg4.square();
        let deg16 = deg8.square();

        let original = OutputGraph::single_output(
            GateOutputLocation::NextWire(0),
            deg16);

        let degree_map = original.degree_map();
        assert_eq!(degree_map.len(), 5);

        assert_eq!(original.count_high_degree_polys(2), 3);
        assert_eq!(original.count_high_degree_polys(3), 3);
        assert_eq!(original.count_high_degree_polys(4), 2);

        let shrunk_deg_2 = original.shrink_degree(2);
        let shrunk_deg_3 = original.shrink_degree(3);
        let shrunk_deg_4 = original.shrink_degree(4);

        // `shrunk_deg_2` should have an intermediate wire for deg2, deg4, and deg8.
        assert_eq!(shrunk_deg_2.max_wire_input_index(), Some(3));

        // `shrunk_deg_3` should also have an intermediate wire for deg2, deg4, and deg8.
        assert_eq!(shrunk_deg_3.max_wire_input_index(), Some(3));

        // `shrunk_deg_4` should have an intermediate wire for deg4 only.
        assert_eq!(shrunk_deg_4.max_wire_input_index(), Some(1));
    }
}
