use plonky2_field::extension_field::Extendable;

use crate::gates::arithmetic_u32::U32ArithmeticGate;
use crate::gates::subtraction_u32::U32SubtractionGate;
use crate::hash::hash_types::RichField;
use crate::iop::target::Target;
use crate::iop::witness::{PartitionWitness, Witness};
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

    /// Checks for special cases where the value of
    /// `x * y + z`
    /// can be determined without adding a `U32ArithmeticGate`.
    pub fn arithmetic_u32_special_cases(
        &mut self,
        x: U32Target,
        y: U32Target,
        z: U32Target,
    ) -> Option<(U32Target, U32Target)> {
        let x_const = self.target_as_constant(x.0);
        let y_const = self.target_as_constant(y.0);
        let z_const = self.target_as_constant(z.0);

        // If both terms are constant, return their (constant) sum.
        let first_term_const = if let (Some(xx), Some(yy)) = (x_const, y_const) {
            Some(xx * yy)
        } else {
            None
        };

        if let (Some(a), Some(b)) = (first_term_const, z_const) {
            let sum = (a + b).to_canonical_u64();
            let (low, high) = (sum as u32, (sum >> 32) as u32);
            return Some((self.constant_u32(low), self.constant_u32(high)));
        }

        None
    }

    // Returns x * y + z.
    pub fn mul_add_u32(
        &mut self,
        x: U32Target,
        y: U32Target,
        z: U32Target,
    ) -> (U32Target, U32Target) {
        if let Some(result) = self.arithmetic_u32_special_cases(x, y, z) {
            return result;
        }

        let gate = U32ArithmeticGate::<F, D>::new_from_config(&self.config);
        let (gate_index, copy) = self.find_u32_arithmetic_gate();

        self.connect(
            Target::wire(gate_index, gate.wire_ith_multiplicand_0(copy)),
            x.0,
        );
        self.connect(
            Target::wire(gate_index, gate.wire_ith_multiplicand_1(copy)),
            y.0,
        );
        self.connect(Target::wire(gate_index, gate.wire_ith_addend(copy)), z.0);

        let output_low = U32Target(Target::wire(
            gate_index,
            gate.wire_ith_output_low_half(copy),
        ));
        let output_high = U32Target(Target::wire(
            gate_index,
            gate.wire_ith_output_high_half(copy),
        ));

        (output_low, output_high)
    }

    pub fn add_u32(&mut self, a: U32Target, b: U32Target) -> (U32Target, U32Target) {
        let one = self.one_u32();
        self.mul_add_u32(a, one, b)
    }

    pub fn add_many_u32(&mut self, to_add: &[U32Target]) -> (U32Target, U32Target) {
        match to_add.len() {
            0 => (self.zero_u32(), self.zero_u32()),
            1 => (to_add[0], self.zero_u32()),
            2 => self.add_u32(to_add[0], to_add[1]),
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
        let gate = U32SubtractionGate::<F, D>::new_from_config(&self.config);
        let (gate_index, copy) = self.find_u32_subtraction_gate();

        self.connect(Target::wire(gate_index, gate.wire_ith_input_x(copy)), x.0);
        self.connect(Target::wire(gate_index, gate.wire_ith_input_y(copy)), y.0);
        self.connect(
            Target::wire(gate_index, gate.wire_ith_input_borrow(copy)),
            borrow.0,
        );

        let output_result = U32Target(Target::wire(gate_index, gate.wire_ith_output_result(copy)));
        let output_borrow = U32Target(Target::wire(gate_index, gate.wire_ith_output_borrow(copy)));

        (output_result, output_borrow)
    }

    pub fn split_to_u32(&mut self, x: Target) -> (U32Target, U32Target) {
        let low = self.add_virtual_u32_target();
        let high = self.add_virtual_u32_target();

        let base = self.constant(F::from_canonical_u64(1u64 << 32));
        let combined = self.mul_add(high.0, base, low.0);
        self.connect(x, combined);

        self.add_simple_generator(SplitToU32Generator::<F, D> {
            x: x.clone(),
            low: low.clone(),
            high: high.clone(),
            _phantom: PhantomData,
        });

        (low, high)
    }
}

#[derive(Debug)]
struct SplitToU32Generator<F: RichField + Extendable<D>, const D: usize> {
    x: Target,
    low: U32Target,
    high: U32Target,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> SimpleGenerator<F>
    for SplitToU32Generator<F, D>
{
    fn dependencies(&self) -> Vec<Target> {
        vec![self.x]
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let x = witness.get_target(self.x.clone());
        let x_u64 = x.to_canonical_u64();
        let low = x_u64 as u32;
        let high: u32 = (x_u64 >> 32).try_into().unwrap();
        println!("LOW: {}", low);
        println!("HIGH: {}", high);

        out_buffer.set_u32_target(self.low.clone(), low);
        out_buffer.set_u32_target(self.high.clone(), high);
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    
    use rand::{thread_rng, Rng};

    use crate::field::goldilocks_field::GoldilocksField;
    use crate::iop::witness::PartialWitness;
    use crate::plonk::circuit_builder::CircuitBuilder;
    use crate::plonk::circuit_data::CircuitConfig;
    use crate::plonk::verifier::verify;

    #[test]
    pub fn test_add_many_u32s() -> Result<()> {
        type F = GoldilocksField;
        const D: usize = 4;

        let config = CircuitConfig::standard_recursion_config();

        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let mut rng = thread_rng();
        let mut to_add = Vec::new();
        for _ in 0..10 {
            to_add.push(builder.constant_u32(rng.gen()));
        }
        let _ = builder.add_many_u32(&to_add);

        let data = builder.build();
        let proof = data.prove(pw).unwrap();
        verify(proof, &data.verifier_only, &data.common)
    }
}