use std::marker::PhantomData;

use plonky2_field::extension_field::Extendable;

use crate::gates::gate::Gate;
use crate::gates::util::StridedConstraintConsumer;
use crate::hash::hash_types::RichField;
use crate::iop::ext_target::ExtensionTarget;
use crate::iop::generator::{GeneratedValues, SimpleGenerator, WitnessGenerator};
use crate::iop::target::Target;
use crate::iop::wire::Wire;
use crate::iop::witness::{PartitionWitness, Witness};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::vars::{EvaluationTargets, EvaluationVars, EvaluationVarsBase};

/// A gate to perform a basic mul-add on 32-bit values (we assume they are range-checked beforehand).
#[derive(Copy, Clone, Debug)]
pub struct MulBiguintBoolGate<F: RichField + Extendable<D>, const D: usize> {
    pub num_limbs: usize,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> MulBiguintBoolGate<F, D> {
    pub fn new(num_limbs: usize) -> Self {
        Self {
            num_limbs,
            _phantom: PhantomData,
        }
    }

    pub fn wire_ith_input_limb(&self, i: usize) -> usize {
        debug_assert!(i < self.num_limbs);
        i
    }
    pub fn wire_input_bool(&self) -> usize {
        self.num_limbs
    }
    pub fn wire_ith_output_limb(&self, i: usize) -> usize {
        debug_assert!(i < self.num_limbs);
        self.num_limbs + 1 + i
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Gate<F, D> for MulBiguintBoolGate<F, D> {
    fn id(&self) -> String {
        format!("{:?}", self)
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        let mut constraints = Vec::with_capacity(self.num_constraints());

        let input_bool = vars.local_wires[self.wire_input_bool()];
        for i in 0..self.num_limbs {
            let input_i = vars.local_wires[self.wire_ith_input_limb(i)];
            let output_i = vars.local_wires[self.wire_ith_output_limb(i)];

            constraints.push(input_i * input_bool - output_i);
        }

        constraints
    }

    fn eval_unfiltered_base_one(
        &self,
        vars: EvaluationVarsBase<F>,
        mut yield_constr: StridedConstraintConsumer<F>,
    ) {
        let input_bool = vars.local_wires[self.wire_input_bool()];
        for i in 0..self.num_limbs {
            let input_i = vars.local_wires[self.wire_ith_input_limb(i)];
            let output_i = vars.local_wires[self.wire_ith_output_limb(i)];

            yield_constr.one(input_i * input_bool - output_i);
        }
    }

    fn eval_unfiltered_recursively(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>> {
        let mut constraints = Vec::with_capacity(self.num_constraints());

        let input_bool = vars.local_wires[self.wire_input_bool()];
        for i in 0..self.num_limbs {
            let input_i = vars.local_wires[self.wire_ith_input_limb(i)];
            let output_i = vars.local_wires[self.wire_ith_output_limb(i)];

            constraints.push(builder.mul_sub_extension(input_i, input_bool, output_i));
        }

        constraints
    }

    fn generators(
        &self,
        gate_index: usize,
        _local_constants: &[F],
    ) -> Vec<Box<dyn WitnessGenerator<F>>> {
        let gen = MulBiguintBoolGenerator {
            gate: *self,
            gate_index,
            _phantom: PhantomData,
        };
        vec![Box::new(gen.adapter())]
    }

    fn num_wires(&self) -> usize {
        self.num_limbs * 2 + 1
    }

    fn num_constants(&self) -> usize {
        0
    }

    fn degree(&self) -> usize {
        2
    }

    fn num_constraints(&self) -> usize {
        self.num_limbs
    }
}

#[derive(Clone, Debug)]
struct MulBiguintBoolGenerator<F: RichField + Extendable<D>, const D: usize> {
    gate: MulBiguintBoolGate<F, D>,
    gate_index: usize,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> SimpleGenerator<F>
    for MulBiguintBoolGenerator<F, D>
{
    fn dependencies(&self) -> Vec<Target> {
        let local_target = |input| Target::wire(self.gate_index, input);

        (0..self.gate.num_limbs)
            .map(|i| local_target(self.gate.wire_ith_input_limb(i)))
            .chain([local_target(self.gate.wire_input_bool())])
            .collect()
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let local_wire = |input| Wire {
            gate: self.gate_index,
            input,
        };

        let get_local_wire = |input| witness.get_wire(local_wire(input));

        let input_bool = get_local_wire(self.gate.wire_input_bool());
        for i in 0..self.gate.num_limbs {
            let input_limb = get_local_wire(self.gate.wire_ith_input_limb(i));
            let output_wire = local_wire(self.gate.wire_ith_output_limb(i));
            let output_limb = input_limb * input_bool;
            out_buffer.set_wire(output_wire, output_limb);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use anyhow::Result;
    use plonky2_field::field_types::Field;
    use plonky2_field::goldilocks_field::GoldilocksField;
    use rand::Rng;

    use crate::gates::gate::Gate;
    use crate::gates::gate_testing::{test_eval_fns, test_low_degree};
    use crate::gates::mul_biguint_bool::MulBiguintBoolGate;
    use crate::hash::hash_types::HashOut;
    use crate::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use crate::plonk::vars::EvaluationVars;

    #[test]
    fn low_degree() {
        test_low_degree::<GoldilocksField, _, 4>(MulBiguintBoolGate::<GoldilocksField, 4> {
            num_limbs: 8,
            _phantom: PhantomData,
        })
    }

    #[test]
    fn eval_fns() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        test_eval_fns::<F, C, _, D>(MulBiguintBoolGate::<GoldilocksField, D> {
            num_limbs: 8,
            _phantom: PhantomData,
        })
    }

    #[test]
    fn test_gate_constraint() {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type FF = <C as GenericConfig<D>>::FE;
        const NUM_LIMBS: usize = 8;

        fn get_wires(input_limbs: Vec<F>, input_bool: bool) -> Vec<FF> {
            let output_limbs = input_limbs
                .iter()
                .map(|&l| if input_bool { l } else { F::ZERO });

            input_limbs
                .iter()
                .cloned()
                .chain([F::from_bool(input_bool)])
                .chain(output_limbs)
                .map(|x| x.into())
                .collect()
        }

        let mut rng = rand::thread_rng();
        let input_limbs: Vec<_> = (0..NUM_LIMBS)
            .map(|_| F::from_canonical_u64(rng.gen()))
            .collect();
        let input_bool: bool = rng.gen();

        let gate = MulBiguintBoolGate::<F, D> {
            num_limbs: NUM_LIMBS,
            _phantom: PhantomData,
        };

        let vars = EvaluationVars {
            local_constants: &[],
            local_wires: &get_wires(input_limbs, input_bool),
            public_inputs_hash: &HashOut::rand(),
        };

        assert!(
            gate.eval_unfiltered(vars).iter().all(|x| x.is_zero()),
            "Gate constraints are not satisfied."
        );
    }
}
