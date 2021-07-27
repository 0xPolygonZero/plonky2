use std::marker::PhantomData;

use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::field::field::Field;
use crate::gates::gate::Gate;
use crate::generator::{GeneratedValues, SimpleGenerator, WitnessGenerator};
use crate::plonk_common::reduce_with_powers;
use crate::target::Target;
use crate::vars::{EvaluationTargets, EvaluationVars};
use crate::wire::Wire;
use crate::witness::PartialWitness;

const MAX_POWER_BITS: usize = 8;

/// A gate for inserting a value into a list at a non-deterministic location.
#[derive(Clone, Debug)]
pub(crate) struct ExponentiationGate<F: Extendable<D>, const D: usize> {
    pub num_power_bits: usize,
    pub _phantom: PhantomData<F>,
}

impl<F: Extendable<D>, const D: usize> ExponentiationGate<F, D> {
    pub fn new(power_bits: usize) -> Self {
        Self {
            num_power_bits: power_bits,
            _phantom: PhantomData,
        }
    }

    pub fn wires_base(&self) -> usize {
        0
    }

    pub fn wires_power(&self) -> usize {
        1
    }

    pub fn wires_power_bit(&self, i: usize) -> usize {
        debug_assert!(i < self.num_power_bits);
        2 + i
    }

    pub fn wires_intermediate_value(&self, i: usize) -> usize {
        debug_assert!(i < self.num_power_bits);
        2 + self.num_power_bits + i
    }
}

impl<F: Extendable<D>, const D: usize> Gate<F, D> for ExponentiationGate<F, D> {
    fn id(&self) -> String {
        format!("{:?}<D={}>", self, D)
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        let base = vars.local_wires[self.wires_base()];
        let power = vars.local_wires[self.wires_power()];
        let computed_output = base.exp(power.to_canonical_u64());

        let power_bits: Vec<_> = (0..self.num_power_bits)
            .map(|i| vars.local_wires[self.wires_power_bit(i)])
            .collect();
        let intermediate_values: Vec<_> = (0..self.num_power_bits)
            .map(|i| vars.local_wires[self.wires_intermediate_value(i)])
            .collect();

        let mut constraints = Vec::new();

        let power_bits_reversed = &power_bits.iter().cloned().rev().collect::<Vec<_>>()[..];
        let computed_power = reduce_with_powers(power_bits_reversed, F::Extension::TWO);
        constraints.push(power - computed_power);

        let mut current_intermediate_value = F::Extension::ONE;
        for i in 0..self.num_power_bits {
            let cur_bit = power_bits[i];
            let not_cur_bit = F::Extension::ONE - cur_bit;
            let computed_intermediate_value =
                current_intermediate_value * (cur_bit * base + not_cur_bit);
            constraints.push(computed_intermediate_value - intermediate_values[i]);
            current_intermediate_value = computed_intermediate_value * computed_intermediate_value;
        }

        constraints.push(computed_output - intermediate_values[self.num_power_bits - 1]);

        constraints
    }

    fn eval_unfiltered_recursively(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>> {
        todo!()
    }

    fn generators(
        &self,
        gate_index: usize,
        _local_constants: &[F],
    ) -> Vec<Box<dyn WitnessGenerator<F>>> {
        let gen = ExponentiationGenerator::<F, D> {
            gate_index,
            gate: self.clone(),
        };
        vec![Box::new(gen)]
    }

    fn num_wires(&self) -> usize {
        self.wires_intermediate_value(self.num_power_bits - 1) + 1
    }

    fn num_constants(&self) -> usize {
        0
    }

    fn degree(&self) -> usize {
        4
    }

    fn num_constraints(&self) -> usize {
        self.num_power_bits + 2
    }
}

#[derive(Debug)]
struct ExponentiationGenerator<F: Extendable<D>, const D: usize> {
    gate_index: usize,
    gate: ExponentiationGate<F, D>,
}

impl<F: Extendable<D>, const D: usize> SimpleGenerator<F> for ExponentiationGenerator<F, D> {
    fn dependencies(&self) -> Vec<Target> {
        let local_target = |input| Target::wire(self.gate_index, input);

        let mut deps = Vec::new();
        deps.push(local_target(self.gate.wires_base()));
        deps.push(local_target(self.gate.wires_power()));
        for i in 0..self.gate.num_power_bits {
            deps.push(local_target(self.gate.wires_power_bit(i)));
        }
        deps
    }

    fn run_once(&self, witness: &PartialWitness<F>) -> GeneratedValues<F> {
        let local_wire = |input| Wire {
            gate: self.gate_index,
            input,
        };

        let get_local_wire = |input| witness.get_wire(local_wire(input));

        let num_power_bits = self.gate.num_power_bits;
        let base = get_local_wire(self.gate.wires_base());
        let power_bits = (0..num_power_bits)
            .map(|i| get_local_wire(self.gate.wires_power_bit(i)))
            .collect::<Vec<_>>();

        let mut intermediate_values = Vec::new();
        let mut current_intermediate_value = F::ONE;
        for i in 0..num_power_bits {
            if power_bits[i] == F::ONE {
                current_intermediate_value *= base;
            }
            intermediate_values.push(current_intermediate_value);
            current_intermediate_value *= current_intermediate_value;
        }

        let mut result = GeneratedValues::<F>::with_capacity(num_power_bits);
        for i in 0..=num_power_bits {
            let intermediate_value_wire = local_wire(self.gate.wires_intermediate_value(i));
            result.set_wire(intermediate_value_wire, intermediate_values[i]);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use rand::{thread_rng, Rng};

    use crate::field::crandall_field::CrandallField;
    use crate::field::extension_field::quartic::QuarticCrandallField;
    use crate::field::field::Field;
    use crate::gates::exponentiation::{ExponentiationGate, MAX_POWER_BITS};
    use crate::gates::gate::Gate;
    use crate::gates::gate_testing::test_low_degree;
    use crate::proof::Hash;
    use crate::util::log2_ceil;
    use crate::vars::EvaluationVars;

    #[test]
    fn wire_indices() {
        let gate = ExponentiationGate::<CrandallField, 4> {
            num_power_bits: 5,
            _phantom: PhantomData,
        };

        assert_eq!(gate.wires_base(), 0);
        assert_eq!(gate.wires_power(), 1);
        assert_eq!(gate.wires_power_bit(0), 2);
        assert_eq!(gate.wires_power_bit(4), 6);
        assert_eq!(gate.wires_intermediate_value(0), 7);
        assert_eq!(gate.wires_intermediate_value(4), 11);
    }

    #[test]
    fn low_degree() {
        test_low_degree::<CrandallField, _, 4>(ExponentiationGate::new(5));
    }

    #[test]
    fn test_gate_constraint() {
        type F = CrandallField;
        type FF = QuarticCrandallField;
        const D: usize = 4;

        /// Returns the local wires for an exponentiation gate given the base, power, and power bit
        /// values.
        fn get_wires(base: F, power: u64) -> Vec<FF> {
            let mut power_bits = Vec::new();
            let mut cur_power = power;
            while cur_power > 0 {
                power_bits.push(cur_power % 2);
                cur_power /= 2;
            }
            power_bits = power_bits.iter().cloned().rev().collect::<Vec<_>>();

            let num_power_bits = power_bits.len();

            let power_F = F::from_canonical_u64(power);
            let power_bits_F: Vec<_> = power_bits
                .iter()
                .map(|b| F::from_canonical_u64(*b))
                .collect();

            let mut v = Vec::new();
            v.push(base);
            v.push(power_F);
            v.extend(power_bits_F.clone());

            let mut intermediate_values = Vec::new();
            let mut current_intermediate_value = F::ONE;
            for i in 0..num_power_bits {
                if power_bits[i] == 1 {
                    current_intermediate_value *= base;
                }
                intermediate_values.push(current_intermediate_value);
                current_intermediate_value *= current_intermediate_value;
            }
            v.extend(intermediate_values);

            v.iter().map(|&x| x.into()).collect::<Vec<_>>()
        }

        let mut rng = rand::thread_rng();

        let base = F::TWO;
        let power = rng.gen::<usize>() % (1 << MAX_POWER_BITS);
        let num_power_bits = log2_ceil(power + 1);
        let gate = ExponentiationGate::<F, D> {
            num_power_bits,
            _phantom: PhantomData,
        };

        let vars = EvaluationVars {
            local_constants: &[],
            local_wires: &get_wires(base, power as u64),
            public_inputs_hash: &Hash::rand(),
        };
        assert!(
            gate.eval_unfiltered(vars).iter().all(|x| x.is_zero()),
            "Gate constraints are not satisfied."
        );
    }
}
