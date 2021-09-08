use std::convert::TryInto;
use std::marker::PhantomData;
use std::ops::Range;

use crate::field::extension_field::algebra::PolynomialCoeffsAlgebra;
use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::{Extendable, FieldExtension};
use crate::field::field_types::RichField;
use crate::field::interpolation::interpolant;
use crate::gadgets::polynomial::PolynomialCoeffsExtAlgebraTarget;
use crate::gates::gate::Gate;
use crate::iop::generator::{GeneratedValues, SimpleGenerator, WitnessGenerator};
use crate::iop::target::Target;
use crate::iop::wire::Wire;
use crate::iop::witness::{PartitionWitness, Witness};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::vars::{EvaluationTargets, EvaluationVars, EvaluationVarsBase};
use crate::polynomial::polynomial::PolynomialCoeffs;

/// Evaluates the interpolant of some given elements from a field extension.
///
/// In particular, this gate takes as inputs `num_points` points, `num_points` values, and the point
/// to evaluate the interpolant at. It computes the interpolant and outputs its evaluation at the
/// given point.
#[derive(Clone, Debug)]
pub(crate) struct InterpolationGate<F: RichField + Extendable<D>, const D: usize> {
    pub num_points: usize,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> InterpolationGate<F, D> {
    pub fn new(num_points: usize) -> Self {
        Self {
            num_points,
            _phantom: PhantomData,
        }
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
        let start = self.start_values() + i * D;
        start..start + D
    }

    fn start_evaluation_point(&self) -> usize {
        self.start_values() + self.num_points * D
    }

    /// Wire indices of the point to evaluate the interpolant at.
    pub fn wires_evaluation_point(&self) -> Range<usize> {
        let start = self.start_evaluation_point();
        start..start + D
    }

    fn start_evaluation_value(&self) -> usize {
        self.start_evaluation_point() + D
    }

    /// Wire indices of the interpolated value.
    pub fn wires_evaluation_value(&self) -> Range<usize> {
        let start = self.start_evaluation_value();
        start..start + D
    }

    fn start_coeffs(&self) -> usize {
        self.start_evaluation_value() + D
    }

    /// Wire indices of the interpolant's `i`th coefficient.
    pub fn wires_coeff(&self, i: usize) -> Range<usize> {
        debug_assert!(i < self.num_points);
        let start = self.start_coeffs() + i * D;
        start..start + D
    }

    /// End of wire indices, exclusive.
    fn end(&self) -> usize {
        self.start_coeffs() + self.num_points * D
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Gate<F, D> for InterpolationGate<F, D> {
    fn id(&self) -> String {
        format!("{:?}<D={}>", self, D)
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        let mut constraints = Vec::with_capacity(self.num_constraints());

        let coeffs = (0..self.num_points)
            .map(|i| vars.get_local_ext_algebra(self.wires_coeff(i)))
            .collect();
        let interpolant = PolynomialCoeffsAlgebra::new(coeffs);

        for i in 0..self.num_points {
            let point = vars.local_wires[self.wire_point(i)];
            let value = vars.get_local_ext_algebra(self.wires_value(i));
            let computed_value = interpolant.eval_base(point);
            constraints.extend(&(value - computed_value).to_basefield_array());
        }

        let evaluation_point = vars.get_local_ext_algebra(self.wires_evaluation_point());
        let evaluation_value = vars.get_local_ext_algebra(self.wires_evaluation_value());
        let computed_evaluation_value = interpolant.eval(evaluation_point);
        constraints.extend(&(evaluation_value - computed_evaluation_value).to_basefield_array());

        constraints
    }

    fn eval_unfiltered_base(&self, vars: EvaluationVarsBase<F>) -> Vec<F> {
        let mut constraints = Vec::with_capacity(self.num_constraints());

        let coeffs = (0..self.num_points)
            .map(|i| vars.get_local_ext(self.wires_coeff(i)))
            .collect();
        let interpolant = PolynomialCoeffs::new(coeffs);

        for i in 0..self.num_points {
            let point = vars.local_wires[self.wire_point(i)];
            let value = vars.get_local_ext(self.wires_value(i));
            let computed_value = interpolant.eval_base(point);
            constraints.extend(&(value - computed_value).to_basefield_array());
        }

        let evaluation_point = vars.get_local_ext(self.wires_evaluation_point());
        let evaluation_value = vars.get_local_ext(self.wires_evaluation_value());
        let computed_evaluation_value = interpolant.eval(evaluation_point);
        constraints.extend(&(evaluation_value - computed_evaluation_value).to_basefield_array());

        constraints
    }

    fn eval_unfiltered_recursively(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>> {
        let mut constraints = Vec::with_capacity(self.num_constraints());

        let coeffs = (0..self.num_points)
            .map(|i| vars.get_local_ext_algebra(self.wires_coeff(i)))
            .collect();
        let interpolant = PolynomialCoeffsExtAlgebraTarget(coeffs);

        for i in 0..self.num_points {
            let point = vars.local_wires[self.wire_point(i)];
            let value = vars.get_local_ext_algebra(self.wires_value(i));
            let computed_value = interpolant.eval_scalar(builder, point);
            constraints.extend(
                &builder
                    .sub_ext_algebra(value, computed_value)
                    .to_ext_target_array(),
            );
        }

        let evaluation_point = vars.get_local_ext_algebra(self.wires_evaluation_point());
        let evaluation_value = vars.get_local_ext_algebra(self.wires_evaluation_value());
        let computed_evaluation_value = interpolant.eval(builder, evaluation_point);
        constraints.extend(
            &builder
                .sub_ext_algebra(evaluation_value, computed_evaluation_value)
                .to_ext_target_array(),
        );

        constraints
    }

    fn generators(
        &self,
        gate_index: usize,
        _local_constants: &[F],
    ) -> Vec<Box<dyn WitnessGenerator<F>>> {
        let gen = InterpolationGenerator::<F, D> {
            gate_index,
            gate: self.clone(),
            _phantom: PhantomData,
        };
        vec![Box::new(gen.adapter())]
    }

    fn num_wires(&self) -> usize {
        self.end()
    }

    fn num_constants(&self) -> usize {
        0
    }

    fn degree(&self) -> usize {
        // The highest power of x is `num_points - 1`, and then multiplication by the coefficient
        // adds 1.
        self.num_points
    }

    fn num_constraints(&self) -> usize {
        // num_points * D constraints to check for consistency between the coefficients and the
        // point-value pairs, plus D constraints for the evaluation value.
        self.num_points * D + D
    }
}

#[derive(Debug)]
struct InterpolationGenerator<F: RichField + Extendable<D>, const D: usize> {
    gate_index: usize,
    gate: InterpolationGate<F, D>,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> SimpleGenerator<F>
    for InterpolationGenerator<F, D>
{
    fn dependencies(&self) -> Vec<Target> {
        let local_target = |input| {
            Target::Wire(Wire {
                gate: self.gate_index,
                input,
            })
        };

        let local_targets = |inputs: Range<usize>| inputs.map(local_target);

        let mut deps = Vec::new();
        deps.extend(local_targets(self.gate.wires_evaluation_point()));
        for i in 0..self.gate.num_points {
            deps.push(local_target(self.gate.wire_point(i)));
            deps.extend(local_targets(self.gate.wires_value(i)));
        }
        deps
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let n = self.gate.num_points;

        let local_wire = |input| Wire {
            gate: self.gate_index,
            input,
        };

        let get_local_wire = |input| witness.get_wire(local_wire(input));

        let get_local_ext = |wire_range: Range<usize>| {
            debug_assert_eq!(wire_range.len(), D);
            let values = wire_range.map(get_local_wire).collect::<Vec<_>>();
            let arr = values.try_into().unwrap();
            F::Extension::from_basefield_array(arr)
        };

        // Compute the interpolant.
        let points = (0..n)
            .map(|i| {
                (
                    F::Extension::from_basefield(get_local_wire(self.gate.wire_point(i))),
                    get_local_ext(self.gate.wires_value(i)),
                )
            })
            .collect::<Vec<_>>();
        let interpolant = interpolant(&points);

        for (i, &coeff) in interpolant.coeffs.iter().enumerate() {
            let wires = self.gate.wires_coeff(i).map(local_wire);
            out_buffer.set_ext_wires(wires, coeff);
        }

        let evaluation_point = get_local_ext(self.gate.wires_evaluation_point());
        let evaluation_value = interpolant.eval(evaluation_point);
        let evaluation_value_wires = self.gate.wires_evaluation_value().map(local_wire);
        out_buffer.set_ext_wires(evaluation_value_wires, evaluation_value);
    }
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use anyhow::Result;

    use crate::field::crandall_field::CrandallField;
    use crate::field::extension_field::quartic::QuarticCrandallField;
    use crate::field::field_types::Field;
    use crate::gates::gate::Gate;
    use crate::gates::gate_testing::{test_eval_fns, test_low_degree};
    use crate::gates::interpolation::InterpolationGate;
    use crate::hash::hash_types::HashOut;
    use crate::plonk::vars::EvaluationVars;
    use crate::polynomial::polynomial::PolynomialCoeffs;

    #[test]
    fn wire_indices() {
        let gate = InterpolationGate::<CrandallField, 4> {
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
    fn low_degree() {
        test_low_degree::<CrandallField, _, 4>(InterpolationGate::new(4));
    }

    #[test]
    fn eval_fns() -> Result<()> {
        test_eval_fns::<CrandallField, _, 4>(InterpolationGate::new(4))
    }

    #[test]
    fn test_gate_constraint() {
        type F = CrandallField;
        type FF = QuarticCrandallField;
        const D: usize = 4;

        /// Returns the local wires for an interpolation gate for given coeffs, points and eval point.
        fn get_wires(
            num_points: usize,
            coeffs: PolynomialCoeffs<FF>,
            points: Vec<F>,
            eval_point: FF,
        ) -> Vec<FF> {
            let mut v = Vec::new();
            v.extend_from_slice(&points);
            for j in 0..num_points {
                v.extend(coeffs.eval(points[j].into()).0);
            }
            v.extend(eval_point.0);
            v.extend(coeffs.eval(eval_point).0);
            for i in 0..coeffs.len() {
                v.extend(coeffs.coeffs[i].0);
            }
            v.iter().map(|&x| x.into()).collect::<Vec<_>>()
        }

        // Get a working row for InterpolationGate.
        let coeffs = PolynomialCoeffs::new(vec![FF::rand(), FF::rand()]);
        let points = vec![F::rand(), F::rand()];
        let eval_point = FF::rand();
        let gate = InterpolationGate::<F, D> {
            num_points: 2,
            _phantom: PhantomData,
        };
        let vars = EvaluationVars {
            local_constants: &[],
            local_wires: &get_wires(2, coeffs, points, eval_point),
            public_inputs_hash: &HashOut::rand(),
        };

        assert!(
            gate.eval_unfiltered(vars).iter().all(|x| x.is_zero()),
            "Gate constraints are not satisfied."
        );
    }
}
