use std::collections::HashMap;
use std::fmt::{Debug, Error, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::Range;
use std::sync::Arc;

use plonky2_field::batch_util::batch_multiply_inplace;
use plonky2_field::extension::{Extendable, FieldExtension};
use plonky2_field::types::Field;

use crate::gates::selectors::UNUSED_SELECTOR;
use crate::gates::util::StridedConstraintConsumer;
use crate::hash::hash_types::RichField;
use crate::iop::ext_target::ExtensionTarget;
use crate::iop::generator::WitnessGenerator;
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

    fn eval_unfiltered_circuit(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>>;

    fn eval_filtered(
        &self,
        mut vars: EvaluationVars<F, D>,
        row: usize,
        selector_index: usize,
        group_range: Range<usize>,
        num_selectors: usize,
    ) -> Vec<F::Extension> {
        let filter = compute_filter(
            row,
            group_range,
            vars.local_constants[selector_index],
            num_selectors > 1,
        );
        vars.remove_prefix(num_selectors);
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
        row: usize,
        selector_index: usize,
        group_range: Range<usize>,
        num_selectors: usize,
    ) -> Vec<F> {
        let filters: Vec<_> = vars_batch
            .iter()
            .map(|vars| {
                compute_filter(
                    row,
                    group_range.clone(),
                    vars.local_constants[selector_index],
                    num_selectors > 1,
                )
            })
            .collect();
        vars_batch.remove_prefix(num_selectors);
        let mut res_batch = self.eval_unfiltered_base_batch(vars_batch);
        for res_chunk in res_batch.chunks_exact_mut(filters.len()) {
            batch_multiply_inplace(res_chunk, &filters);
        }
        res_batch
    }

    /// Adds this gate's filtered constraints into the `combined_gate_constraints` buffer.
    fn eval_filtered_circuit(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        mut vars: EvaluationTargets<D>,
        row: usize,
        selector_index: usize,
        group_range: Range<usize>,
        num_selectors: usize,
        combined_gate_constraints: &mut [ExtensionTarget<D>],
    ) {
        let filter = compute_filter_circuit(
            builder,
            row,
            group_range,
            vars.local_constants[selector_index],
            num_selectors > 1,
        );
        vars.remove_prefix(num_selectors);
        let my_constraints = self.eval_unfiltered_circuit(builder, vars);
        for (acc, c) in combined_gate_constraints.iter_mut().zip(my_constraints) {
            *acc = builder.mul_add_extension(filter, c, *acc);
        }
    }

    /// The generators used to populate the witness.
    /// Note: This should return exactly 1 generator per operation in the gate.
    fn generators(&self, row: usize, local_constants: &[F]) -> Vec<Box<dyn WitnessGenerator<F>>>;

    /// The number of wires used by this gate.
    fn num_wires(&self) -> usize;

    /// The number of constants used by this gate.
    fn num_constants(&self) -> usize;

    /// The maximum degree among this gate's constraint polynomials.
    fn degree(&self) -> usize;

    fn num_constraints(&self) -> usize;

    /// Number of operations performed by the gate.
    fn num_ops(&self) -> usize {
        self.generators(0, &vec![F::ZERO; self.num_constants()])
            .len()
    }

    /// Enables gates to store some "routed constants", if they have both unused constants and
    /// unused routed wires.
    ///
    /// Each entry in the returned `Vec` has the form `(constant_index, wire_index)`. `wire_index`
    /// must correspond to a *routed* wire.
    fn extra_constant_wires(&self) -> Vec<(usize, usize)> {
        vec![]
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

/// Map between gate parameters and available slots.
/// An available slot is of the form `(row, op)`, meaning the current available slot
/// is at gate index `row` in the `op`-th operation.
#[derive(Clone, Debug, Default)]
pub struct CurrentSlot<F: RichField + Extendable<D>, const D: usize> {
    pub current_slot: HashMap<Vec<F>, (usize, usize)>,
}

/// A gate along with any constants used to configure it.
#[derive(Clone)]
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

/// A gate's filter designed so that it is non-zero if `s = row`.
fn compute_filter<K: Field>(row: usize, group_range: Range<usize>, s: K, many_selector: bool) -> K {
    debug_assert!(group_range.contains(&row));
    group_range
        .filter(|&i| i != row)
        .chain(many_selector.then_some(UNUSED_SELECTOR))
        .map(|i| K::from_canonical_usize(i) - s)
        .product()
}

fn compute_filter_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    row: usize,
    group_range: Range<usize>,
    s: ExtensionTarget<D>,
    many_selectors: bool,
) -> ExtensionTarget<D> {
    debug_assert!(group_range.contains(&row));
    let v = group_range
        .filter(|&i| i != row)
        .chain(many_selectors.then_some(UNUSED_SELECTOR))
        .map(|i| {
            let c = builder.constant_extension(F::Extension::from_canonical_usize(i));
            builder.sub_extension(c, s)
        })
        .collect::<Vec<_>>();
    builder.mul_many_extension(v)
}
