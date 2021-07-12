use std::convert::TryInto;
use std::marker::PhantomData;
use std::ops::Range;

use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::algebra::ExtensionAlgebra;
use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::{Extendable, FieldExtension};
use crate::field::field::Field;
use crate::gates::gate::{Gate, GateRef};
use crate::generator::{SimpleGenerator, WitnessGenerator};
use crate::target::Target;
use crate::vars::{EvaluationTargets, EvaluationVars};
use crate::wire::Wire;
use crate::witness::PartialWitness;

/// A gate for inserting a value into a list at a non-deterministic location.
#[derive(Clone, Debug)]
pub(crate) struct InsertionGate<F: Extendable<D>, const D: usize> {
    pub vec_size: usize,
    pub _phantom: PhantomData<F>,
}

impl<F: Extendable<D>, const D: usize> InsertionGate<F, D> {
    pub fn new(vec_size: usize) -> GateRef<F, D> {
        let gate = Self {
            vec_size,
            _phantom: PhantomData,
        };
        GateRef::new(gate)
    }

    pub fn wires_insertion_index() -> usize {
        0
    }

    pub fn wires_element_to_insert() -> Range<usize> {
        1..D + 1
    }

    pub fn wires_list_item(i: usize) -> Range<usize> {
        let start = (i + 1) * D + 1;
        start..start + D
    }

    fn start_of_output_wires(&self) -> usize {
        (self.vec_size + 1) * D + 1
    }

    pub fn wires_output_list_item(&self, i: usize) -> Range<usize> {
        let start = self.start_of_output_wires() + i * D;
        start..start + D
    }

    fn start_of_intermediate_wires(&self) -> usize {
        self.start_of_output_wires() + self.vec_size * D
    }

    pub fn equality_dummy_for_round_r(&self, r: usize) -> usize {
        self.start_of_intermediate_wires() + r
    }

    pub fn insert_here_for_round_r(&self, r: usize) -> usize {
        self.start_of_intermediate_wires() + (self.vec_size + 1) + r
    }
}

impl<F: Extendable<D>, const D: usize> Gate<F, D> for InsertionGate<F, D> {
    fn id(&self) -> String {
        format!("{:?}<D={}>", self, D)
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        let insertion_index = vars.local_wires[Self::wires_insertion_index()];

        let mut list_items = Vec::new();
        for i in 0..self.vec_size {
            list_items.push(vars.get_local_ext_algebra(Self::wires_list_item(i)));
        }
        let dummy_value: ExtensionAlgebra<F::Extension, D> = F::Extension::ZERO.into(); // will never be reached
        list_items.push(dummy_value);

        let mut output_list_items = Vec::new();
        for i in 0..self.vec_size + 1 {
            output_list_items.push(vars.get_local_ext_algebra(self.wires_output_list_item(i)));
        }

        let element_to_insert = vars.get_local_ext_algebra(Self::wires_element_to_insert());

        let mut constraints = Vec::new();

        let mut already_inserted = F::Extension::ZERO;
        for r in 0..self.vec_size + 1 {
            let cur_index = F::Extension::from_canonical_usize(r);

            let equality_dummy = vars.local_wires[self.equality_dummy_for_round_r(r)];

            let difference = cur_index - insertion_index;
            let insert_here = vars.local_wires[self.insert_here_for_round_r(r)];

            // The two equality constraints.
            let equality_dummy_constraint: ExtensionAlgebra<F::Extension, D> =
                (difference * equality_dummy - insert_here).into();
            constraints.extend(equality_dummy_constraint.to_basefield_array());
            let mul_to_zero_constraint: ExtensionAlgebra<F::Extension, D> =
                ((F::Extension::ONE - insert_here) * difference).into();
            constraints.extend(mul_to_zero_constraint.to_basefield_array());

            let mut new_item = element_to_insert * insert_here.into() + already_inserted.into();
            if r > 0 {
                new_item += list_items[r - 1] * already_inserted.into();
            }
            already_inserted += insert_here;

            new_item += list_items[r] * (F::Extension::ONE - already_inserted).into();

            constraints.extend((new_item - output_list_items[r]).to_basefield_array());
        }

        constraints
    }

    fn eval_unfiltered_recursively(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>> {
        todo!()
    }

    fn generators(
        &self,
        gate_index: usize,
        local_constants: &[F],
    ) -> Vec<Box<dyn WitnessGenerator<F>>> {
        let gen = InsertionGenerator::<F, D> {
            gate_index,
            gate: self.clone(),
            _phantom: PhantomData,
        };
        vec![Box::new(gen)]
    }

    fn num_wires(&self) -> usize {
        let num_input_wires = self.vec_size + 1; // the original vector, and the insertion index
        let num_output_wires = self.vec_size + 1; // the final vector, with the inserted element
        let num_intermediate_wires = 6 * self.vec_size; // six intermediate variables needed for each element of the vector
        self.vec_size + 1
    }

    fn num_constants(&self) -> usize {
        0
    }

    fn degree(&self) -> usize {
        2
    }

    fn num_constraints(&self) -> usize {
        (self.vec_size + 1) * 3
    }
}

#[derive(Debug)]
struct InsertionGenerator<F: Extendable<D>, const D: usize> {
    gate_index: usize,
    gate: InsertionGate<F, D>,
    _phantom: PhantomData<F>,
}

impl<F: Extendable<D>, const D: usize> SimpleGenerator<F> for InsertionGenerator<F, D> {
    fn dependencies(&self) -> Vec<Target> {
        let local_target = |input| {
            Target::Wire(Wire {
                gate: self.gate_index,
                input,
            })
        };

        let local_targets = |inputs: Range<usize>| inputs.map(local_target);

        let mut deps = Vec::new();
        deps.push(local_target(InsertionGate::<F, D>::wires_insertion_index()));
        deps.extend(local_targets(
            InsertionGate::<F, D>::wires_element_to_insert(),
        ));
        for i in 0..self.gate.vec_size {
            deps.extend(local_targets(InsertionGate::<F, D>::wires_list_item(i)));
        }
        deps
    }

    fn run_once(&self, witness: &PartialWitness<F>) -> PartialWitness<F> {
        let local_wire = |input| Wire {
            gate: self.gate_index,
            input,
        };

        let get_local_wire = |input| witness.get_wire(local_wire(input));

        let get_local_ext = |wire_range: Range<usize>| {
            debug_assert_eq!(wire_range.len(), D);
            let values = wire_range.map(get_local_wire).collect::<Vec<_>>();
            let arr = values.try_into().unwrap();
            F::Extension::from_basefield_array(arr)
        };

        // Compute the new vector and the values for equality_dummy and insert_here
        let n = self.gate.vec_size;
        let orig_vec = (0..n)
            .map(|i| get_local_ext(InsertionGate::<F, D>::wires_list_item(i)))
            .collect::<Vec<_>>();
        let to_insert = get_local_ext(InsertionGate::<F, D>::wires_element_to_insert());
        let insertion_index_f = get_local_wire(InsertionGate::<F, D>::wires_insertion_index());

        let insertion_index = insertion_index_f.to_canonical_u64() as usize;
        let mut new_vec = Vec::new();
        new_vec.extend(&orig_vec[..insertion_index]);
        new_vec.push(to_insert);
        new_vec.extend(&orig_vec[insertion_index..]);

        let mut result = PartialWitness::<F>::new();
        for i in 0..=n {
            let output_wires = self.gate.wires_output_list_item(i).map(local_wire);
            result.set_ext_wires(output_wires, new_vec[i]);
        }

        result
    }
}

#[cfg(test)]
mod tests {}
