use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::gates::interpolation::InterpolationGate;
use crate::iop::target::Target;
use crate::plonk::circuit_builder::CircuitBuilder;

impl<F: Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    /// Interpolate a list of point/evaluation pairs at a given point.
    /// Returns the evaluation of the interpolated polynomial at `evaluation_point`.
    pub fn interpolate(
        &mut self,
        interpolation_points: &[(Target, ExtensionTarget<D>)],
        evaluation_point: ExtensionTarget<D>,
    ) -> ExtensionTarget<D> {
        let gate = InterpolationGate::new(interpolation_points.len());
        let gate_index = self.add_gate(gate.clone(), vec![]);
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
    use anyhow::Result;

    use crate::field::crandall_field::CrandallField;
    use crate::field::extension_field::quartic::QuarticCrandallField;
    use crate::field::extension_field::FieldExtension;
    use crate::field::field_types::Field;
    use crate::field::interpolation::interpolant;
    use crate::iop::witness::PartialWitness;
    use crate::plonk::circuit_builder::CircuitBuilder;
    use crate::plonk::circuit_data::CircuitConfig;
    use crate::plonk::verifier::verify;

    #[test]
    fn test_interpolate() -> Result<()> {
        type F = CrandallField;
        type FF = QuarticCrandallField;
        let config = CircuitConfig::large_config();
        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, 4>::new(config);

        let len = 4;
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
        let proof = data.prove(pw)?;

        verify(proof, &data.verifier_only, &data.common)
    }
}
