use std::marker::PhantomData;

use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::field::field_types::RichField;
use crate::gates::gate::Gate;
use crate::iop::generator::{GeneratedValues, SimpleGenerator, WitnessGenerator};
use crate::iop::target::Target;
use crate::iop::wire::Wire;
use crate::iop::witness::{PartitionWitness, Witness};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::vars::{EvaluationTargets, EvaluationVars, EvaluationVarsBase};

/// Number of arithmetic operations performed by an arithmetic gate.
pub const NUM_U32_ARITHMETIC_OPS: usize = 12;

/// A gate to perform a basic mul-add on 32-bit values (we assume they are range-checked beforehand).
#[derive(Debug)]
pub struct U32ArithmeticGate;

impl U32ArithmeticGate {
    pub fn wire_ith_multiplicand_0(i: usize) -> usize {
        5 * i
    }
    pub fn wire_ith_multiplicand_1(i: usize) -> usize {
        5 * i + 1
    }
    pub fn wire_ith_addend(i: usize) -> usize {
        5 * i + 2
    }
    pub fn wire_ith_output_small_limb(i: usize) -> usize {
        5 * i + 3
    }
    pub fn wire_ith_output_large_limb(i: usize) -> usize {
        5 * i + 4
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Gate<F, D> for U32ArithmeticGate {
    fn id(&self) -> String {
        format!("{:?}", self)
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        let mut constraints = Vec::new();
        for i in 0..NUM_U32_ARITHMETIC_OPS {
            let multiplicand_0 = vars.local_wires[Self::wire_ith_multiplicand_0(i)];
            let multiplicand_1 = vars.local_wires[Self::wire_ith_multiplicand_1(i)];
            let addend = vars.local_wires[Self::wire_ith_addend(i)];

            let computed_output = multiplicand_0 * multiplicand_1 + addend;

            let output_small = vars.local_wires[Self::wire_ith_output_small_limb(i)];
            let output_large = vars.local_wires[Self::wire_ith_output_large_limb(i)];

            let base: F::Extension = F::from_canonical_u64(1 << 32u64).into();
            let combined_output = output_large * base + output_small;

            constraints.push(combined_output - computed_output);
        }

        constraints
    }

    fn eval_unfiltered_base(&self, vars: EvaluationVarsBase<F>) -> Vec<F> {
        let mut constraints = Vec::new();
        for i in 0..NUM_U32_ARITHMETIC_OPS {
            let multiplicand_0 = vars.local_wires[Self::wire_ith_multiplicand_0(i)];
            let multiplicand_1 = vars.local_wires[Self::wire_ith_multiplicand_1(i)];
            let addend = vars.local_wires[Self::wire_ith_addend(i)];

            let computed_output = multiplicand_0 * multiplicand_1 + addend;

            let output_small = vars.local_wires[Self::wire_ith_output_small_limb(i)];
            let output_large = vars.local_wires[Self::wire_ith_output_large_limb(i)];

            let base = F::from_canonical_u64(1 << 32u64);
            let combined_output = output_large * base + output_small;

            constraints.push(combined_output - computed_output);
        }

        constraints
    }

    fn eval_unfiltered_recursively(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>> {
        let mut constraints = Vec::new();

        for i in 0..NUM_U32_ARITHMETIC_OPS {
            let multiplicand_0 = vars.local_wires[Self::wire_ith_multiplicand_0(i)];
            let multiplicand_1 = vars.local_wires[Self::wire_ith_multiplicand_1(i)];
            let addend = vars.local_wires[Self::wire_ith_addend(i)];

            let computed_output = builder.mul_add_extension(multiplicand_0, multiplicand_1, addend);

            let output_small = vars.local_wires[Self::wire_ith_output_small_limb(i)];
            let output_large = vars.local_wires[Self::wire_ith_output_large_limb(i)];

            let base: F::Extension = F::from_canonical_u64(1 << 32u64).into();
            let base_target = builder.constant_extension(base);
            let combined_output =
                builder.mul_add_extension(output_large, base_target, output_small);

            constraints.push(builder.sub_extension(combined_output, computed_output));
        }

        constraints
    }

    fn generators(
        &self,
        gate_index: usize,
        local_constants: &[F],
    ) -> Vec<Box<dyn WitnessGenerator<F>>> {
        (0..NUM_U32_ARITHMETIC_OPS)
            .map(|i| {
                let g: Box<dyn WitnessGenerator<F>> = Box::new(
                    U32ArithmeticGenerator {
                        gate_index,
                        i,
                        _phantom: PhantomData,
                    }
                    .adapter(),
                );
                g
            })
            .collect::<Vec<_>>()
    }

    fn num_wires(&self) -> usize {
        NUM_U32_ARITHMETIC_OPS * 5
    }

    fn num_constants(&self) -> usize {
        0
    }

    fn degree(&self) -> usize {
        2
    }

    fn num_constraints(&self) -> usize {
        NUM_U32_ARITHMETIC_OPS
    }
}

#[derive(Clone, Debug)]
struct U32ArithmeticGenerator<F: RichField + Extendable<D>, const D: usize> {
    gate_index: usize,
    i: usize,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> SimpleGenerator<F>
    for U32ArithmeticGenerator<F, D>
{
    fn dependencies(&self) -> Vec<Target> {
        let local_target = |input| Target::wire(self.gate_index, input);

        let mut deps = Vec::new();
        deps.push(local_target(U32ArithmeticGate::wire_ith_multiplicand_0(
            self.i,
        )));
        deps.push(local_target(U32ArithmeticGate::wire_ith_multiplicand_1(
            self.i,
        )));
        deps.push(local_target(U32ArithmeticGate::wire_ith_addend(self.i)));
        deps
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let local_wire = |input| Wire {
            gate: self.gate_index,
            input,
        };

        let get_local_wire = |input| witness.get_wire(local_wire(input));

        let multiplicand_0 = get_local_wire(U32ArithmeticGate::wire_ith_multiplicand_0(self.i));
        let multiplicand_1 = get_local_wire(U32ArithmeticGate::wire_ith_multiplicand_1(self.i));
        let addend = get_local_wire(U32ArithmeticGate::wire_ith_addend(self.i));

        let output = multiplicand_0 * multiplicand_1 + addend;
        let output_u64 = output.to_canonical_u64();

        let output_large_u64 = output_u64 >> 32;
        let output_small_u64 = output_u64 & (1 << 32 - 1);

        let output_large = F::from_canonical_u64(output_large_u64);
        let output_small = F::from_canonical_u64(output_small_u64);

        let output_large_wire = local_wire(U32ArithmeticGate::wire_ith_output_large_limb(self.i));
        let output_small_wire = local_wire(U32ArithmeticGate::wire_ith_output_small_limb(self.i));

        out_buffer.set_wire(output_large_wire, output_large);
        out_buffer.set_wire(output_small_wire, output_small);
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use crate::field::crandall_field::CrandallField;
    use crate::gates::arithmetic_u32::U32ArithmeticGate;
    use crate::gates::gate_testing::{test_eval_fns, test_low_degree};

    #[test]
    fn low_degree() {
        test_low_degree::<CrandallField, _, 4>(U32ArithmeticGate)
    }
    #[test]
    fn eval_fns() -> Result<()> {
        test_eval_fns::<CrandallField, _, 4>(U32ArithmeticGate)
    }
}
