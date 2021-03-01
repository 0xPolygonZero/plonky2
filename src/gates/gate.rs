use std::hash::{Hash, Hasher};
use std::rc::Rc;

use crate::circuit_data::CircuitConfig;
use crate::constraint_polynomial::ConstraintPolynomial;
use crate::field::field::Field;
use crate::generator::WitnessGenerator;
use num::ToPrimitive;

/// A custom gate.
// TODO: Remove CircuitConfig params? Could just use fields within each struct.
pub trait Gate<F: Field>: 'static {
    fn id(&self) -> String;

    /// A set of expressions which must evaluate to zero.
    fn constraints(&self, config: CircuitConfig) -> Vec<ConstraintPolynomial<F>>;

    fn generators(
        &self,
        config: CircuitConfig,
        gate_index: usize,
        local_constants: Vec<F>,
        next_constants: Vec<F>,
    ) -> Vec<Box<dyn WitnessGenerator<F>>>;

    /// The number of constants used by this gate.
    fn num_constants(&self, config: CircuitConfig) -> usize {
        self.constraints(config)
            .into_iter()
            .map(|c| c.max_constant_index().map_or(0, |i| i + 1))
            .max()
            .unwrap_or(0)
    }

    /// The minimum number of wires required to use this gate.
    fn min_wires(&self, config: CircuitConfig) -> usize {
        self.constraints(config)
            .into_iter()
            .map(|c| c.max_wire_input_index().map_or(0, |i| i + 1))
            .max()
            .unwrap_or(0)
    }

    /// The maximum degree among this gate's constraint polynomials.
    fn degree(&self, config: CircuitConfig) -> usize {
        self.constraints(config)
            .into_iter()
            .map(|c| c.degree().to_usize().expect("degree too large"))
            .max()
            .unwrap_or(0)
    }
}

/// A wrapper around an `Rc<Gate>` which implements `PartialEq`, `Eq` and `Hash` based on gate IDs.
#[derive(Clone)]
pub struct GateRef<F: Field>(pub(crate) Rc<dyn Gate<F>>);

impl<F: Field> GateRef<F> {
    pub fn new<G: Gate<F>>(gate: G) -> GateRef<F> {
        GateRef(Rc::new(gate))
    }
}

impl<F: Field> PartialEq for GateRef<F> {
    fn eq(&self, other: &Self) -> bool {
        self.0.id() == other.0.id()
    }
}

impl<F: Field> Hash for GateRef<F> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.id().hash(state)
    }
}

impl<F: Field> Eq for GateRef<F> {}

/// A gate along with any constants used to configure it.
pub struct GateInstance<F: Field> {
    pub gate_type: GateRef<F>,
    pub constants: Vec<F>,
}
