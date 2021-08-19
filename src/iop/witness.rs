use std::convert::TryInto;

use anyhow::{ensure, Result};

use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::{Extendable, FieldExtension};
use crate::field::field_types::Field;
use crate::gates::gate::GateInstance;
use crate::hash::hash_types::HashOutTarget;
use crate::hash::hash_types::{HashOut, MerkleCapTarget};
use crate::hash::merkle_tree::MerkleCap;
use crate::iop::target::{BoolTarget, Target};
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
    pub(crate) wire_values: Vec<Vec<Option<F>>>,
    pub(crate) virtual_target_values: Vec<Option<F>>,
    pub(crate) set_targets: Vec<(Target, F)>,
}

impl<F: Field> PartialWitness<F> {
    pub fn new(num_wires: usize) -> Self {
        PartialWitness {
            wire_values: vec![vec![None; num_wires]],
            virtual_target_values: vec![],
            set_targets: vec![],
        }
    }

    pub fn get_target(&self, target: Target) -> F {
        match target {
            Target::Wire(Wire { gate, input }) => self.wire_values[gate][input].unwrap(),
            Target::VirtualTarget { index } => self.virtual_target_values[index].unwrap(),
        }
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
        match target {
            Target::Wire(Wire { gate, input }) => self.wire_values[gate][input],
            Target::VirtualTarget { index } => self.virtual_target_values[index],
        }
    }

    pub fn get_wire(&self, wire: Wire) -> F {
        self.get_target(Target::Wire(wire))
    }

    pub fn try_get_wire(&self, wire: Wire) -> Option<F> {
        self.try_get_target(Target::Wire(wire))
    }

    pub fn contains(&self, target: Target) -> bool {
        match target {
            Target::Wire(Wire { gate, input }) => {
                self.wire_values.len() > gate && self.wire_values[gate][input].is_some()
            }
            Target::VirtualTarget { index } => {
                self.virtual_target_values.len() > index
                    && self.virtual_target_values[index].is_some()
            }
        }
    }

    pub fn contains_all(&self, targets: &[Target]) -> bool {
        targets.iter().all(|&t| self.contains(t))
    }

    pub fn set_target(&mut self, target: Target, value: F) {
        match target {
            Target::Wire(Wire { gate, input }) => {
                if gate >= self.wire_values.len() {
                    self.wire_values
                        .resize(gate + 1, vec![None; self.wire_values[0].len()]);
                }
                if let Some(old_value) = self.wire_values[gate][input] {
                    assert_eq!(
                        old_value, value,
                        "Target was set twice with different values: {:?}",
                        target
                    );
                } else {
                    self.wire_values[gate][input] = Some(value);
                }
            }
            Target::VirtualTarget { index } => {
                if index >= self.virtual_target_values.len() {
                    self.virtual_target_values.resize(index + 1, None);
                }
                if let Some(old_value) = self.virtual_target_values[index] {
                    assert_eq!(
                        old_value, value,
                        "Target was set twice with different values: {:?}",
                        target
                    );
                } else {
                    self.virtual_target_values[index] = Some(value);
                }
            }
        }
        self.set_targets.push((target, value));
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
        assert!(self.wire_values.len() <= degree);
        for i in 0..self.wire_values.len() {
            for j in 0..num_wires {
                wire_values[j][i] = self.wire_values[i][j].unwrap_or(F::ZERO);
            }
        }
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
