use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
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

impl InsertionGate {
    pub fn new(vec_size: usize) -> GateRef<F, D> {
        let gate = Self {
            vec_size,
            _phantom: PhantomData,
        };
        GateRef::new(gate)
    }

    pub fn wires_insertion_index() -> Range<usize> {
        0..D
    }

    pub fn wires_list_item(i: usize) -> Range<usize> {
        let start = (i + 1) * D;
        start..start + D
    }

    fn start_of_intermediate_wires() -> usize {
        (i + 2) * D
    }

    fn wires_per_round() -> {
        // D wires needed for each of cur_index, insert_here, equality_dummy, new_item, new_item_plus_old_item,
        // already_inserted, and not_already_inserted
        7 * D
    }

    /// The wires corresponding to the "cur_index" variable in the gadget (non-gate) insert function.
    pub fn cur_index_for_round_r(r: usize) -> Range<usize> {
        let intermediate_index = 0;
        let start = start_of_intermediate_wires() + r * wires_per_round() + D * intermediate_index;
        start..start + D
    }

    /// The wires corresponding to the "insert_here" variable in the gadget (non-gate) insert function.
    pub fn insert_here_for_round_r(r: usize) -> Range<usize> {
        let intermediate_index = 1;
        let start = start_of_intermediate_wires() + r * wires_per_round() + D * intermediate_index;
        start..start + D
    }

    /// The wires corresponding to the "equality_dummy" variable in the gadget (non-gate) insert function.
    pub fn equality_dummy_for_round_r(r: usize) -> Range<usize> {
        let intermediate_index = 1;
        let start = start_of_intermediate_wires() + r * wires_per_round() + D * intermediate_index;
        start..start + D
    }

    /// The wires corresponding to the "new_item" variable in the gadget (non-gate) insert function.
    pub fn new_item_for_round_r(r: usize) -> Range<usize> {
        let intermediate_index = 2;
        let start = start_of_intermediate_wires() + r * wires_per_round() + D * intermediate_index;
        start..start + D
    }

    /// The wires corresponding to the "new_item_plus_old_item" variable in the gadget (non-gate) insert function.
    pub fn new_item_plus_old_item_for_round_r(r: usize) -> Range<usize> {
        let intermediate_index = 3;
        let start = start_of_intermediate_wires() + r * wires_per_round() + D * intermediate_index;
        start..start + D
    }

    /// The wires corresponding to the "already_inserted" variable in the gadget (non-gate) insert function.
    pub fn already_inserted_for_round_r(r: usize) -> Range<usize> {
        let intermediate_index = 4;
        let start = start_of_intermediate_wires() + r * wires_per_round() + D * intermediate_index;
        start..start + D
    }

    /// The wires corresponding to the "not_already_inserted" variable in the gadget (non-gate) insert function.
    pub fn not_already_inserted_for_round_r(r: usize) -> Range<usize> {
        let intermediate_index = 5;
        let start = start_of_intermediate_wires() + r * wires_per_round() + D * intermediate_index;
        start..start + D
    }
}

impl<F: Extendable<D>, const D: usize> Gate<F, D> for InsertionGate {
    fn id(&self) -> String {
        format!("{:?}<D={}>", self, D)
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        let insertion_index = vars.get_local_ext_algebra(Self::wires_insertion_index());
        let mut list_items = Vec::new();
        for i in 0..self::vec_size {
            list_items.push(vars.get_local_ext_algebra(Self::wires_list_item(i)));
        }

        let mut constraints = Vec::new();

        for r in 0..self::vec_size {
            let this_round_cur_index = vars.get_local_ext_algebra(Self::cur_index_for_round_r(r));
            // TODO: set value of cur_index

            let this_round_insert_here = vars.get_local_ext_algebra(Self::insert_here_for_round_r(r));
            // enforce "insert_here = is_equal(cur_index, insertion_index)"
            let this_round_equality_dummy = vars.get_local_ext_algebra(Self::equality_dummy_for_round_r(r));
            let computed_insert_here = (this_round_cur_index - insertion_index) * this_round_equality_dummy;
            constraints.extend(computed_insert_here - this_round_insert_here).to_basefield_array());


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
        1
    }

    fn num_constraints(&self) -> usize {
        1
    }
}

#[derive(Debug)]
struct InsertionGenerator<F: Field> {
    gate_index: usize,
    gate: InterpolationGate<F, D>,
    _phantom: PhantomData<F>,
}

impl<F: Field> SimpleGenerator<F> for InsertionGenerator<F> {
    fn dependencies(&self) -> Vec<Target> {
        Vec::new()
    }

    fn run_once(&self, _witness: &PartialWitness<F>) -> PartialWitness<F> {
        let wire = Wire {
            gate: self.gate_index,
            input: ConstantGate::WIRE_OUTPUT,
        };
        PartialWitness::singleton_target(Target::Wire(wire), self.constant)
    }
}

#[cfg(test)]
mod tests {
    
}
