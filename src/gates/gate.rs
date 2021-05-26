use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::generator::WitnessGenerator;
use crate::vars::{EvaluationTargets, EvaluationVars, EvaluationVarsBase};

/// A custom gate.
pub trait Gate<F: Extendable<D>, const D: usize>: 'static + Send + Sync {
    fn id(&self) -> String;

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension>;

    /// Like `eval_unfiltered`, but specialized for points in the base field.
    fn eval_unfiltered_base(&self, vars: EvaluationVarsBase<F>) -> Vec<F> {
        todo!()
    }

    fn eval_unfiltered_recursively(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>>;

    fn eval_filtered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        // TODO: Filter
        self.eval_unfiltered(vars)
    }

    /// Like `eval_filtered`, but specialized for points in the base field.
    fn eval_filtered_base(&self, vars: EvaluationVarsBase<F>) -> Vec<F> {
        todo!()
    }

    fn eval_filtered_recursively(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>> {
        // TODO: Filter
        self.eval_unfiltered_recursively(builder, vars)
    }

    fn generators(
        &self,
        gate_index: usize,
        local_constants: &[F],
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
pub struct GateRef<F: Extendable<D>, const D: usize>(pub(crate) Arc<dyn Gate<F, D>>);

impl<F: Extendable<D>, const D: usize> GateRef<F, D> {
    pub fn new<G: Gate<F, D>>(gate: G) -> GateRef<F, D> {
        GateRef(Arc::new(gate))
    }
}

impl<F: Extendable<D>, const D: usize> PartialEq for GateRef<F, D> {
    fn eq(&self, other: &Self) -> bool {
        self.0.id() == other.0.id()
    }
}

impl<F: Extendable<D>, const D: usize> Hash for GateRef<F, D> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.id().hash(state)
    }
}

impl<F: Extendable<D>, const D: usize> Eq for GateRef<F, D> {}

/// A gate along with any constants used to configure it.
pub struct GateInstance<F: Extendable<D>, const D: usize> {
    pub gate_type: GateRef<F, D>,
    pub constants: Vec<F>,
}
