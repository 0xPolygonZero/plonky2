use std::convert::TryInto;
use std::marker::PhantomData;
use std::ops::Range;

use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::{Extendable, FieldExtension};
use crate::field::field::Field;
use crate::field::lagrange::interpolant;
use crate::gates::gate::{Gate, GateRef};
use crate::generator::{SimpleGenerator, WitnessGenerator};
use crate::polynomial::polynomial::PolynomialCoeffs;
use crate::target::Target;
use crate::vars::{EvaluationTargets, EvaluationVars};
use crate::wire::Wire;
use crate::witness::PartialWitness;

/// The size of the field extension, in terms of number of base elements per extension element.
const EXT_SIZE: usize = 4;

/// Evaluates the interpolant of some given elements from a quartic field extension.
///
/// In particular, this gate takes as inputs `num_points` points, `num_points` values, and the point
/// to evaluate the interpolant at. It computes the interpolant and outputs its evaluation at the
/// given point.
#[derive(Clone, Debug)]
pub(crate) struct QuarticInterpolationGate<F: Field + Extendable<D>, const D: usize> {
    num_points: usize,
    _phantom: PhantomData<F>,
}

impl<F: Field + Extendable<D>, const D: usize> QuarticInterpolationGate<F, D> {
    pub fn new(num_points: usize) -> GateRef<F> {
        let gate = Self {
            num_points,
            _phantom: PhantomData,
        };
        GateRef::new(gate)
    }

    fn start_points(&self) -> usize {
        0
    }

    /// Wire indices of the `i`th interpolant point.
    pub fn wire_point(&self, i: usize) -> usize {
        debug_assert!(i < self.num_points);
        self.start_points() + i
    }

    fn start_values(&self) -> usize {
        self.start_points() + self.num_points
    }

    /// Wire indices of the `i`th interpolant value.
    pub fn wires_value(&self, i: usize) -> Range<usize> {
        debug_assert!(i < self.num_points);
        let start = self.start_values() + i * EXT_SIZE;
        start..start + EXT_SIZE
    }

    fn start_evaluation_point(&self) -> usize {
        self.start_values() + self.num_points * EXT_SIZE
    }

    /// Wire indices of the point to evaluate the interpolant at.
    pub fn wires_evaluation_point(&self) -> Range<usize> {
        let start = self.start_evaluation_point();
        start..start + EXT_SIZE
    }

    fn start_evaluation_value(&self) -> usize {
        self.start_evaluation_point() + EXT_SIZE
    }

    /// Wire indices of the interpolated value.
    pub fn wires_evaluation_value(&self) -> Range<usize> {
        let start = self.start_evaluation_value();
        start..start + EXT_SIZE
    }

    fn start_coeffs(&self) -> usize {
        self.start_evaluation_value() + EXT_SIZE
    }

    /// Wire indices of the interpolant's `i`th coefficient.
    pub fn wires_coeff(&self, i: usize) -> Range<usize> {
        debug_assert!(i < self.num_points);
        let start = self.start_coeffs() + i * EXT_SIZE;
        start..start + EXT_SIZE
    }

    fn end(&self) -> usize {
        self.start_coeffs() + self.num_points * EXT_SIZE
    }
}

impl<F: Field + Extendable<D>, const D: usize> Gate<F> for QuarticInterpolationGate<F, D> {
    fn id(&self) -> String {
        let qfe_name = std::any::type_name::<F::Extension>();
        format!("{} {:?}", qfe_name, self)
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F>) -> Vec<F> {
        let lookup_fe = |wire_range: Range<usize>| {
            debug_assert_eq!(wire_range.len(), D);
            let arr = vars.local_wires[wire_range].try_into().unwrap();
            F::Extension::from_basefield_array(arr)
        };

        let mut constraints = Vec::with_capacity(self.num_constraints());

        let coeffs = (0..self.num_points)
            .map(|i| lookup_fe(self.wires_coeff(i)))
            .collect();
        let interpolant = PolynomialCoeffs::new(coeffs);
        let x_eval = lookup_fe(self.wires_evaluation_point());
        let x_eval_powers = x_eval.powers().take(self.num_points);

        // TODO

        constraints
    }

    fn eval_unfiltered_recursively(
        &self,
        builder: &mut CircuitBuilder<F>,
        vars: EvaluationTargets,
    ) -> Vec<Target> {
        todo!()
    }

    fn generators(
        &self,
        gate_index: usize,
        _local_constants: &[F],
    ) -> Vec<Box<dyn WitnessGenerator<F>>> {
        let gen = QuarticInterpolationGenerator::<F, D> {
            gate_index,
            gate: self.clone(),
            _phantom: PhantomData,
        };
        vec![Box::new(gen)]
    }

    fn num_wires(&self) -> usize {
        self.end()
    }

    fn num_constants(&self) -> usize {
        0
    }

    fn degree(&self) -> usize {
        self.num_points - 1
    }

    fn num_constraints(&self) -> usize {
        todo!()
    }
}

struct QuarticInterpolationGenerator<F: Field + Extendable<D>, const D: usize> {
    gate_index: usize,
    gate: QuarticInterpolationGate<F, D>,
    _phantom: PhantomData<F>,
}

impl<F: Field + Extendable<D>, const D: usize> SimpleGenerator<F>
    for QuarticInterpolationGenerator<F, D>
{
    fn dependencies(&self) -> Vec<Target> {
        todo!()
    }

    fn run_once(&self, witness: &PartialWitness<F>) -> PartialWitness<F> {
        let n = self.gate.num_points;

        let local_wire = |input| {
            Wire { gate: self.gate_index, input }
        };

        let lookup_fe = |wire_range: Range<usize>| {
            debug_assert_eq!(wire_range.len(), D);
            let values = wire_range
                .map(|input| {
                    witness.get_wire(local_wire(input))
                })
                .collect::<Vec<_>>();
            let arr = values.try_into().unwrap();
            F::Extension::from_basefield_array(arr)
        };

        // Compute the interpolant.
        let points = (0..n)
            .map(|i| {
                (
                    F::Extension::from_basefield(witness.get_wire(Wire {
                        gate: self.gate_index,
                        input: self.gate.wire_point(i),
                    })),
                    lookup_fe(self.gate.wires_value(i)),
                )
            })
            .collect::<Vec<_>>();
        let interpolant = interpolant(&points);

        let mut result = PartialWitness::<F>::new();
        for (i, &coeff) in interpolant.coeffs.iter().enumerate() {
            let wire_range = self.gate.wires_coeff(i);
            let wires = wire_range.map(|i| local_wire(i)).collect::<Vec<_>>();
            result.set_ext_wires(&wires, coeff);
        }

        todo!()
    }
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use crate::field::crandall_field::CrandallField;
    use crate::gates::gate::Gate;
    use crate::gates::interpolation_quartic::QuarticInterpolationGate;

    #[test]
    fn wire_indices_2_points() {
        let gate = QuarticInterpolationGate::<CrandallField, 4> {
            num_points: 2,
            _phantom: PhantomData,
        };
        // The exact indices aren't really important, but we want to make sure we don't have any
        // overlaps or gaps.
        assert_eq!(gate.wire_point(0), 0);
        assert_eq!(gate.wire_point(1), 1);
        assert_eq!(gate.wires_value(0), 2..6);
        assert_eq!(gate.wires_value(1), 6..10);
        assert_eq!(gate.wires_evaluation_point(), 10..14);
        assert_eq!(gate.wires_evaluation_value(), 14..18);
        assert_eq!(gate.wires_coeff(0), 18..22);
        assert_eq!(gate.wires_coeff(1), 22..26);
        assert_eq!(gate.num_wires(), 26);
    }

    #[test]
    fn wire_indices_4_points() {
        let gate = QuarticInterpolationGate::<CrandallField, 4> {
            num_points: 4,
            _phantom: PhantomData,
        };
        assert_eq!(gate.num_wires(), 44);
    }
}
