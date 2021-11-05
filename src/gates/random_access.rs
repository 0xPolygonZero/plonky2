use std::marker::PhantomData;

use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::field::field_types::{Field, RichField};
use crate::gates::gate::Gate;
use crate::iop::generator::{GeneratedValues, SimpleGenerator, WitnessGenerator};
use crate::iop::target::Target;
use crate::iop::wire::Wire;
use crate::iop::witness::{PartitionWitness, Witness};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::circuit_data::CircuitConfig;
use crate::plonk::vars::{EvaluationTargets, EvaluationVars, EvaluationVarsBase};

/// A gate for checking that a particular element of a list matches a given value.
#[derive(Copy, Clone, Debug)]
pub(crate) struct RandomAccessGate<F: RichField + Extendable<D>, const D: usize> {
    pub vec_size: usize,
    pub num_copies: usize,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> RandomAccessGate<F, D> {
    pub fn new(num_copies: usize, vec_size: usize) -> Self {
        Self {
            vec_size,
            num_copies,
            _phantom: PhantomData,
        }
    }

    pub fn new_from_config(config: &CircuitConfig, vec_size: usize) -> Self {
        let num_copies = Self::max_num_copies(config.num_routed_wires, config.num_wires, vec_size);
        Self::new(num_copies, vec_size)
    }

    pub fn max_num_copies(num_routed_wires: usize, num_wires: usize, vec_size: usize) -> usize {
        // Need `(2 + vec_size) * num_copies` routed wires
        (num_routed_wires / (2 + vec_size)).min(
            // Need `(2 + 3*vec_size) * num_copies` wires
            num_wires / (2 + 3 * vec_size),
        )
    }

    pub fn wire_access_index(&self, copy: usize) -> usize {
        debug_assert!(copy < self.num_copies);
        (2 + self.vec_size) * copy
    }

    pub fn wire_claimed_element(&self, copy: usize) -> usize {
        debug_assert!(copy < self.num_copies);
        (2 + self.vec_size) * copy + 1
    }

    pub fn wire_list_item(&self, i: usize, copy: usize) -> usize {
        debug_assert!(i < self.vec_size);
        debug_assert!(copy < self.num_copies);
        (2 + self.vec_size) * copy + 2 + i
    }

    fn start_of_intermediate_wires(&self) -> usize {
        (2 + self.vec_size) * self.num_copies
    }

    pub(crate) fn num_routed_wires(&self) -> usize {
        self.start_of_intermediate_wires()
    }

    /// An intermediate wire for a dummy variable used to show equality.
    /// The prover sets this to 1/(x-y) if x != y, or to an arbitrary value if
    /// x == y.
    pub fn wire_equality_dummy_for_index(&self, i: usize, copy: usize) -> usize {
        debug_assert!(i < self.vec_size);
        debug_assert!(copy < self.num_copies);
        self.start_of_intermediate_wires() + copy * self.vec_size + i
    }

    /// An intermediate wire for the "index_matches" variable (1 if the current index is the index at
    /// which to compare, 0 otherwise).
    pub fn wire_index_matches_for_index(&self, i: usize, copy: usize) -> usize {
        debug_assert!(i < self.vec_size);
        debug_assert!(copy < self.num_copies);
        self.start_of_intermediate_wires()
            + self.vec_size * self.num_copies
            + self.vec_size * copy
            + i
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Gate<F, D> for RandomAccessGate<F, D> {
    fn id(&self) -> String {
        format!("{:?}<D={}>", self, D)
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        let mut constraints = Vec::with_capacity(self.num_constraints());

        for copy in 0..self.num_copies {
            let access_index = vars.local_wires[self.wire_access_index(copy)];
            let list_items = (0..self.vec_size)
                .map(|i| vars.local_wires[self.wire_list_item(i, copy)])
                .collect::<Vec<_>>();
            let claimed_element = vars.local_wires[self.wire_claimed_element(copy)];

            for i in 0..self.vec_size {
                let cur_index = F::Extension::from_canonical_usize(i);
                let difference = cur_index - access_index;
                let equality_dummy = vars.local_wires[self.wire_equality_dummy_for_index(i, copy)];
                let index_matches = vars.local_wires[self.wire_index_matches_for_index(i, copy)];

                // The two index equality constraints.
                constraints.push(difference * equality_dummy - (F::Extension::ONE - index_matches));
                constraints.push(index_matches * difference);
                // Value equality constraint.
                constraints.push((list_items[i] - claimed_element) * index_matches);
            }
        }

        constraints
    }

    fn eval_unfiltered_base(&self, vars: EvaluationVarsBase<F>) -> Vec<F> {
        let mut constraints = Vec::with_capacity(self.num_constraints());

        for copy in 0..self.num_copies {
            let access_index = vars.local_wires[self.wire_access_index(copy)];
            let list_items = (0..self.vec_size)
                .map(|i| vars.local_wires[self.wire_list_item(i, copy)])
                .collect::<Vec<_>>();
            let claimed_element = vars.local_wires[self.wire_claimed_element(copy)];

            for i in 0..self.vec_size {
                let cur_index = F::from_canonical_usize(i);
                let difference = cur_index - access_index;
                let equality_dummy = vars.local_wires[self.wire_equality_dummy_for_index(i, copy)];
                let index_matches = vars.local_wires[self.wire_index_matches_for_index(i, copy)];

                // The two index equality constraints.
                constraints.push(difference * equality_dummy - (F::ONE - index_matches));
                constraints.push(index_matches * difference);
                // Value equality constraint.
                constraints.push((list_items[i] - claimed_element) * index_matches);
            }
        }

        constraints
    }

    fn eval_unfiltered_recursively(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>> {
        let mut constraints = Vec::with_capacity(self.num_constraints());

        for copy in 0..self.num_copies {
            let access_index = vars.local_wires[self.wire_access_index(copy)];
            let list_items = (0..self.vec_size)
                .map(|i| vars.local_wires[self.wire_list_item(i, copy)])
                .collect::<Vec<_>>();
            let claimed_element = vars.local_wires[self.wire_claimed_element(copy)];

            for i in 0..self.vec_size {
                let cur_index_ext = F::Extension::from_canonical_usize(i);
                let cur_index = builder.constant_extension(cur_index_ext);
                let difference = builder.sub_extension(cur_index, access_index);
                let equality_dummy = vars.local_wires[self.wire_equality_dummy_for_index(i, copy)];
                let index_matches = vars.local_wires[self.wire_index_matches_for_index(i, copy)];

                let one = builder.one_extension();
                let not_index_matches = builder.sub_extension(one, index_matches);
                let first_equality_constraint =
                    builder.mul_sub_extension(difference, equality_dummy, not_index_matches);
                constraints.push(first_equality_constraint);

                let second_equality_constraint = builder.mul_extension(index_matches, difference);
                constraints.push(second_equality_constraint);

                // Output constraint.
                let diff = builder.sub_extension(list_items[i], claimed_element);
                let conditional_diff = builder.mul_extension(index_matches, diff);
                constraints.push(conditional_diff);
            }
        }

        constraints
    }

    fn generators(
        &self,
        gate_index: usize,
        _local_constants: &[F],
    ) -> Vec<Box<dyn WitnessGenerator<F>>> {
        (0..self.num_copies)
            .map(|copy| {
                let g: Box<dyn WitnessGenerator<F>> = Box::new(
                    RandomAccessGenerator {
                        gate_index,
                        gate: *self,
                        copy,
                    }
                    .adapter(),
                );
                g
            })
            .collect::<Vec<_>>()
    }

    fn num_wires(&self) -> usize {
        self.wire_index_matches_for_index(self.vec_size - 1, self.num_copies - 1) + 1
    }

    fn num_constants(&self) -> usize {
        0
    }

    fn degree(&self) -> usize {
        2
    }

    fn num_constraints(&self) -> usize {
        3 * self.num_copies * self.vec_size
    }
}

#[derive(Debug)]
struct RandomAccessGenerator<F: RichField + Extendable<D>, const D: usize> {
    gate_index: usize,
    gate: RandomAccessGate<F, D>,
    copy: usize,
}

impl<F: RichField + Extendable<D>, const D: usize> SimpleGenerator<F>
    for RandomAccessGenerator<F, D>
{
    fn dependencies(&self) -> Vec<Target> {
        let local_target = |input| Target::wire(self.gate_index, input);

        let mut deps = Vec::new();
        deps.push(local_target(self.gate.wire_access_index(self.copy)));
        deps.push(local_target(self.gate.wire_claimed_element(self.copy)));
        for i in 0..self.gate.vec_size {
            deps.push(local_target(self.gate.wire_list_item(i, self.copy)));
        }
        deps
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let local_wire = |input| Wire {
            gate: self.gate_index,
            input,
        };

        let get_local_wire = |input| witness.get_wire(local_wire(input));

        // Compute the new vector and the values for equality_dummy and index_matches
        let vec_size = self.gate.vec_size;
        let access_index_f = get_local_wire(self.gate.wire_access_index(self.copy));

        let access_index = access_index_f.to_canonical_u64() as usize;
        debug_assert!(
            access_index < vec_size,
            "Access index {} is larger than the vector size {}",
            access_index,
            vec_size
        );

        for i in 0..vec_size {
            let equality_dummy_wire =
                local_wire(self.gate.wire_equality_dummy_for_index(i, self.copy));
            let index_matches_wire =
                local_wire(self.gate.wire_index_matches_for_index(i, self.copy));

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
    use rand::{thread_rng, Rng};

    use crate::field::field_types::Field;
    use crate::field::goldilocks_field::GoldilocksField;
    use crate::gates::gate::Gate;
    use crate::gates::gate_testing::{test_eval_fns, test_low_degree};
    use crate::gates::random_access::RandomAccessGate;
    use crate::hash::hash_types::HashOut;
    use crate::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use crate::plonk::vars::EvaluationVars;

    #[test]
    fn low_degree() {
        test_low_degree::<GoldilocksField, _, 4>(RandomAccessGate::new(4, 4));
    }

    #[test]
    fn eval_fns() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        test_eval_fns::<F, C, _, D>(RandomAccessGate::new(4, 4))
    }

    #[test]
    fn test_gate_constraint() {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type FF = <C as GenericConfig<D>>::FE;

        /// Returns the local wires for a random access gate given the vectors, elements to compare,
        /// and indices.
        fn get_wires(
            lists: Vec<Vec<F>>,
            access_indices: Vec<usize>,
            claimed_elements: Vec<F>,
        ) -> Vec<FF> {
            let num_copies = lists.len();
            let vec_size = lists[0].len();

            let mut v = Vec::new();
            let mut equality_dummy_vals = Vec::new();
            let mut index_matches_vals = Vec::new();
            for copy in 0..num_copies {
                let access_index = access_indices[copy];
                v.push(F::from_canonical_usize(access_index));
                v.push(claimed_elements[copy]);
                for j in 0..vec_size {
                    v.push(lists[copy][j]);
                }

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
            }
            v.extend(equality_dummy_vals);
            v.extend(index_matches_vals);

            v.iter().map(|&x| x.into()).collect::<Vec<_>>()
        }

        let vec_size = 3;
        let num_copies = 4;
        let lists = (0..num_copies)
            .map(|_| F::rand_vec(vec_size))
            .collect::<Vec<_>>();
        let access_indices = (0..num_copies)
            .map(|_| thread_rng().gen_range(0..vec_size))
            .collect::<Vec<_>>();
        let gate = RandomAccessGate::<F, D> {
            vec_size,
            num_copies,
            _phantom: PhantomData,
        };

        let good_claimed_elements = lists
            .iter()
            .zip(&access_indices)
            .map(|(l, &i)| l[i])
            .collect();
        let good_vars = EvaluationVars {
            local_constants: &[],
            local_wires: &get_wires(lists.clone(), access_indices.clone(), good_claimed_elements),
            public_inputs_hash: &HashOut::rand(),
        };
        let bad_claimed_elements = F::rand_vec(4);
        let bad_vars = EvaluationVars {
            local_constants: &[],
            local_wires: &get_wires(lists, access_indices, bad_claimed_elements),
            public_inputs_hash: &HashOut::rand(),
        };

        assert!(
            gate.eval_unfiltered(good_vars).iter().all(|x| x.is_zero()),
            "Gate constraints are not satisfied."
        );
        assert!(
            !gate.eval_unfiltered(bad_vars).iter().all(|x| x.is_zero()),
            "Gate constraints are satisfied but should not be."
        );
    }
}
