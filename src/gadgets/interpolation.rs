use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::gates::interpolation::InterpolationGate;
use crate::target::Target;
use std::marker::PhantomData;

impl<F: Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    /// Interpolate two points. No need for an `InterpolationGate` since the coefficients
    /// of the linear interpolation polynomial can be easily computed with arithmetic operations.
    pub fn interpolate2(
        &mut self,
        interpolation_points: [(ExtensionTarget<D>, ExtensionTarget<D>); 2],
        evaluation_point: ExtensionTarget<D>,
    ) -> ExtensionTarget<D> {
        // a0 -> a1
        // b0 -> b1
        // x  -> a1 + (x-a0)*(b1-a1)/(b0-a0)

        let x_m_a0 = self.sub_extension(evaluation_point, interpolation_points[0].0);
        let b1_m_a1 = self.sub_extension(interpolation_points[1].1, interpolation_points[0].1);
        let b0_m_a0 = self.sub_extension(interpolation_points[1].0, interpolation_points[0].0);
        let quotient = self.div_unsafe_extension(b1_m_a1, b0_m_a0);

        self.mul_add_extension(x_m_a0, quotient, interpolation_points[0].1)
    }

    /// Interpolate a list of point/evaluation pairs at a given point.
    /// Returns the evaluation of the interpolated polynomial at `evaluation_point`.
    pub fn interpolate(
        &mut self,
        interpolation_points: &[(Target, ExtensionTarget<D>)],
        evaluation_point: ExtensionTarget<D>,
    ) -> ExtensionTarget<D> {
        let gate = InterpolationGate::<F, D> {
            num_points: interpolation_points.len(),
            _phantom: PhantomData,
        };
        let gate_index =
            self.add_gate_no_constants(InterpolationGate::new(interpolation_points.len()));
        for (i, &(p, v)) in interpolation_points.iter().enumerate() {
            self.route(p, Target::wire(gate_index, gate.wire_point(i)));
            self.route_extension(
                v,
                ExtensionTarget::from_range(gate_index, gate.wires_value(i)),
            );
        }
        self.route_extension(
            evaluation_point,
            ExtensionTarget::from_range(gate_index, gate.wires_evaluation_point()),
        );

        ExtensionTarget::from_range(gate_index, gate.wires_evaluation_value())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit_data::CircuitConfig;
    use crate::field::crandall_field::CrandallField;
    use crate::field::extension_field::quartic::QuarticCrandallField;
    use crate::field::extension_field::FieldExtension;
    use crate::field::field::Field;
    use crate::field::lagrange::{interpolant, interpolate};
    use crate::witness::PartialWitness;
    use std::convert::TryInto;

    #[test]
    fn test_interpolate() {
        type F = CrandallField;
        type FF = QuarticCrandallField;
        let config = CircuitConfig {
            num_routed_wires: 18,
            ..CircuitConfig::large_config()
        };
        let mut builder = CircuitBuilder::<F, 4>::new(config);

        let len = 2;
        let points = (0..len)
            .map(|_| (F::rand(), FF::rand()))
            .collect::<Vec<_>>();

        let homogeneous_points = points
            .iter()
            .map(|&(a, b)| (<FF as FieldExtension<4>>::from_basefield(a), b))
            .collect::<Vec<_>>();

        let true_interpolant = interpolant(&homogeneous_points);

        let z = FF::rand();
        let true_eval = true_interpolant.eval(z);

        let points_target = points
            .iter()
            .map(|&(p, v)| (builder.constant(p), builder.constant_extension(v)))
            .collect::<Vec<_>>();

        let zt = builder.constant_extension(z);

        let eval = builder.interpolate(&points_target, zt);
        let true_eval_target = builder.constant_extension(true_eval);
        builder.assert_equal_extension(eval, true_eval_target);

        let data = builder.build();
        let proof = data.prove(PartialWitness::new());
    }

    #[test]
    fn test_interpolate2() {
        type F = CrandallField;
        type FF = QuarticCrandallField;
        let config = CircuitConfig::large_config();
        let mut builder = CircuitBuilder::<F, 4>::new(config);

        let len = 2;
        let points = (0..len)
            .map(|_| (FF::rand(), FF::rand()))
            .collect::<Vec<_>>();

        let true_interpolant = interpolant(&points);

        let z = FF::rand();
        let true_eval = true_interpolant.eval(z);

        let points_target = points
            .iter()
            .map(|&(p, v)| (builder.constant_extension(p), builder.constant_extension(v)))
            .collect::<Vec<_>>();

        let zt = builder.constant_extension(z);

        let eval = builder.interpolate2(points_target.try_into().unwrap(), zt);
        let true_eval_target = builder.constant_extension(true_eval);
        builder.assert_equal_extension(eval, true_eval_target);

        let data = builder.build();
        let proof = data.prove(PartialWitness::new());
    }
}
