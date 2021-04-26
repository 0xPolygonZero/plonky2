use std::sync::Arc;

use crate::circuit_builder::CircuitBuilder;
use crate::field::field::Field;
use crate::gates::gate::{Gate, GateRef};
use crate::generator::{SimpleGenerator, WitnessGenerator};
use crate::gmimc::gmimc_automatic_constants;
use crate::target::Target;
use crate::vars::{EvaluationTargets, EvaluationVars};
use crate::wire::Wire;
use crate::witness::PartialWitness;

/// The width of the permutation, in field elements.
const W: usize = 12;

/// Evaluates a full GMiMC permutation with 12 state elements, and writes the output to the next
/// gate's first `width` wires (which could be the input of another `GMiMCGate`).
///
/// This also has some extra features to make it suitable for efficiently verifying Merkle proofs.
/// It has a flag which can be used to swap the first four inputs with the next four, for ordering
/// sibling digests. It also has an accumulator that computes the weighted sum of these flags, for
/// computing the index of the leaf based on these swap bits.
#[derive(Debug)]
pub struct GMiMCGate<F: Field, const R: usize> {
    constants: Arc<[F; R]>,
}

impl<F: Field, const R: usize> GMiMCGate<F, R> {
    pub fn with_constants(constants: Arc<[F; R]>) -> GateRef<F> {
        let gate = GMiMCGate::<F, R> { constants };
        GateRef::new(gate)
    }

    pub fn with_automatic_constants() -> GateRef<F> {
        let constants = Arc::new(gmimc_automatic_constants::<F, R>());
        Self::with_constants(constants)
    }

    /// The wire index for the `i`th input to the permutation.
    pub fn wire_input(i: usize) -> usize {
        i
    }

    /// The wire index for the `i`th output to the permutation.
    pub fn wire_output(i: usize) -> usize {
        W + i
    }

    /// Used to incrementally compute the index of the leaf based on a series of swap bits.
    pub const WIRE_INDEX_ACCUMULATOR_OLD: usize = 2 * W;
    pub const WIRE_INDEX_ACCUMULATOR_NEW: usize = 2 * W + 1;

    /// If this is set to 1, the first four inputs will be swapped with the next four inputs. This
    /// is useful for ordering hashes in Merkle proofs. Otherwise, this should be set to 0.
    pub const WIRE_SWAP: usize = 2 * W + 2;

    /// A wire which stores the input to the `i`th cubing.
    fn wire_cubing_input(i: usize) -> usize {
        2 * W + 3 + i
    }
}

impl<F: Field, const R: usize> Gate<F> for GMiMCGate<F, R> {
    fn id(&self) -> String {
        format!("<R={}> {:?}", R, self)
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F>) -> Vec<F> {
        let mut constraints = Vec::with_capacity(W + R);

        // Assert that `swap` is binary.
        let swap = vars.local_wires[Self::WIRE_SWAP];
        constraints.push(swap * (swap - F::ONE));

        let old_index_acc = vars.local_wires[Self::WIRE_INDEX_ACCUMULATOR_OLD];
        let new_index_acc = vars.local_wires[Self::WIRE_INDEX_ACCUMULATOR_NEW];
        let computed_new_index_acc = F::TWO * old_index_acc + swap;
        constraints.push(computed_new_index_acc - new_index_acc);

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

        for r in 0..R {
            let active = r % W;
            let cubing_input = state[active] + addition_buffer + self.constants[r];
            let cubing_input_wire = vars.local_wires[Self::wire_cubing_input(r)];
            constraints.push(cubing_input - cubing_input_wire);
            let f = cubing_input_wire.cube();
            addition_buffer += f;
            state[active] -= f;
        }

        for i in 0..W {
            state[i] += addition_buffer;
            constraints.push(state[i] - vars.local_wires[Self::wire_output(i)]);
        }

        constraints
    }

    fn eval_unfiltered_recursively(
        &self,
        builder: &mut CircuitBuilder<F>,
        vars: EvaluationTargets,
    ) -> Vec<Target> {
        unimplemented!()
    }

    fn generators(
        &self,
        gate_index: usize,
        _local_constants: &[F],
    ) -> Vec<Box<dyn WitnessGenerator<F>>> {
        let gen = GMiMCGenerator {
            gate_index,
            constants: self.constants.clone(),
        };
        vec![Box::new(gen)]
    }

    fn num_wires(&self) -> usize {
        W + 1 + R
    }

    fn num_constants(&self) -> usize {
        0
    }

    fn degree(&self) -> usize {
        3
    }

    fn num_constraints(&self) -> usize {
        R + W + 2
    }
}

#[derive(Debug)]
struct GMiMCGenerator<F: Field, const R: usize> {
    gate_index: usize,
    constants: Arc<[F; R]>,
}

impl<F: Field, const R: usize> SimpleGenerator<F> for GMiMCGenerator<F, R> {
    fn dependencies(&self) -> Vec<Target> {
        let mut dep_input_indices = Vec::with_capacity(W + 2);
        for i in 0..W {
            dep_input_indices.push(GMiMCGate::<F, R>::wire_input(i));
        }
        dep_input_indices.push(GMiMCGate::<F, R>::WIRE_SWAP);
        dep_input_indices.push(GMiMCGate::<F, R>::WIRE_INDEX_ACCUMULATOR_OLD);

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

    fn run_once(&self, witness: &PartialWitness<F>) -> PartialWitness<F> {
        let mut result = PartialWitness::new();

        let mut state = (0..W)
            .map(|i| {
                witness.get_wire(Wire {
                    gate: self.gate_index,
                    input: GMiMCGate::<F, R>::wire_input(i),
                })
            })
            .collect::<Vec<_>>();

        let swap_value = witness.get_wire(Wire {
            gate: self.gate_index,
            input: GMiMCGate::<F, R>::WIRE_SWAP,
        });
        debug_assert!(swap_value == F::ZERO || swap_value == F::ONE);
        if swap_value == F::ONE {
            for i in 0..4 {
                state.swap(i, 4 + i);
            }
        }

        // Update the index accumulator.
        let old_index_acc_value = witness.get_wire(Wire {
            gate: self.gate_index,
            input: GMiMCGate::<F, R>::WIRE_INDEX_ACCUMULATOR_OLD,
        });
        let new_index_acc_value = F::TWO * old_index_acc_value + swap_value;
        result.set_wire(
            Wire {
                gate: self.gate_index,
                input: GMiMCGate::<F, R>::WIRE_INDEX_ACCUMULATOR_NEW,
            },
            new_index_acc_value,
        );

        // Value that is implicitly added to each element.
        // See https://affine.group/2020/02/starkware-challenge
        let mut addition_buffer = F::ZERO;

        for r in 0..R {
            let active = r % W;
            let cubing_input = state[active] + addition_buffer + self.constants[r];
            result.set_wire(
                Wire {
                    gate: self.gate_index,
                    input: GMiMCGate::<F, R>::wire_cubing_input(r),
                },
                cubing_input,
            );
            let f = cubing_input.cube();
            addition_buffer += f;
            state[active] -= f;
        }

        for i in 0..W {
            state[i] += addition_buffer;
            result.set_wire(
                Wire {
                    gate: self.gate_index,
                    input: GMiMCGate::<F, R>::wire_output(i),
                },
                state[i],
            );
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use std::convert::TryInto;
    use std::sync::Arc;

    use crate::circuit_data::CircuitConfig;
    use crate::field::crandall_field::CrandallField;
    use crate::field::field::Field;
    use crate::gates::gmimc::{GMiMCGate, W};
    use crate::generator::generate_partial_witness;
    use crate::gmimc::gmimc_permute_naive;
    use crate::wire::Wire;
    use crate::witness::PartialWitness;

    #[test]
    fn generated_output() {
        type F = CrandallField;
        const R: usize = 101;
        let constants = Arc::new([F::TWO; R]);
        type Gate = GMiMCGate<F, R>;
        let gate = Gate::with_constants(constants.clone());

        let config = CircuitConfig {
            num_wires: 134,
            num_routed_wires: 200,
            ..Default::default()
        };

        let permutation_inputs = (0..W).map(F::from_canonical_usize).collect::<Vec<_>>();

        let mut witness = PartialWitness::new();
        witness.set_wire(
            Wire {
                gate: 0,
                input: Gate::WIRE_INDEX_ACCUMULATOR_OLD,
            },
            F::from_canonical_usize(7),
        );
        witness.set_wire(
            Wire {
                gate: 0,
                input: Gate::WIRE_SWAP,
            },
            F::ZERO,
        );
        for i in 0..W {
            witness.set_wire(
                Wire {
                    gate: 0,
                    input: Gate::wire_input(i),
                },
                permutation_inputs[i],
            );
        }

        let generators = gate.0.generators(0, &[]);
        generate_partial_witness(&mut witness, &generators);

        let expected_outputs: [F; W] =
            gmimc_permute_naive(permutation_inputs.try_into().unwrap(), constants);

        for i in 0..W {
            let out = witness.get_wire(Wire {
                gate: 0,
                input: Gate::wire_output(i),
            });
            assert_eq!(out, expected_outputs[i]);
        }

        let acc_new = witness.get_wire(Wire {
            gate: 0,
            input: Gate::WIRE_INDEX_ACCUMULATOR_NEW,
        });
        assert_eq!(acc_new, F::from_canonical_usize(7 * 2));
    }
}
