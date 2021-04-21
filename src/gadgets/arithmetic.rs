use crate::circuit_builder::CircuitBuilder;
use crate::field::field::Field;
use crate::gates::arithmetic::ArithmeticGate;
use crate::target::Target;
use crate::wire::Wire;

impl<F: Field> CircuitBuilder<F> {
    pub fn neg(&mut self, x: Target) -> Target {
        let neg_one = self.neg_one();
        self.mul(x, neg_one)
    }

    pub fn add(&mut self, x: Target, y: Target) -> Target {
        let zero = self.zero();
        let one = self.one();
        if x == zero {
            return y;
        }
        if y == zero {
            return x;
        }

        let gate = self.add_gate(ArithmeticGate::new(), vec![F::ONE, F::ONE]);

        let wire_multiplicand_0 = Wire {
            gate,
            input: ArithmeticGate::WIRE_MULTIPLICAND_0,
        };
        let wire_multiplicand_1 = Wire {
            gate,
            input: ArithmeticGate::WIRE_MULTIPLICAND_1,
        };
        let wire_addend = Wire {
            gate,
            input: ArithmeticGate::WIRE_ADDEND,
        };
        let wire_output = Wire {
            gate,
            input: ArithmeticGate::WIRE_OUTPUT,
        };

        self.route(x, Target::Wire(wire_multiplicand_0));
        self.route(one, Target::Wire(wire_multiplicand_1));
        self.route(y, Target::Wire(wire_addend));
        Target::Wire(wire_output)
    }

    pub fn add_many(&mut self, terms: &[Target]) -> Target {
        let mut sum = self.zero();
        for term in terms {
            sum = self.add(sum, *term);
        }
        sum
    }

    pub fn sub(&mut self, x: Target, y: Target) -> Target {
        let zero = self.zero();
        if x == zero {
            return y;
        }
        if y == zero {
            return x;
        }

        // TODO: Inefficient impl for now.
        let neg_y = self.neg(y);
        self.add(x, neg_y)
    }

    pub fn mul(&mut self, x: Target, y: Target) -> Target {
        // TODO: Check if one operand is 0 or 1.
        todo!()
    }

    pub fn mul_many(&mut self, terms: &[Target]) -> Target {
        let mut product = self.one();
        for term in terms {
            product = self.mul(product, *term);
        }
        product
    }

    pub fn div(&mut self, x: Target, y: Target) -> Target {
        // TODO: Check if one operand is 0 or 1.
        todo!()
    }
}
