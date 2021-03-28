use crate::circuit_builder::CircuitBuilder;
use crate::field::field::Field;
use crate::target::Target;

impl<F: Field> CircuitBuilder<F> {
    pub fn add(&mut self, x: Target, y: Target) -> Target {
        todo!()
    }

    pub fn sub(&mut self, x: Target, y: Target) -> Target {
        todo!()
    }

    pub fn mul(&mut self, x: Target, y: Target) -> Target {
        todo!()
    }

    pub fn div(&mut self, x: Target, y: Target) -> Target {
        todo!()
    }
}
