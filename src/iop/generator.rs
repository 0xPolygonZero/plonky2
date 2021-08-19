use std::convert::{identity, TryInto};
use std::fmt::Debug;

use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::{Extendable, FieldExtension};
use crate::field::field_types::Field;
use crate::hash::hash_types::{HashOut, HashOutTarget, MerkleCapTarget};
use crate::hash::merkle_tree::MerkleCap;
use crate::iop::target::{BoolTarget, Target};
use crate::iop::wire::Wire;
use crate::iop::witness::{PartialWitness, Witness};
use crate::plonk::permutation_argument::ForestNode;
use crate::timed;
use crate::util::timing::TimingTree;

pub struct Yo<F: Field>(
    pub Vec<ForestNode<Target, F>>,
    pub Box<dyn Fn(Target) -> usize>,
);
impl<F: Field> Yo<F> {
    pub fn get_target(&self, target: Target) -> F {
        self.0[self.0[self.1(target)].parent].value.unwrap()
    }

    pub fn get_targets(&self, targets: &[Target]) -> Vec<F> {
        targets.iter().map(|&t| self.get_target(t)).collect()
    }

    pub fn get_extension_target<const D: usize>(&self, et: ExtensionTarget<D>) -> F::Extension
    where
        F: Extendable<D>,
    {
        F::Extension::from_basefield_array(
            self.get_targets(&et.to_target_array()).try_into().unwrap(),
        )
    }

    pub fn get_extension_targets<const D: usize>(
        &self,
        ets: &[ExtensionTarget<D>],
    ) -> Vec<F::Extension>
    where
        F: Extendable<D>,
    {
        ets.iter()
            .map(|&et| self.get_extension_target(et))
            .collect()
    }

    pub fn get_bool_target(&self, target: BoolTarget) -> bool {
        let value = self.get_target(target.target).to_canonical_u64();
        match value {
            0 => false,
            1 => true,
            _ => panic!("not a bool"),
        }
    }

    pub fn get_hash_target(&self, ht: HashOutTarget) -> HashOut<F> {
        HashOut {
            elements: self.get_targets(&ht.elements).try_into().unwrap(),
        }
    }

    pub fn try_get_target(&self, target: Target) -> Option<F> {
        self.0[self.0[self.1(target)].parent].value
    }

    pub fn get_wire(&self, wire: Wire) -> F {
        self.get_target(Target::Wire(wire))
    }

    pub fn try_get_wire(&self, wire: Wire) -> Option<F> {
        self.try_get_target(Target::Wire(wire))
    }

    pub fn contains(&self, target: Target) -> bool {
        self.0[self.0[self.1(target)].parent].value.is_some()
    }

    pub fn contains_all(&self, targets: &[Target]) -> bool {
        targets.iter().all(|&t| self.contains(t))
    }

    pub fn set_target(&mut self, target: Target, value: F) {
        let i = self.0[self.1(target)].parent;
        self.0[i].value = Some(value);
    }

    pub fn set_hash_target(&mut self, ht: HashOutTarget, value: HashOut<F>) {
        ht.elements
            .iter()
            .zip(value.elements)
            .for_each(|(&t, x)| self.set_target(t, x));
    }

    pub fn set_cap_target(&mut self, ct: &MerkleCapTarget, value: &MerkleCap<F>) {
        for (ht, h) in ct.0.iter().zip(&value.0) {
            self.set_hash_target(*ht, *h);
        }
    }

    pub fn set_extension_target<const D: usize>(
        &mut self,
        et: ExtensionTarget<D>,
        value: F::Extension,
    ) where
        F: Extendable<D>,
    {
        let limbs = value.to_basefield_array();
        (0..D).for_each(|i| {
            self.set_target(et.0[i], limbs[i]);
        });
    }

    pub fn set_extension_targets<const D: usize>(
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

    pub fn set_bool_target(&mut self, target: BoolTarget, value: bool) {
        self.set_target(target.target, F::from_bool(value))
    }

    pub fn set_wire(&mut self, wire: Wire, value: F) {
        self.set_target(Target::Wire(wire), value)
    }

    pub fn set_wires<W>(&mut self, wires: W, values: &[F])
    where
        W: IntoIterator<Item = Wire>,
    {
        // If we used itertools, we could use zip_eq for extra safety.
        for (wire, &value) in wires.into_iter().zip(values) {
            self.set_wire(wire, value);
        }
    }

    pub fn set_ext_wires<W, const D: usize>(&mut self, wires: W, value: F::Extension)
    where
        F: Extendable<D>,
        W: IntoIterator<Item = Wire>,
    {
        self.set_wires(wires, &value.to_basefield_array());
    }

    pub fn extend<I: Iterator<Item = (Target, F)>>(&mut self, pairs: I) {
        for (t, v) in pairs {
            self.set_target(t, v);
        }
    }

    pub fn full_witness(self, degree: usize, num_wires: usize) -> Witness<F> {
        let mut wire_values = vec![vec![F::ZERO; degree]; num_wires];
        // assert!(self.wire_values.len() <= degree);
        for i in 0..degree {
            for j in 0..num_wires {
                let t = Target::Wire(Wire { gate: i, input: j });
                wire_values[j][i] = self.0[self.0[self.1(t)].parent].value.unwrap_or(F::ZERO);
            }
        }
        Witness { wire_values }
    }
}

/// Given a `PartialWitness` that has only inputs set, populates the rest of the witness using the
/// given set of generators.
pub(crate) fn generate_partial_witness<F: Field>(
    witness: &mut Yo<F>,
    generators: &[Box<dyn WitnessGenerator<F>>],
    num_wires: usize,
    degree: usize,
    max_virtual_target: usize,
    timing: &mut TimingTree,
) {
    // let target_index = |t: Target| -> usize {
    //     match t {
    //         Target::Wire(Wire { gate, input }) => gate * num_wires + input,
    //         Target::VirtualTarget { index } => degree * num_wires + index,
    //     }
    // };
    let max_target_index = witness.0.len();
    // Index generator indices by their watched targets.
    let mut generator_indices_by_watches = vec![Vec::new(); max_target_index];
    timed!(timing, "index generators by their watched targets", {
        for (i, generator) in generators.iter().enumerate() {
            for watch in generator.watch_list() {
                generator_indices_by_watches[witness.1(watch)].push(i);
            }
        }
    });

    // Build a list of "pending" generators which are queued to be run. Initially, all generators
    // are queued.
    let mut pending_generator_indices: Vec<_> = (0..generators.len()).collect();

    // We also track a list of "expired" generators which have already returned false.
    let mut generator_is_expired = vec![false; generators.len()];

    let mut buffer = GeneratedValues::empty();

    // Keep running generators until no generators are queued.
    while !pending_generator_indices.is_empty() {
        let mut next_pending_generator_indices = Vec::new();

        for &generator_idx in &pending_generator_indices {
            if generator_is_expired[generator_idx] {
                continue;
            }

            let finished = generators[generator_idx].run(&witness, &mut buffer);
            if finished {
                generator_is_expired[generator_idx] = true;
            }

            // Enqueue unfinished generators that were watching one of the newly populated targets.
            for &(watch, _) in &buffer.target_values {
                for &watching_generator_idx in &generator_indices_by_watches[witness.1(watch)] {
                    next_pending_generator_indices.push(watching_generator_idx);
                }
            }

            witness.extend(buffer.target_values.drain(..));
        }

        pending_generator_indices = next_pending_generator_indices;
    }

    for i in 0..degree {
        for j in 0..num_wires {
            if !witness.contains(Target::Wire(Wire { gate: i, input: j })) {
                println!("{} {}", i, j);
            }
        }
    }
    // for i in 0..generator_is_expired.len() {
    //     if !generator_is_expired[i] {
    //         println!("{:?}", generators[i]);
    //         println!("{:?}", generators[i].watch_list());
    //     }
    // }
    assert!(
        generator_is_expired.into_iter().all(identity),
        "Some generators weren't run."
    );
}

/// A generator participates in the generation of the witness.
pub trait WitnessGenerator<F: Field>: 'static + Send + Sync + Debug {
    /// Targets to be "watched" by this generator. Whenever a target in the watch list is populated,
    /// the generator will be queued to run.
    fn watch_list(&self) -> Vec<Target>;

    /// Run this generator, returning a flag indicating whether the generator is finished. If the
    /// flag is true, the generator will never be run again, otherwise it will be queued for another
    /// run next time a target in its watch list is populated.
    fn run(&self, witness: &Yo<F>, out_buffer: &mut GeneratedValues<F>) -> bool;
}

/// Values generated by a generator invocation.
pub struct GeneratedValues<F: Field> {
    pub(crate) target_values: Vec<(Target, F)>,
}

impl<F: Field> From<Vec<(Target, F)>> for GeneratedValues<F> {
    fn from(target_values: Vec<(Target, F)>) -> Self {
        Self { target_values }
    }
}

impl<F: Field> GeneratedValues<F> {
    pub fn with_capacity(capacity: usize) -> Self {
        Vec::with_capacity(capacity).into()
    }

    pub fn empty() -> Self {
        Vec::new().into()
    }

    pub fn singleton_wire(wire: Wire, value: F) -> Self {
        Self::singleton_target(Target::Wire(wire), value)
    }

    pub fn singleton_target(target: Target, value: F) -> Self {
        vec![(target, value)].into()
    }

    pub fn clear(&mut self) {
        self.target_values.clear();
    }

    pub fn singleton_extension_target<const D: usize>(
        et: ExtensionTarget<D>,
        value: F::Extension,
    ) -> Self
    where
        F: Extendable<D>,
    {
        let mut witness = Self::with_capacity(D);
        witness.set_extension_target(et, value);
        witness
    }

    pub fn set_target(&mut self, target: Target, value: F) {
        self.target_values.push((target, value))
    }

    pub fn set_hash_target(&mut self, ht: HashOutTarget, value: HashOut<F>) {
        ht.elements
            .iter()
            .zip(value.elements)
            .for_each(|(&t, x)| self.set_target(t, x));
    }

    pub fn set_extension_target<const D: usize>(
        &mut self,
        et: ExtensionTarget<D>,
        value: F::Extension,
    ) where
        F: Extendable<D>,
    {
        let limbs = value.to_basefield_array();
        (0..D).for_each(|i| {
            self.set_target(et.0[i], limbs[i]);
        });
    }

    pub fn set_wire(&mut self, wire: Wire, value: F) {
        self.set_target(Target::Wire(wire), value)
    }

    pub fn set_wires<W>(&mut self, wires: W, values: &[F])
    where
        W: IntoIterator<Item = Wire>,
    {
        // If we used itertools, we could use zip_eq for extra safety.
        for (wire, &value) in wires.into_iter().zip(values) {
            self.set_wire(wire, value);
        }
    }

    pub fn set_ext_wires<W, const D: usize>(&mut self, wires: W, value: F::Extension)
    where
        F: Extendable<D>,
        W: IntoIterator<Item = Wire>,
    {
        self.set_wires(wires, &value.to_basefield_array());
    }
}

/// A generator which runs once after a list of dependencies is present in the witness.
pub trait SimpleGenerator<F: Field>: 'static + Send + Sync + Debug {
    fn dependencies(&self) -> Vec<Target>;

    fn run_once(&self, witness: &Yo<F>, out_buffer: &mut GeneratedValues<F>);
}

impl<F: Field, SG: SimpleGenerator<F>> WitnessGenerator<F> for SG {
    fn watch_list(&self) -> Vec<Target> {
        self.dependencies()
    }

    fn run(&self, witness: &Yo<F>, out_buffer: &mut GeneratedValues<F>) -> bool {
        if witness.contains_all(&self.dependencies()) {
            self.run_once(witness, out_buffer);
            true
        } else {
            false
        }
    }
}

/// A generator which copies one wire to another.
#[derive(Debug)]
pub(crate) struct CopyGenerator {
    pub(crate) src: Target,
    pub(crate) dst: Target,
}

impl<F: Field> SimpleGenerator<F> for CopyGenerator {
    fn dependencies(&self) -> Vec<Target> {
        vec![self.src]
    }

    fn run_once(&self, witness: &Yo<F>, out_buffer: &mut GeneratedValues<F>) {
        let value = witness.get_target(self.src);
        out_buffer.set_target(self.dst, value);
    }
}

/// A generator for including a random value
#[derive(Debug)]
pub(crate) struct RandomValueGenerator {
    pub(crate) target: Target,
}

impl<F: Field> SimpleGenerator<F> for RandomValueGenerator {
    fn dependencies(&self) -> Vec<Target> {
        Vec::new()
    }

    fn run_once(&self, _witness: &Yo<F>, out_buffer: &mut GeneratedValues<F>) {
        let random_value = F::rand();

        out_buffer.set_target(self.target, random_value);
    }
}

/// A generator for testing if a value equals zero
#[derive(Debug)]
pub(crate) struct NonzeroTestGenerator {
    pub(crate) to_test: Target,
    pub(crate) dummy: Target,
}

impl<F: Field> SimpleGenerator<F> for NonzeroTestGenerator {
    fn dependencies(&self) -> Vec<Target> {
        vec![self.to_test]
    }

    fn run_once(&self, witness: &Yo<F>, out_buffer: &mut GeneratedValues<F>) {
        let to_test_value = witness.get_target(self.to_test);

        let dummy_value = if to_test_value == F::ZERO {
            F::ONE
        } else {
            to_test_value.inverse()
        };

        out_buffer.set_target(self.dummy, dummy_value);
    }
}
