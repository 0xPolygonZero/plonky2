use std::marker::PhantomData;

use crate::circuit_data::CircuitConfig;
use crate::constraint_polynomial::{ConstraintPolynomial, EvaluationVars};
use crate::field::field::Field;
use crate::gates::gate::Gate;
use crate::generator::{SimpleGenerator, WitnessGenerator};
use crate::target::Target;
use crate::wire::Wire;
use crate::witness::PartialWitness;
use crate::gates::output_graph::{OutputGraph, GateOutputLocation};

/// A deterministic gate. Each entry in `outputs()` describes how that output is evaluated; this is
/// used to create both the constraint set and the generator set.
///
/// `DeterministicGate`s do not automatically implement `Gate`; they should instead be wrapped in
/// `DeterministicGateAdapter`.
pub trait DeterministicGate<F: Field>: 'static {
    /// A unique identifier for this gate.
    fn id(&self) -> String;

    /// A vector of `(loc, out)` pairs, where `loc` is the location of an output and `out` is a
    /// polynomial defining how that output is evaluated.
    fn outputs(&self, config: CircuitConfig) -> OutputGraph<F>;

    /// Any additional constraints to be enforced, besides the (automatically provided) ones that
    /// constraint output values.
    fn additional_constraints(&self, _config: CircuitConfig) -> Vec<ConstraintPolynomial<F>> {
        Vec::new()
    }

    /// Any additional generators, besides the (automatically provided) ones that generate output
    /// values.
    fn additional_generators(
        &self,
        _config: CircuitConfig,
        _gate_index: usize,
        local_constants: Vec<F>,
        next_constants: Vec<F>,
    ) -> Vec<Box<dyn WitnessGenerator<F>>> {
        Vec::new()
    }
}

/// A wrapper around `DeterministicGate` which implements `Gate`. Note that a blanket implementation
/// is not possible in this context given Rust's coherence rules.
pub struct DeterministicGateAdapter<F: Field, DG: DeterministicGate<F> + ?Sized> {
    gate: Box<DG>,
    _phantom: PhantomData<F>,
}

impl<F: Field, DG: DeterministicGate<F>> DeterministicGateAdapter<F, DG> {
    pub fn new(gate: DG) -> Self {
        Self { gate: Box::new(gate), _phantom: PhantomData }
    }
}

impl<F: Field, DG: DeterministicGate<F>> Gate<F> for DeterministicGateAdapter<F, DG> {
    fn id(&self) -> String {
        self.gate.id()
    }

    fn constraints(&self, config: CircuitConfig) -> Vec<ConstraintPolynomial<F>> {
        // For each output, we add a constraint of the form `out - expression = 0`,
        // then we append any additional constraints that the gate defines.
        self.gate.outputs(config).outputs.into_iter()
            .map(|(output_loc, out)| out - ConstraintPolynomial::from_gate_output(output_loc))
            .chain(self.gate.additional_constraints(config).into_iter())
            .collect()
    }

    fn generators(
        &self,
        config: CircuitConfig,
        gate_index: usize,
        local_constants: Vec<F>,
        next_constants: Vec<F>,
    ) -> Vec<Box<dyn WitnessGenerator<F>>> {
        self.gate.outputs(config).outputs
            .into_iter()
            .map(|(location, out)| {
                let og = OutputGenerator {
                    gate_index,
                    location,
                    out,
                    local_constants: local_constants.clone(),
                    next_constants: next_constants.clone(),
                };

                // We need the type system to treat this as a boxed `WitnessGenerator2<F>`, rather
                // than a boxed `OutputGenerator<F>`.
                let b: Box::<dyn WitnessGenerator<F>> = Box::new(og);
                b
            })
            .chain(self.gate.additional_generators(
                config, gate_index, local_constants.clone(), next_constants.clone()))
            .collect()
    }
}

struct OutputGenerator<F: Field> {
    gate_index: usize,
    location: GateOutputLocation,
    out: ConstraintPolynomial<F>,
    local_constants: Vec<F>,
    next_constants: Vec<F>,
}

impl<F: Field> SimpleGenerator<F> for OutputGenerator<F> {
    fn dependencies(&self) -> Vec<Target> {
        self.out.dependencies(self.gate_index)
            .into_iter()
            .map(Target::Wire)
            .collect()
    }

    fn run_once(&self, witness: &PartialWitness<F>) -> PartialWitness<F> {
        let mut local_wire_values = Vec::new();
        let mut next_wire_values = Vec::new();

        // Get an exclusive upper bound on the largest input index in this constraint.
        let input_limit_exclusive = self.out.max_wire_input_index()
            .map_or(0, |i| i + 1);

        for input in 0..input_limit_exclusive {
            let local_wire = Wire { gate: self.gate_index, input };
            let next_wire = Wire { gate: self.gate_index + 1, input };

            // Lookup the values if they exist. If not, we can just insert a zero, knowing
            // that it will not be used. (If it was used, it would have been included in our
            // dependencies, and this generator would not have run yet.)
            let local_value = witness.try_get_target(Target::Wire(local_wire)).unwrap_or(F::ZERO);
            let next_value = witness.try_get_target(Target::Wire(next_wire)).unwrap_or(F::ZERO);

            local_wire_values.push(local_value);
            next_wire_values.push(next_value);
        }

        let vars = EvaluationVars {
            local_constants: &self.local_constants,
            next_constants: &self.next_constants,
            local_wires: &local_wire_values,
            next_wires: &next_wire_values,
        };

        let result_wire = match self.location {
            GateOutputLocation::LocalWire(input) =>
                Wire { gate: self.gate_index, input },
            GateOutputLocation::NextWire(input) =>
                Wire { gate: self.gate_index + 1, input },
        };

        let result_value = self.out.evaluate(vars);
        PartialWitness::singleton(Target::Wire(result_wire), result_value)
    }
}
