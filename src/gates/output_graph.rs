use std::{fmt, iter};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};

use num::{BigUint, FromPrimitive, One, ToPrimitive};

use crate::constraint_polynomial::{ConstraintPolynomial, EvaluationVars};
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
            .map(|(loc, out)| out.degree().to_usize().unwrap())
            .max()
            .unwrap_or(0)
    }

    /// The largest local wire index among this graph's output locations.
    pub fn max_local_output_index(&self) -> Option<usize> {
        self.outputs.iter()
            .filter_map(|(loc, out)| match loc {
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
