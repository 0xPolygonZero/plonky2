use std::collections::HashMap;
use std::{iter, fmt};

use num::{BigUint, FromPrimitive, One, ToPrimitive};

use crate::constraint_polynomial::ConstraintPolynomial;
use crate::field::field::Field;
use std::fmt::{Display, Formatter};

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

    /// The largest local wire index in this entire graph.
    pub(crate) fn max_wire_input_index(&self) -> Option<usize> {
        self.outputs.iter()
            .flat_map(|(loc, out)| out.max_wire_input_index())
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
