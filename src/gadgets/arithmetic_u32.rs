use std::collections::BTreeMap;
use std::marker::PhantomData;

use crate::field::field_types::RichField;
use crate::field::{extension_field::Extendable, field_types::Field};
use crate::gates::arithmetic_u32::U32ArithmeticGate;
use crate::gates::switch::SwitchGate;
use crate::iop::generator::{GeneratedValues, SimpleGenerator};
use crate::iop::target::Target;
use crate::iop::witness::{PartitionWitness, Witness};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::util::bimap::bimap_from_lists;

pub struct U32Target(Target);

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    pub fn add_virtual_u32_target(&self) -> U32Target {
        U32Target(self.add_virtual_target())
    }

    pub fn zero_u32(&self) -> U32Target {
        U32Target(self.zero())
    }

    pub fn one_u32(&self) -> U32Target {
        U32Target(self.one())
    }

    pub fn add_mul_u32(
        &mut self,
        x: U32Target,
        y: U32Target,
        z: U32Target,
    ) -> (U32Target, U32Target) {
        let (gate_index, copy) = match self.current_u32_arithmetic_gate {
            None => {
                let gate = U32ArithmeticGate {
                    _phantom: PhantomData,
                };
                let gate_index = self.add_gate(gate.clone(), vec![]);
                (gate_index, 0)
            },
            Some((gate_index, copy) => (gate_index, copy),
        };

        let output_low = self.add_virtual_u32_target();
        let output_high = self.add_virtual_u32_target();

        self.connect(Target::wire(gate_index, gate.wire_ith_multiplicand_0(copy)), x);
        self.connect(Target::wire(gate_index, gate.wire_ith_multiplicand_1(copy)), y);
        self.connect(Target::wire(gate_index, gate.wire_ith_addend(copy)), z);
        self.connect(Target::wire(gate_index, gate.wire_ith_output_low_half(copy)), output_low);
        self.connect(Target::wire(gate_index, gate.wire_ith_output_high_half(copy)), output_high);

        self.current_u32_arithmetic_gate = Some((gate_index, 0));

        (output_low, output_high)
    }

    pub fn add_u32(&mut self, a: U32Target, b: U32Target) -> (U32Target, U32Target) {
        self.add_mul_u32(a, self.one_u32(), b)
    }

    pub fn mul_u32(&mut self, a: U32Target, b: U32Target) -> (U32Target, U32Target) {
        self.add_mul_u32(a, b, self.zero_u32())
    }
}
