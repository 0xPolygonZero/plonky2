use std::collections::HashMap;
use std::convert::TryInto;

use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::{Extendable, FieldExtension};
use crate::field::field_types::Field;
use crate::hash::hash_types::HashOutTarget;
use crate::hash::hash_types::{HashOut, MerkleCapTarget};
use crate::hash::merkle_tree::MerkleCap;
use crate::iop::target::{BoolTarget, Target};
use crate::iop::wire::Wire;
use crate::plonk::permutation_argument::ForestNode;

/// A witness holds information on the values of targets in a circuit.
pub trait Witness<F: Field> {
    fn try_get_target(&self, target: Target) -> Option<F>;

    fn set_target(&mut self, target: Target, value: F);

    fn get_target(&self, target: Target) -> F {
        self.try_get_target(target).unwrap()
    }

    fn get_targets(&self, targets: &[Target]) -> Vec<F> {
        targets.iter().map(|&t| self.get_target(t)).collect()
    }

    fn get_extension_target<const D: usize>(&self, et: ExtensionTarget<D>) -> F::Extension
    where
        F: Extendable<D>,
    {
        F::Extension::from_basefield_array(
            self.get_targets(&et.to_target_array()).try_into().unwrap(),
        )
    }

    fn get_extension_targets<const D: usize>(&self, ets: &[ExtensionTarget<D>]) -> Vec<F::Extension>
    where
        F: Extendable<D>,
    {
        ets.iter()
            .map(|&et| self.get_extension_target(et))
            .collect()
    }

    fn get_bool_target(&self, target: BoolTarget) -> bool {
        let value = self.get_target(target.target);
        if value.is_zero() {
            return false;
        }
        if value.is_one() {
            return true;
        }
        panic!("not a bool")
    }

    fn get_hash_target(&self, ht: HashOutTarget) -> HashOut<F> {
        HashOut {
            elements: self.get_targets(&ht.elements).try_into().unwrap(),
        }
    }

    fn get_wire(&self, wire: Wire) -> F {
        self.get_target(Target::Wire(wire))
    }

    fn try_get_wire(&self, wire: Wire) -> Option<F> {
        self.try_get_target(Target::Wire(wire))
    }

    fn contains(&self, target: Target) -> bool {
        self.try_get_target(target).is_some()
    }

    fn contains_all(&self, targets: &[Target]) -> bool {
        targets.iter().all(|&t| self.contains(t))
    }

    fn set_hash_target(&mut self, ht: HashOutTarget, value: HashOut<F>) {
        ht.elements
            .iter()
            .zip(value.elements)
            .for_each(|(&t, x)| self.set_target(t, x));
    }

    fn set_cap_target(&mut self, ct: &MerkleCapTarget, value: &MerkleCap<F>) {
        for (ht, h) in ct.0.iter().zip(&value.0) {
            self.set_hash_target(*ht, *h);
        }
    }

    fn set_extension_target<const D: usize>(&mut self, et: ExtensionTarget<D>, value: F::Extension)
    where
        F: Extendable<D>,
    {
        let limbs = value.to_basefield_array();
        (0..D).for_each(|i| {
            self.set_target(et.0[i], limbs[i]);
        });
    }

    fn set_extension_targets<const D: usize>(
        &mut self,
        ets: &[ExtensionTarget<D>],
        values: &[F::Extension],
    ) where
        F: Extendable<D>,
    {
        debug_assert_eq!(ets.len(), values.len());
        ets.iter()
            .zip(values)
            .for_each(|(&et, &v)| self.set_extension_target(et, v));
    }

    fn set_bool_target(&mut self, target: BoolTarget, value: bool) {
        self.set_target(target.target, F::from_bool(value))
    }

    fn set_wire(&mut self, wire: Wire, value: F) {
        self.set_target(Target::Wire(wire), value)
    }

    fn set_wires<W>(&mut self, wires: W, values: &[F])
    where
        W: IntoIterator<Item = Wire>,
    {
        // If we used itertools, we could use zip_eq for extra safety.
        for (wire, &value) in wires.into_iter().zip(values) {
            self.set_wire(wire, value);
        }
    }

    fn set_ext_wires<W, const D: usize>(&mut self, wires: W, value: F::Extension)
    where
        F: Extendable<D>,
        W: IntoIterator<Item = Wire>,
    {
        self.set_wires(wires, &value.to_basefield_array());
    }

    fn extend<I: Iterator<Item = (Target, F)>>(&mut self, pairs: I) {
        for (t, v) in pairs {
            self.set_target(t, v);
        }
    }
}

#[derive(Clone, Debug)]
pub struct MatrixWitness<F: Field> {
    pub(crate) wire_values: Vec<Vec<F>>,
}

impl<F: Field> MatrixWitness<F> {
    pub fn get_wire(&self, gate: usize, input: usize) -> F {
        self.wire_values[input][gate]
    }
}

#[derive(Clone, Debug)]
pub struct PartialWitness<F: Field> {
    pub(crate) target_values: HashMap<Target, F>,
}

impl<F: Field> PartialWitness<F> {
    pub fn new() -> Self {
        PartialWitness {
            target_values: HashMap::new(),
        }
    }
}

impl<F: Field> Witness<F> for PartialWitness<F> {
    fn try_get_target(&self, target: Target) -> Option<F> {
        self.target_values.get(&target).copied()
    }

    fn set_target(&mut self, target: Target, value: F) {
        let opt_old_value = self.target_values.insert(target, value);
        if let Some(old_value) = opt_old_value {
            assert_eq!(
                old_value, value,
                "Target {:?} was set twice with different values",
                target
            );
        }
    }
}

/// `PartitionWitness` holds a disjoint-set forest of the targets respecting a circuit's copy constraints.
/// The value of a target is defined to be the value of its root in the forest.
#[derive(Clone)]
pub struct PartitionWitness<F: Field> {
    pub forest: Vec<ForestNode<Target, F>>,
    pub num_wires: usize,
    pub num_routed_wires: usize,
    pub degree: usize,
}

impl<F: Field> Witness<F> for PartitionWitness<F> {
    fn try_get_target(&self, target: Target) -> Option<F> {
        let parent_index = self.forest[self.target_index(target)].parent;
        self.forest[parent_index].value
    }

    fn set_target(&mut self, target: Target, value: F) {
        let parent_index = self.forest[self.target_index(target)].parent;
        let parent_value = &mut self.forest[parent_index].value;
        if let Some(old_value) = *parent_value {
            assert_eq!(
                value, old_value,
                "Partition containing {:?} was set twice with different values",
                target
            );
        } else {
            *parent_value = Some(value);
        }
    }
}

impl<F: Field> PartitionWitness<F> {
    pub fn target_index(&self, target: Target) -> usize {
        match target {
            Target::Wire(Wire { gate, input }) => gate * self.num_wires + input,
            Target::VirtualTarget { index } => self.degree * self.num_wires + index,
        }
    }

    pub fn full_witness(self) -> MatrixWitness<F> {
        let mut wire_values = vec![vec![F::ZERO; self.degree]; self.num_wires];
        for i in 0..self.degree {
            for j in 0..self.num_wires {
                let t = Target::Wire(Wire { gate: i, input: j });
                if let Some(x) = self.try_get_target(t) {
                    wire_values[j][i] = x;
                }
            }
        }

        MatrixWitness { wire_values }
    }
}
