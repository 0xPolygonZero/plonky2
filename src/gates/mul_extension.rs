use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::{Extendable, FieldExtension, OEF};
use crate::field::field::Field;
use crate::gates::gate::{Gate, GateRef};
use crate::generator::{SimpleGenerator, WitnessGenerator};
use crate::target::Target;
use crate::vars::{EvaluationTargets, EvaluationVars};
use crate::wire::Wire;
use crate::witness::PartialWitness;
use std::convert::TryInto;
use std::ops::Range;

// TODO: Replace this when https://github.com/mir-protocol/plonky2/issues/56 is resolved.
fn mul_vec<F: Field>(a: &[F], b: &[F], w: F) -> Vec<F> {
    let (a0, a1, a2, a3) = (a[0], a[1], a[2], a[3]);
    let (b0, b1, b2, b3) = (b[0], b[1], b[2], b[3]);

    let c0 = a0 * b0 + w * (a1 * b3 + a2 * b2 + a3 * b1);
    let c1 = a0 * b1 + a1 * b0 + w * (a2 * b3 + a3 * b2);
    let c2 = a0 * b2 + a1 * b1 + a2 * b0 + w * a3 * b3;
    let c3 = a0 * b3 + a1 * b2 + a2 * b1 + a3 * b0;

    vec![c0, c1, c2, c3]
}
impl<F: Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    fn mul_vec(
        &mut self,
        a: &[ExtensionTarget<D>],
        b: &[ExtensionTarget<D>],
        w: ExtensionTarget<D>,
    ) -> Vec<ExtensionTarget<D>> {
        let (a0, a1, a2, a3) = (a[0], a[1], a[2], a[3]);
        let (b0, b1, b2, b3) = (b[0], b[1], b[2], b[3]);

        // TODO: Optimize this.
        let c0 = {
            let tmp0 = self.mul_extension(a0, b0);
            let tmp1 = self.mul_extension(a1, b3);
            let tmp2 = self.mul_extension(a2, b2);
            let tmp3 = self.mul_extension(a3, b1);
            let tmp = self.add_extension(tmp1, tmp2);
            let tmp = self.add_extension(tmp, tmp3);
            let tmp = self.mul_extension(w, tmp);
            let tmp = self.add_extension(tmp0, tmp);
            tmp
        };
        let c1 = {
            let tmp0 = self.mul_extension(a0, b1);
            let tmp1 = self.mul_extension(a1, b0);
            let tmp2 = self.mul_extension(a2, b3);
            let tmp3 = self.mul_extension(a3, b2);
            let tmp = self.add_extension(tmp2, tmp3);
            let tmp = self.mul_extension(w, tmp);
            let tmp = self.add_extension(tmp, tmp0);
            let tmp = self.add_extension(tmp, tmp1);
            tmp
        };
        let c2 = {
            let tmp0 = self.mul_extension(a0, b2);
            let tmp1 = self.mul_extension(a1, b1);
            let tmp2 = self.mul_extension(a2, b0);
            let tmp3 = self.mul_extension(a3, b3);
            let tmp = self.mul_extension(w, tmp3);
            let tmp = self.add_extension(tmp, tmp2);
            let tmp = self.add_extension(tmp, tmp1);
            let tmp = self.add_extension(tmp, tmp0);
            tmp
        };
        let c3 = {
            let tmp0 = self.mul_extension(a0, b3);
            let tmp1 = self.mul_extension(a1, b2);
            let tmp2 = self.mul_extension(a2, b1);
            let tmp3 = self.mul_extension(a3, b0);
            let tmp = self.add_extension(tmp3, tmp2);
            let tmp = self.add_extension(tmp, tmp1);
            let tmp = self.add_extension(tmp, tmp0);
            tmp
        };

        vec![c0, c1, c2, c3]
    }
}

/// A gate which can multiply two field extension elements.
/// TODO: Add an addend if `NUM_ROUTED_WIRES` is large enough.
#[derive(Debug)]
pub struct MulExtensionGate<const D: usize>;

impl<const D: usize> MulExtensionGate<D> {
    pub fn new<F: Extendable<D>>() -> GateRef<F, D> {
        GateRef::new(MulExtensionGate)
    }

    pub fn wires_multiplicand_0() -> Range<usize> {
        0..D
    }
    pub fn wires_multiplicand_1() -> Range<usize> {
        D..2 * D
    }
    pub fn wires_output() -> Range<usize> {
        2 * D..3 * D
    }
}

impl<F: Extendable<D>, const D: usize> Gate<F, D> for MulExtensionGate<D> {
    fn id(&self) -> String {
        format!("{:?}", self)
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        let const_0 = vars.local_constants[0];
        let multiplicand_0 = vars.local_wires[Self::wires_multiplicand_0()].to_vec();
        let multiplicand_1 = vars.local_wires[Self::wires_multiplicand_1()].to_vec();
        let output = vars.local_wires[Self::wires_output()].to_vec();
        let computed_output = mul_vec(
            &[
                const_0,
                F::Extension::ZERO,
                F::Extension::ZERO,
                F::Extension::ZERO,
            ],
            &multiplicand_0,
            F::Extension::W.into(),
        );
        let computed_output = mul_vec(&computed_output, &multiplicand_1, F::Extension::W.into());
        output
            .into_iter()
            .zip(computed_output)
            .map(|(o, co)| o - co)
            .collect()
    }

    fn eval_unfiltered_recursively(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>> {
        let const_0 = vars.local_constants[0];
        let multiplicand_0 = vars.local_wires[Self::wires_multiplicand_0()].to_vec();
        let multiplicand_1 = vars.local_wires[Self::wires_multiplicand_1()].to_vec();
        let output = vars.local_wires[Self::wires_output()].to_vec();
        let w = builder.constant_extension(F::Extension::W.into());
        let zero = builder.zero_extension();
        let computed_output = builder.mul_vec(&[const_0, zero, zero, zero], &multiplicand_0, w);
        let computed_output = builder.mul_vec(&computed_output, &multiplicand_1, w);
        output
            .into_iter()
            .zip(computed_output)
            .map(|(o, co)| builder.sub_extension(o, co))
            .collect()
    }

    fn generators(
        &self,
        gate_index: usize,
        local_constants: &[F],
    ) -> Vec<Box<dyn WitnessGenerator<F>>> {
        let gen = MulExtensionGenerator {
            gate_index,
            const_0: local_constants[0],
        };
        vec![Box::new(gen)]
    }

    fn num_wires(&self) -> usize {
        12
    }

    fn num_constants(&self) -> usize {
        1
    }

    fn degree(&self) -> usize {
        3
    }

    fn num_constraints(&self) -> usize {
        D
    }
}

struct MulExtensionGenerator<F: Extendable<D>, const D: usize> {
    gate_index: usize,
    const_0: F,
}

impl<F: Extendable<D>, const D: usize> SimpleGenerator<F> for MulExtensionGenerator<F, D> {
    fn dependencies(&self) -> Vec<Target> {
        MulExtensionGate::<D>::wires_multiplicand_0()
            .chain(MulExtensionGate::<D>::wires_multiplicand_1())
            .map(|i| {
                Target::Wire(Wire {
                    gate: self.gate_index,
                    input: i,
                })
            })
            .collect()
    }

    fn run_once(&self, witness: &PartialWitness<F>) -> PartialWitness<F> {
        let multiplicand_0 = MulExtensionGate::<D>::wires_multiplicand_0()
            .map(|i| {
                witness.get_wire(Wire {
                    gate: self.gate_index,
                    input: i,
                })
            })
            .collect::<Vec<_>>();
        let multiplicand_0 = F::Extension::from_basefield_array(multiplicand_0.try_into().unwrap());
        let multiplicand_1 = MulExtensionGate::<D>::wires_multiplicand_1()
            .map(|i| {
                witness.get_wire(Wire {
                    gate: self.gate_index,
                    input: i,
                })
            })
            .collect::<Vec<_>>();
        let multiplicand_1 = F::Extension::from_basefield_array(multiplicand_1.try_into().unwrap());
        let output = MulExtensionGate::<D>::wires_output()
            .map(|i| Wire {
                gate: self.gate_index,
                input: i,
            })
            .collect::<Vec<_>>();

        let computed_output =
            F::Extension::from_basefield(self.const_0) * multiplicand_0 * multiplicand_1;

        let mut pw = PartialWitness::new();
        pw.set_ext_wires(output, computed_output);
        pw
    }
}

#[cfg(test)]
mod tests {
    use crate::field::crandall_field::CrandallField;
    use crate::gates::arithmetic::ArithmeticGate;
    use crate::gates::gate_testing::test_low_degree;
    use crate::gates::mul_extension::MulExtensionGate;

    #[test]
    fn low_degree() {
        test_low_degree(MulExtensionGate::<4>::new::<CrandallField>())
    }
}
