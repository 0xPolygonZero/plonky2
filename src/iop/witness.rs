use std::collections::HashMap;
use std::convert::TryInto;

use anyhow::{ensure, Result};

use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::{Extendable, FieldExtension};
use crate::field::field_types::Field;
use crate::gates::gate::GateInstance;
use crate::hash::hash_types::HashOut;
use crate::hash::hash_types::HashOutTarget;
use crate::iop::target::Target;
use crate::iop::wire::Wire;
use crate::plonk::copy_constraint::CopyConstraint;

#[derive(Clone, Debug)]
pub struct Witness<F: Field> {
    pub(crate) wire_values: Vec<Vec<F>>,
}

impl<F: Field> Witness<F> {
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

    pub fn is_empty(&self) -> bool {
        self.target_values.is_empty()
    }

    pub fn get_target(&self, target: Target) -> F {
        self.target_values[&target]
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

    pub fn get_hash_target(&self, ht: HashOutTarget) -> HashOut<F> {
        HashOut {
            elements: self.get_targets(&ht.elements).try_into().unwrap(),
        }
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
        for (target, value) in pairs {
            self.set_target(target, value);
        }
    }

    pub fn full_witness(self, degree: usize, num_wires: usize) -> Witness<F> {
        let mut wire_values = vec![vec![F::ZERO; degree]; num_wires];
        self.target_values.into_iter().for_each(|(t, v)| {
            if let Target::Wire(Wire { gate, input }) = t {
                wire_values[input][gate] = v;
            }
        });
        Witness { wire_values }
    }

    /// Checks that the copy constraints are satisfied in the witness.
    pub fn check_copy_constraints<const D: usize>(
        &self,
        copy_constraints: &[CopyConstraint],
        gate_instances: &[GateInstance<F, D>],
    ) -> Result<()>
    where
        F: Extendable<D>,
    {
        for CopyConstraint { pair: (a, b), name } in copy_constraints {
            let va = self.try_get_target(*a).unwrap_or(F::ZERO);
            let vb = self.try_get_target(*b).unwrap_or(F::ZERO);
            let desc = |t: &Target| -> String {
                match t {
                    Target::Wire(Wire { gate, input }) => format!(
                        "wire {} of gate #{} (`{}`)",
                        input,
                        gate,
                        gate_instances[*gate].gate_ref.0.id()
                    ),
                    Target::VirtualTarget { index } => format!("{}-th virtual target", index),
                }
            };
            ensure!(
                va == vb,
                "Copy constraint '{}' between {} and {} is not satisfied. \
                Got values of {} and {} respectively.",
                name,
                desc(a),
                desc(b),
                va,
                vb
            );
        }
        Ok(())
    }
}

impl<F: Field> Default for PartialWitness<F> {
    fn default() -> Self {
        Self::new()
    }
}
