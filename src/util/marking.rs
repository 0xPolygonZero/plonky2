use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::hash::hash_types::HashOutTarget;
use crate::iop::target::Target;
use crate::iop::witness::{PartitionWitness, Witness};

/// Enum representing all types of targets, so that they can be marked.
#[derive(Clone)]
pub enum Markable<const D: usize> {
    Target(Target),
    ExtensionTarget(ExtensionTarget<D>),
    HashTarget(HashOutTarget),
    Vec(Vec<Markable<D>>),
}

impl<const D: usize> From<Target> for Markable<D> {
    fn from(t: Target) -> Self {
        Self::Target(t)
    }
}
impl<const D: usize> From<ExtensionTarget<D>> for Markable<D> {
    fn from(et: ExtensionTarget<D>) -> Self {
        Self::ExtensionTarget(et)
    }
}
impl<const D: usize> From<HashOutTarget> for Markable<D> {
    fn from(ht: HashOutTarget) -> Self {
        Self::HashTarget(ht)
    }
}
impl<M: Into<Markable<D>>, const D: usize> From<Vec<M>> for Markable<D> {
    fn from(v: Vec<M>) -> Self {
        Self::Vec(v.into_iter().map(|m| m.into()).collect())
    }
}

impl<const D: usize> Markable<D> {
    /// Display a `Markable` by querying a partial witness.
    fn print_markable<F: Extendable<D>>(&self, pw: &PartitionWitness<F>) {
        match self {
            Markable::Target(t) => println!("{}", pw.get_target(*t)),
            Markable::ExtensionTarget(et) => println!("{}", pw.get_extension_target(*et)),
            Markable::HashTarget(ht) => println!("{:?}", pw.get_hash_target(*ht)),
            Markable::Vec(v) => v.iter().for_each(|m| m.print_markable(pw)),
        }
    }
}

/// A named collection of targets.
#[derive(Clone)]
pub struct MarkedTargets<const D: usize> {
    pub targets: Markable<D>,
    pub name: String,
}

impl<const D: usize> MarkedTargets<D> {
    /// Display the collection of targets along with its name by querying a partial witness.
    pub fn display<F: Extendable<D>>(&self, pw: &PartitionWitness<F>) {
        println!("Values for {}:", self.name);
        self.targets.print_markable(pw);
        println!("End of values for {}", self.name);
    }
}
