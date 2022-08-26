use plonky2::iop::generator::GeneratedValues;
use plonky2::iop::witness::Witness;
use plonky2_field::types::{Field, PrimeField64};

use crate::gadgets::arithmetic_u32::U32Target;

pub trait WitnessU32<F: PrimeField64>: Witness<F> {
    fn set_u32_target(&mut self, target: U32Target, value: u32);
    fn get_u32_target(&self, target: U32Target) -> (u32, u32);
}

impl<T: Witness<F>, F: PrimeField64> WitnessU32<F> for T {
    fn set_u32_target(&mut self, target: U32Target, value: u32) {
        self.set_target(target.0, F::from_canonical_u32(value));
    }

    fn get_u32_target(&self, target: U32Target) -> (u32, u32) {
        let x_u64 = self.get_target(target.0).to_canonical_u64();
        let low = x_u64 as u32;
        let high = (x_u64 >> 32) as u32;
        (low, high)
    }
}

pub trait GeneratedValuesU32<F: Field> {
    fn set_u32_target(&mut self, target: U32Target, value: u32);
}

impl<F: Field> GeneratedValuesU32<F> for GeneratedValues<F> {
    fn set_u32_target(&mut self, target: U32Target, value: u32) {
        self.set_target(target.0, F::from_canonical_u32(value))
    }
}
