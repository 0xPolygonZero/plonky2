use std::marker::PhantomData;

use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::field::field_types::{Field, RichField};
use crate::gates::gate::Gate;
use crate::hash::gmimc;
use crate::hash::gmimc::GMiMC;
use crate::iop::generator::{GeneratedValues, SimpleGenerator, WitnessGenerator};
use crate::iop::target::Target;
use crate::iop::wire::Wire;
use crate::iop::witness::{PartitionWitness, Witness};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::vars::{EvaluationTargets, EvaluationVars, EvaluationVarsBase};

/// Evaluates a full GMiMC permutation with 12 state elements.
///
/// This also has some extra features to make it suitable for efficiently verifying Merkle proofs.
/// It has a flag which can be used to swap the first four inputs with the next four, for ordering
/// sibling digests.
#[derive(Debug)]
pub struct GMiMCGate<
    F: RichField + Extendable<D> + GMiMC<WIDTH>,
    const D: usize,
    const WIDTH: usize,
> {
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D> + GMiMC<WIDTH>, const D: usize, const WIDTH: usize>
    GMiMCGate<F, D, WIDTH>
{
    pub fn new() -> Self {
        GMiMCGate {
            _phantom: PhantomData,
        }
    }

    /// The wire index for the `i`th input to the permutation.
    pub fn wire_input(i: usize) -> usize {
        i
    }

    /// The wire index for the `i`th output to the permutation.
    pub fn wire_output(i: usize) -> usize {
        WIDTH + i
    }

    /// If this is set to 1, the first four inputs will be swapped with the next four inputs. This
    /// is useful for ordering hashes in Merkle proofs. Otherwise, this should be set to 0.
    pub const WIRE_SWAP: usize = 2 * WIDTH;

    /// A wire which stores the input to the `i`th cubing.
    fn wire_cubing_input(i: usize) -> usize {
        2 * WIDTH + 1 + i
    }

    /// End of wire indices, exclusive.
    fn end() -> usize {
        2 * WIDTH + 1 + gmimc::NUM_ROUNDS
    }
}

impl<F: RichField + Extendable<D> + GMiMC<WIDTH>, const D: usize, const WIDTH: usize> Gate<F, D>
    for GMiMCGate<F, D, WIDTH>
{
    fn id(&self) -> String {
        format!("<WIDTH={}> {:?}", WIDTH, self)
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        let mut constraints = Vec::with_capacity(self.num_constraints());

        // Assert that `swap` is binary.
        let swap = vars.local_wires[Self::WIRE_SWAP];
        constraints.push(swap * (swap - F::Extension::ONE));

        let mut state = Vec::with_capacity(12);
        for i in 0..4 {
            let a = vars.local_wires[i];
            let b = vars.local_wires[i + 4];
            state.push(a + swap * (b - a));
        }
        for i in 0..4 {
            let a = vars.local_wires[i + 4];
            let b = vars.local_wires[i];
            state.push(a + swap * (b - a));
        }
        for i in 8..12 {
            state.push(vars.local_wires[i]);
        }

        // Value that is implicitly added to each element.
        // See https://affine.group/2020/02/starkware-challenge
        let mut addition_buffer = F::Extension::ZERO;

        for r in 0..gmimc::NUM_ROUNDS {
            let active = r % WIDTH;
            let constant = F::from_canonical_u64(<F as GMiMC<WIDTH>>::ROUND_CONSTANTS[r]);
            let cubing_input = state[active] + addition_buffer + constant.into();
            let cubing_input_wire = vars.local_wires[Self::wire_cubing_input(r)];
            constraints.push(cubing_input - cubing_input_wire);
            let f = cubing_input_wire.cube();
            addition_buffer += f;
            state[active] -= f;
        }

        for i in 0..WIDTH {
            state[i] += addition_buffer;
            constraints.push(state[i] - vars.local_wires[Self::wire_output(i)]);
        }

        constraints
    }

    fn eval_unfiltered_base(&self, vars: EvaluationVarsBase<F>) -> Vec<F> {
        let mut constraints = Vec::with_capacity(self.num_constraints());

        // Assert that `swap` is binary.
        let swap = vars.local_wires[Self::WIRE_SWAP];
        constraints.push(swap * (swap - F::ONE));

        let mut state = Vec::with_capacity(12);
        for i in 0..4 {
            let a = vars.local_wires[i];
            let b = vars.local_wires[i + 4];
            state.push(a + swap * (b - a));
        }
        for i in 0..4 {
            let a = vars.local_wires[i + 4];
            let b = vars.local_wires[i];
            state.push(a + swap * (b - a));
        }
        for i in 8..12 {
            state.push(vars.local_wires[i]);
        }

        // Value that is implicitly added to each element.
        // See https://affine.group/2020/02/starkware-challenge
        let mut addition_buffer = F::ZERO;

        for r in 0..gmimc::NUM_ROUNDS {
            let active = r % WIDTH;
            let constant = F::from_canonical_u64(<F as GMiMC<WIDTH>>::ROUND_CONSTANTS[r]);
            let cubing_input = state[active] + addition_buffer + constant;
            let cubing_input_wire = vars.local_wires[Self::wire_cubing_input(r)];
            constraints.push(cubing_input - cubing_input_wire);
            let f = cubing_input_wire.cube();
            addition_buffer += f;
            state[active] -= f;
        }

        for i in 0..WIDTH {
            state[i] += addition_buffer;
            constraints.push(state[i] - vars.local_wires[Self::wire_output(i)]);
        }

        constraints
    }

    fn eval_unfiltered_recursively(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>> {
        let mut constraints = Vec::with_capacity(self.num_constraints());

        let swap = vars.local_wires[Self::WIRE_SWAP];
        constraints.push(builder.mul_sub_extension(swap, swap, swap));

        let mut state = Vec::with_capacity(12);
        for i in 0..4 {
            let a = vars.local_wires[i];
            let b = vars.local_wires[i + 4];
            let delta = builder.sub_extension(b, a);
            state.push(builder.mul_add_extension(swap, delta, a));
        }
        for i in 0..4 {
            let a = vars.local_wires[i + 4];
            let b = vars.local_wires[i];
            let delta = builder.sub_extension(b, a);
            state.push(builder.mul_add_extension(swap, delta, a));
        }
        for i in 8..12 {
            state.push(vars.local_wires[i]);
        }

        // Value that is implicitly added to each element.
        // See https://affine.group/2020/02/starkware-challenge
        let mut addition_buffer = builder.zero_extension();

        for r in 0..gmimc::NUM_ROUNDS {
            let active = r % WIDTH;

            let constant = F::from_canonical_u64(<F as GMiMC<WIDTH>>::ROUND_CONSTANTS[r]);
            let constant = builder.constant_extension(constant.into());
            let cubing_input =
                builder.add_many_extension(&[state[active], addition_buffer, constant]);
            let cubing_input_wire = vars.local_wires[Self::wire_cubing_input(r)];
            constraints.push(builder.sub_extension(cubing_input, cubing_input_wire));
            let f = builder.cube_extension(cubing_input_wire);
            addition_buffer = builder.add_extension(addition_buffer, f);
            state[active] = builder.sub_extension(state[active], f);
        }

        for i in 0..WIDTH {
            state[i] = builder.add_extension(state[i], addition_buffer);
            constraints
                .push(builder.sub_extension(state[i], vars.local_wires[Self::wire_output(i)]));
        }

        constraints
    }

    fn generators(
        &self,
        gate_index: usize,
        _local_constants: &[F],
    ) -> Vec<Box<dyn WitnessGenerator<F>>> {
        let gen = GMiMCGenerator::<F, D, WIDTH> {
            gate_index,
            _phantom: PhantomData,
        };
        vec![Box::new(gen.adapter())]
    }

    fn num_wires(&self) -> usize {
        Self::end()
    }

    fn num_constants(&self) -> usize {
        0
    }

    fn degree(&self) -> usize {
        3
    }

    fn num_constraints(&self) -> usize {
        gmimc::NUM_ROUNDS + WIDTH + 1
    }
}

#[derive(Debug)]
struct GMiMCGenerator<
    F: RichField + Extendable<D> + GMiMC<WIDTH>,
    const D: usize,
    const WIDTH: usize,
> {
    gate_index: usize,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D> + GMiMC<WIDTH>, const D: usize, const WIDTH: usize>
    SimpleGenerator<F> for GMiMCGenerator<F, D, WIDTH>
{
    fn dependencies(&self) -> Vec<Target> {
        let mut dep_input_indices = Vec::with_capacity(WIDTH + 1);
        for i in 0..WIDTH {
            dep_input_indices.push(GMiMCGate::<F, D, WIDTH>::wire_input(i));
        }
        dep_input_indices.push(GMiMCGate::<F, D, WIDTH>::WIRE_SWAP);

        dep_input_indices
            .into_iter()
            .map(|input| {
                Target::Wire(Wire {
                    gate: self.gate_index,
                    input,
                })
            })
            .collect()
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let mut state = (0..WIDTH)
            .map(|i| {
                witness.get_wire(Wire {
                    gate: self.gate_index,
                    input: GMiMCGate::<F, D, WIDTH>::wire_input(i),
                })
            })
            .collect::<Vec<_>>();

        let swap_value = witness.get_wire(Wire {
            gate: self.gate_index,
            input: GMiMCGate::<F, D, WIDTH>::WIRE_SWAP,
        });
        debug_assert!(swap_value == F::ZERO || swap_value == F::ONE);
        if swap_value == F::ONE {
            for i in 0..4 {
                state.swap(i, 4 + i);
            }
        }

        // Value that is implicitly added to each element.
        // See https://affine.group/2020/02/starkware-challenge
        let mut addition_buffer = F::ZERO;

        for r in 0..gmimc::NUM_ROUNDS {
            let active = r % WIDTH;
            let constant = F::from_canonical_u64(<F as GMiMC<WIDTH>>::ROUND_CONSTANTS[r]);
            let cubing_input = state[active] + addition_buffer + constant;
            out_buffer.set_wire(
                Wire {
                    gate: self.gate_index,
                    input: GMiMCGate::<F, D, WIDTH>::wire_cubing_input(r),
                },
                cubing_input,
            );
            let f = cubing_input.cube();
            addition_buffer += f;
            state[active] -= f;
        }

        for i in 0..WIDTH {
            state[i] += addition_buffer;
            out_buffer.set_wire(
                Wire {
                    gate: self.gate_index,
                    input: GMiMCGate::<F, D, WIDTH>::wire_output(i),
                },
                state[i],
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::convert::TryInto;

    use anyhow::Result;

    use crate::field::crandall_field::CrandallField;
    use crate::field::field_types::Field;
    use crate::gates::gate_testing::{test_eval_fns, test_low_degree};
    use crate::gates::gmimc::GMiMCGate;
    use crate::hash::gmimc::GMiMC;
    use crate::iop::generator::generate_partial_witness;
    use crate::iop::wire::Wire;
    use crate::iop::witness::{PartialWitness, Witness};
    use crate::plonk::circuit_builder::CircuitBuilder;
    use crate::plonk::circuit_data::CircuitConfig;

    #[test]
    fn generated_output() {
        type F = CrandallField;
        const WIDTH: usize = 12;

        let config = CircuitConfig::large_config();
        let mut builder = CircuitBuilder::new(config);
        type Gate = GMiMCGate<F, 4, WIDTH>;
        let gate = Gate::new();
        let gate_index = builder.add_gate(gate, vec![]);
        let circuit = builder.build_prover();

        let permutation_inputs = (0..WIDTH).map(F::from_canonical_usize).collect::<Vec<_>>();

        let mut inputs = PartialWitness::new();
        inputs.set_wire(
            Wire {
                gate: gate_index,
                input: Gate::WIRE_SWAP,
            },
            F::ZERO,
        );
        for i in 0..WIDTH {
            inputs.set_wire(
                Wire {
                    gate: gate_index,
                    input: Gate::wire_input(i),
                },
                permutation_inputs[i],
            );
        }

        let witness = generate_partial_witness(inputs, &circuit.prover_only);

        let expected_outputs: [F; WIDTH] =
            F::gmimc_permute_naive(permutation_inputs.try_into().unwrap());
        for i in 0..WIDTH {
            let out = witness.get_wire(Wire {
                gate: 0,
                input: Gate::wire_output(i),
            });
            assert_eq!(out, expected_outputs[i]);
        }
    }

    #[test]
    fn low_degree() {
        type F = CrandallField;
        const WIDTH: usize = 12;
        let gate = GMiMCGate::<F, 4, WIDTH>::new();
        test_low_degree(gate)
    }

    #[test]
    fn eval_fns() -> Result<()> {
        type F = CrandallField;
        const WIDTH: usize = 12;
        let gate = GMiMCGate::<F, 4, WIDTH>::new();
        test_eval_fns(gate)
    }
}
