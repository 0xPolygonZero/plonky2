use std::collections::BTreeMap;
use std::marker::PhantomData;

use crate::field::field_types::RichField;
use crate::field::{extension_field::Extendable, field_types::Field};
use crate::gates::arithmetic_u32::{NUM_U32_ARITHMETIC_OPS, U32ArithmeticGate};
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

    pub fn add_virtual_u32_targets(&self, n: usize) -> Vec<U32Target> {
        self.add_virtual_targets(n)
            .iter()
            .cloned()
            .map(U32Target)
            .collect()
    }

    pub fn zero_u32(&self) -> U32Target {
        U32Target(self.zero())
    }

    pub fn one_u32(&self) -> U32Target {
        U32Target(self.one())
    }

    // Returns x * y + z.
    pub fn mul_add_u32(
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
                let gate_index = self.add_gate(gate, vec![]);
                (gate_index, 0)
            }
            Some((gate_index, copy)) => (gate_index, copy),
        };

        let output_low = self.add_virtual_u32_target();
        let output_high = self.add_virtual_u32_target();

        self.connect(
            Target::wire(
                gate_index,
                U32ArithmeticGate::<F, D>::wire_ith_multiplicand_0(copy),
            ),
            x.0,
        );
        self.connect(
            Target::wire(
                gate_index,
                U32ArithmeticGate::<F, D>::wire_ith_multiplicand_1(copy),
            ),
            y.0,
        );
        self.connect(
            Target::wire(gate_index, U32ArithmeticGate::<F, D>::wire_ith_addend(copy)),
            z.0,
        );
        self.connect(
            Target::wire(
                gate_index,
                U32ArithmeticGate::<F, D>::wire_ith_output_low_half(copy),
            ),
            output_low.0,
        );
        self.connect(
            Target::wire(
                gate_index,
                U32ArithmeticGate::<F, D>::wire_ith_output_high_half(copy),
            ),
            output_high.0,
        );

        if copy == NUM_U32_ARITHMETIC_OPS - 1 {
            let gate = U32ArithmeticGate {
                _phantom: PhantomData,
            };
            let gate_index = self.add_gate(gate, vec![]);
            self.current_u32_arithmetic_gate = Some((gate_index, 0));
        } else {
            self.current_u32_arithmetic_gate = Some((gate_index, copy + 1));
        }

        (output_low, output_high)
    }

    pub fn add_u32(&mut self, a: U32Target, b: U32Target) -> (U32Target, U32Target) {
        self.mul_add_u32(a, self.one_u32(), b)
    }

    pub fn add_three_u32(
        &mut self,
        a: U32Target,
        b: U32Target,
        c: U32Target,
    ) -> (U32Target, U32Target) {
        let (init_low, carry1) = self.add_u32(a, b);
        let (final_low, carry2) = self.add_u32(c, init_low);
        let (combined_carry, _zero) = self.add_u32(carry1, carry2);
        (final_low, combined_carry)
    }

    pub fn mul_u32(&mut self, a: U32Target, b: U32Target) -> (U32Target, U32Target) {
        self.mul_add_u32(a, b, self.zero_u32())
    }
}
