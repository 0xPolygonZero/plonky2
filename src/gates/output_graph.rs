use std::fmt;
use std::fmt::{Display, Formatter};

use num::ToPrimitive;

use crate::constraint_polynomial::ConstraintPolynomial;
use crate::field::field::Field;

/// Represents a set of deterministic gate outputs, expressed as polynomials over witness
/// values.
#[derive(Clone, Debug)]
pub struct OutputGraph<F: Field> {
    pub(crate) outputs: Vec<(GateOutputLocation, ConstraintPolynomial<F>)>
}

impl<F: Field> OutputGraph<F> {
    /// Creates a new output graph with no outputs.
    pub fn new() -> Self {
        Self { outputs: Vec::new() }
    }

    /// Creates an output graph with a single output.
    pub fn single_output(loc: GateOutputLocation, out: ConstraintPolynomial<F>) -> Self {
        Self { outputs: vec![(loc, out)] }
    }

    pub fn add(&mut self, location: GateOutputLocation, poly: ConstraintPolynomial<F>) {
        self.outputs.push((location, poly))
    }

    /// The largest polynomial degree among all polynomials in this graph.
    pub fn degree(&self) -> usize {
        self.outputs.iter()
            .map(|(_loc, out)| out.degree().to_usize().unwrap())
            .max()
            .unwrap_or(0)
    }

    /// The largest local wire index among this graph's output locations.
    pub fn max_local_output_index(&self) -> Option<usize> {
        self.outputs.iter()
            .filter_map(|(loc, _out)| match loc {
                GateOutputLocation::LocalWire(i) => Some(*i),
                GateOutputLocation::NextWire(_) => None
            })
            .max()
    }
}

impl<F: Field> Display for OutputGraph<F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for (loc, out) in &self.outputs {
            write!(f, "{} := {}, ", loc, out)?;
        }
        Ok(())
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

impl Display for GateOutputLocation {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            GateOutputLocation::LocalWire(i) => write!(f, "local_wire_{}", i),
            GateOutputLocation::NextWire(i) => write!(f, "next_wire_{}", i),
        }
    }
}

/// Like `OutputGraph`, but tracks the index of the next available local wire.
///
/// Many gates have special wires, such as inputs and outputs, which must be placed at specific,
/// known indices. Many gates also use some wires for "intermediate values" which are internal to
/// the gate, and so can be placed at any available index.
///
/// This object helps to track which wire indices are available to be used for intermediate values.
/// It starts placing intermediate values at a given initial index, and repeatedly increments the
/// index as more intermediate values are added.
///
/// This assumes that there are no "reserved" indices greater than the given initial value. It also
/// assumes that there is no better place to store these intermediate values (such as wires of the
/// next gate). So this model may not make sense for all gates, which is why this is provided as an
/// optional utility.
pub struct ExpandableOutputGraph<F: Field> {
    pub(crate) output_graph: OutputGraph<F>,
    next_unused_index: usize,
}

impl<F: Field> ExpandableOutputGraph<F> {
    pub(crate) fn new(next_unused_index: usize) -> Self {
        ExpandableOutputGraph {
            output_graph: OutputGraph::new(),
            next_unused_index,
        }
    }

    /// Adds an intermediate value at the next available wire index, and returns a
    /// `ConstraintPolynomial` pointing to the newly created wire.
    pub(crate) fn add(&mut self, poly: ConstraintPolynomial<F>) -> ConstraintPolynomial<F> {
        let index = self.next_unused_index;
        self.next_unused_index += 1;

        self.output_graph.add(GateOutputLocation::LocalWire(index), poly);
        ConstraintPolynomial::local_wire(index)
    }
}
