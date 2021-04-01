use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::circuit_builder::CircuitBuilder;
use crate::constraint_polynomial::{EvaluationTargets, EvaluationVars};
use crate::field::field::Field;
use crate::generator::WitnessGenerator;
use crate::target::Target;

/// A custom gate.
pub trait Gate<F: Field>: 'static + Send + Sync {
    fn id(&self) -> String;

    fn eval_unfiltered(&self, vars: EvaluationVars<F>) -> Vec<F>;

    fn eval_unfiltered_recursively(
        &self,
        builder: &mut CircuitBuilder<F>,
        vars: EvaluationTargets,
    ) -> Vec<Target>;

    fn eval_filtered(&self, vars: EvaluationVars<F>) -> Vec<F> {
        // TODO: Filter
        self.eval_unfiltered(vars)
    }

    fn generators(
        &self,
        gate_index: usize,
        local_constants: &[F],
        next_constants: &[F],
    ) -> Vec<Box<dyn WitnessGenerator<F>>>;

    /// The number of wires used by this gate.
    fn num_wires(&self) -> usize;

    /// The number of constants used by this gate.
    fn num_constants(&self) -> usize;

    /// The maximum degree among this gate's constraint polynomials.
    fn degree(&self) -> usize;

    fn num_constraints(&self) -> usize;
}

/// A wrapper around an `Rc<Gate>` which implements `PartialEq`, `Eq` and `Hash` based on gate IDs.
#[derive(Clone)]
pub struct GateRef<F: Field>(pub(crate) Arc<dyn Gate<F>>);

impl<F: Field> GateRef<F> {
    pub fn new<G: Gate<F>>(gate: G) -> GateRef<F> {
        GateRef(Arc::new(gate))
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
