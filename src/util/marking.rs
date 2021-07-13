use std::convert::TryInto;

use crate::field::extension_field::target::ExtensionTarget;
use crate::field::field::Field;
use crate::proof::HashTarget;
use crate::target::Target;
use crate::witness::{PartialWitness, Witness};

pub trait Markable {
    fn targets(&self) -> Vec<Target>;
}

impl Markable for Target {
    fn targets(&self) -> Vec<Target> {
        vec![*self]
    }
}

impl<const D: usize> Markable for ExtensionTarget<D> {
    fn targets(&self) -> Vec<Target> {
        self.0.try_into().unwrap()
    }
}

impl Markable for HashTarget {
    fn targets(&self) -> Vec<Target> {
        self.elements.try_into().unwrap()
    }
}

impl<M: Markable> Markable for Vec<M> {
    fn targets(&self) -> Vec<Target> {
        self.iter().flat_map(|m| m.targets()).collect()
    }
}

pub struct MarkedTargets {
    pub targets: Box<dyn Markable>,
    pub name: String,
}

impl MarkedTargets {
    pub fn display<F: Field>(&self, wit: &Witness<F>) {
        let targets = self.targets.targets();
        println!("Values for {}:", self.name);
        for &t in &targets {
            match t {
                Target::Wire(w) => println!("{}", wit.get_wire(w.gate, w.input)),
                _ => println!("Not a wire."),
            }
        }
        println!("End of values for {}", self.name);
    }
}
