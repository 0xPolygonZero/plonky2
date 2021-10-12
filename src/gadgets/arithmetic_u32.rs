use std::marker::PhantomData;

use crate::field::field_types::RichField;
use crate::field::extension_field::Extendable;
use crate::gates::arithmetic_u32::{U32ArithmeticGate, NUM_U32_ARITHMETIC_OPS};
use crate::iop::target::Target;
use crate::plonk::circuit_builder::CircuitBuilder;

#[derive(Clone)]
pub struct U32Target(pub Target);

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    pub fn add_virtual_u32_target(&mut self) -> U32Target {
        U32Target(self.add_virtual_target())
    }

    pub fn add_virtual_u32_targets(&mut self, n: usize) -> Vec<U32Target> {
        self.add_virtual_targets(n)
            .into_iter()
            .map(U32Target)
            .collect()
    }

    pub fn zero_u32(&mut self) -> U32Target {
        U32Target(self.zero())
    }

    pub fn one_u32(&mut self) -> U32Target {
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
                let gate = U32ArithmeticGate::new();
                let gate_index = self.add_gate(gate, vec![]);
                (gate_index, 0)
            }
            Some((gate_index, copy)) => (gate_index, copy),
        };

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

        let output_low = U32Target(Target::wire(
            gate_index,
            U32ArithmeticGate::<F, D>::wire_ith_output_low_half(copy),
        ));
        let output_high = U32Target(Target::wire(
            gate_index,
            U32ArithmeticGate::<F, D>::wire_ith_output_high_half(copy),
        ));

        if copy == NUM_U32_ARITHMETIC_OPS - 1 {
            self.current_u32_arithmetic_gate = None;
        } else {
            self.current_u32_arithmetic_gate = Some((gate_index, copy + 1));
        }

        (output_low, output_high)
    }

    pub fn add_u32(&mut self, a: U32Target, b: U32Target) -> (U32Target, U32Target) {
        let one = self.one_u32();
        self.mul_add_u32(a, one, b)
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
        let zero = self.zero_u32();
        self.mul_add_u32(a, b, zero)
    }
}
