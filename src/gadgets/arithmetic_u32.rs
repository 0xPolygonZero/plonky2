use crate::field::extension_field::Extendable;
use crate::field::field_types::RichField;
use crate::gates::arithmetic_u32::U32ArithmeticGate;
use crate::gates::subtraction_u32::U32SubtractionGate;
use crate::iop::target::Target;
use crate::plonk::circuit_builder::CircuitBuilder;

#[derive(Clone, Copy, Debug)]
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

    pub fn connect_u32(&mut self, x: U32Target, y: U32Target) {
        self.connect(x.0, y.0)
    }

    pub fn assert_zero_u32(&mut self, x: U32Target) {
        self.assert_zero(x.0)
    }

    // Returns x * y + z.
    pub fn mul_add_u32(
        &mut self,
        x: U32Target,
        y: U32Target,
        z: U32Target,
    ) -> (U32Target, U32Target) {
        let (gate_index, copy) = self.find_u32_arithmetic_gate();

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

    pub fn add_many_u32(&mut self, to_add: Vec<U32Target>) -> (U32Target, U32Target) {
        match to_add.len() {
            0 => (self.zero_u32(), self.zero_u32()),
            1 => (to_add[0], self.zero_u32()),
            2 => self.add_u32(to_add[0], to_add[1]),
            3 => self.add_three_u32(to_add[0], to_add[1], to_add[2]),
            _ => {
                let (mut low, mut carry) = self.add_u32(to_add[0], to_add[1]);
                for i in 2..to_add.len() {
                    let (new_low, new_carry) = self.add_u32(to_add[i], low);
                    let (combined_carry, _zero) = self.add_u32(carry, new_carry);
                    low = new_low;
                    carry = combined_carry;
                }
                (low, carry)
            }
        }
    }

    pub fn mul_u32(&mut self, a: U32Target, b: U32Target) -> (U32Target, U32Target) {
        let zero = self.zero_u32();
        self.mul_add_u32(a, b, zero)
    }

    // Returns x - y - borrow, as a pair (result, borrow), where borrow is 0 or 1 depending on whether borrowing from the next digit is required (iff y + borrow > x).
    pub fn sub_u32(
        &mut self,
        x: U32Target,
        y: U32Target,
        borrow: U32Target,
    ) -> (U32Target, U32Target) {
        let (gate_index, copy) = self.find_u32_subtraction_gate();

        self.connect(
            Target::wire(
                gate_index,
                U32SubtractionGate::<F, D>::wire_ith_input_x(copy),
            ),
            x.0,
        );
        self.connect(
            Target::wire(
                gate_index,
                U32SubtractionGate::<F, D>::wire_ith_input_y(copy),
            ),
            y.0,
        );
        self.connect(
            Target::wire(
                gate_index,
                U32SubtractionGate::<F, D>::wire_ith_input_borrow(copy),
            ),
            borrow.0,
        );

        let output_result = U32Target(Target::wire(
            gate_index,
            U32SubtractionGate::<F, D>::wire_ith_output_result(copy),
        ));
        let output_borrow = U32Target(Target::wire(
            gate_index,
            U32SubtractionGate::<F, D>::wire_ith_output_borrow(copy),
        ));

        (output_result, output_borrow)
    }
}
