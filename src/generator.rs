use std::collections::{HashMap, HashSet};
use std::fmt::Debug;

use crate::field::field::Field;
use crate::target::Target;
use crate::witness::PartialWitness;

/// Given a `PartialWitness` that has only inputs set, populates the rest of the witness using the
/// given set of generators.
pub(crate) fn generate_partial_witness<F: Field>(
    witness: &mut PartialWitness<F>,
    generators: &[Box<dyn WitnessGenerator<F>>],
) {
    // Index generator indices by their watched targets.
    let mut generator_indices_by_watches = HashMap::new();
    for (i, generator) in generators.iter().enumerate() {
        for watch in generator.watch_list() {
            generator_indices_by_watches
                .entry(watch)
                .or_insert_with(Vec::new)
                .push(i);
        }
    }

    // Build a list of "pending" generators which are queued to be run. Initially, all generators
    // are queued.
    let mut pending_generator_indices = HashSet::new();
    for i in 0..generators.len() {
        pending_generator_indices.insert(i);
    }

    // We also track a list of "expired" generators which have already returned false.
    let mut expired_generator_indices = HashSet::new();

    // Keep running generators until no generators are queued.
    while !pending_generator_indices.is_empty() {
        let mut next_pending_generator_indices = HashSet::new();

        for &generator_idx in &pending_generator_indices {
            let (result, finished) = generators[generator_idx].run(&witness);
            if finished {
                expired_generator_indices.insert(generator_idx);
            }

            // Enqueue unfinished generators that were watching one of the newly populated targets.
            for watch in result.target_values.keys() {
                if let Some(watching_generator_indices) = generator_indices_by_watches.get(watch) {
                    for watching_generator_idx in watching_generator_indices {
                        if !expired_generator_indices.contains(watching_generator_idx) {
                            next_pending_generator_indices.insert(*watching_generator_idx);
                        }
                    }
                }
            }

            witness.extend(result);
        }

        pending_generator_indices = next_pending_generator_indices;
    }
}

/// A generator participates in the generation of the witness.
pub trait WitnessGenerator<F: Field>: 'static + Send + Sync {
    /// Targets to be "watched" by this generator. Whenever a target in the watch list is populated,
    /// the generator will be queued to run.
    fn watch_list(&self) -> Vec<Target>;

    /// Run this generator, returning a `PartialWitness` containing any new witness elements, and a
    /// flag indicating whether the generator is finished. If the flag is true, the generator will
    /// never be run again, otherwise it will be queued for another run next time a target in its
    /// watch list is populated.
    fn run(&self, witness: &PartialWitness<F>) -> (PartialWitness<F>, bool);
}

/// A generator which runs once after a list of dependencies is present in the witness.
pub trait SimpleGenerator<F: Field>: 'static + Send + Sync {
    fn dependencies(&self) -> Vec<Target>;

    fn run_once(&self, witness: &PartialWitness<F>) -> PartialWitness<F>;
}

impl<F: Field, SG: SimpleGenerator<F>> WitnessGenerator<F> for SG {
    fn watch_list(&self) -> Vec<Target> {
        self.dependencies()
    }

    fn run(&self, witness: &PartialWitness<F>) -> (PartialWitness<F>, bool) {
        if witness.contains_all(&self.dependencies()) {
            (self.run_once(witness), true)
        } else {
            (PartialWitness::new(), false)
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

    fn run_once(&self, witness: &PartialWitness<F>) -> PartialWitness<F> {
        let value = witness.get_target(self.src);
        PartialWitness::singleton_target(self.dst, value)
    }
}

/// A generator for including a random value
struct RandomValueGenerator {
    pub(crate) target: Target,
}

impl<F: Field> SimpleGenerator<F> for RandomValueGenerator {
    fn dependencies(&self) -> Vec<Target> {
        Vec::new()
    }

    fn run_once(&self, _witness: &PartialWitness<F>) -> PartialWitness<F> {
        let random_value = F::rand();
        
        PartialWitness::singleton_target(self.target, random_value)
    }
}
