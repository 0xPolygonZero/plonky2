use std::collections::HashMap;
use std::fmt::{Debug, Error, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use plonky2_field::batch_util::batch_multiply_inplace;
use plonky2_field::extension_field::{Extendable, FieldExtension};
use plonky2_field::field_types::Field;

use crate::gates::gate_tree::Tree;
use crate::gates::util::StridedConstraintConsumer;
use crate::hash::hash_types::RichField;
use crate::iop::ext_target::ExtensionTarget;
use crate::iop::generator::WitnessGenerator;
use crate::iop::target::Target;
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::vars::{
    EvaluationTargets, EvaluationVars, EvaluationVarsBase, EvaluationVarsBaseBatch,
};

/// A custom gate.
pub trait Gate<F: RichField + Extendable<D>, const D: usize>: 'static + Send + Sync {
    fn id(&self) -> String;

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension>;

    /// Like `eval_unfiltered`, but specialized for points in the base field.
    ///
    ///
    /// `eval_unfiltered_base_batch` calls this method by default. If `eval_unfiltered_base_batch`
    /// is overridden, then `eval_unfiltered_base_one` is not necessary.
    ///
    /// By default, this just calls `eval_unfiltered`, which treats the point as an extension field
    /// element. This isn't very efficient.
    fn eval_unfiltered_base_one(
        &self,
        vars_base: EvaluationVarsBase<F>,
        mut yield_constr: StridedConstraintConsumer<F>,
    ) {
        // Note that this method uses `yield_constr` instead of returning its constraints.
        // `yield_constr` abstracts out the underlying memory layout.
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
        values.into_iter().for_each(|value| {
            debug_assert!(F::Extension::is_in_basefield(&value));
            yield_constr.one(value.to_basefield_array()[0])
        })
    }

    fn eval_unfiltered_base_batch(&self, vars_base: EvaluationVarsBaseBatch<F>) -> Vec<F> {
        let mut res = vec![F::ZERO; vars_base.len() * self.num_constraints()];
        for (i, vars_base_one) in vars_base.iter().enumerate() {
            self.eval_unfiltered_base_one(
                vars_base_one,
                StridedConstraintConsumer::new(&mut res, vars_base.len(), i),
            );
        }
        res
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

    /// The result is an array of length `vars_batch.len() * self.num_constraints()`. Constraint `j`
    /// for point `i` is at index `j * batch_size + i`.
    fn eval_filtered_base_batch(
        &self,
        mut vars_batch: EvaluationVarsBaseBatch<F>,
        prefix: &[bool],
    ) -> Vec<F> {
        let filters: Vec<_> = vars_batch
            .iter()
            .map(|vars| compute_filter(prefix, vars.local_constants))
            .collect();
        vars_batch.remove_prefix(prefix);
        let mut res_batch = self.eval_unfiltered_base_batch(vars_batch);
        for res_chunk in res_batch.chunks_exact_mut(filters.len()) {
            batch_multiply_inplace(res_chunk, &filters);
        }
        res_batch
    }

    /// Adds this gate's filtered constraints into the `combined_gate_constraints` buffer.
    fn eval_filtered_recursively(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        mut vars: EvaluationTargets<D>,
        prefix: &[bool],
        combined_gate_constraints: &mut [ExtensionTarget<D>],
    ) {
        let filter = compute_filter_recursively(builder, prefix, vars.local_constants);
        vars.remove_prefix(prefix);
        let my_constraints = self.eval_unfiltered_recursively(builder, vars);
        for (acc, c) in combined_gate_constraints.iter_mut().zip(my_constraints) {
            *acc = builder.mul_add_extension(filter, c, *acc);
        }
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

    /// Number of operations performed by the gate.
    fn num_ops(&self) -> usize;

    /// Dependencies (inputs) for the i-th operation.
    fn dependencies_ith_op(&self, gate_index: usize, i: usize) -> Vec<Target>;

    /// Fill the dependencies of the
    fn fill_gate(
        &self,
        params: &[F],
        current_slot: &CurrentSlot<F, D>,
        builder: &mut CircuitBuilder<F, D>,
    ) {
        if let Some(&(gate_index, op)) = current_slot.current_slot.get(params) {
            let zero = builder.zero();
            for i in op..self.num_ops() {
                for dep in self.dependencies_ith_op(gate_index, i) {
                    builder.connect(dep, zero);
                }
            }
        }
    }
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

#[derive(Clone, Debug)]
pub struct CurrentSlot<F: RichField + Extendable<D>, const D: usize> {
    pub current_slot: HashMap<Vec<F>, (usize, usize)>,
}

/// A gate along with any constants used to configure it.
#[derive(Clone)]
pub struct GateInstance<F: RichField + Extendable<D>, const D: usize> {
    pub gate_ref: GateRef<F, D>,
    pub constants: Vec<F>,
    pub params: Vec<F>,
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
fn compute_filter<'a, K: Field, T: IntoIterator<Item = &'a K>>(prefix: &[bool], constants: T) -> K {
    prefix
        .iter()
        .zip(constants)
        .map(|(&b, &c)| if b { c } else { K::ONE - c })
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
