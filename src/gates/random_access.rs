use std::marker::PhantomData;
use std::ops::Range;

use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::{Extendable, FieldExtension};
use crate::field::field_types::Field;
use crate::gates::gate::Gate;
use crate::iop::generator::{GeneratedValues, SimpleGenerator, WitnessGenerator};
use crate::iop::target::Target;
use crate::iop::wire::Wire;
use crate::iop::witness::PartialWitness;
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::vars::{EvaluationTargets, EvaluationVars, EvaluationVarsBase};

/// A gate for checking that a particular value in a list matches a given
#[derive(Clone, Debug)]
pub(crate) struct RandomAccessGate<F: Extendable<D>, const D: usize> {
    pub vec_size: usize,
    _phantom: PhantomData<F>,
}

impl<F: Extendable<D>, const D: usize> RandomAccessGate<F, D> {
    pub fn new(vec_size: usize) -> Self {
        Self {
            vec_size,
            _phantom: PhantomData,
        }
    }

    pub fn wires_access_index(&self) -> usize {
        0
    }

    pub fn wires_claimed_element(&self) -> Range<usize> {
        1..D + 1
    }

    pub fn wires_list_item(&self, i: usize) -> Range<usize> {
        debug_assert!(i < self.vec_size);
        let start = (i + 1) * D + 1;
        start..start + D
    }

    fn start_of_intermediate_wires(&self) -> usize {
        (self.vec_size + 1) * D + 1
    }

    /// An intermediate wire for a dummy variable used to show equality.
    /// The prover sets this to 1/(x-y) if x != y, or to an arbitrary value if
    /// x == y.
    pub fn wire_equality_dummy_for_index(&self, i: usize) -> usize {
        debug_assert!(i < self.vec_size);
        self.start_of_intermediate_wires() + i
    }

    /// An intermediate wire for the "index_matches" variable (1 if the current index is the index at
    /// which to compare, 0 otherwise).
    pub fn wire_index_matches_for_index(&self, i: usize) -> usize {
        debug_assert!(i < self.vec_size);
        self.start_of_intermediate_wires() + self.vec_size + i
    }
}

impl<F: Extendable<D>, const D: usize> Gate<F, D> for RandomAccessGate<F, D> {
    fn id(&self) -> String {
        format!("{:?}<D={}>", self, D)
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        let access_index = vars.local_wires[self.wires_access_index()];
        let list_items = (0..self.vec_size)
            .map(|i| vars.get_local_ext_algebra(self.wires_list_item(i)))
            .collect::<Vec<_>>();
        let claimed_element = vars.get_local_ext_algebra(self.wires_claimed_element());

        let mut constraints = Vec::with_capacity(self.num_constraints());
        for i in 0..self.vec_size {
            let cur_index = F::Extension::from_canonical_usize(i);
            let difference = cur_index - access_index;
            let equality_dummy = vars.local_wires[self.wire_equality_dummy_for_index(i)];
            let index_matches = vars.local_wires[self.wire_index_matches_for_index(i)];

            // The two index equality constraints.
            constraints.push(difference * equality_dummy - (F::Extension::ONE - index_matches));
            constraints.push(index_matches * difference);
            // Value equality constraint.
            constraints.extend(
                ((list_items[i] - claimed_element) * index_matches.into()).to_basefield_array(),
            );
        }

        constraints
    }

    fn eval_unfiltered_base(&self, vars: EvaluationVarsBase<F>) -> Vec<F> {
        let access_index = vars.local_wires[self.wires_access_index()];
        let list_items = (0..self.vec_size)
            .map(|i| vars.get_local_ext(self.wires_list_item(i)))
            .collect::<Vec<_>>();
        let claimed_element = vars.get_local_ext(self.wires_claimed_element());

        let mut constraints = Vec::with_capacity(self.num_constraints());
        for i in 0..self.vec_size {
            let cur_index = F::from_canonical_usize(i);
            let difference = cur_index - access_index;
            let equality_dummy = vars.local_wires[self.wire_equality_dummy_for_index(i)];
            let index_matches = vars.local_wires[self.wire_index_matches_for_index(i)];

            // The two equality constraints.
            constraints.push(difference * equality_dummy - (F::ONE - index_matches));
            constraints.push(index_matches * difference);

            // Value equality constraint.
            constraints.extend(
                ((list_items[i] - claimed_element) * index_matches.into()).to_basefield_array(),
            );
        }

        constraints
    }

    fn eval_unfiltered_recursively(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>> {
        let access_index = vars.local_wires[self.wires_access_index()];
        let list_items = (0..self.vec_size)
            .map(|i| vars.get_local_ext_algebra(self.wires_list_item(i)))
            .collect::<Vec<_>>();
        let claimed_element = vars.get_local_ext_algebra(self.wires_claimed_element());

        let mut constraints = Vec::with_capacity(self.num_constraints());
        for i in 0..self.vec_size {
            let cur_index_ext = F::Extension::from_canonical_usize(i);
            let cur_index = builder.constant_extension(cur_index_ext);

            let difference = builder.sub_extension(cur_index, access_index);
            let equality_dummy = vars.local_wires[self.wire_equality_dummy_for_index(i)];
            let index_matches = vars.local_wires[self.wire_index_matches_for_index(i)];

            // The two equality constraints.
            let one = builder.one_extension();
            let not_index_matches = builder.sub_extension(one, index_matches);
            let first_equality_constraint =
                builder.mul_sub_extension(difference, equality_dummy, not_index_matches);
            constraints.push(first_equality_constraint);

            let second_equality_constraint = builder.mul_extension(index_matches, difference);
            constraints.push(second_equality_constraint);

            // Output constraint.
            let diff = builder.sub_ext_algebra(list_items[i], claimed_element);
            let conditional_diff = builder.scalar_mul_ext_algebra(index_matches, diff);
            constraints.extend(conditional_diff.to_ext_target_array());
        }

        constraints
    }

    fn generators(
        &self,
        gate_index: usize,
        _local_constants: &[F],
    ) -> Vec<Box<dyn WitnessGenerator<F>>> {
        let gen = RandomAccessGenerator::<F, D> {
            gate_index,
            gate: self.clone(),
        };
        vec![Box::new(gen)]
    }

    fn num_wires(&self) -> usize {
        self.wire_index_matches_for_index(self.vec_size - 1) + 1
    }

    fn num_constants(&self) -> usize {
        0
    }

    fn degree(&self) -> usize {
        2
    }

    fn num_constraints(&self) -> usize {
        self.vec_size * (2 + D)
    }
}

#[derive(Debug)]
struct RandomAccessGenerator<F: Extendable<D>, const D: usize> {
    gate_index: usize,
    gate: RandomAccessGate<F, D>,
}

impl<F: Extendable<D>, const D: usize> SimpleGenerator<F> for RandomAccessGenerator<F, D> {
    fn dependencies(&self) -> Vec<Target> {
        let local_target = |input| Target::wire(self.gate_index, input);

        let local_targets = |inputs: Range<usize>| inputs.map(local_target);

        let mut deps = Vec::new();
        deps.push(local_target(self.gate.wires_access_index()));
        deps.extend(local_targets(self.gate.wires_claimed_element()));
        for i in 0..self.gate.vec_size {
            deps.extend(local_targets(self.gate.wires_list_item(i)));
        }
        deps
    }

    fn run_once(&self, witness: &PartialWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let local_wire = |input| Wire {
            gate: self.gate_index,
            input,
        };

        let get_local_wire = |input| witness.get_wire(local_wire(input));

        // Compute the new vector and the values for equality_dummy and index_matches
        let vec_size = self.gate.vec_size;
        let access_index_f = get_local_wire(self.gate.wires_access_index());

        let access_index = access_index_f.to_canonical_u64() as usize;
        debug_assert!(
            access_index < vec_size,
            "Access index {} is larger than the vector size {}",
            access_index,
            vec_size
        );

        for i in 0..vec_size {
            let equality_dummy_wire = local_wire(self.gate.wire_equality_dummy_for_index(i));
            let index_matches_wire = local_wire(self.gate.wire_index_matches_for_index(i));

            if i == access_index {
                out_buffer.set_wire(equality_dummy_wire, F::ONE);
                out_buffer.set_wire(index_matches_wire, F::ONE);
            } else {
                out_buffer.set_wire(
                    equality_dummy_wire,
                    (F::from_canonical_usize(i) - F::from_canonical_usize(access_index)).inverse(),
                );
                out_buffer.set_wire(index_matches_wire, F::ZERO);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use anyhow::Result;

    use crate::field::crandall_field::CrandallField;
    use crate::field::extension_field::quartic::QuarticCrandallField;
    use crate::field::field_types::Field;
    use crate::gates::gate::Gate;
    use crate::gates::gate_testing::{test_eval_fns, test_low_degree};
    use crate::gates::random_access::RandomAccessGate;
    use crate::hash::hash_types::HashOut;
    use crate::plonk::vars::EvaluationVars;

    #[test]
    fn wire_indices() {
        let gate = RandomAccessGate::<CrandallField, 4> {
            vec_size: 3,
            _phantom: PhantomData,
        };

        assert_eq!(gate.wires_access_index(), 0);
        assert_eq!(gate.wires_claimed_element(), 1..5);
        assert_eq!(gate.wires_list_item(0), 5..9);
        assert_eq!(gate.wires_list_item(2), 13..17);
        assert_eq!(gate.wire_equality_dummy_for_index(0), 17);
        assert_eq!(gate.wire_equality_dummy_for_index(2), 19);
        assert_eq!(gate.wire_index_matches_for_index(0), 20);
        assert_eq!(gate.wire_index_matches_for_index(2), 22);
    }

    #[test]
    fn low_degree() {
        test_low_degree::<CrandallField, _, 4>(RandomAccessGate::new(4));
    }

    #[test]
    fn eval_fns() -> Result<()> {
        test_eval_fns::<CrandallField, _, 4>(RandomAccessGate::new(4))
    }

    #[test]
    fn test_gate_constraint() {
        type F = CrandallField;
        type FF = QuarticCrandallField;
        const D: usize = 4;

        /// Returns the local wires for a random access gate given the vector, element to compare,
        /// and index.
        fn get_wires(list: Vec<FF>, access_index: usize, claimed_element: FF) -> Vec<FF> {
            let vec_size = list.len();

            let mut v = Vec::new();
            v.push(F::from_canonical_usize(access_index));
            v.extend(claimed_element.0);
            for j in 0..vec_size {
                v.extend(list[j].0);
            }

            let mut equality_dummy_vals = Vec::new();
            let mut index_matches_vals = Vec::new();
            for i in 0..vec_size {
                if i == access_index {
                    equality_dummy_vals.push(F::ONE);
                    index_matches_vals.push(F::ONE);
                } else {
                    equality_dummy_vals.push(
                        (F::from_canonical_usize(i) - F::from_canonical_usize(access_index))
                            .inverse(),
                    );
                    index_matches_vals.push(F::ZERO);
                }
            }

            v.extend(equality_dummy_vals);
            v.extend(index_matches_vals);

            v.iter().map(|&x| x.into()).collect::<Vec<_>>()
        }

        let list = vec![FF::rand(); 3];
        let access_index = 1;
        let gate = RandomAccessGate::<F, D> {
            vec_size: 3,
            _phantom: PhantomData,
        };

        let good_claimed_element = list[access_index];
        let good_vars = EvaluationVars {
            local_constants: &[],
            local_wires: &get_wires(list.clone(), access_index, good_claimed_element),
            public_inputs_hash: &HashOut::rand(),
        };
        let bad_claimed_element = FF::rand();
        let bad_vars = EvaluationVars {
            local_constants: &[],
            local_wires: &get_wires(list, access_index, bad_claimed_element),
            public_inputs_hash: &HashOut::rand(),
        };

        assert!(
            gate.eval_unfiltered(good_vars).iter().all(|x| x.is_zero()),
            "Gate constraints are not satisfied."
        );
        assert!(
            !gate.eval_unfiltered(bad_vars).iter().all(|x| x.is_zero()),
            "Gate constraints are satisfied but shouold not be."
        );
    }
}
