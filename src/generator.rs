use crate::field::field::Field;
use crate::target::Target;
use crate::witness::PartialWitness;

/// A generator participates in the generation of the witness.
pub trait WitnessGenerator2<F: Field>: 'static {
    /// Targets to be "watched" by this generator. Whenever a target in the watch list is populated,
    /// the generator will be queued to run.
    fn watch_list(&self) -> Vec<Target>;

    /// Run this generator, returning a `PartialWitness` containing any new witness elements, and a
    /// flag indicating whether the generator is finished. If the flag is true, the generator will
    /// never be run again, otherwise it will be queued for another run next time a target in its
    /// watch list is populated.
    fn run(&mut self, witness: &PartialWitness<F>) -> (PartialWitness<F>, bool);
}

/// A generator which runs once after a list of dependencies is present in the witness.
pub trait SimpleGenerator<F: Field>: 'static {
    fn dependencies(&self) -> Vec<Target>;

    fn run_once(&mut self, witness: &PartialWitness<F>) -> PartialWitness<F>;
}

impl<F: Field, SG: SimpleGenerator<F>> WitnessGenerator2<F> for SG {
    fn watch_list(&self) -> Vec<Target> {
        self.dependencies()
    }

    fn run(&mut self, witness: &PartialWitness<F>) -> (PartialWitness<F>, bool) {
        if witness.contains_all(&self.dependencies()) {
            (self.run_once(witness), true)
        } else {
            (PartialWitness::new(), false)
        }
    }
}

/// A generator which copies one wire to another.
pub(crate) struct CopyGenerator {
    pub(crate) src: Target,
    pub(crate) dst: Target,
}

impl<F: Field> SimpleGenerator<F> for CopyGenerator {
    fn dependencies(&self) -> Vec<Target> {
        vec![self.src]
    }

    fn run_once(&mut self, witness: &PartialWitness<F>) -> PartialWitness<F> {
        let value = witness.get_target(self.src);
        PartialWitness::singleton(self.dst, value)
    }
}
