use std::collections::HashMap;

use crate::field::field::Field;
use crate::target::Target2;
use crate::wire::Wire;

#[derive(Debug)]
pub struct PartialWitness<F: Field> {
    target_values: HashMap<Target2, F>,
}

impl<F: Field> PartialWitness<F> {
    pub fn new() -> Self {
        PartialWitness {
            target_values: HashMap::new(),
        }
    }

    pub fn singleton(target: Target2, value: F) -> Self {
        let mut witness = PartialWitness::new();
        witness.set_target(target, value);
        witness
    }

    pub fn is_empty(&self) -> bool {
        self.target_values.is_empty()
    }

    pub fn get_target(&self, target: Target2) -> F {
        self.target_values[&target]
    }

    pub fn try_get_target(&self, target: Target2) -> Option<F> {
        self.target_values.get(&target).cloned()
    }

    pub fn get_wire(&self, wire: Wire) -> F {
        self.get_target(Target2::Wire(wire))
    }

    pub fn contains(&self, target: Target2) -> bool {
        self.target_values.contains_key(&target)
    }

    pub fn contains_all(&self, targets: &[Target2]) -> bool {
        targets.iter().all(|&t| self.contains(t))
    }

    pub fn set_target(&mut self, target: Target2, value: F) {
        self.target_values.insert(target, value);
    }

    pub fn set_wire(&mut self, wire: Wire, value: F) {
        self.set_target(Target2::Wire(wire), value)
    }
}
