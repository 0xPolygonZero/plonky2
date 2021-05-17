use std::marker::PhantomData;

use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::quartic::QuarticFieldExtension;
use crate::gates::gate::{Gate, GateRef};
use crate::generator::{SimpleGenerator, WitnessGenerator};
use crate::target::Target;
use crate::vars::{EvaluationTargets, EvaluationVars};
use crate::witness::PartialWitness;

/// The size of the field extension, in terms of number of base elements per extension element.
const EXT_SIZE: usize = 4;

/// Evaluates the interpolant of some given elements from a quartic field extension.
///
/// In particular, this gate takes as inputs `num_points` points, `num_points` values, and the point
/// to evaluate the interpolant at. It computes the interpolant and outputs its evaluation at the
/// given point.
#[derive(Debug)]
pub(crate) struct QuarticInterpolationGate<QFE: QuarticFieldExtension> {
    num_points: usize,
    _phantom: PhantomData<QFE>,
}

impl<QFE: QuarticFieldExtension> QuarticInterpolationGate<QFE> {
    pub fn new(num_points: usize) -> GateRef<QFE::BaseField> {
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
    pub fn wires_value(&self, i: usize) -> Vec<usize> {
        debug_assert!(i < self.num_points);
        (0..EXT_SIZE)
            .map(|j| self.start_values() + i * EXT_SIZE + j)
            .collect()
    }

    fn start_interpolated_point(&self) -> usize {
        self.start_values() + self.num_points * EXT_SIZE
    }

    /// Wire indices of the point to evaluate the interpolant at.
    pub fn wires_interpolated_point(&self) -> Vec<usize> {
        (0..EXT_SIZE).map(|j| self.start_interpolated_point() + j).collect()
    }

    fn start_interpolated_value(&self) -> usize {
        self.start_interpolated_point() + EXT_SIZE
    }

    /// Wire indices of the interpolated value.
    pub fn wires_interpolated_value(&self) -> Vec<usize> {
        (0..EXT_SIZE)
            .map(|j| self.start_interpolated_value() + j)
            .collect()
    }

    fn start_coeffs(&self) -> usize {
        self.start_interpolated_value() + EXT_SIZE
    }

    /// Wire indices of the interpolant's `i`th coefficient.
    pub fn wires_coeff(&self, i: usize) -> Vec<usize> {
        debug_assert!(i < self.num_points);
        (0..EXT_SIZE)
            .map(|j| self.start_coeffs() + i * EXT_SIZE + j)
            .collect()
    }
}

impl<QFE: QuarticFieldExtension> Gate<QFE::BaseField> for QuarticInterpolationGate<QFE> {
    fn id(&self) -> String {
        let qfe_name = std::any::type_name::<QFE>();
        format!("{} {:?}", qfe_name, self)
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<QFE::BaseField>) -> Vec<QFE::BaseField> {
        todo!()
    }

    fn eval_unfiltered_recursively(
        &self,
        builder: &mut CircuitBuilder<QFE::BaseField>,
        vars: EvaluationTargets,
    ) -> Vec<Target> {
        todo!()
    }

    fn generators(
        &self,
        gate_index: usize,
        local_constants: &[QFE::BaseField],
    ) -> Vec<Box<dyn WitnessGenerator<QFE::BaseField>>> {
        todo!()
    }

    fn num_wires(&self) -> usize {
        todo!()
    }

    fn num_constants(&self) -> usize {
        todo!()
    }

    fn degree(&self) -> usize {
        todo!()
    }

    fn num_constraints(&self) -> usize {
        todo!()
    }
}

struct QuarticInterpolationGenerator<QFE: QuarticFieldExtension> {
    _phantom: PhantomData<QFE>,
}

impl<QFE: QuarticFieldExtension> SimpleGenerator<QFE::BaseField>
    for QuarticInterpolationGenerator<QFE>
{
    fn dependencies(&self) -> Vec<Target> {
        todo!()
    }

    fn run_once(&self, witness: &PartialWitness<QFE::BaseField>) -> PartialWitness<QFE::BaseField> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use crate::gates::interpolation_quartic::QuarticInterpolationGate;
    use crate::field::extension_field::quartic::QuarticCrandallField;

    #[test]
    fn wire_indices() {
        let gate = QuarticInterpolationGate::<QuarticCrandallField> { num_points: 2, _phantom: PhantomData };
        // The exact indices aren't really important, but we want to make sure we don't have any
        // overlaps or gaps.
        assert_eq!(gate.wire_point(0), 0);
        assert_eq!(gate.wire_point(1), 1);
        assert_eq!(gate.wires_value(0), vec![2, 3, 4, 5]);
        assert_eq!(gate.wires_value(1), vec![6, 7, 8, 9]);
        assert_eq!(gate.wires_interpolated_point(), vec![10, 11, 12, 13]);
        assert_eq!(gate.wires_interpolated_value(), vec![14, 15, 16, 17]);
        assert_eq!(gate.wires_coeff(0), vec![18, 19, 20, 21]);
        assert_eq!(gate.wires_coeff(1), vec![22, 23, 24, 25]);
    }
}
