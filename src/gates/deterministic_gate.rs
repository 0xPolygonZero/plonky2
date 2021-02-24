use std::marker::PhantomData;

use crate::circuit_data::CircuitConfig;
use crate::constraint_polynomial::{ConstraintPolynomial, EvaluationVars};
use crate::field::field::Field;
use crate::gates::gate::Gate;
use crate::generator::{SimpleGenerator, WitnessGenerator2};
use crate::target::Target2;
use crate::wire::Wire;
use crate::witness::PartialWitness;

/// A deterministic gate. Each entry in `outputs()` describes how that output is evaluated; this is
/// used to create both the constraint set and the generator set.
///
/// `DeterministicGate`s do not automatically implement `Gate`; they should instead be wrapped in
/// `DeterministicGateAdapter`.
pub trait DeterministicGate<F: Field>: 'static {
    /// A unique identifier for this gate.
    fn id(&self) -> String;

    /// A vector of `(i, c)` pairs, where `i` is the index of an output and `c` is the polynomial
    /// defining how that output is evaluated.
    fn outputs(&self, config: CircuitConfig) -> Vec<(usize, ConstraintPolynomial<F>)>;

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
    ) -> Vec<Box<dyn WitnessGenerator2<F>>> {
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
        self.gate.outputs(config).into_iter()
            .map(|(i, out)| out - ConstraintPolynomial::local_wire_value(i))
            .chain(self.gate.additional_constraints(config).into_iter())
            .collect()
    }

    fn generators(
        &self,
        config: CircuitConfig,
        gate_index: usize,
        local_constants: Vec<F>,
        next_constants: Vec<F>,
    ) -> Vec<Box<dyn WitnessGenerator2<F>>> {
        self.gate.outputs(config)
            .into_iter()
            .map(|(input_index, out)| {
                let og = OutputGenerator {
                    gate_index,
                    input_index,
                    out,
                    local_constants: local_constants.clone(),
                    next_constants: next_constants.clone(),
                };

                // We need the type system to treat this as a boxed `WitnessGenerator2<F>`, rather
                // than a boxed `OutputGenerator<F>`.
                let b: Box::<dyn WitnessGenerator2<F>> = Box::new(og);
                b
            })
            .chain(self.gate.additional_generators(config, gate_index))
            .collect()
    }
}

struct OutputGenerator<F: Field> {
    gate_index: usize,
    input_index: usize,
    out: ConstraintPolynomial<F>,
    local_constants: Vec<F>,
    next_constants: Vec<F>,
}

impl<F: Field> SimpleGenerator<F> for OutputGenerator<F> {
    fn dependencies(&self) -> Vec<Target2> {
        self.out.dependencies(self.gate_index)
            .into_iter()
            .map(Target2::Wire)
            .collect()
    }

    fn run_once(&mut self, witness: &PartialWitness<F>) -> PartialWitness<F> {
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
            let local_value = witness.try_get_target(Target2::Wire(local_wire)).unwrap_or(F::ZERO);
            let next_value = witness.try_get_target(Target2::Wire(next_wire)).unwrap_or(F::ZERO);

            local_wire_values.push(local_value);
            next_wire_values.push(next_value);
        }

        let vars = EvaluationVars {
            local_constants: &self.local_constants,
            next_constants: &self.next_constants,
            local_wire_values: &local_wire_values,
            next_wire_values: &next_wire_values,
        };

        let result_wire = Wire { gate: self.gate_index, input: self.input_index };
        let result_value = self.out.evaluate(vars);
        PartialWitness::singleton(Target2::Wire(result_wire), result_value)
    }
}
