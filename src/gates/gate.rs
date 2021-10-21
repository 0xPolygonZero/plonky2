use std::fmt::{Debug, Error, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::{Extendable, FieldExtension};
use crate::field::field_types::{Field, RichField};
use crate::gates::gate_tree::Tree;
use crate::iop::generator::WitnessGenerator;
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::vars::{EvaluationTargets, EvaluationVars, EvaluationVarsBase};

/// A custom gate.
pub trait Gate<F: RichField + Extendable<D>, const D: usize>: 'static + Send + Sync {
    fn id(&self) -> String;

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension>;

    /// Like `eval_unfiltered`, but specialized for points in the base field.
    ///
    /// By default, this just calls `eval_unfiltered`, which treats the point as an extension field
    /// element. This isn't very efficient.
    fn eval_unfiltered_base(&self, vars_base: EvaluationVarsBase<F>) -> Vec<F> {
        let local_constants = &vars_base
            .local_constants
            .iter()
            .map(|c| F::Extension::from_basefield(*c))
            .collect::<Vec<_>>();
        let local_wires = &vars_base
            .local_wires
            .iter()
            .map(|w| F::Extension::from_basefield(*w))
            .collect::<Vec<_>>();
        let public_inputs_hash = &vars_base.public_inputs_hash;
        let vars = EvaluationVars {
            local_constants,
            local_wires,
            public_inputs_hash,
        };
        let values = self.eval_unfiltered(vars);

        // Each value should be in the base field, i.e. only the degree-zero part should be nonzero.
        values
            .into_iter()
            .map(|value| {
                debug_assert!(F::Extension::is_in_basefield(&value));
                value.to_basefield_array()[0]
            })
            .collect()
    }

    fn eval_unfiltered_recursively(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>>;

    fn eval_filtered(&self, mut vars: EvaluationVars<F, D>, prefix: &[bool]) -> Vec<F::Extension> {
        let filter = compute_filter(prefix, vars.local_constants);
        vars.remove_prefix(prefix);
        self.eval_unfiltered(vars)
            .into_iter()
            .map(|c| filter * c)
            .collect()
    }

    /// Like `eval_filtered`, but specialized for points in the base field.
    fn eval_filtered_base(&self, mut vars: EvaluationVarsBase<F>, prefix: &[bool]) -> Vec<F> {
        let filter = compute_filter(prefix, vars.local_constants);
        vars.remove_prefix(prefix);
        let mut res = self.eval_unfiltered_base(vars);
        res.iter_mut().for_each(|c| {
            *c *= filter;
        });
        res
    }

    fn eval_filtered_base_batch(
        &self,
        vars_batch: &[EvaluationVarsBase<F>],
        prefix: &[bool],
    ) -> Vec<Vec<F>> {
        vars_batch.iter().map(|&vars| self.eval_filtered_base(vars, prefix)).collect()
    }

    fn eval_filtered_recursively(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        mut vars: EvaluationTargets<D>,
        prefix: &[bool],
    ) -> Vec<ExtensionTarget<D>> {
        let filter = compute_filter_recursively(builder, prefix, vars.local_constants);
        vars.remove_prefix(prefix);
        self.eval_unfiltered_recursively(builder, vars)
            .into_iter()
            .map(|c| builder.mul_extension(filter, c))
            .collect()
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
pub struct GateRef<F: RichField + Extendable<D>, const D: usize>(pub(crate) Arc<dyn Gate<F, D>>);

impl<F: RichField + Extendable<D>, const D: usize> GateRef<F, D> {
    pub fn new<G: Gate<F, D>>(gate: G) -> GateRef<F, D> {
        GateRef(Arc::new(gate))
    }
}

impl<F: RichField + Extendable<D>, const D: usize> PartialEq for GateRef<F, D> {
    fn eq(&self, other: &Self) -> bool {
        self.0.id() == other.0.id()
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Hash for GateRef<F, D> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.id().hash(state)
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Eq for GateRef<F, D> {}

impl<F: RichField + Extendable<D>, const D: usize> Debug for GateRef<F, D> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "{}", self.0.id())
    }
}

/// A gate along with any constants used to configure it.
pub struct GateInstance<F: RichField + Extendable<D>, const D: usize> {
    pub gate_ref: GateRef<F, D>,
    pub constants: Vec<F>,
}

/// Map each gate to a boolean prefix used to construct the gate's selector polynomial.
#[derive(Debug, Clone)]
pub struct PrefixedGate<F: RichField + Extendable<D>, const D: usize> {
    pub gate: GateRef<F, D>,
    pub prefix: Vec<bool>,
}

impl<F: RichField + Extendable<D>, const D: usize> PrefixedGate<F, D> {
    pub fn from_tree(tree: Tree<GateRef<F, D>>) -> Vec<Self> {
        tree.traversal()
            .into_iter()
            .map(|(gate, prefix)| PrefixedGate { gate, prefix })
            .collect()
    }
}

/// A gate's filter is computed as `prod b_i*c_i + (1-b_i)*(1-c_i)`, with `(b_i)` the prefix and
/// `(c_i)` the local constants, which is one if the prefix of `constants` matches `prefix`.
fn compute_filter<K: Field>(prefix: &[bool], constants: &[K]) -> K {
    prefix
        .iter()
        .enumerate()
        .map(|(i, &b)| {
            if b {
                constants[i]
            } else {
                K::ONE - constants[i]
            }
        })
        .product()
}

fn compute_filter_recursively<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    prefix: &[bool],
    constants: &[ExtensionTarget<D>],
) -> ExtensionTarget<D> {
    let one = builder.one_extension();
    let v = prefix
        .iter()
        .enumerate()
        .map(|(i, &b)| {
            if b {
                constants[i]
            } else {
                builder.sub_extension(one, constants[i])
            }
        })
        .collect::<Vec<_>>();

    builder.mul_many_extension(&v)
}
