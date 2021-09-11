use std::marker::PhantomData;

use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::field::field_types::{Field, RichField};
use crate::gates::gate::Gate;
use crate::iop::generator::{GeneratedValues, SimpleGenerator, WitnessGenerator};
use crate::iop::target::Target;
use crate::iop::wire::Wire;
use crate::iop::witness::{PartitionWitness, Witness};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::circuit_data::CircuitConfig;
use crate::plonk::vars::{EvaluationTargets, EvaluationVars, EvaluationVarsBase};

/// A gate for raising a value to a power.
#[derive(Clone, Debug)]
pub(crate) struct ExponentiationGate<F: RichField + Extendable<D>, const D: usize> {
    pub num_power_bits: usize,
    pub _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> ExponentiationGate<F, D> {
    pub fn new(num_power_bits: usize) -> Self {
        Self {
            num_power_bits,
            _phantom: PhantomData,
        }
    }

    pub fn new_from_config(config: CircuitConfig) -> Self {
        let num_power_bits = Self::max_power_bits(config.num_wires, config.num_routed_wires);
        Self::new(num_power_bits)
    }

    fn max_power_bits(num_wires: usize, num_routed_wires: usize) -> usize {
        // 2 wires are reserved for the base and output.
        let max_for_routed_wires = num_routed_wires - 2;
        let max_for_wires = (num_wires - 2) / 2;
        max_for_routed_wires.min(max_for_wires)
    }

    pub fn wire_base(&self) -> usize {
        0
    }

    /// The `i`th bit of the exponent, in little-endian order.
    pub fn wire_power_bit(&self, i: usize) -> usize {
        debug_assert!(i < self.num_power_bits);
        1 + i
    }

    pub fn wire_output(&self) -> usize {
        1 + self.num_power_bits
    }

    pub fn wire_intermediate_value(&self, i: usize) -> usize {
        debug_assert!(i < self.num_power_bits);
        2 + self.num_power_bits + i
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Gate<F, D> for ExponentiationGate<F, D> {
    fn id(&self) -> String {
        format!("{:?}<D={}>", self, D)
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        let base = vars.local_wires[self.wire_base()];

        let power_bits: Vec<_> = (0..self.num_power_bits)
            .map(|i| vars.local_wires[self.wire_power_bit(i)])
            .collect();
        let intermediate_values: Vec<_> = (0..self.num_power_bits)
            .map(|i| vars.local_wires[self.wire_intermediate_value(i)])
            .collect();

        let output = vars.local_wires[self.wire_output()];

        let mut constraints = Vec::with_capacity(self.num_constraints());

        for i in 0..self.num_power_bits {
            let prev_intermediate_value = if i == 0 {
                F::Extension::ONE
            } else {
                intermediate_values[i - 1].square()
            };

            // power_bits is in LE order, but we accumulate in BE order.
            let cur_bit = power_bits[self.num_power_bits - i - 1];

            let not_cur_bit = F::Extension::ONE - cur_bit;
            let computed_intermediate_value =
                prev_intermediate_value * (cur_bit * base + not_cur_bit);
            constraints.push(computed_intermediate_value - intermediate_values[i]);
        }

        constraints.push(output - intermediate_values[self.num_power_bits - 1]);

        constraints
    }

    fn eval_unfiltered_base(&self, vars: EvaluationVarsBase<F>) -> Vec<F> {
        let base = vars.local_wires[self.wire_base()];

        let power_bits: Vec<_> = (0..self.num_power_bits)
            .map(|i| vars.local_wires[self.wire_power_bit(i)])
            .collect();
        let intermediate_values: Vec<_> = (0..self.num_power_bits)
            .map(|i| vars.local_wires[self.wire_intermediate_value(i)])
            .collect();

        let output = vars.local_wires[self.wire_output()];

        let mut constraints = Vec::with_capacity(self.num_constraints());

        for i in 0..self.num_power_bits {
            let prev_intermediate_value = if i == 0 {
                F::ONE
            } else {
                intermediate_values[i - 1].square()
            };

            // power_bits is in LE order, but we accumulate in BE order.
            let cur_bit = power_bits[self.num_power_bits - i - 1];

            let not_cur_bit = F::ONE - cur_bit;
            let computed_intermediate_value =
                prev_intermediate_value * (cur_bit * base + not_cur_bit);
            constraints.push(computed_intermediate_value - intermediate_values[i]);
        }

        constraints.push(output - intermediate_values[self.num_power_bits - 1]);

        constraints
    }

    fn eval_unfiltered_recursively(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>> {
        let base = vars.local_wires[self.wire_base()];

        let power_bits: Vec<_> = (0..self.num_power_bits)
            .map(|i| vars.local_wires[self.wire_power_bit(i)])
            .collect();
        let intermediate_values: Vec<_> = (0..self.num_power_bits)
            .map(|i| vars.local_wires[self.wire_intermediate_value(i)])
            .collect();

        let output = vars.local_wires[self.wire_output()];

        let mut constraints = Vec::with_capacity(self.num_constraints());

        let one = builder.one_extension();
        for i in 0..self.num_power_bits {
            let prev_intermediate_value = if i == 0 {
                one
            } else {
                builder.square_extension(intermediate_values[i - 1])
            };

            // power_bits is in LE order, but we accumulate in BE order.
            let cur_bit = power_bits[self.num_power_bits - i - 1];
            let mul_by = builder.select_ext_generalized(cur_bit, base, one);
            let intermediate_value_diff =
                builder.mul_sub_extension(prev_intermediate_value, mul_by, intermediate_values[i]);
            constraints.push(intermediate_value_diff);
        }

        let output_diff =
            builder.sub_extension(output, intermediate_values[self.num_power_bits - 1]);
        constraints.push(output_diff);

        constraints
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
        vec![Box::new(gen.adapter())]
    }

    fn num_wires(&self) -> usize {
        self.wire_intermediate_value(self.num_power_bits - 1) + 1
    }

    fn num_constants(&self) -> usize {
        0
    }

    fn degree(&self) -> usize {
        4
    }

    fn num_constraints(&self) -> usize {
        self.num_power_bits + 1
    }
}

#[derive(Debug)]
struct ExponentiationGenerator<F: RichField + Extendable<D>, const D: usize> {
    gate_index: usize,
    gate: ExponentiationGate<F, D>,
}

impl<F: RichField + Extendable<D>, const D: usize> SimpleGenerator<F>
    for ExponentiationGenerator<F, D>
{
    fn dependencies(&self) -> Vec<Target> {
        let local_target = |input| Target::wire(self.gate_index, input);

        let mut deps = Vec::with_capacity(self.gate.num_power_bits + 1);
        deps.push(local_target(self.gate.wire_base()));
        for i in 0..self.gate.num_power_bits {
            deps.push(local_target(self.gate.wire_power_bit(i)));
        }
        deps
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let local_wire = |input| Wire {
            gate: self.gate_index,
            input,
        };

        let get_local_wire = |input| witness.get_wire(local_wire(input));

        let num_power_bits = self.gate.num_power_bits;
        let base = get_local_wire(self.gate.wire_base());

        let power_bits = (0..num_power_bits)
            .map(|i| get_local_wire(self.gate.wire_power_bit(i)))
            .collect::<Vec<_>>();
        let mut intermediate_values = Vec::new();

        let mut current_intermediate_value = F::ONE;
        for i in 0..num_power_bits {
            if power_bits[num_power_bits - i - 1] == F::ONE {
                current_intermediate_value *= base;
            }
            intermediate_values.push(current_intermediate_value);
            current_intermediate_value *= current_intermediate_value;
        }

        for i in 0..num_power_bits {
            let intermediate_value_wire = local_wire(self.gate.wire_intermediate_value(i));
            out_buffer.set_wire(intermediate_value_wire, intermediate_values[i]);
        }

        let output_wire = local_wire(self.gate.wire_output());
        out_buffer.set_wire(output_wire, intermediate_values[num_power_bits - 1]);
    }
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use anyhow::Result;
    use rand::Rng;

    use crate::field::crandall_field::CrandallField;
    use crate::field::extension_field::quartic::QuarticExtension;
    use crate::field::field_types::Field;
    use crate::gates::exponentiation::ExponentiationGate;
    use crate::gates::gate::Gate;
    use crate::gates::gate_testing::{test_eval_fns, test_low_degree};
    use crate::hash::hash_types::HashOut;
    use crate::plonk::circuit_data::CircuitConfig;
    use crate::plonk::vars::EvaluationVars;
    use crate::util::log2_ceil;

    const MAX_POWER_BITS: usize = 17;

    #[test]
    fn wire_indices() {
        let gate = ExponentiationGate::<CrandallField, 4> {
            num_power_bits: 5,
            _phantom: PhantomData,
        };

        assert_eq!(gate.wire_base(), 0);
        assert_eq!(gate.wire_power_bit(0), 1);
        assert_eq!(gate.wire_power_bit(4), 5);
        assert_eq!(gate.wire_output(), 6);
        assert_eq!(gate.wire_intermediate_value(0), 7);
        assert_eq!(gate.wire_intermediate_value(4), 11);
    }

    #[test]
    fn low_degree() {
        let config = CircuitConfig {
            num_wires: 120,
            num_routed_wires: 30,
            ..CircuitConfig::large_config()
        };

        test_low_degree::<CrandallField, _, 4>(ExponentiationGate::new_from_config(config));
    }

    #[test]
    fn eval_fns() -> Result<()> {
        test_eval_fns::<CrandallField, _, 4>(ExponentiationGate::new_from_config(
            CircuitConfig::large_config(),
        ))
    }

    #[test]
    fn test_gate_constraint() {
        type F = CrandallField;
        type FF = QuarticExtension<CrandallField>;
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

            let num_power_bits = power_bits.len();

            let power_bits_f: Vec<_> = power_bits
                .iter()
                .map(|b| F::from_canonical_u64(*b))
                .collect();

            let mut v = Vec::new();
            v.push(base);
            v.extend(power_bits_f.clone());

            let mut intermediate_values = Vec::new();
            let mut current_intermediate_value = F::ONE;
            for i in 0..num_power_bits {
                if power_bits[num_power_bits - i - 1] == 1 {
                    current_intermediate_value *= base;
                }
                intermediate_values.push(current_intermediate_value);
                current_intermediate_value *= current_intermediate_value;
            }
            let output_value = intermediate_values[num_power_bits - 1];
            v.push(output_value);
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
            public_inputs_hash: &HashOut::rand(),
        };
        assert!(
            gate.eval_unfiltered(vars).iter().all(|x| x.is_zero()),
            "Gate constraints are not satisfied."
        );
    }
}
