use std::collections::HashMap;

use crate::field::field::Field;
use crate::target::Target;
use crate::wire::Wire;

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

    pub fn singleton_wire(wire: Wire, value: F) -> Self {
        Self::singleton_target(Target::Wire(wire), value)
    }

    pub fn singleton_target(target: Target, value: F) -> Self {
        let mut witness = PartialWitness::new();
        witness.set_target(target, value);
        witness
    }

    pub fn is_empty(&self) -> bool {
        self.target_values.is_empty()
    }

    pub fn get_target(&self, target: Target) -> F {
        self.target_values[&target]
    }

    pub fn get_targets(&self, targets: &[Target]) -> Vec<F> {
        targets.iter().map(|&t| self.get_target(t)).collect()
    }

    pub fn try_get_target(&self, target: Target) -> Option<F> {
        self.target_values.get(&target).cloned()
    }

    pub fn get_wire(&self, wire: Wire) -> F {
        self.get_target(Target::Wire(wire))
    }

    pub fn try_get_wire(&self, wire: Wire) -> Option<F> {
        self.try_get_target(Target::Wire(wire))
    }

    pub fn contains(&self, target: Target) -> bool {
        self.target_values.contains_key(&target)
    }

    pub fn contains_all(&self, targets: &[Target]) -> bool {
        targets.iter().all(|&t| self.contains(t))
    }

    pub fn set_target(&mut self, target: Target, value: F) {
        let opt_old_value = self.target_values.insert(target, value);
        if let Some(old_value) = opt_old_value {
            assert_eq!(
                old_value, value,
                "Target was set twice with different values: {:?}",
                target
            );
        }
    }

    pub fn set_wire(&mut self, wire: Wire, value: F) {
        self.set_target(Target::Wire(wire), value)
    }

    pub fn extend(&mut self, other: PartialWitness<F>) {
        for (target, value) in other.target_values {
            self.set_target(target, value);
        }
    }
}
